# PRD 11: Tests, Benchmarks, and Fuzz Migration

## Goal

Bring the full verification harness onto the v2 model.

This PRD updates fixtures, benchmark schemas, rows, query catalogs, SQLite comparisons, property tests, and fuzz targets after core/schema/storage/query changes land.

## Current Test/Bench Anchors

- Shared schemas: `crates/bumbledb-test-support/src/schemas.rs:1-132`
- Shared rows: `crates/bumbledb-test-support/src/rows.rs:1-107`
- Shared workloads: `crates/bumbledb-test-support/src/workloads.rs:1-28`
- Property operation helpers: `crates/bumbledb-test-support/src/operations.rs:1-52`
- Reference executor: `crates/bumbledb-test-support/src/reference.rs:1-497`
- SQLite comparison helpers: `crates/bumbledb-test-support/src/sqlite.rs:1-142`
- Synthetic benchmark schemas/queries: `crates/bumbledb-bench/src/main.rs:1900-2832`
- Open/JOB schemas/queries: `crates/bumbledb-bench/src/open.rs:1-2619`
- LMDB query tests: `crates/bumbledb-lmdb/src/query.rs:9519+`
- LMDB storage tests: `crates/bumbledb-lmdb/src/storage.rs:2142+`
- Query image tests: `crates/bumbledb-lmdb/src/query_image.rs:1531+`
- Sorted/hash trie tests: `crates/bumbledb-lmdb/src/sorted_trie.rs`, `hash_trie.rs`

## Required Fixture Style

All test schemas must use the v2 explicit style.

Example helper:

```rust
pub fn identity(type_name: &str, owning_relation: &str) -> ValueType {
    ValueType::Identity {
        type_name: type_name.to_owned(),
        owning_relation: owning_relation.to_owned(),
        allocation: IdentityAllocation::Serial,
    }
}
```

Example relation:

```rust
RelationDescriptor::new(
    "Holder",
    vec![
        FieldDescriptor::new("id", identity("HolderId", "Holder")),
        FieldDescriptor::new("name", ValueType::String),
    ],
)
.with_covering_unique("by_id", ["id"])
.with_constraint(ConstraintDescriptor::unique("name", ["name"]))
```

Example FK:

```rust
RelationDescriptor::new(
    "Account",
    vec![
        FieldDescriptor::new("id", identity("AccountId", "Account")),
        FieldDescriptor::new("holder", identity("HolderId", "Holder")),
        FieldDescriptor::new("currency", ValueType::Enum { name: "Currency".to_owned() }),
    ],
)
.with_covering_unique("by_id", ["id"])
.with_constraint(ConstraintDescriptor::unique("holder_currency", ["holder", "currency"]))
.with_constraint(ConstraintDescriptor::foreign_key("holder_fk", ["holder"], "Holder", "by_id"))
```

## Required Row Style

Replace:

```rust
Value::Id(id)
Value::Ref(holder)
Value::Code(code)
```

with:

```rust
Value::Identity(IdentityValue::Serial(id))
Value::U64(code)
```

If implementation chooses `Value::Identity(u64)` initially, use that consistently.

## Required Query Style

All workload queries must use query builder specs from PRD 01.

No test should parse query text.

No benchmark should inspect query source text.

## SQLite Comparison Rules

SQLite remains a comparison baseline, but its bag semantics must be normalized to Bumbledb set semantics.

Rules:

- Projection SQL must use `SELECT DISTINCT` when comparing materialized rows.
- Count of projection rows should count distinct projected tuples.
- Aggregate SQL must match the explicit aggregate semantics decided in PRD 09.
- If global `count(empty)` is one row `[0]`, SQL comparison must account for that.

## Property Tests

Update property operations:

- Remove replace-by-primary-key operations.
- Insert duplicate exact rows and assert idempotency.
- Delete exact rows and assert absent delete is non-error.
- Generate FK-valid rows in relation order.
- Generate unique-conflicting rows as negative tests.

Current property helpers at `crates/bumbledb-test-support/src/operations.rs:8-19` include `Replace`, `DeleteHolder`, and `DeleteAccount`. Replace them with set operations:

```rust
pub enum Operation {
    Insert(Row),
    Delete(Row),
}
```

## Fuzz Targets

Keep encoding fuzz.

Delete parser fuzz in PRD 03.

Add optional new fuzz target:

```text
fuzz/fuzz_targets/fuzz_query_builder.rs
```

It should generate small schemas and builder operations, then assert one of:

- builder rejects invalid shape cleanly
- builder produces dense IDs and normalization does not panic

This is optional for this PRD but recommended if time permits.

## Benchmark Preservation

The benchmark suite must still run:

- non-JOB preset
- practical JOB preset with `--job-dir` when dataset exists

But exact performance numbers may shift during architecture cleanup. This PRD should not enforce old microsecond gates unless final PRD chooses to preserve performance gates.

Correctness is mandatory. Performance regression should be reported, not silently accepted.

## Tests Required

- Shared ledger schema validates under v2.
- Changed ledger schema changes fingerprint.
- Shared row fixtures insert successfully.
- Exact duplicate fixtures are idempotent.
- Unique-conflict fixtures fail.
- FK violation fixtures fail.
- SQLite comparison tests pass with set semantics.
- Reference executor agrees with LMDB executor.
- Bench query catalogs build all queries without parser.
- JOB query catalog builds all queries without parser.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- `cargo run -p bumbledb-bench --release -- --preset nonjob --runs 1` or equivalent smoke command, if CLI supports run count.

## Completion Criteria

- All tests and benchmarks are on v2 schema/value/query APIs.
- No test support code imports or mentions Datalog as an active frontend.
- No test fixture uses old Id/Ref/Code variants.
- SQLite comparison explicitly accounts for set semantics.
