# NAPI prepared-query pointer is used as &mut on a worker while JS still holds RefMut
- id: 100
- severity: critical
- confidence: confirmed
- area: ffi
- components: ts/crate/src/lib.rs, crates/bumbledb/src/api/prepared.rs
- status: fixed (2026-08-13)

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

## Verification (2026-08-12)

**Verdict:** confirmed. Severity unchanged (critical).

**Trace:** `prepared_execute` takes `live_mut` (`ts/crate/src/lib.rs:1349`) then `std::ptr::from_mut(&mut prepared_inner.prepared) as usize` (`:1351`) and blocks in `SnapWorker::call` (`:579-583`, send then `recv`). The worker rebuilds `&mut *(prepared as *mut PreparedQuery<'static, …>)` (`:652-654`) and runs `execute_answers`. `prepared_inner: RefMut<PreparedInner>` is still in scope for the whole `reply!`. `prepared_explain` is the same exclusive path (`:1377-1380`); `prepared_staleness` ships a shared `&` of a `!Sync` type (`:1398-1400`, worker `:686-687`). Engine: `PreparedQuery` is `!Sync` via `PhantomData<(Cell<()>, fn() -> S)>` (`crates/bumbledb/src/api/prepared.rs:176-187, 287-290, 521-522`).

**Why it holds:** The module doc (`ts/crate/src/lib.rs:15-19`) claims the JS thread is blocked so “at most one thread touches any engine object,” and the worker SAFETY comment says “no second reference exists anywhere.” That is false: `RefMut` is a live exclusive borrow of the same `PreparedQuery` on the JS thread for the entire `recv`. Two overlapping exclusive (execute/explain) or shared-`!Sync` (staleness) references across threads is aliasing UB on every successful call, not only a hostile caller. The `usize` round-trip also drops provenance.

## Resolution (2026-08-13)

`PreparedQuery` now lives in `UnsafeCell` behind an `in_flight` lease: the JS thread drops every `Ref`/`RefMut` before the snapshot worker dereferences the raw pointer, and close refuses the lease. `preparedExplain` / `preparedStaleness` stay on the NAPI bridge.
