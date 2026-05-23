# PRD 09: Delete Node Implementation Indirection

## Status

Not started.

## Objective

Remove `NodeImpl` while there is only one real node implementation. Reintroduce it later only when a second executable Free Join node implementation exists.

## Problem

`NodeImpl::SortedLeapfrog` is a single-variant enum. It adds masks, diagnostics, tests, and planner costs for a choice that does not exist.

## Implementation Steps

1. Delete `NodeImpl` enum.
2. Remove `PlanNode.implementation`.
3. Remove implementation masks from cost keys and explain output.
4. Remove tests that assert the single implementation enum.
5. Rename `is_free_join_sorted_leapfrog` to a structural validation helper or delete it.

## Passing Criteria

- Zero Rust matches for `NodeImpl`.
- Free Join plan validation still proves nodes are executable.
- Benchmark output no longer lists impl masks or impl arrays.
- Full validation passes.

## Failure Modes

- Replacing enum with string names is failure.
- Keeping implementation masks for future use is failure.
- Hiding the enum in another module is failure.

## Completion

Delete this PRD and commit.
