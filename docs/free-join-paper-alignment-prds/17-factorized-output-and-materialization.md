# PRD 17: Factorized Output And Materialization

## Purpose

Add an internal factorized output path inspired by the paper and factorized databases while preserving Bumbledb's public duplicate-free `QueryResultSet` contract.

## Dependencies

- PRD 12.
- PRD 14.

## Scope

- Internal output representation.
- Projection deduplication before final decoding.
- Optional count/materialization shortcuts for benchmarks if kept internal.
- Final materialization into `QueryResultSet`.

## Required Semantics

- Public query execution returns a canonical duplicate-free `QueryResultSet`.
- Internal factorized representation may avoid expanding Cartesian products where projection does not require them.
- Duplicate witnesses must not multiply public output.
- Factorized mode and materialized mode must return identical final result sets.
- Any count-like internal metric must not become a public scalar query API.

## Technical Direction

- Start by factorizing only projection-safe branches where variables are independent after a node.
- If a full factorized representation is too large, implement a conservative compressed projection sink for common Cartesian-product shapes.
- Keep encoded values until final materialization.
- Add counters for logical facts represented, materialized facts, duplicate witnesses suppressed, and expansions avoided.
- Ensure final decoding still uses dictionary reverse lookups correctly.

## Non-Goals

- Do not expose public factorized output in this PRD.
- Do not add aggregation or group-by.
- Do not remove regular materialized `QueryResultSet`.

## Acceptance Criteria

- Factorized output mode returns exactly the same `QueryResultSet` as regular materialization.
- Duplicate projection witnesses remain deduplicated.
- Output-heavy fixture shows fewer internal expansions or records a clear compression counter.
- Empty outputs and single-row outputs work.
- Explain/metrics can report output mode after PRD 18, or temporary tests can inspect counters before then.

## Required Tests

- Factorized versus materialized output equality for clover.
- Factorized versus materialized output equality for triangle.
- Duplicate witness suppression.
- Large Cartesian product projection where factorized internals avoid full pre-projection expansion.
- Dictionary decode output for strings/bytes.
- Empty output.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb factorized_output --all-features
cargo test --workspace --all-features
```
