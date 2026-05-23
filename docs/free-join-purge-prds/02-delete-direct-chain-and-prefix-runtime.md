# PRD 02: Delete Direct Chain And Prefix Runtime

## Status

Not started.

## Severity

Critical architecture cleanup.

## Prerequisite

PRD 01 must be complete. Direct kernels must no longer be selected by planner or public plan structures.

## Problem

Once selection is gone, the runtime implementation and tests for direct chain and direct prefix/range become dead architecture. Keeping them around invites reactivation and preserves the hybrid model the paper rejects.

## Code To Delete

- `DirectPrefixRangePlan`
- `DirectChainProbePlan`
- `DirectExistenceCheck`
- `DirectChainStep`
- `DirectChainExecutor`
- `execute_direct_kernel`
- `execute_direct_prefix_range`
- `direct_range_variable`
- `direct_term_available_depth`
- `direct_chain_plan_is_valid`
- `direct_prefix`
- `direct_relation` if only used by direct runtime
- `bind_atom_variables` if only used by direct runtime
- `unbind_variables` if only used by direct runtime
- `direct_fact_satisfies_atom`
- `DirectImageFact`
- direct-chain and prefix/range-only tests

## Required Replacement

All behavior must continue through Free Join/LFTJ or static proof. The new tests should assert result equality, not direct-path counters.

## Implementation Steps

1. Delete the runtime structs.
2. Delete executor methods.
3. Delete helper functions only used by those executors.
4. Delete direct-specific counters from `PlanCounters`.
5. Delete direct-specific timing/allocation reporting from benchmark output.
6. Delete or rewrite unit tests that only test direct internals.
7. Keep regression tests as query-result tests if they verify important semantics.
8. Run grep to prove no direct runtime symbols remain.

## Required Tests

- Former direct-chain success query returns exact same result through remaining engine.
- Former direct-chain empty query returns exact empty result through remaining engine.
- Former prefix/range query returns exact same result through remaining engine.
- Former direct-chain post-step existence regressions remain as semantic tests using LFTJ/static proof.
- No tests assert direct-specific counters.

## Strict Passing Criteria

- Zero Rust matches for `DirectChain`.
- Zero Rust matches for `DirectPrefixRange`.
- Zero Rust matches for `direct_prefix`.
- Zero Rust matches for `direct_bind`.
- Zero Rust matches for `direct_batch`.
- Zero Rust matches for `IndexNestedLoop`.
- Full validation gate passes.

## Failure Modes

- Keeping a test-only direct executor is failure.
- Keeping unused direct counters is failure.
- Replacing direct chain with another sidecar chain executor is failure.

## Non-Goals

- Do not delete hash trie yet if still used elsewhere.
- Do not rewrite optimizer beyond removing direct runtime references.
