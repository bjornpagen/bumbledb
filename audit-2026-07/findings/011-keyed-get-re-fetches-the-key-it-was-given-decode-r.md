## Keyed get re-fetches the key it was given: decode resolves determinant string fields through the reverse dictionary

category: missing-free-feature | severity: high | verdict: CONFIRMED | finder: perf:points

### Summary

On a keyed-get hit, the full-fact decode resolves every string field through the interning dictionary's reverse map — an extra LMDB B-tree descent per string field — with no carve-out for the fields of the key statement's own projection, whose values the caller just supplied. The caller's bytes and the stored bytes are byte-identical by the system's own accepted collision axiom, so the descent re-derives information already in hand. For the p5 bench lane (`Doc(key) -> Doc`, where the key is the relation's only `str` field) this is the fourth of four descents per hit, in the single lane on the board where the SQLite ratio is 1.00.

### Evidence

All citations verified against the code directly.

- `crates/bumbledb/src/api/db/snapshot.rs:200-204` — `Snapshot::get_dyn`'s hit path:
  ```rust
  crate::encoding::decode_values(fact, rel.layout(), |id| {
      Ok(Box::from(dict::resolve(&self.txn, id)?))
  })
  ```
  The resolver closure runs for every String field; `decode_values` (`crates/bumbledb/src/encoding/decode.rs:235-260`) calls it unconditionally on every `ValueRef::String(id)` with no knowledge of the statement projection.
- Descent count per hit, each a direct LMDB `get` I read in the code:
  1. `dict::lookup_str` — forward `_dict` descent (`crates/bumbledb/src/storage/dict.rs:97-104`), invoked from `encode_determinant_with` (`crates/bumbledb/src/api/db/get.rs:56-101`);
  2. `read::determinant_row` — `U` index descent (`crates/bumbledb/src/storage/read/determinant_row.rs:16-25`);
  3. `read::fetch` — `F` fact descent (`crates/bumbledb/src/storage/read/fetch.rs:18-37`);
  4. `dict::resolve` — reverse `_dict` descent (`crates/bumbledb/src/storage/dict.rs:158-163`) plus a `Box::from` copy at the snapshot.rs call site.
  The SQLite twin (`crates/bumbledb-bench/src/translate.rs:115` — `SELECT <all columns> WHERE key = ?`) is a UNIQUE-index seek + rowid lookup: 2 descents.
- Soundness of eliding descent 4: the forward hit fixed `id = forward[blake3(supplied)]`; the `U` probe matched the determinant byte-for-byte against what `keys::determinant_image` slices from the stored fact, so the fact's key-field word IS that `id`; the reverse entry for `id` was written under the same hash. `docs/architecture/10-data-model.md:479` — "**hash equality is treated as fact equality — collisions are an accepted axiom**" — makes `resolve(id)` byte-identical to the supplied string. (Note: the finding cited the axiom comment at `dict.rs:53-56`; that comment sits inside the `#[cfg(test)]` `intern_str` — the normative statement is the data-model doc. Substance unchanged.)
- Scenario and numbers: `crates/bumbledb-bench/src/scenarios/points.rs:25-31` — `Doc`'s only `str` field is `key`; p5 registers `Surface::KeyedGet` through `Doc(key) -> Doc` (points.rs:317-327). `bench-out/night-2026-07-20/scenarios/scenarios.json`: p5 ours p50 1416ns vs SQLite 1417ns, `ratio_p50: 0.9993` — the only parity-shaped lane in the run; p2_by_key ours 916ns. `README.md:167` calls p5 "a ~1.00" and points.rs:161 calls the surface "0.5.0's flagship".
- The same unconditional resolve exists on the two sibling surfaces: `WriteTx::get_dyn` → `decode_values` at `crates/bumbledb/src/api/db/get.rs:283,344` (resolver: `plumbing::resolve_string_write`), and the macro-generated typed `Fact::decode` (`crates/bumbledb-macros/src/lib.rs:2376` — `resolve_string{suffix}(ctx, id)` per str field), so the typed `Snapshot::get` pays the same reverse descent even though its `Key` struct carries the `&str`.

### Bench impact

p5_keyed_get: each of the 3 hit param-sets per rotation pays one full reverse-map descent in `_dict` to reconstruct `"doc/xxxxxxxx"` for the caller who supplied it. Eliding it cuts the hit path from 4 LMDB descents to 3 — against SQLite's 2 — in the one lane where the ratio is 1.00, and removes proportionally the most in exactly the small-fact keyed-get regime the surface was built for. Two calibrations against the original claim: (a) the +0.5us p2→p5 delta is not attributable solely to this resolve — p5's full-fact decode also allocates the `Vec<Value>` and the `bytes<32>` payload Box, and the surfaces differ — so treat the descent as one component, not the whole delta; (b) substituting the caller's `Value` still costs one heap copy under the owned-return API (`Value::String(Box<[u8]>)` clone copies), so the win is the descent, not the copy, unless the surface is changed to move or borrow.

### Suggested fix

Give the get decode the statement projection and the caller's `key_values`: for each field in the projection, produce the supplied `Value` (clone in the dyn surfaces) instead of invoking the resolver; non-projected fields decode as today. Concretely: a `decode_values_with_projection(fact, layout, projection, key_values, resolver)` used by `Snapshot::get_dyn` (snapshot.rs:200) and `WriteTx::decode_values`' get caller (get.rs:283), and — as a follow-up requiring lifetime plumbing — the typed `Fact::decode` seam, where the generated key struct already holds the `&str` and could hand it through. This is representation-first: the projection already states which fields the determinant fixed; using it erases a per-hit storage round-trip that only re-derives the input.
