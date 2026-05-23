# PRD 07: Collapse Query Plan Diagnostics

## Status

Not started.

## Objective

Reduce `QueryPlan` to diagnostics that directly correspond to the remaining Free Join execution path.

## Problem

`QueryPlan` still contains broad optimizer and execution diagnostics that made sense when multiple side paths existed. With one Free Join path, diagnostics should be small, concrete, and non-sticky.

## Remove Or Rename

- `uses_indexed_multiway_join`
- `hash_build_facts`
- candidate rank fields if only one candidate remains
- implementation masks if only one implementation remains
- missing index recommendations if not used by benchmarks/tests
- node timing fields that are always zero
- counters for removed execution structures

## Keep

- variable order
- Free Join plan summary
- query image/cache stats required by benchmarks
- timing summary
- allocation summary
- essential counters for LFTJ/lazy access/projection

## Implementation Steps

1. Audit every `QueryPlan` field and every `PlanCounters` field.
2. Delete fields not asserted by meaningful tests or benchmark gates.
3. Rename stale fields to Free Join terms.
4. Update explain output and benchmark JSON/markdown.
5. Update tests to assert behavior/results, not deleted diagnostics.

## Passing Criteria

- `QueryPlan` has no stale or single-path classifier fields.
- `PlanEstimates` contains no hash-specific field names unless a real hash implementation exists.
- Benchmark output has no removed diagnostic fields.
- Full validation passes.

## Failure Modes

- Keeping zero-valued counters for later is failure.
- Renaming stale fields without changing meaning is failure.
- Deleting tracing spans is failure.

## Completion

Delete this PRD and commit.
