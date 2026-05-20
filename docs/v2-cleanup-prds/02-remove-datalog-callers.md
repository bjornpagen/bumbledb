# PRD 02: Migrate All Callers Off Datalog

## Goal

Replace every production, benchmark, test, and fuzz caller of `parse_and_typecheck` with typed IR builder construction.

After this PRD, `datalog.rs` may still exist but must be unused outside its own tests. PRD 03 deletes it.

## Current Callers

Current grep anchors:

- `crates/bumbledb-lmdb/src/query.rs:9524+` uses `parse_and_typecheck` heavily in tests.
- `crates/bumbledb-lmdb/src/lib.rs:522+` uses benchmark Datalog in tests.
- `crates/bumbledb-lmdb/src/benchmark.rs:11-19` exposes `BenchmarkQuery { datalog, sqlite }`.
- `crates/bumbledb-bench/src/main.rs:8` imports `parse_and_typecheck`.
- `crates/bumbledb-bench/src/main.rs:554-560` defines `BenchQuery { datalog, ... }`.
- `crates/bumbledb-bench/src/main.rs:832-837` parses per query.
- `crates/bumbledb-bench/src/main.rs:1305-1308` detects count queries by string matching.
- `crates/bumbledb-bench/src/open.rs:932-1209` stores JOB query Datalog strings.
- `crates/bumbledb-test-support/src/workloads.rs:1-28` stores reusable Datalog strings.
- `crates/bumbledb-test-support/tests/*` consume those strings.
- `fuzz/fuzz_targets/fuzz_datalog_parser.rs` fuzzes parser input.

## Required Design

Introduce typed query catalogs in test support and benchmarks.

### Shared Workload Catalog

Replace:

```rust
pub fn ledger_queries() -> Vec<&'static str>
```

with:

```rust
pub struct TestQuerySpec {
    pub name: &'static str,
    pub build: fn(&SchemaDescriptor) -> QueryBuildResult<TypedQuery>,
}

pub fn ledger_queries() -> Vec<TestQuerySpec>;
```

### Benchmark Query Catalog

Replace:

```rust
pub(crate) struct BenchQuery {
    name: &'static str,
    datalog: &'static str,
    inputs: Vec<(&'static str, Value)>,
    sqlite: &'static str,
    sqlite_params: Vec<SqlParam>,
}
```

with:

```rust
pub(crate) struct BenchQuery {
    name: &'static str,
    build: fn(&SchemaDescriptor) -> QueryBuildResult<TypedQuery>,
    inputs: Vec<(&'static str, Value)>,
    sqlite: &'static str,
    sqlite_params: Vec<SqlParam>,
}
```

or:

```rust
pub(crate) struct BenchQuery {
    name: &'static str,
    typed: TypedQuery,
    inputs: Vec<(&'static str, Value)>,
    sqlite: &'static str,
    sqlite_params: Vec<SqlParam>,
}
```

Prefer `build` because schemas are often constructed next to datasets.

### Count Detection

Replace string matching:

```rust
query.datalog.contains("count(")
```

with IR inspection:

```rust
fn query_has_count(query: &TypedQuery) -> bool {
    query.find.iter().any(|term| {
        matches!(term, TypedFindTerm::Aggregate {
            function: AggregateFunction::Count,
            ..
        })
    })
}
```

## Migration Requirements

### bumbledb-bench

In `crates/bumbledb-bench/src/main.rs`:

- Remove `use bumbledb_core::datalog::parse_and_typecheck`.
- Add imports for `QueryBuilder`, `QueryBuildResult`, and `TypedQuery`.
- Change `BenchQuery` to store a query builder function or typed query.
- Build the query before `Environment::prepare_query`.
- Replace `query.datalog.contains("count(")` with IR inspection.

In `crates/bumbledb-bench/src/open.rs`:

- Replace JOB Datalog strings with builder functions.
- Keep SQL strings unchanged unless count semantics change later.

### bumbledb-lmdb Tests

In `crates/bumbledb-lmdb/src/query.rs` tests:

- Replace all parser calls with query builders.
- If many tests repeat common patterns, add local helper functions rather than recreating verbose builders.

In `crates/bumbledb-lmdb/src/lib.rs` tests:

- Stop using `benchmark_queries()[0].datalog`.
- Use the new benchmark query builder.

In `crates/bumbledb-lmdb/src/benchmark.rs`:

- Remove `BenchmarkQuery.datalog`.
- Add builder functions for benchmark queries.

### Test Support

In `crates/bumbledb-test-support/src/workloads.rs`:

- Replace query text list with typed query specs.

In test files:

- Replace calls that parse workload strings with query builds.

### Fuzz

Do not delete the Datalog fuzz target yet. Mark it unused or leave it for PRD 03 deletion.

Optionally add a new fuzz target later for query builder normalization, but that is not required here.

## Non-Goals

- Do not delete `datalog.rs` in this PRD.
- Do not refactor schema v2 yet.
- Do not change query semantics yet.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- Grep for `parse_and_typecheck` must return only `crates/bumbledb-core/src/datalog.rs` and its internal tests.
- Grep for `.datalog` and `datalog:` must return no benchmark/test caller fields, except old Datalog module internals.

## Completion Criteria

- Benchmarks execute using prebuilt typed IR.
- Tests execute using typed IR builders.
- No runtime path parses query text.
- Datalog exists only as dead code ready for deletion.
