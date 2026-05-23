# PRD 13: Dynamic Cover Selection

## Purpose

Implement the paper's runtime cover choice. A Free Join node may have multiple covers, and execution should iterate the cover with the fewest exact keys or best safe estimate.

## Dependencies

- PRD 12.

## Scope

- Cover candidate enumeration.
- Runtime key-count estimation.
- Chosen-cover recording.
- Tests proving different covers are chosen under different cardinalities/prefixes.

## Required Behavior

- For each node, compute all cover candidates from the formal plan.
- At execution time, evaluate cover candidates against their current GHT/COLT node state.
- Prefer exact key count when the node is already a map.
- Use offset-vector length as an estimate when exact key count would force a COLT map.
- Do not force a COLT solely to count keys unless a later cost model explicitly chooses that behavior.
- Choose the minimum count/estimate with deterministic tie-breaking.
- Record chosen cover per node entry for counters/explain.

## Technical Direction

- Add `key_count_estimate` or equivalent to the GHT API.
- Distinguish exact counts from estimates in the returned type.
- Tie-break by node subatom order to keep plans deterministic.
- Ensure cover choice can vary by prefix/subtrie during recursion.
- Keep static first-cover mode available for tests and ablations.

## Non-Goals

- Do not implement vectorized execution here.
- Do not implement an advanced cost model here.
- Do not force materialization just to mimic Generic Join if COLT estimates suffice.

## Acceptance Criteria

- Nodes with multiple covers choose the smaller exact map when available.
- Nodes with unforced offset vectors choose by vector length estimate.
- Cover choice can differ for different subtries in the same query execution.
- Static cover mode and dynamic cover mode produce identical result sets.
- Counters record cover candidates, choices, exact counts, estimates, and tie-breaks.

## Required Tests

- Triangle with asymmetric relation sizes chooses the smaller cover.
- Clover/triangle prefix case where cover changes under a subtrie.
- Dynamic and static modes return identical results.
- Tie-break is deterministic.
- Counting an unforced COLT vector does not force it.
- Explain includes chosen cover policy and observed choices once PRD 18 lands, or a temporary debug assertion exists before then.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb dynamic_cover --all-features
cargo test -p bumbledb-lmdb free_join_executor --all-features
cargo test --workspace --all-features
```
