# PRD 04: Delete Aggregate Query Surface

## Status

Not started.

## Objective

Remove aggregate query support from the minimal Free Join set database. The remaining query surface is set projection only.

## Rationale

The paper allows systems to support aggregation, but aggregation is not necessary for the minimal Free Join engine. Aggregates keep a large amount of IR, validation, sink, benchmark, and reference complexity. They can be reintroduced later with explicit domain proofs.

## Code To Delete

- `AggregateFunction`
- `TypedFindTerm::Aggregate`
- builder methods `find_count_domain`, `find_count_distinct`, `find_sum_over`, `find_min_over`, `find_max_over`
- aggregate output plan variants
- aggregate sink and aggregate states
- aggregate counters
- aggregate-specific benchmark correctness modes
- aggregate-specific reference evaluator code
- aggregate tests and golden examples

## Required Replacement

Projection-only queries return `QueryResultSet` facts. Counting for benchmarks can use result-set cardinality after materialization.

## Implementation Steps

1. Delete aggregate IR variants from `bumbledb-core`.
2. Delete aggregate builder APIs and errors.
3. Delete aggregate normalization/output-plan handling.
4. Delete `AggregateSink` and aggregate counters.
5. Rewrite benchmarks and tests to projection queries or remove aggregate-only fixtures.
6. Update docs to say aggregates are out of scope for the minimal engine.

## Passing Criteria

- Zero Rust matches for `AggregateFunction`.
- Zero Rust matches for `AggregateSink`.
- Zero Rust matches for `TypedFindTerm::Aggregate`.
- No benchmark JSON/markdown contains aggregate mode fields.
- Projection/set correctness tests pass.
- Full validation passes.

## Failure Modes

- Keeping count aggregates just for benchmarks is failure.
- Keeping aggregate states with no public API is failure.
- Replacing aggregates with ad hoc count APIs is failure.

## Completion

Delete this PRD and commit.
