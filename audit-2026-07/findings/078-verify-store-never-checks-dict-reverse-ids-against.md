## verify_store never bounds `_dict` reverse ids by META_DICT_NEXT_ID — a regressed counter is certified clean, then armed for silent reverse-map reuse

category: missing-free-feature | severity: medium | verdict: CONFIRMED | finder: r2:crash-recovery-lifecycle

### Summary

The offline sweeper (`Db::verify_store`) checks the intern-id namespace against the `_meta` dictionary next-id counter only from the fact side: `InternBeyondNextId` fires when a live fact references an id `>= dict_next_id` (facts.rs:89-96). The dictionary pass itself — the one pass that actually cursors the `_dict` reverse map — never consults the counter: `dict_stat::dangling` compares each reverse id only against `referenced_interns` to produce the informational dangling statistic (crates/bumbledb/src/verify_store/dict_stat.rs:11-24), and the forward map is never scanned at all. So a store whose META_DICT_NEXT_ID has regressed below existing reverse ids (bit-rot, a partial `_meta` restore, any forged state of the class the sweeper exists to convict) passes with **zero findings** as long as the high ids are dangling.

This is the exact invariant the sweeper already pins for the sibling counter: `RowIdHighWaterLow` convicts an `S` high-water that does not exceed an observed row id (crates/bumbledb/src/verify_store/counters.rs:49-58), with a raw-plant fixture (`low_high_water_is_found_against_the_max_row_id`, verify_store/tests.rs:989-1007). The dictionary counter has no such check and no such fixture, while the docs claim both: `docs/architecture/50-storage.md` (offline-checker paragraph) says `verify_store` proves "counters, **and dictionary bounds** over one snapshot" with "a deterministic corruption fixture that plants the defect raw ... per claim", and `storage/dict.rs:15-16` pins "Ids are monotonic, **never reused**, append-only". A reverse id at or beyond the counter is precisely the state that makes reuse reachable.

### Evidence (all verified in-tree)

- **The gap.** `crates/bumbledb/src/verify_store/dict_stat.rs:13-22` — the whole pass:
  ```rust
  for entry in dict::reverse_ids(s.txn)? {
      match entry? {
          ReverseId::Id(id) => {
              if !s.referenced_interns.contains(&id) { dangling += 1; }
          }
          ReverseId::Malformed(key) => s.malformed(key, "dict reverse id"),
      }
  }
  ```
  `s.dict_next_id` is loaded at `verify_store.rs:254` and sits in the `Sweep` (verify_store.rs:309) but is never read here. Grep confirms its only verify-side consumer is facts.rs:89.
- **Fact-side check only.** `crates/bumbledb/src/verify_store/facts.rs:85-98` — `InternBeyondNextId` covers ids referenced by live facts; a *dangling* high id is invisible.
- **The existing template.** `crates/bumbledb/src/verify_store/counters.rs:49-58` — `RowIdHighWaterLow { relation, stored, max_row_id }` when `stored <= tally.max_row_id`; fixture at `verify_store/tests.rs:989-1007` plants the regressed counter raw and asserts the conviction.
- **The reuse mechanism.** Mint: `crates/bumbledb/src/storage/delta/intern.rs:64-78` assigns `next = view.dict_next_id()` sequentially (only `u64::MAX` is refused). Flush: `crates/bumbledb/src/storage/commit/write.rs:360-363` calls `dict::put_pending` then `put_dict_next_id`. Overwrite: `crates/bumbledb/src/storage/dict.rs:113-122` — `put_pending` is a plain `dict.put(...)` on both `forward_key(raw)` and `reverse_key(id)`, no `NO_OVERWRITE` flag, so `reverse_key(id)` is silently clobbered while the OLD string's forward entry `blake3(old) -> id` survives.
- **No other guard.** `storage/env/read_meta.rs:115-119` rejects only a stored `u64::MAX` counter; nothing at open/exhume reconciles the counter against `_dict` contents.
- **Doc pins checked.** `docs/architecture/50-storage.md` offline-checker paragraph ("counters, and dictionary bounds ... fixture per claim") and `storage/dict.rs:15-16` ("monotonic, never reused, append-only") — the code diverges from both for this bound.

### Failure scenario

Store holds `_dict` reverse entries 0..10 (with matching forward entries) but META_DICT_NEXT_ID reads 5; ids 5..10 are referenced by no live fact. `verify_store` returns `Ok` with an empty findings list (5..10 land only in the informational `dangling_intern_ids` count). The next `Db::write` interning a novel string mints id 5 (`intern.rs:70`); at commit, `put_pending` overwrites `reverse(5)` with the new bytes (`dict.rs:120`). The old string's forward entry still maps to 5, so a later fact interning the old string stores intern id 5, and every decode of that field (`dict::resolve`, dict.rs:158-162) returns the NEW string — wrong data, no error, permanent, and `verify_store` stays clean throughout (the fact's id 5 is now below the advanced counter, and forward/reverse are internally consistent for the new string).

### Suggested fix

One compare inside the loop dict_stat already runs: on a well-formed `ReverseId::Id(id)` with `id >= s.dict_next_id`, push a new `StoreFinding::DictNextIdLow { stored: s.dict_next_id, reverse_id: id }` (mirroring `RowIdHighWaterLow`'s shape), plus the raw-plant fixture the 50-storage doc promises per claim. Representation-level hardening in the same spirit: `put_pending`'s reverse put can use `PutFlags::NO_OVERWRITE` so id reuse is a loud commit-time corruption error instead of a silent clobber — the append-only invariant enforced by the write itself rather than by an offline check.
