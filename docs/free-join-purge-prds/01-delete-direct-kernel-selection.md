# PRD 01: Delete Direct Kernel Selection

## Status

Not started.

## Severity

Critical architecture cleanup.

## Problem

The current query planner can still attach a direct kernel to an otherwise normal execution plan. Even after pre-planning bypasses are removed, this keeps a second join execution family alive beside Free Join.

The Free Join paper explicitly rejects this kind of split. The point is not to keep direct paths as a faster sidecar. The point is to represent direct iteration/probe behavior inside the same Free Join plan space.

## Code To Target

- `try_direct_kernel`
- `try_direct_prefix_range_kernel`
- `try_direct_chain_kernel`
- `ExecutionPlan.direct_kernel`
- `QueryPlan.direct_kernel`
- `DirectKernelPlan`
- `DirectKernel`
- `DirectKernelSummary`
- `DirectKernelKind`
- direct-kernel optimizer trace fields

## Required Replacement

For this PRD, do not implement new Free Join node kinds. The safe replacement is pure LFTJ/static proof fallback.

This PRD deliberately accepts temporary performance regression to eliminate the sidecar selection mechanism. Later PRDs introduce real Free Join replacements.

## Implementation Steps

1. Remove direct-kernel selection from `plan_query`.
2. Remove `ExecutionPlan.direct_kernel`.
3. Remove `QueryPlan.direct_kernel`.
4. Remove direct-kernel explain output.
5. Remove `DirectKernelSummary` and `DirectKernelKind` from public exports.
6. Remove direct-kernel selected runtime assignment in `execute_free_join`.
7. Make all former direct-kernel-eligible queries run through existing Free Join/LFTJ plan path or static-empty proof.
8. Update tests that asserted direct runtime selection.
9. Rename tests to assert planned Free Join fallback where appropriate.
10. Remove benchmark fields that report direct-kernel kind or direct-kernel target.

## Required Tests

- A former single-relation static lookup returns the same result through LFTJ or static proof.
- A former prefix/range direct query returns the same result through LFTJ or static proof.
- A former direct chain query returns the same result through LFTJ or static proof.
- Prepared and non-prepared forms behave identically.
- Query explain output contains no direct-kernel section.
- Public query plan contains no direct-kernel field.

## Strict Passing Criteria

- Rust source has zero matches for `DirectKernelSummary`.
- Rust source has zero matches for `DirectKernelKind`.
- Rust source has zero matches for `DirectKernelPlan`.
- Rust source has zero matches for `try_direct_kernel`.
- Rust source has zero matches for `direct_kernel` except possibly benchmark fixture strings removed in this same PRD.
- Full validation gate passes.

## Failure Modes

- Leaving direct kernels selected but hidden from explain is failure.
- Keeping public fields for direct kernels is failure.
- Replacing direct kernels with a new differently named sidecar is failure.
- Deleting tests instead of rewriting them to assert fallback correctness is failure.

## Non-Goals

- Do not delete hash trie here.
- Do not add COLT here.
- Do not implement new Free Join node kinds here.
- Do not optimize the fallback path here.
