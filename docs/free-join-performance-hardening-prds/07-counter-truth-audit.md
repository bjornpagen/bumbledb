# PRD 07: Counter Truth Audit

## Purpose

Guarantee that every reported metric is real, scoped, and actionable.

## Required Audit

Review all benchmark and explain fields in:

- `crates/bumbledb-bench/src/report.rs`;
- `crates/bumbledb-bench/src/job/mod.rs`;
- `crates/bumbledb-lmdb/src/query/explain.rs`;
- trace modules added by this suite.

## Required Rules

- A counter must be incremented at the exact operation it counts.
- A counter must be named after the operation, not the interpretation.
- A timing must be measured by span boundaries.
- Allocation deltas must come from allocator snapshots.
- If a value is an estimate, the field name must say `estimate`.
- If a value is exact, tests must prove it for a small fixture.

## Required Deletions

- Delete synthetic counters.
- Delete stale fields that always report zero.
- Delete duplicate benchmark fields whose meaning is superseded by trace summaries.
- Delete vague names like `source_mode` if they do not identify an actual source implementation.

## Passing Criteria

- Every benchmark JSON field is documented in code or in this PRD suite.
- A test fails if a report tries to render an enabled counter group with no source measurements.
- `explain` reports trace availability accurately.
- `rg "synthetic|placeholder|not collected|TODO|fake" crates/bumbledb-bench crates/bumbledb-lmdb/src/query` returns no relevant metric placeholders.
- Global acceptance from PRD 00 passes.
