# 12 Benchmark And Measurement Contract

## Purpose

Make benchmarks honest under hard set semantics. Correctness must compare exact result sets and aggregate values before timing numbers matter.

## Current Bad Shape

`sqlite_count()` counts returned SQLite rows. For `SELECT COUNT(*)`, this only verifies that SQLite returned one row, not that Bumbledb computed the same count value.

Rows-mode fabricates fake `QueryOutput` rows from cardinality-only results.

## Required Benchmark Model

Each benchmark query declares one correctness mode:

```text
ResultSet
AggregateValues
CardinalityOnlyPerformance
```

Only `ResultSet` and `AggregateValues` are correctness modes. `CardinalityOnlyPerformance` may be reported only after a correctness run for the same query has passed.

SQLite reference SQL must be set-correct:

- projection SQL uses `SELECT DISTINCT`
- aggregate SQL uses domain-correct subqueries
- count SQL returns scalar values that are compared, not just counted as one row

## Required Code Changes

- Replace `sqlite_count` correctness with typed SQLite row/value materialization.
- Add benchmark metadata for result shape and correctness mode.
- Remove `count_output_as_query_output`.
- Emit benchmark JSON fields distinguishing result cardinality, aggregate scalar values, and performance-only cardinality.
- Update gates to fail on correctness mismatch before timing comparison.

## Acceptance Gates

- Every benchmark query validates exact Bumbledb vs SQLite values at least once.
- Every `COUNT` benchmark compares the count value.
- Prepared-result cache hits are explicitly reported and never hidden as normal execution.
- Non-JOB and JOB subset benchmarks pass correctness before performance gates.
- Benchmark output labels cold, warm, prepared-plan, and prepared-result modes honestly.

## Tests Required

- Unit test proving SQLite `COUNT(*)` mismatch is caught.
- Unit test proving projection duplicate mismatch is caught.
- JSON output test for new correctness mode fields.
- Benchmark smoke test for each dataset preset with minimal scale.

## Non-Goals

- No SQL frontend support.
- No performance-only benchmark without correctness preflight.
