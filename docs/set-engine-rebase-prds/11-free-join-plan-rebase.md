# PRD 11: Free Join Plan Rebase

## 01. Status

Not started.

## 02. Severity

High architecture.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must understand the Free Join paper plan model.

The implementer must not delete LFTJ.

The implementer must not leave FreeJoinPlan as explain-only metadata.

The implementer must preserve existing query semantics while replacing plan authority.

## 04. Dependency Order

PRDs 01 through 04 are mandatory.

PRD 09 and PRD 10 should be complete first if payload demand is already used.

PRD 12 depends directly on this PRD.

PRD 13 depends directly on this PRD.

PRD 14 depends directly on this PRD.

PRD 15 depends directly on this PRD.

## 05. Problem Statement

The current `FreeJoinPlan` is not the actual general physical plan.

It mostly describes pure single-variable LFTJ.

`NodeImpl` has only `SortedLeapfrog`.

Validation assumes subatom variables must be bound by the same node.

Execution reconstructs atom plans from normalized query atoms rather than executing plan nodes as authority.

Direct kernels are separate from FreeJoinPlan.

Hash-trie paths are separate from FreeJoinPlan.

There is no plan representation for multi-variable covers.

There is no plan representation for binary-style iteration and probe.

There is no plan representation for COLT/lazy GHT.

This blocks the rest of the rebase.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/free_join.rs`.
- `crates/bumbledb-lmdb/src/query.rs`.
- Free Join access abstraction code.
- `crates/bumbledb-lmdb/src/sorted_trie.rs`.
- Free Join lazy access code.

Relevant current regions:

- `free_join.rs:6-15` for plan shell.
- `free_join.rs:43-48` for sorted-leapfrog Free Join detection.
- `free_join.rs:51-99` for node, subatom, and payload definitions.
- `free_join.rs:66-71` for single implementation variant.
- `query.rs:5333-5422` for LFTJ execution setup.
- `query.rs:7342-7402` for current plan building.

## 07. Current Behavior

Planner chooses variable order.

Planner creates one FreeJoin node per variable.

Each node binds a single variable.

Every node uses `SortedLeapfrog`.

The executor checks that node order matches variable order.

The executor builds LFTJ atom plans from `query.atoms`.

The executor does not execute node covers or probes from `FreeJoinPlan`.

`SubAtom.access` is mostly descriptive.

Node-level output demand metadata is absent until it is made executable.

Direct paths build their own execution plans outside FreeJoinPlan.

## 08. Target Behavior

`FreeJoinPlan` is the physical plan authority.

Execution dispatches by plan nodes.

Plan nodes can bind one variable or multiple variables.

Plan nodes can iterate one cover and probe other subatoms.

Plan nodes can represent LFTJ value intersection.

Plan nodes can represent hash/GHT lookup.

Plan nodes can represent durable access iteration.

Plan nodes can represent planned lazy GHT/COLT execution.

Direct kernels become FreeJoin node implementations or are explicitly isolated for deletion.

Pure LFTJ remains a valid specialization.

## 09. Research Context

Free Join unifies binary-style joins and worst-case optimal joins.

A Free Join node specifies subatoms, available variables, bound variables, iteration cover, and probes.

Generic Join is a special case where each node binds one variable and intersects all participating relation projections.

Binary join is a special case where each node iterates/probes relation-shaped keys.

Bumbledb currently implements only the Generic Join-like slice.

The plan must be generalized before factoring, COLT, vectorization, or cover choice can be implemented correctly.

## 10. Definitions

Available vars are variables bound by prior nodes.

Bind vars are variables introduced by the current node.

Cover is the subatom or access source iterated by the node.

Probe is a subatom or access source checked using available and current node values.

Node payload is the projected or aggregate information that becomes available at the node.

Implementation kind is the physical strategy used by a node.

Pure LFTJ node is a node that binds one variable through sorted trie intersection.

Hash probe node is a node that probes hash/GHT access with available keys.

Access iteration node is a node that streams facts or key entries from a durable access path.

## 11. Desired Invariants

Every probe key uses only available vars plus current node bind vars.

No node may require a variable that is neither available nor bound by the node.

No relation atom partition may be consumed twice inconsistently.

Node order is deterministic.

Plan validation catches unavailable probe variables.

Plan validation catches duplicate same-atom subatoms in one invalid context.

Execution uses plan nodes, not reconstructed assumptions.

Pure LFTJ plans continue to execute correctly.

## 12. Data Structure Plan

Redesign `PlanNode` fields if needed.

Represent cover candidates explicitly.

Represent selected cover explicitly.

Represent probes explicitly.

Represent subatom field-variable mappings explicitly.

Represent required access path explicitly.

Represent implementation kind explicitly.

Represent payload demand at node level.

Keep existing `FreeJoinPlan.output`.

Keep existing `PlanEstimates` but extend as needed later.

## 13. NodeImpl Plan

Keep `SortedLeapfrog`.

Add `HashProbe` or equivalent.

Add `AccessIterProbe` or equivalent.

Add `LazyGht` placeholder for PRD 13.

Add `Vectorized` only in PRD 14, not here.

Every implementation kind must have a validation rule.

Every implementation kind must have either an executor or be rejected before execution.

Do not add unused variants without tests unless they are explicitly blocked from selection.

## 14. Validation Plan

Validate dense node IDs.

Validate available variable progression.

Validate bind vars are new unless intentionally rebound for equality.

Validate probe vars are subset of available plus bind vars.

Validate cover vars include all new bind vars required by node.

Validate field and variable lengths match.

Validate access IDs exist in schema.

Validate relation IDs exist in schema.

Validate output payload vars are bound by or before the node where emitted.

Validate sorted-leapfrog Free Join specialization remains valid.

## 15. Execution Plan

Introduce an executor that consumes `FreeJoinPlan.nodes`.

For sorted-leapfrog Free Join, route to existing LFTJ mechanics through the plan representation.

For unsupported implementation kinds, return a clear internal error unless the planner never selects them.

Move atom-plan construction behind node execution abstractions.

Preserve counters.

Preserve timings.

Preserve plan summaries.

Preserve result semantics.

Do not optimize in this PRD beyond plan authority cleanup.

## 16. Direct Kernel Position

Do not fully rewrite retired auxiliary paths here unless it is small.

Add a documented bridge plan if retired auxiliary paths remain separate temporarily.

Mark removed-path cleanup as a PRD 15 target.

Do not let retired auxiliary paths bypass correctness fixes from PRD 02.

Do not broaden removed-path selection here.

## 17. Required Unit Tests

Manual sorted-leapfrog Free Join plan validates.

Manual multi-variable cover plan validates.

Manual probe using prior available vars validates.

Manual probe requiring unavailable vars fails validation.

Manual plan with bad relation ID fails validation.

Manual plan with bad access ID fails validation.

Manual plan with duplicate invalid subatom consumption fails validation.

Manual plan with payload vars not bound fails validation.

## 18. Required Query Tests

Single-relation query executes through FreeJoinPlan authority.

Two-relation join executes through FreeJoinPlan authority.

Triangle or cyclic query executes through FreeJoinPlan authority.

Projection query executes correctly.

Aggregate query executes correctly.

Prepared query executes correctly.

Plan explanation shows nodes from FreeJoinPlan.

## 19. Required Diagnostics

Plan summary must show node implementation.

Plan summary must show bind vars.

Plan summary must show cover if selected.

Plan summary must show probes.

Plan summary must show payload demand.

Existing optimizer trace must remain stable enough for tests.

If trace shape changes, update tests intentionally.

## 20. Migration Strategy

Step one: extend data structures and validation.

Step two: adapt existing sorted-leapfrog Free Join plan generation to new structures.

Step three: route existing LFTJ execution through new plan authority.

Step four: keep old helper functions only if used internally by the new executor.

Step five: delete explain-only assumptions.

Step six: update tests and docs.

## 21. Passing Criteria

`FreeJoinPlan` can represent more than single-variable LFTJ nodes.

Plan validation uses available variables, not only same-node bound variables.

Query execution consumes FreeJoinPlan as physical authority for the normal LFTJ path.

Pure LFTJ remains correct.

Multi-variable node validation is tested.

Unavailable probe validation is tested.

The global validation gate passes.

The query-focused validation gate passes.

## 22. Failure Modes

Leaving FreeJoinPlan as metadata is a failure.

Deleting LFTJ is a failure.

Adding implementation variants that can be selected but cannot execute is a failure.

Weakening plan validation is a failure.

Moving retired auxiliary paths into FreeJoinPlan before fixing correctness is a failure.

Changing query results is a failure.

## 23. Non-Goals

Do not implement factoring.

Do not implement COLT.

Do not implement vectorized batches.

Do not implement full cover-cost optimizer.

Do not change storage layout.

Do not change public query APIs.

## 24. Completion Notes

Update architecture docs to state FreeJoinPlan is executable.

Record unsupported implementation variants clearly.

Keep validation tests permanent.

This PRD unlocks the rest of the Free Join rebase.
