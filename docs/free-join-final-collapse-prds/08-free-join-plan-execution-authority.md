# PRD 08: Free Join Plan Execution Authority

## Status

Not started.

## Objective

Make execution consume `FreeJoinPlan` as authority instead of rebuilding atom plans from normalized query atoms as a parallel structure.

## Problem

Execution currently checks the Free Join plan shape but still uses `plan.relation_atoms`, `plan.variable_order_ids`, and normalized atoms to build access sources. This leaves `FreeJoinPlan` partly explanatory.

## Required Direction

`ExecutionPlan` should not carry duplicate atom/order structures that can diverge from `FreeJoinPlan`. If data is needed at runtime, it must be referenced by plan nodes or by a single normalized query reference.

## Implementation Steps

1. Audit `ExecutionPlan` fields for duplication with `FreeJoinPlan`.
2. Move atom/subatom access requirements into `PlanNode`/`SubAtom` if needed.
3. Build atom access sources from `FreeJoinPlan` nodes.
4. Remove `ExecutionPlan.relation_atoms` if redundant.
5. Remove `ExecutionPlan.variable_order_ids` if derivable from nodes.
6. Add tests that corrupting plan/normalized divergence is impossible.

## Passing Criteria

- Runtime access-source construction reads node/subatom plan data.
- No duplicate variable order vector exists unless justified by tests.
- `FreeJoinPlan::validate` catches invalid execution shapes.
- Full validation passes.

## Failure Modes

- Keeping a parallel execution plan that can disagree with `FreeJoinPlan` is failure.
- Weakening validation to make old code compile is failure.
- Adding unexecutable node variants is failure.

## Completion

Delete this PRD and commit.
