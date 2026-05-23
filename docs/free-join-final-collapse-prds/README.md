# Free Join Final Collapse PRD Suite

## Status

Completed. The ordered final-collapse suite has been applied. Completed and cancelled PRD files were removed according to project policy.

## Result

Bumbledb is reduced to a minimal embedded set database with one query execution architecture: Free Join with lazy durable access.

The final state keeps:

- LMDB-backed embedded environment and transactions.
- typed schema descriptors and schema validation.
- fact insert, exact fact delete, and bulk ETL loading.
- set query execution returning duplicate-free result facts.
- tracing, benchmarks, allocation telemetry, fuzz checks, and correctness fixtures.
- private scoped query images and private planner/runtime internals.

The final state removes the obsolete alternate query paths, stale benchmark mechanics, broad query-image loading, broad public storage scans, and oversized source layout that motivated this suite.

## Current Contract

- Relations are sets of full facts.
- Query results are sets of result facts.
- Free Join is the only query execution algorithm.
- LFTJ is an internal Free Join implementation technique.
- Lazy durable access is the retained access abstraction.
- Typed IR and the Rust public query interface remain unstable; execution-boundary validation rejects malformed IR.
- Query images are private, scoped, bounded, and compact enough for the current embedded engine.
- Raw storage cursors and encoded access components are not exported as public API.
- Large source files are split by responsibility and checked by `scripts/check-line-counts.sh`.

## Validation Gate

The completed suite was validated with:

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
bash scripts/check-line-counts.sh
git diff --check
```

Query-focused validation remains:

```text
cargo test -p bumbledb-test-support --test golden_examples --all-features
cargo test -p bumbledb-test-support --test property_and_differential --all-features
cargo test -p bumbledb-test-support --test sqlite_comparison --all-features
```

Benchmark renderer validation remains:

```text
cargo test -p bumbledb-bench --bin bumbledb-bench renderer --all-features
```
