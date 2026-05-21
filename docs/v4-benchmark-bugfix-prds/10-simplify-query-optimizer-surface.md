# PRD 10: Simplify Query Optimizer Surface

## Goal

Reduce complex, brittle query optimizer code that accumulated during recovery work.

The current engine now has static semijoin proof, factorized counts, direct kernels, LFTJ, hash probe, prepared count caches, and multiple cache paths. This PRD trims and organizes them around basic concepts.

## Explicit Non-Goals

- No backwards compatibility with internal optimizer function names.
- No preserving unused code paths.
- No adding new heuristic families.
- No relation-name-specific optimizers.
- No changing storage semantics.

## Required Cleanup

Split or organize query code into explicit modules if not already done enough:

```text
query_access.rs
query_static_proof.rs
query_factorized_count.rs
query_direct.rs
query_sinks.rs
query_tests.rs
```

Exact filenames may vary. The point is to stop hiding everything in one giant file.

## Required Concept Boundaries

### Static Proof

Only proves emptiness.

Must not count rows.

Must not choose execution plans.

### Factorized Count

Only handles global count and count-compatible shapes.

Must not do general projection materialization.

### Direct Kernels

Handle simple direct scans/chains/ranges.

Must not run speculative proof.

### LFTJ/Hash/Mixed

Handle general joins when direct/proof/count paths do not apply.

## Required Deletions

Delete or collapse:

- dead helper functions
- unused NodeImpl variants
- unused cache code
- broad speculative fallback functions that are no longer needed
- duplicated prefix construction helpers

Do not delete code just to reduce line count if tests depend on it. Delete code because a clearer concept owns the behavior.

## Required Tests

Existing tests should continue to pass.

Add tests if moving code reveals missing coverage.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB gates still pass after cleanup.
- JOB q09/q16/q24 gates still pass after cleanup.

## Completion Criteria

- Query code is easier to navigate.
- Static proof, factorized count, direct kernels, and general joins have clear ownership.
- This PRD is deleted and committed after passing.
