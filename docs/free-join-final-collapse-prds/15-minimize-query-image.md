# PRD 15: Minimize Query Image

## Status

Not started.

## Objective

Make query images field-scoped, access-scoped, compact, and mostly private.

## Problem

`QueryImage` still exposes many internals and can load more relation/field/access data than a query needs. The final engine should load only the columns and accesses required by the Free Join plan and output decoding.

## Implementation Steps

1. Audit every public `QueryImage`/`RelationImage` method.
2. Make internals `pub(crate)` unless benchmarks require public diagnostics.
3. Ensure query image scopes include only needed relations.
4. Extend scopes to required fields and access IDs.
5. Ensure projection decoding can request only projected fields.
6. Add memory diagnostics for encoded columns and access bytes by scope.
7. Update tests to assert scoped image contents.

## Passing Criteria

- Public API no longer exports query image internals.
- Query image cache key includes relation, field, and access scope.
- Focused query test loads fewer fields than full relation image.
- Full validation passes.

## Failure Modes

- Loading full schema for every query is failure.
- Making internals public to satisfy tests is failure.
- Caching scoped images under non-scoped keys is failure.

## Completion

Delete this PRD and commit.
