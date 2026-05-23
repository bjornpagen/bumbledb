# PRD 05: Delete Cardinality-Only Query API

## Status

Not started.

## Objective

Remove duplicate cardinality-only execution. The engine executes one query path that returns a result set; callers can ask the result set for cardinality.

## Problem

`execute_result_cardinality`, `execute_prepared_query_cardinality`, `QueryResultCardinality`, and `CardinalitySink` duplicate execution plumbing and preserve a second query API.

## Required Direction

Delete cardinality-only APIs and sink. Benchmarks must materialize exact result sets before measuring or derive cardinality from materialized correctness output.

## Implementation Steps

1. Remove `QueryResultCardinality`.
2. Remove read transaction cardinality methods.
3. Remove `OutputSink::new_count_facts` and `CardinalitySink`.
4. Rewrite benchmarks that use `CompareMode::Facts` to use normal query execution or delete that mode.
5. Remove benchmark fields that distinguish cardinality support/fallback.
6. Update tests to call `output.result.cardinality()`.

## Passing Criteria

- Zero Rust matches for `QueryResultCardinality`.
- Zero Rust matches for `execute_result_cardinality`.
- Zero Rust matches for `CardinalitySink`.
- Benchmarks still compare exact values before timing.
- Full validation passes.

## Failure Modes

- Keeping hidden cardinality wrappers is failure.
- Computing cardinality from incomplete projections is failure.
- Keeping benchmark facts/materialized split fields without need is failure.

## Completion

Delete this PRD and commit.
