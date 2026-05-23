# PRD 17: Hard Module Split Gate

## Status

Not started.

## Current State

This PRD runs only after behavior is smaller. Current large/hard-to-review files include:

- `crates/bumbledb-lmdb/src/storage.rs`
- `crates/bumbledb-lmdb/src/query_image.rs`
- `crates/bumbledb-core/src/schema.rs`
- `crates/bumbledb-core/src/schema_tests.rs`
- `crates/bumbledb-core/src/query_builder.rs`
- `crates/bumbledb-test-support/tests/golden_examples.rs`
- `crates/bumbledb-bench/src/main/datasets.rs`
- `crates/bumbledb-bench/src/open.rs`
- benchmark `include!`-based pseudo-modules in `main.rs` and `open.rs`

## Objective

Enforce the small-file source style. No production Rust file remains huge, and large test fixtures are split by topic.

## File Size Targets

Production Rust files:

- hard target: under 500 lines
- temporary exception: under 800 lines for schema/storage/query-image modules if actively shrinking

Test files:

- hard target: under 700 lines
- fixture helper files may exceed only with documented reason

## Required Splits

- split `storage.rs` into value/fact/cursor/write/read/constraints/dictionary/layout helpers
- split `query_image.rs` into key/scope/image/cache/relation/access/columns/builder
- split `schema.rs` into descriptors/validation/access_layout/fingerprint/value_type
- split planner and lazy access files if over target
- split benchmark loaders by dataset and query family
- replace benchmark `include!` chunks with semantic Rust modules where visibility allows
- split large golden/schema tests by family/topic

## Passing Criteria

- A line-count script reports no production file over target unless listed as a temporary exception in README.
- No file contains unrelated test fixtures and production code together.
- Module names describe one responsibility.
- Splits do not widen visibility just to compile.
- Full validation passes.

## Failure Modes

- `include!` chunks with no semantic names are acceptable only as an intermediate step; final structure must be real modules where visibility allows.
- Moving code without reducing file size is failure.
- Widening visibility to split files is failure.

## Completion

Delete this PRD and commit.
