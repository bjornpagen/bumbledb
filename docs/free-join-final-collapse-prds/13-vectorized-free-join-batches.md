# PRD 13: Vectorized Free Join Batches

## Status

Not started.

## Objective

Add vectorized Free Join batching for iteration and probe operations.

## Problem

Current LFTJ recursion processes one candidate at a time. The paper's vectorized algorithm batches outer iteration and grouped probes for locality.

## Required Direction

Introduce batch APIs inside the Free Join runtime without adding a second algorithm. The scalar path may remain only as a degenerate batch size of one during transition.

## Implementation Steps

1. Add batch iterator abstraction for lazy access slices.
2. Add batched seek/probe loops for Free Join nodes.
3. Preserve exact result-set semantics.
4. Add batch counters and tracing spans.
5. Add tests for batch size one and larger batch sizes producing identical results.
6. Benchmark renderer reports batch counters.

## Passing Criteria

- Query execution can run with batch size greater than one.
- Batch and scalar-equivalent outputs are identical.
- No separate runtime family is introduced.
- Full validation passes.

## Failure Modes

- Adding vectorization as a second query engine is failure.
- Changing output facts under batching is failure.
- Dropping tracing is failure.

## Completion

Delete this PRD and commit.
