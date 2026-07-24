## verify_store's `_dict` pass checks nothing: referenced-id-without-reverse, forward/reverse desync, and out-of-bounds dict ids all pass a clean sweep

category: missing-free-feature | severity: high | verdict: CONFIRMED | finder: engine:storage
outcome: fixed db9b934d

### Summary

`Db::verify_store` gives every other namespace exhaustive bidirectional verification (F↔M, F↔U with pointwise disjointness, F↔R with φ re-checks, S against the F tallies, the `_meta` descriptor fingerprint), but the dictionary pass (`crates/bumbledb/src/verify_store/dict_stat.rs:11-24`) is only the accepted-leak statistic: one cursor over the reverse map counting entries no live fact references. Three corruption classes therefore survive a clean sweep:

1. **Referenced id without a reverse entry** — the exact corruption the runtime types as `Corruption(DanglingInternId)` (`storage/dict.rs:158-162`). The liveness set needed for this check, `Sweep.referenced_interns`, is already built by the F pass (`verify_store/facts.rs:85-98`) and then used only for the leak count.
2. **Forward↔reverse desync** — no pass reads the forward map at all (grep for forward-map access under `verify_store/` finds only comments). A forward entry `blake3("alice") → id_bob` silently rebinds every selection literal and query filter on `"alice"`.
3. **Dictionary ids at/beyond `dict_next_id`** — the F pass bounds *fact-referenced* ids (`InternBeyondNextId`, facts.rs:89-96), but the dictionary's own entries are never bounded, even though the sweep already carries the counter (`verify_store.rs:254,309`). `put_pending` puts blindly (`storage/dict.rs:113-122`), so a future mint of a different string overwrites such a reverse entry while the stale forward entry keeps pointing at the reused id — manufacturing desync (2).

### Evidence (verified)

- `crates/bumbledb/src/verify_store/dict_stat.rs:11-24` — the entire pass: `for entry in dict::reverse_ids { Id(id) => if !referenced_interns.contains(&id) { dangling += 1 }, Malformed(key) => finding }`. No reverse-presence check, no forward read, no bound check.
- `crates/bumbledb/src/verify_store/facts.rs:85-98` — `referenced_interns.insert(id)` plus `id >= dict_next_id → InternBeyondNextId`; the set is consumed nowhere else.
- `crates/bumbledb/src/storage/dict.rs:158-162` — `resolve(...).ok_or(Error::Corruption(CorruptionError::DanglingInternId(id)))`: the runtime conviction the sweeper cannot reproduce, violating the module's own bar, "the sweeper's knowledge is the engine's knowledge, never a second implementation" (`verify_store.rs:5-6`).
- `crates/bumbledb/src/storage/commit/judgment.rs:161-163` — `Selections::encode_committed` resolves φ/ψ string literals through `dict::lookup` (the forward map). Under desync (2) the sweeper's own selection re-checks consume the tampered resolution, so the corruption is self-consistent and invisible to every existing pass.
- `crates/bumbledb/src/storage/dict.rs:66-70,113-122` — counter-minted ids plus blind `put_pending`: the reuse/overwrite path for desync (3).
- `crates/bumbledb/src/verify_store/tests.rs` (lines ~420-422, 651-666, 668-682) — dict coverage is exactly: the dangling statistic, `InternBeyondNextId` for a fact-referenced id, and a malformed reverse key. No fixture plants any of the three desyncs above. This breaks the fixture-per-claim doctrine of `docs/architecture/50-storage.md` § verify_store ("every verifier pass has a deterministic corruption fixture ... an empty finding list is backed by a fixture per claim").
- Spec check (`docs/architecture/50-storage.md`): line 409 declares "a dangling intern id" corrupt data, a hard error, never a skip; the verify_store paragraph (~line 420) claims the checker proves "...counters, and **dictionary bounds** over one snapshot" — the code meets that only for fact-referenced ids, not the dictionary's own entries. The "accepted leak" doctrine (lines 332-333, and the dict_stat.rs header) covers only *unreferenced reverse entries* and does not exempt these desyncs.

### Failure scenario

- Reverse entry `id_alice → "alice"` lost (torn page, tamper) while facts still carry `id_alice` below the counter: `verify_store` returns `Ok` with zero findings; the next export/decode of any such fact aborts with `Corruption(DanglingInternId)`.
- Forward entry rebound to `blake3("alice") → id_bob`: `find users where name == "alice"` returns Bob's rows on every snapshot; containment/window judgments over ψ = "alice" judge Bob's facts; the offline tool whose one job is convicting this store reports a coherent store.

### Suggested fix

Three additions to the dict pass, all on machinery the sweep already holds:

1. After the F pass, for each id in `referenced_interns`, require a reverse entry (point-get via `reverse_key(id)`); a miss is a new `DanglingInternId`-shaped `StoreFinding`.
2. While cursoring the reverse map, recompute `forward_key(raw)` from the stored raw bytes and require the forward entry to exist and map back to the same id — one blake3 per entry, the price the M pass already pays per entry (`membership.rs:37-39` recomputes `fact_hash`).
3. Convict any reverse-map id ≥ `Sweep.dict_next_id` (the counter is already in the sweep struct).

Add one corruption fixture per new conviction in `verify_store/tests.rs`, per the 50-storage.md fixture-per-claim doctrine.
