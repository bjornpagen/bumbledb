# PRD 07: Rebase Optimizer And Plan Families

## Status

Not started.

## Severity

Critical final architecture cleanup.

## Prerequisite

PRDs 01 through 06 must be complete.

## Problem

Once sidecars are gone, optimizer language must stop encoding old families. There should be Free Join plans with implementation choices, not separate direct/index-nested/static families.

## Code To Target

- `PlanFamily::Direct`
- `PlanFamily::IndexNestedLoop`
- `PlanFamily::StaticEmpty`
- `QueryRuntimeKind::DirectKernel`
- `QueryRuntimeKind::IndexNestedLoop`
- `QueryRuntimeKind::StaticEmpty`
- optimizer candidate names that describe deleted sidecars
- benchmark grouping by deleted runtime families

## Required Replacement

Use Free Join plan family with node implementation masks and execution features. Runtime kind may distinguish `Lftj`, `LazyGht`, `Vectorized`, or `StaticFreeJoinProof` only if these are real Free Join execution modes.

## Implementation Steps

1. Delete old plan family variants.
2. Delete old runtime kind variants.
3. Replace optimizer trace names with Free Join node implementation names.
4. Update benchmark output and tests.
5. Update explain output.
6. Ensure no public API exposes old families.

## Strict Passing Criteria

- Zero Rust matches for `IndexNestedLoop`.
- Zero Rust matches for `PlanFamily::Direct`.
- Zero Rust matches for `QueryRuntimeKind::DirectKernel`.
- All tests and benchmarks use Free Join terms.
- Full validation gate passes.

## Failure Modes

- Keeping old variants with zero use is failure.
- Using generic `Unknown` to hide missing rebase is failure.
- Breaking benchmark JSON without updating tests is failure.

## Non-Goals

- Do not add new optimizer strategy here beyond nomenclature and selected real implementations.
