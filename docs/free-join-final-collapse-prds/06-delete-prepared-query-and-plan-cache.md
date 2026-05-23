# PRD 06: Delete Prepared Query And Plan Cache

## Status

Not started.

## Objective

Remove prepared query APIs and prepared physical plan caching. The minimal engine executes typed queries directly.

## Problem

Prepared queries and prepared plan caches add normalization cache state, snapshot scoping, diagnostics, benchmark modes, and invalidation paths. They are useful performance features but not core to a minimal Free Join set database.

## Code To Delete

- `PreparedQuery`
- `Environment::prepare_query`
- `ReadTxn::execute_prepared_query`
- prepared normalized query cache
- prepared physical plan cache in `QueryImage`
- `PreparedPlanCacheDiagnostics`
- benchmark cache mode `prepared-plan`
- tests that only assert prepared cache behavior

## Required Replacement

Use `ReadTxn::execute_query(schema, query, inputs)` everywhere.

## Implementation Steps

1. Delete prepared query struct and constructor.
2. Delete prepared query methods from environment and read txn.
3. Delete prepared plan cache structs and fields from query image.
4. Delete prepared diagnostics from `QueryPlan` and explain output.
5. Remove benchmark cache mode and CLI arguments.
6. Remove or rewrite prepared-cache tests.

## Passing Criteria

- Zero Rust matches for `PreparedQuery`.
- Zero Rust matches for `prepare_query`.
- Zero Rust matches for `PreparedPlanCache`.
- Benchmark CLI no longer mentions prepared-plan.
- Full validation passes.

## Failure Modes

- Keeping cache structs with no public API is failure.
- Keeping benchmark mode aliases is failure.
- Adding a compatibility execution wrapper is failure.

## Completion

Delete this PRD and commit.
