# PRD 10: Delete Explanatory Free Join Plan Surface

## Status

Not started.

## Current State

As of `ec25f85`, `FreeJoinPlan` no longer contains fake implementation choices or plan-cost estimates. Runtime derives variable order from `FreeJoinPlan.nodes[].bind_vars`, which is real execution data.

However, `PlanNode.subatoms` and `SubAtom` are still built only for validation/explain output. Runtime builds LFTJ atom plans from `NormalizedQuery.atoms` in `query/lftj_runtime.rs` and `query/lftj_access.rs`; it does not consume `FreeJoinPlan.subatoms`. This makes the plan look more authoritative than it is.

## Objective

Delete Free Join plan fields that are not consumed by execution. The physical plan must either execute or disappear.

## Required Direction

Take the smallest correct path: delete `SubAtom` and `PlanNode.subatoms` unless implementation work in this PRD makes runtime consume them directly. Do not keep explanatory copies for pretty explain output.

`FreeJoinPlan` should remain only:

- dense node order with exactly one bound variable per node
- output/projection plan
- validation for the executable node-order shape

## Implementation Steps

1. Audit every use of `PlanNode.subatoms`, `SubAtom`, and `free_join_subatom` explain text.
2. Delete `SubAtom` and the `subatoms` field from `PlanNode` if no runtime use is added.
3. Remove subatom construction from `build_free_join_plan`.
4. Remove subatom validation branches from `FreeJoinPlan::validate`.
5. Remove public `SubAtom` export and any tests that assert subatom explain text.
6. Keep `FreeJoinPlan::validate` strict about dense node IDs and one variable per node.
7. Add or update tests proving variable order still comes from `FreeJoinPlan` nodes.

## Passing Criteria

- Zero Rust matches for `SubAtom`.
- Zero Rust matches for `free_join_subatom`.
- `FreeJoinPlan` contains no field that runtime ignores except documented diagnostics required by a later PRD.
- Query execution still derives variable order from `FreeJoinPlan.nodes`.
- Full validation passes.

## Failure Modes

- Keeping plan subatoms only for explain output is failure.
- Moving subatoms into another struct without runtime use is failure.
- Weakening plan validation is failure.
- Reintroducing implementation/cost/candidate fields is failure.

## Completion

Delete this PRD and commit.
