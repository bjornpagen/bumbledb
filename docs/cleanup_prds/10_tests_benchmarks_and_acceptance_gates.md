# PRD 10: Tests, Benchmarks, And Acceptance Gates

## Status

Draft. This PRD applies to every cleanup PRD.

## Problem

The codebase has good unit/property/failpoint coverage, but benchmark correctness is often row-count-only, parser fuzzing does not typecheck or execute, crash tests are ignored, and performance gates do not yet enforce the desired non-join fast paths.

## Goals

- Make correctness gates stricter.
- Make performance gates reflect target runtime choices.
- Add schema/constraint/enum negative coverage.
- Preserve JOB and triangle performance while improving simple workloads.
- Ensure docs-only and code PRDs have clear acceptance requirements.

## Non-Goals

- No new benchmark feature scope beyond existing relational workloads.
- No vector/FlatBuffer benchmarks.
- No migration tests.

## Current Code References

- Benchmark harness in `crates/bumbledb-bench/src/main.rs`.
- JOB/open datasets in `crates/bumbledb-bench/src/open.rs`.
- Ledger benchmark fixture in `crates/bumbledb-lmdb/src/benchmark.rs`.
- Reference evaluator in `crates/bumbledb-test-support/src/reference.rs`.
- Property/differential tests in `crates/bumbledb-test-support/tests/property_and_differential.rs`.
- Failpoint tests in `crates/bumbledb-test-support/tests/failpoints.rs`.
- Fuzz targets in `fuzz/fuzz_targets`.
- Benchmark scripts in `scripts/`.

## Required Correctness Coverage

### Schema Validation

Add negative tests for:

- duplicate relation
- duplicate field
- empty primary key
- duplicate primary key field
- unknown primary key field
- invalid generated ID
- duplicate enum
- duplicate enum variant
- duplicate index
- reserved index collision
- duplicate constraint
- invalid FK arity
- invalid FK target
- incompatible FK types

### Constraints

Add tests for:

- single-field unique insert violation
- compound unique insert violation
- replace into conflicting unique value
- delete then reinsert unique value
- single-field FK insert/delete restrict
- compound FK insert/delete restrict
- transactional rollback after FK/unique failure

### Enums

Add tests for:

- enum value insert/decode
- invalid enum value rejection
- enum domain mismatch in query input
- enum equality filter
- enum join domain mismatch rejection

### Direct Paths

Add tests for:

- primary lookup runtime kind
- unique lookup runtime kind
- prefix scan runtime kind
- range scan runtime kind
- direct count runtime kind
- index nested-loop runtime kind
- direct path result equality vs reference evaluator

## Required Benchmark Improvements

The benchmark harness must support exact output comparisons where practical.

Required changes:

- Add optional exact SQLite result comparison mode for selected queries.
- Keep row-count comparison for queries where SQL aggregate shape intentionally differs, but document each exception.
- Emit runtime family in JSON and markdown.
- Emit whether query image was built during query execution.
- Emit prepared shape cache stats after PRD 07/08.

## Required Performance Gates

Existing gates remain:

- `joinstress/triangle_count`
- `ledger/tag_lookup_join`
- `sailors/red_boat_sailors`
- `sailors/sailor_range_reserves`
- `joinstress/chain4_from_a`
- `tpch/supplier_nation_orders`

Add runtime-kind gates:

- `sailors/sailor_range_reserves` must be direct range/prefix runtime.
- `joinstress/chain4_from_a` must be direct or index nested-loop runtime.
- `joinstress/triangle_count` must remain Free Join/LFTJ or approved WCOJ runtime.
- Single relation point lookup tests must not build hash trie or sorted trie.

Add non-JOB SQLite competitiveness gates after PRD 07:

- `sailors/sailor_range_reserves` average must be within 3x SQLite or faster.
- `joinstress/chain4_from_a` average must be within 3x SQLite or faster.
- `ledger/postings_for_holder_range` must materially improve from current baseline.
- Any exceptions require recorded benchmark artifact and explanation.

## Fuzz Requirements

Near-term:

- Existing parser fuzz target continues compiling until custom parser deletion.
- Encoding fuzz target continues compiling.

After query IR cleanup:

- Add typed IR validation fuzz target.
- Add generated small-schema differential query test or fuzz-like proptest.

After Logica implementation later:

- Replace Datalog parser fuzz with Logica parser/lowering fuzz.

## Required Verification Commands

Every code PRD must state which commands are required. Default full gate:

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
scripts/bench-focused.sh --fail-gates
```

JOB-sensitive PRDs must also run a JOB benchmark when local data exists:

```sh
cargo run -p bumbledb-bench --release -- --job-dir "$JOB_DIR" --dataset job --scale 10000 --warmup 2 --repeats 10 --format markdown --fail-gates
```

## Strict Passing Criteria

- Every PRD adds or updates tests covering its behavior.
- Bench harness can report exact result equality for selected queries.
- New direct/runtime-kind gates exist after PRD 07.
- All existing failpoint tests still pass.
- Benchmark JSON/markdown include enough counters to diagnose regressions.
- No PRD is considered complete with only row-count correctness when exact result equality is practical.
