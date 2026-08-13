# NAPI prepared-query pointer is used as &mut on a worker while JS still holds RefMut
- id: 100
- severity: critical
- confidence: confirmed
- area: ffi
- components: ts/crate/src/lib.rs, crates/bumbledb/src/api/prepared.rs
- status: open (do not fix)

## Summary
`preparedExecute` / `preparedExplain` / `preparedStaleness` send a raw address of `PreparedQuery` to a snapshot worker thread and reconstruct a Rust reference there, while the JS thread still holds a live `RefMut`/`Ref` to the same `PreparedInner`. `PreparedQuery` is explicitly `!Sync` (interior scratch). This is aliasing UB under Stacked/Tree Borrows and a data-race if anything else ever touches the handle.

## Evidence
- Ownership: JS `External<PreparedHandle>` owns `PreparedInner { prepared, _db }`. Execute takes `live_mut`, then ships `from_mut(&mut prepared_inner.prepared) as usize` over mpsc while that `RefMut` remains in scope for the blocking `recv`.
- Worker reconstructs `&mut *(prepared as *mut PreparedQuery<'static, …>)` and runs the engine on that reference (`ts/crate/src/lib.rs` around the `SnapReq::Execute` arm and `prepared_execute`).
- Engine contract: `PreparedQuery` is `!Sync` by construction (`Cell` marker plus mutable scratch) and “executes from one thread at a time” (`crates/bumbledb/src/api/prepared.rs`).
- The same pattern is used for `Explain` (`&mut`) and `Staleness` (`&` to a `!Sync` type on another thread).

## Why this is a bug
Two exclusive (execute/explain) or shared-but-`!Sync` (staleness) Rust references to the same object exist at once, on two threads. That is undefined behavior even if the JS thread does not dereference the `RefMut` during the wait. A panic, a second Node isolate, or a future engine read of scratch via `&self` turns this into a real data race / use-after-free. Casting the pointer through `usize` also strips provenance.

## How to trigger / repro sketch
1. `dbPrepare` a query, `dbSnapshot`, then `preparedExecute` (the normal SDK path already hits this).
2. Run under Miri is not practical (napi), but a Tree-Borrows-aware review of the two references is enough; TSan on a build that also calls `preparedStaleness` concurrent with anything else would be the next step.
3. The overlapping borrows exist on every successful execute, not only on a hostile caller.

## Spec / docs notes
The crate module doc claims “at most one thread touches any engine object at any instant, which is the whole soundness argument for the one raw pointer that crosses threads.” The JS thread still holds a typed `&mut`/`&` to that object, so the argument does not hold.

## Related
- 112 (C ABI has no exclusive lock on the same `!Sync` type)
- Prior Node finding 018 (witness/`&'static Snapshot`) was a sibling lifetime lie; write_from now moves a `Witness` value, but this prepared-pointer path was not given the same treatment.
