# PRD 04: LFTJ Baseline And Generic Join Special Case

## Purpose

Stop mislabeling the current singleton-variable executor as full paper Free Join. Preserve it as an LFTJ/Generic Join baseline and, where useful, lower it into a validated singleton-subatom Free Join special case.

## Dependencies

- PRD 03.

## Scope

- `crates/bumbledb-lmdb/src/free_join.rs`
- `crates/bumbledb-lmdb/src/query/planner.rs`
- `crates/bumbledb-lmdb/src/query/lftj_runtime.rs`
- `crates/bumbledb-lmdb/src/query/explain.rs`
- Query tests that assert singleton Free Join nodes.

## Required Changes

- Rename current plan/executor internals to LFTJ or Generic Join terminology unless the type is replaced entirely by the formal PRD 03 plan.
- Remove explain output that calls singleton `bind_vars` nodes a paper `free_join_plan`.
- Add a lowering path from current variable order to a formal singleton-subatom Free Join plan.
- Validate that lowered plan with the PRD 03 validator.
- Keep the existing LFTJ executor as a baseline execution mode behind explicit names.
- Make tests assert LFTJ mode where they depend on singleton variable behavior.
- Add tests proving the lowered singleton plan is a valid Free Join plan, while the executor remains LFTJ until PRD 12.

## Technical Direction

- If a direct rename causes too much churn, introduce `LftjPlanSummary` and leave a short-lived adapter with clear comments.
- `execute_free_join` must not simply dispatch to LFTJ under a misleading name after this PRD.
- If a compatibility method remains, its name must make the special case explicit, such as `execute_lftj_as_generic_join`.
- Explain output should say `execution_mode: lftj` for the old path.
- Formal plans should say `plan_model: free_join` only when they contain subatoms/covers.

## Non-Goals

- Do not implement node/cover Free Join execution here.
- Do not delete LFTJ yet.
- Do not implement `binary2fj` here.

## Acceptance Criteria

- No explain output falsely presents singleton `bind_vars` as the entire paper Free Join model.
- The existing LFTJ executor still passes all query correctness tests.
- The current variable-order planner can produce a formal singleton Free Join plan and validate it.
- Tests that previously asserted `free_join.nodes[*].bind_vars.len() == 1` are renamed or replaced so they assert LFTJ baseline behavior, not paper Free Join behavior.
- Public exports do not expose internal `FreeJoinPlan` fields that are not formal paper Free Join.

## Required Tests

- LFTJ baseline still returns exact golden outputs.
- Lowered singleton plan validates under PRD 03.
- Explain for old execution says LFTJ or Generic Join special case.
- Formal Free Join validator still accepts multi-variable cover nodes independent of the LFTJ path.

## Validation Commands

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo test -p bumbledb-lmdb query --all-features
cargo test -p bumbledb-test-support --test golden_examples --all-features
rg "free_join_node id=.*bind_vars|Free Join node must bind one variable" crates docs/ROSETTA_STONE.md
```

The final `rg` must return no production/explain claims after this PRD. Test names may mention removed behavior only if asserting legacy rejection no longer exists.
