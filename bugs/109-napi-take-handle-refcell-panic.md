# NAPI take_handle uses RefCell::borrow_mut and panics into Node on re-entrant close
- id: 109
- severity: medium
- confidence: likely
- area: ffi
- components: ts/crate/src/lib.rs
- status: open (do not fix)

## Summary
`live` / `live_mut` use `try_borrow` / `try_borrow_mut` and convert a second borrow into a typed “re-entrant use” error. `take_handle` (every `*Close`, `txCommit`, `txAbort`) uses `cell.borrow_mut()`, which **panics** if a `Ref`/`RefMut` is already held. A panic across napi into V8 is undefined / process-aborting, not a JS exception.

## Evidence
- `take_handle`: `cell.borrow_mut().take().ok_or_else(|| closed_handle(what))` (`ts/crate/src/lib.rs`).
- `live` / `live_mut`: `try_borrow(_mut)` → typed `re-entrant use of a {what} handle`.
- Close/commit/abort all go through `take_handle`: `db_close`, `snapshot_close`, `exhume_close`, `prepared_close`, `tx_commit`, `tx_abort`.
- Same-thread re-entry is normally impossible while blocked in `recv`, but: (1) napi finalizers / other threads if an `External` is sent to a worker; (2) any future callback into JS while a `live()` guard is held; (3) inconsistency means the “typed close” path is the one that will abort instead of throwing if re-entry ever happens.

## Why this is a bug
The crate already knows RefCell re-entry is a programming error that should *throw*. Close uses the panicking API. Unwinding through napi/C is the same class of FFI panic the C++ bridge built `guard` to prevent.

## How to trigger / repro sketch
Today’s JS thread is blocked during worker `recv`, so a straightforward SDK script may not hit it. A Node worker thread that calls `snapshotClose` on a handle while the main thread is inside `preparedExecute` (which holds `live` on that snapshot) is the intended race: `RefCell` is `Send` but `!Sync` — two threads calling into the same `External` is already a data race (see 100). Even on one thread, replacing `borrow_mut` with a test that nests `snapshot_close` under `live(&snap)` panics.

## Spec / docs notes
Bridge error taxonomy: programming errors THROW. A panic is not a throw.

## Related
- 100 (same handles, cross-thread)
- 107 (C++ panic wall)
