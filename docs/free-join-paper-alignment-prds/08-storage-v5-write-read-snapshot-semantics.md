# PRD 08: Storage V5 Write, Read, And Snapshot Semantics

## Purpose

Implement v5 storage operations over the new columnar set layout. This PRD restores full storage correctness after the storage-format break.

## Dependencies

- PRD 07.

## Scope

- `Environment::open` and schema verification for v5.
- `WriteTxn::insert`, `WriteTxn::delete`, and bulk load.
- Read diagnostics and relation fact counts.
- Constraint guards and dictionary behavior.
- Failpoint atomicity.
- LMDB MVCC snapshot behavior.

## Required Insert Semantics

- Resolve relation and validate field values.
- Encode a full fact in schema field order.
- Intern strings/bytes in create mode.
- Compute content-derived fact handle.
- Check canonical membership `T`.
- If present, return `AlreadyPresent` without changing logical storage tx ID.
- Check FKs through `U` guard entries.
- Check unique constraints through `U` guard entries.
- Write `T`, `H`, one `C` entry per field, `L`, `U`, `R`, optional `A`, and `S` updates atomically.
- Advance logical storage tx ID only for successful logical insert.

## Required Delete Semantics

- Resolve relation and validate field values.
- Encode using existing dictionary entries only.
- If dictionary value is absent, return `Absent`.
- Check canonical membership `T`.
- If absent, return `Absent` without changing logical tx ID.
- Check restrict delete through `R` guards.
- Delete optional `A`, `R`, `U`, `L`, `C`, `H`, `T`, and stats atomically.
- Advance logical storage tx ID only for successful logical delete.

## Required Read Semantics

- Relation fact counts come from v5 stats or live row scans.
- Full fact reconstruction is possible through `H` or `C` columns for tests and diagnostics.
- A read transaction sees a stable LMDB snapshot while concurrent writes commit.
- Dictionary reverse lookup still decodes output values correctly.

## Technical Direction

- Keep all changes behind the new format. Do not support mixed v4/v5 writes.
- Rebuild storage tests around v5 namespaces rather than old access-entry assumptions.
- Preserve failpoints around dictionary, canonical, column, live-row, guard, stats, and commit stages.
- Optional accelerators may initially be absent or minimal. Query correctness must not depend on them.

## Non-Goals

- Do not implement query execution over v5 here beyond storage tests.
- Do not implement COLT here.
- Do not expose fact handles publicly.

## Acceptance Criteria

- Duplicate insert remains idempotent and does not advance logical tx ID.
- Absent delete remains idempotent and does not advance logical tx ID.
- Exact delete removes all v5 row/column/guard state.
- Unique constraints reject conflicting facts.
- FK constraints reject missing target keys.
- Restrict delete rejects target deletion when reverse FK guards exist.
- Bulk load is one write transaction and rolls back completely on error.
- Failpoint tests prove no partial `T/H/L/C/U/R/A/S` state commits.
- Reader snapshot tests prove stable read behavior across concurrent writes.

## Required Tests

- Duplicate insert no-op.
- Delete absent no-op.
- Delete then reinsert.
- Unique violation.
- FK violation.
- Restrict violation.
- Bulk load rollback on invalid row.
- Failpoints before and after dictionary, canonical, column, live row, guard, stats, and commit stages.
- Reopen database and verify counts/facts.
- Reader snapshot stability.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb storage --all-features
cargo test -p bumbledb-test-support --test failpoints --all-features
cargo test -p bumbledb-test-support --test concurrency --all-features
cargo check --manifest-path fuzz/Cargo.toml
```
