## restore_determinants rescans the relation's whole pending-fact set per key statement, making cancel-heavy transactions quadratic

category: perf | severity: low | verdict: CONFIRMED | finder: engine:storage

### Summary

When a delete cancels a pending insert, `restore_determinants` must revert each of the cancelled fact's key tuples to whatever still owns it in the final state. Because the determinant overlay stores only the *last* disposition per tuple (`BTreeMap<DeterminantImage, DeterminantDisposition>`, last-disposition-wins), the previous owner is unrecoverable from the representation and is rediscovered by a full linear scan of the relation's remaining pending facts — with a fresh determinant-image derivation per candidate — nested inside the per-key-statement loop. Every other delta operation is O(log |delta|); this one is O(|delta_rel| × keys) per cancellation, so a transaction that bulk-inserts N facts and then retracts them before commit pays O(N²) determinant derivations while holding the single-writer mutex. This is a representation-over-control-flow violation: state the forward path already knew (who owned the tuple before) is recomputed by scan instead of held as data.

### Evidence

- `crates/bumbledb/src/storage/delta/determinants.rs:82-133` — `restore_determinants`: for each `key_id` in `relation.keys()` (line 85), the resolver loop at lines 99-120 iterates `self.facts.range((rel, [0u8; 32])..=(rel, [0xFF; 32]))` — every pending fact of the relation — and calls `keys::determinant_image(...)` per candidate (lines 104-109). The loop breaks early only on a matching `Insert` (line 117); when the tuple resolves to `Absent` or to no pending owner, every pending fact of the relation is visited and image-derived.
- `crates/bumbledb/src/storage/delta/delete.rs:32-42` — the trigger: a delete that finds a pending `Insert` removes it from `self.facts` and calls `self.restore_determinants(rel, fact_bytes)`.
- `crates/bumbledb/src/storage/delta/determinants.rs:21-69` — the contrast: the forward path `record_determinants` is one map probe per key statement (`per_key.get`/`get_mut`/`insert`), O(log) as every other delta operation.
- `crates/bumbledb/src/storage/delta.rs:111` and `delta.rs:47-56` — the lossy representation: `determinants: BTreeMap<KeyId, BTreeMap<DeterminantImage, DeterminantDisposition>>`, documented "last disposition wins, mirroring the fact map". The pre-insert owner is discarded at record time; restoration must rediscover it.
- Spec check: `docs/architecture/50-storage.md` (§ `WriteTx` point reads) and `docs/architecture/70-api.md:504,585` specify the point-read contract restore_determinants exists to uphold (a pre-commit point read answers exactly what a post-commit read would), but neither doc mandates or accepts a scan-based restoration — the semantics are required, the scan is an implementation choice.
- Test check: `crates/bumbledb/src/storage/delta/tests.rs:379-473` (the "cancel trio": committed-owner, earlier-pending-owner, keeps-absence) pins restoration *correctness*, and the scratch-clone counter pins allocation discipline — but no test bounds the scan cost, so the quadratic path is unguarded.

No allocation violation accompanies the scan (the `candidate` scratch at line 84 is reused across candidates); the cost is pure CPU: repeated field-slicing derivations plus the range walk.

### Bench impact

Correctness is unaffected. A host transaction doing bulk reconciliation — insert N rows, then retract many of them before commit (or any upsert-shaped churn that leaves the delta large while cancellations occur) — degrades quadratically in the write region: the k-th cancellation scans the O(N) remaining pending facts per key statement, summing to O(N² × keys) determinant derivations, all inside the single-writer mutex window. Note the shape matters: repeated `insert(f); delete(f)` of a *single* fact on an otherwise-empty delta stays cheap because each cancel shrinks the delta back to ~0; the quadratic case is a large resident delta with interleaved cancellations. Any commit-shaped write-lane benchmark with cancellations regresses superlinearly with delta size.

### Suggested fix

Hold the revert target as data instead of recomputing it by scan: represent the per-key overlay as an owner stack/set per tuple, e.g. `BTreeMap<DeterminantImage, SmallVec<[(ArenaSlice, DeterminantDisposition); 1]>>`. Recording pushes; a cancel removes its own entry and the tuple reverts to what remains (or to no-overlay when empty) — exactly the resolution `restore_determinants` reconstructs today by scanning `self.facts`, held explicitly. Point reads read the resolved top (any remaining `Present` wins over `Absent`, matching the current resolver's insert-wins rule at determinants.rs:113-119). This makes restoration O(log |delta|) like every other delta operation and erases the nested scan, at the cost of one small-vec slot per touched tuple in the common single-owner case. The existing cancel-trio tests (tests.rs:379-473) pin the semantics the new representation must preserve.
