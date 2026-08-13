# Unit test stale_snapshot_ref_is_misuse is itself use-after-free
- id: 115
- severity: info
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/tests.rs
- status: open (do not fix)

## Summary
The ABI test that pins “stashed snapshot → MISUSE” dereferences a `*const bdb_snapshot_ref` after the `bdb_db_read` stack frame has dropped that object. The test file comments that the frame is gone “in principle.” Passing CI does not prove the MISUSE contract; it proves the stack slot was not reused yet. This is recorded separately from 101 so a coordinator does not treat the test as evidence the feature works.

## Evidence
- `stale_snapshot_ref_is_misuse` saves `stashed = snap` inside the callback, then after `db_read` returns calls `db_write_from(db, stashed, …)` and expects `bdb_status::Misuse` (`cpp/bridge/src/tests.rs`).
- `bdb_snapshot_ref` is a local in `bdb_db_read`’s closure (`cpp/bridge/src/db.rs`).

## Why this is a bug
The test is UB. Under ASan, stack reuse, or a different optimizer, it can crash or spuriously pass. It currently green-washes 101.

## How to trigger / repro sketch
Run `cpp/bridge` tests with ASan/Miri (Miri can run this crate’s `#[test]`s without C++). Insert a large stack array between `db_read` and `db_write_from` to increase reuse chance.

## Spec / docs notes
None.

## Related
- 101

## Verification (2026-08-12)

**Verdict:** confirmed. Severity unchanged (info).

**Trace:** `stale_snapshot_ref_is_misuse` (`cpp/bridge/src/tests.rs:995-1012`) saves `stashed = snap` inside the callback, then after `db_read` returns calls `db_write_from(db, stashed, …)` and expects `bdb_status::Misuse`. The pointee is the `bdb_snapshot_ref` local in `bdb_db_read`’s closure (`db.rs:428-431`), dropped when that closure returns. The test itself comments that the frame is gone “in principle.”

**Why it holds:** The test is UAF. Passing CI does not prove the MISUSE contract; it proves the stack slot was not reused yet. Recorded separately from 101 so a coordinator does not treat the test as evidence the feature works.
