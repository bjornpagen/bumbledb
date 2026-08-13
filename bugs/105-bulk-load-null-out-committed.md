# bdb_db_bulk_load can commit facts then return MISUSE if out_committed is null
- id: 105
- severity: high
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/db.rs, cpp/foreign/bumbledb_c.h
- status: open (do not fix)

## Summary
`bdb_db_bulk_load` copies rows, takes the writer flag, then calls `Db::bulk_load_dyn`. Only *after* the engine returns does it `out(out_committed, total)` (success) or `out(out_committed, bulk.committed)` (typed failure). A null `out_committed` turns a completed (or partially durable) import into `BDB_STATUS_MISUSE` with no error payload and no committed-count, contradicting the header (“`out_committed` always carries the durable count”).

## Evidence
- After `bulk_load_dyn`:
  - `Ok(total) => out(out_committed, total)?`
  - `Err(bulk) => out(out_committed, bulk.committed)?; Err(fail_engine(BulkLoad { … }))`
  (`cpp/bridge/src/db.rs`)
- `out` on null is `Fail::Misuse` (`cpp/bridge/src/lib.rs`).
- Header: “`out_committed` always carries the durable count (§24), and a failure is `BDB_ERROR_KIND_BULK_LOAD`” (`cpp/foreign/bumbledb_c.h`).
- Prior chunks stay committed by engine contract; the first 4096-row chunk can already be durable when the second fails.

## Why this is a bug
Durable state changes with a status that means “nothing happened, no error object.” Callers that check only the status will retry and duplicate, or skip `bdb_error_get_bulk_committed`. This is not a hypothetical: the ABI documents `out_committed` as required, and null required pointers are the defined MISUSE lane — so the engine work must not run before the out-param is checked, or the committed count must be stored on the error object only (it already is for `BulkLoad`).

## How to trigger / repro sketch
Import ≥1 valid row with `out_committed == NULL`. Status is MISUSE; scanning the relation still shows the rows. For the partial-failure variant, use the existing second-chunk arity-error fixture with a null count pointer: chunk 1 durable, status MISUSE, no `bdb_error`.

## Spec / docs notes
Violates the bulk-load count contract in `bumbledb_c.h` and `TODO_CPP.md` §24 as restated on the export. C++ dialect currently has no raii wrapper for bulk_load, so only the raw ABI hits this until one is added.

## Related
- 104 (same `out()`-after-side-effect ordering)
- 110 (another “mutate then fail” boundary)
