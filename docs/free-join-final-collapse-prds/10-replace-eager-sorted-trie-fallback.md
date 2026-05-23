# PRD 10: Replace Eager Sorted Trie Fallback

## Status

Not started.

## Objective

Eliminate eager temporary sorted-trie construction from query execution by extending lazy durable access coverage.

## Problem

The current engine can still build temporary relation images and sorted tries for unsupported atom shapes. This is the main remaining Generic Join-style eager build cost the Free Join paper explicitly avoids with COLT.

## Required Direction

Add lazy access support for all atom shapes currently falling back to `build_lftj_sorted_trie`, or reject/reshape unsupported plans so execution does not eagerly materialize atom relations.

## Implementation Steps

1. Inventory all current fallback triggers.
2. Add lazy access support for repeated variables.
3. Add lazy access support for local comparison predicates.
4. Add lazy access support for wildcard fields.
5. Add lazy access support for atoms with more than two variables, if still needed.
6. Add regression tests comparing lazy output with old eager fixtures before deleting eager code.
7. Delete `build_lftj_sorted_trie` after coverage is complete.

## Passing Criteria

- Zero Rust matches for `build_lftj_sorted_trie`.
- No query execution path builds temporary relation images for atom access.
- Existing query tests pass through lazy access or durable access only.
- Benchmarks expose lazy access metrics, not eager build metrics.
- Full validation passes.

## Failure Modes

- Keeping eager fallback for rare shapes is failure.
- Copying full atom columns before proving need is failure.
- Dropping correctness for repeated-variable atoms is failure.

## Completion

Delete this PRD and commit.
