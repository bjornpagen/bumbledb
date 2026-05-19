# PRD 03: Count-Only Execution And Benchmark Fairness

## Status

Draft.

## Problem

The benchmark harness compares Bumbledb's full owned result materialization against SQLite row counting. SQLite benchmark code prepares a query and counts rows with `query_map(..., |_| Ok(())).count()`. It does not decode projected values. Bumbledb executes `execute_query`, builds owned `Vec<Vec<Value>>`, deduplicates/sorts projection rows, and decodes output values.

This makes large-output non-JOB comparisons unfair and can mislead performance decisions.

## Root Cause Analysis

SQLite path:

```rust
stmt.query_map(params, |_| Ok(()))?.count()
```

Bumbledb path:

```text
execute query
bind variables
emit projected encoded rows
dedupe/sort in BTreeSet
decode each output Value
return owned rows
caller counts rows.len()
```

This disproportionately affects:

```text
ledger/tag_lookup_join -> 10000 rows, 20000 decoded values
sailors/red_boat_sailors -> 10000 rows, 20000 decoded values
sailors/high_rating_red_boats -> 6660 rows, 13320 decoded values
tpch/supplier_nation_orders -> 5716 rows, 11432 decoded values
```

The benchmark is useful for current API latency, but not a fair engine-vs-engine row-count comparison.

## Goal

Add a count-only execution/reporting path that can compare Bumbledb and SQLite under equivalent row-count work without changing normal query semantics.

## Non-Goals

- Do not remove normal owned result execution.
- Do not change public query result semantics.
- Do not make count-only the default application API.
- Do not fake counts by skipping query execution.
- Do not weaken correctness tests.

## Technical Design

Add an internal benchmark/execution mode:

```rust
ReadTxn::execute_query_count_only(schema, query, inputs) -> Result<QueryCountOutput>
```

or equivalent benchmark-only path that:

- runs the same planner/runtime selection,
- executes relation predicates and joins normally,
- increments output count instead of materializing projected rows,
- avoids decoding projected values,
- preserves counters and plan diagnostics.

Output shape:

```rust
pub struct QueryCountOutput {
    pub rows: usize,
    pub plan: QueryPlan,
}
```

For aggregate queries, count-only mode must not replace aggregate semantics unless the SQL comparison also only counts aggregate result rows. Aggregate value correctness remains covered by ordinary tests/reference tests.

## Benchmark Harness Changes

Add CLI flag:

```text
--compare-mode rows
--compare-mode materialized
```

Default can remain materialized for API realism, but benchmark reports must say which mode was used.

`rows` mode:

- Bumbledb uses count-only output where legal.
- SQLite uses current `sqlite_count` path.

`materialized` mode:

- Bumbledb uses normal `execute_query`.
- SQLite must decode/project result values for selected comparable queries, or the report must label SQLite as row-count-only.

## Required Correctness

Count-only result count must equal normal execution row count for:

- projections with set semantics,
- direct storage single-relation path,
- direct chain path,
- hash probe path,
- LFTJ path,
- mixed path,
- static empty path.

For projection set semantics, count-only cannot simply count emitted bindings unless deduplication is equivalent. It must count distinct projected tuples or be explicitly limited to queries whose projected tuple is unique by schema constraints.

The first implementation may support a safe subset:

- projected fields include a primary key or unique identity,
- direct chain outputs are known unique,
- otherwise fall back to materialized mode.

## Required Tests

Add tests for:

- Count-only on single primary-key lookup equals materialized result count.
- Count-only on FK chain with unique output equals materialized result count.
- Count-only on `tag_lookup_join` equals materialized result count.
- Count-only refuses or falls back for projection where distinctness is not guaranteed.
- Aggregate queries retain ordinary aggregate semantics.

## Required Benchmark Reporting

Benchmark JSON and markdown must include:

- compare mode,
- whether Bumbledb materialized rows,
- whether SQLite materialized rows,
- Bumbledb output row count,
- Bumbledb materialized value count,
- count-only supported/fallback reason.

## Strict Passing Criteria

- The benchmark report labels row-count-only comparisons explicitly.
- In count-only mode, `ledger/tag_lookup_join` avoids decoding 20000 projected values.
- In count-only mode, `sailors/red_boat_sailors` avoids decoding 20000 projected values.
- Count-only row counts match materialized row counts on all supported target queries.
- Count-only mode does not change normal `execute_query` behavior.
- Existing correctness/property/differential tests continue to use materialized results.
- Full workspace test/clippy/fuzz gates pass.

## Verification Commands

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
cargo run -p bumbledb-bench --release -- --scale 10000 --warmup 2 --repeats 30 --format json --dataset ledger --query tag_lookup_join --compare-mode rows
cargo run -p bumbledb-bench --release -- --scale 10000 --warmup 2 --repeats 30 --format json --dataset sailors --query red_boat_sailors --compare-mode rows
```
