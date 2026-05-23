# PRD 12: Free Join Factoring

## Status

Not started.

## Objective

Implement plan factoring so lookups whose keys are already available are moved earlier in the Free Join plan.

## Problem

The current plan is mostly variable-order LFTJ. Free Join's paper contribution includes factoring lookup/probe work earlier to avoid unnecessary witness expansion.

## Required Direction

Add a conservative factoring pass over Free Join nodes. It must only move a subatom/probe when all needed key variables are available and the move preserves plan validity.

## Implementation Steps

1. Represent probe-only subatoms distinctly from cover/iterator subatoms if needed.
2. Compute available variables per node.
3. Move eligible probe subatoms to the earliest valid node.
4. Preserve relation uniqueness per node.
5. Add clover-like and chain-like tests proving reduced work and identical results.
6. Add diagnostics for factored probes.

## Passing Criteria

- At least one benchmark/test query has fewer completed bindings after factoring.
- Exact result equality with unfactored logical expectation is proven.
- Invalid factoring is rejected by plan validation.
- Full validation passes.

## Failure Modes

- Factoring by heuristic without availability proof is failure.
- Changing results is failure.
- Adding a non-executed plan annotation is failure.

## Completion

Delete this PRD and commit.
