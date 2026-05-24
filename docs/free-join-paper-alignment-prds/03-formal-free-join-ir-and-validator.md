# PRD 03: Formal Free Join IR And Validator

## Purpose

Add the paper's Free Join plan representation and validator without changing execution yet. This PRD creates the hard boundary for the rebuilt engine: only plans with subatoms, atom partitions, and covers are formal Free Join plans.

## Dependencies

- PRD 00.
- PRD 02.

## Scope

- New or replacement plan module in `crates/bumbledb-lmdb/src/`.
- Existing `free_join.rs` if retained.
- Unit tests for plan construction and validation.

## Required Types

- `PlanVariableId` or reuse validated `VarId` with bounds checks.
- `AtomOccurrenceId` from PRD 02.
- `PlanSubatom { atom, vars, field_ids }` where `vars` is ordered and belongs to the atom occurrence.
- `FreeJoinNode { id, subatoms, available_vars, new_vars, covers }` or equivalent derived fields.
- `FreeJoinPlan { nodes, atom_partitions, query_variables }` without output materialization embedded in the formal plan.
- `CoverCandidate { node, subatom, vars }` or equivalent.
- `FreeJoinPlanValidationError` with precise invalid-plan reasons.

## Validation Rules

- Node IDs are dense and ordered.
- Every subatom references a known atom occurrence.
- Every subatom variable belongs to that atom occurrence.
- No subatom contains the same variable twice.
- For each atom occurrence, subatoms across the whole plan partition the atom's variable set exactly once.
- No atom occurrence appears twice in the same node.
- For each node, compute available variables from all prior nodes.
- For each node, compute new variables as node variables minus available variables.
- Every non-empty node has at least one cover containing all new variables.
- Probe subatoms in a node may mention only variables available before the node or variables introduced by the cover tuple.
- Empty-variable static atoms have an explicit representation and validation rule.
- Projection/output handling is not part of formal Free Join validation.
- Aggregation handling is not part of formal Free Join validation; future aggregate consumers must use validated plans rather than changing plan-validity rules.

## Technical Direction

- Keep the validator pure and deterministic.
- Do not use storage indexes or planner stats inside validation.
- Add helper constructors only in tests if the public API is unstable.
- Use paper examples as golden plan shapes: clover binary Free Join, clover Generic Free Join, factorized clover, triangle Generic Join, chain binary-derived plan.
- Make invalid plans fail before any storage/query image access occurs.

## Non-Goals

- Do not implement `binary2fj` here.
- Do not execute formal plans here.
- Do not add COLT here.

## Acceptance Criteria

- A paper-valid multi-variable cover node such as `[R(x, a), S(x)]` validates.
- A singleton Generic Join-style plan validates when its subatoms partition every atom.
- The previous invariant that every node binds exactly one variable is removed from formal Free Join validation.
- Invalid partition, duplicate atom occurrence in one node, missing cover, unavailable probe variable, unknown variable, and duplicate subatom variable all fail with precise errors.
- Formal `FreeJoinPlan` does not own projection or aggregation output.
- Formal `FreeJoinPlan` exposes enough node/subatom/variable ordering metadata for private execution sinks and future aggregate folds, without owning those folds.

## Required Tests

- Valid clover binary plan.
- Valid clover factorized plan.
- Valid clover Generic Join plan.
- Valid triangle singleton plan.
- Valid chain plan from the paper's `binary2fj` example.
- Invalid missing partition.
- Invalid duplicate partition assignment.
- Invalid duplicate relation occurrence in node.
- Invalid missing cover.
- Invalid unavailable variable in probe.
- Invalid variable outside atom occurrence.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb free_join --all-features
cargo test --workspace --all-features
```
