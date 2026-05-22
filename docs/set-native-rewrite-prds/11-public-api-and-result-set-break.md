# 11 Public API And Result Set Break

## Purpose

Make the public API express set semantics instead of exposing bag-shaped carriers.

## Required Public Shape

Replace `QueryOutput.rows: Vec<Vec<Value>>` with an explicit result set type:

```rust
pub struct QueryResultSet {
    pub columns: Vec<ResultColumn>,
    pub rows: Vec<ResultTuple>,
}
```

`QueryResultSet` must guarantee:

- no duplicate tuples
- deterministic order or explicitly documented canonical order
- column types match query output

Cardinality APIs must be named as cardinality APIs:

```rust
execute_result_cardinality(...)
QueryResultCardinality { cardinality, plan }
```

They must not fabricate result rows.

## Required Code Changes

- Remove or rename `QueryCountOutput`.
- Remove benchmark conversion from cardinality to fake `QueryOutput`.
- Remove `Vec<Vec<Value>>` from public query result contract where feasible.
- Keep conversion helpers only as explicit test/compat-free utilities.
- Rename diagnostics from `row_count` to `tuple_count` or `cardinality` where public.

## Acceptance Gates

- Public query result type documents and enforces duplicate-free invariant.
- Public cardinality APIs cannot be confused with aggregate count values.
- No production code creates fake empty rows for cardinality-only execution.
- Golden assertions compare `QueryResultSet` values.

## Tests Required

- Duplicate construction rejected or impossible for result set helper.
- Deterministic ordering test.
- Cardinality API returns output cardinality for projection and grouped aggregate outputs.
- Aggregate count value test remains separate from output cardinality test.

## Non-Goals

- No backward-compatible `rows` alias.
- No SQL row bag adapter.
