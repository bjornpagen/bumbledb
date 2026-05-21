# PRD 02: Instrument Hidden Query Phases

## Goal

Make the benchmark timing gap impossible to hide.

Current artifacts show total query time in the hundreds of milliseconds while named engine subphases add up to only milliseconds. The missing time is likely static semijoin proof or other pre-planning work. This PRD adds explicit timing fields so every significant part of `execute_query` and `execute_prepared_query` is visible.

## Explicit Non-Goals

- No backwards compatibility for JSON benchmark schema.
- No preserving old timing field layout if it hides work.
- No optimizer behavior changes.
- No static proof behavior changes.
- No migration of old benchmark artifacts.

## Current Code Anchors

- `crates/bumbledb-lmdb/src/query.rs`
- `QueryTimings`
- `execute_query`
- `execute_prepared_query`
- `execute_query_count_only`
- `static_query_proves_empty`
- `try_execute_direct_count_query`
- `try_execute_direct_storage_project`
- `crates/bumbledb-bench/src/main.rs`
- JSON renderer for `phase_timing`

## Required Timing Fields

Extend `QueryTimings` with explicit fields:

```rust
pub static_empty_lookup_micros: u128,
pub static_literal_proof_micros: u128,
pub static_semijoin_proof_micros: u128,
pub direct_count_micros: u128,
pub direct_storage_micros: u128,
pub prepared_count_cache_lookup_micros: u128,
pub prepared_count_cache_emit_micros: u128,
pub unaccounted_micros: u128,
```

Exact names may vary, but the fields must distinguish:

- static empty cache lookup
- static literal proof
- static semijoin proof
- direct count execution
- direct storage execution
- prepared count cache lookup and emit
- unaccounted wall-clock gap

## Required Accounting Rule

At the end of query execution, compute:

```text
unaccounted = total - sum(known phases)
```

Do not include `unaccounted_micros` in the sum.

If the total is less than known phase sum due to timer noise, saturate at zero.

## Required Benchmark JSON

Update benchmark JSON `phase_timing` to include all new timing fields.

Update markdown/text renderer to show at least:

```text
static_semijoin_proof_us
direct_count_us
prepared_count_cache_us
unaccounted_us
```

## Required Tests

Add unit tests for timing accounting:

- default timings have zero unaccounted.
- unaccounted saturates to zero if known phases exceed total.
- JSON renderer includes new timing keys.

Add an integration-style query test where static semijoin proof runs and `static_semijoin_proof_micros > 0`.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- non-JOB benchmark JSON includes `static_semijoin_proof_us` and `unaccounted_us`.
- For blown-up non-JOB baseline before PRD 03, the previously hidden time must be visible in a named field.

## Completion Criteria

- No large query wall-clock gap can hide outside named timings.
- Benchmark artifacts can prove whether static proof is the culprit.
- This PRD is deleted and committed after passing.
