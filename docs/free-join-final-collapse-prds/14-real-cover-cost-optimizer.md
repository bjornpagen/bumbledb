# PRD 14: Real Cover Cost Optimizer

## Status

Not started.

## Objective

Replace the single-candidate variable-order optimizer with real Free Join cover costing over executable plan choices.

## Problem

The optimizer still has candidate/cost scaffolding, but only one actual candidate. That is fake complexity. Either delete it or make it real. This PRD makes it real only after factoring/lazy/vectorized execution exists.

## Prerequisite

PRDs 12 and 13 must be complete.

## Required Direction

For each node, enumerate covers that can bind node variables. Cost cover iteration, probe count, lazy force cost, materialization, and expected result-set work. Select deterministic minimum-cost executable covers.

## Implementation Steps

1. Define cover candidate struct with required variables, relation, access, and estimated cost.
2. Enumerate only executable covers.
3. Estimate lazy force/build work from access/image stats.
4. Estimate probe count and candidate keys.
5. Estimate result-set work separately from completed bindings.
6. Remove fake candidate rank if no longer needed.
7. Add tests for deterministic tie behavior.

## Passing Criteria

- Optimizer trace lists real cover choices.
- Every listed candidate can execute.
- No candidate exists solely by name.
- Cost model distinguishes setup, probe, iteration, and result-set work.
- Full validation passes.

## Failure Modes

- Keeping a one-candidate optimizer after this PRD is failure.
- Selecting unimplemented covers is failure.
- Using rank to hide bad cost is failure.

## Completion

Delete this PRD and commit.
