# PRD 04: Collapse Static Proof Into Free Join

## Status

Not started.

## Severity

High architecture cleanup.

## Prerequisite

PRDs 01 through 03 must be complete.

## Problem

Static-empty and static semijoin proof are separate pre-planning systems. They are correctness-preserving and useful, but the paper would still be skeptical because they are a distinct proof engine with its own cache, counters, and query-shape logic outside the Free Join plan.

The target is not to lose empty-query speedups permanently. The target is to express those proofs as Free Join semijoin/probe nodes or delete them until that expression exists.

## Required Decision

Choose one path:

- Path A: convert static proof into Free Join preflight nodes.
- Path B: delete static proof entirely and accept LFTJ execution for empty cases until planned semijoin nodes exist.

No third path is allowed.

## Technical Direction For Path A

- Add explicit semijoin/proof node representation to `FreeJoinPlan`.
- Static literal constraints become node-level probes.
- Static relation existence checks become semijoin nodes.
- Proof cache keys become plan cache keys or are deleted.
- Counters move from `static_*` to Free Join node counters.

## Technical Direction For Path B

- Delete static proof functions.
- Delete static proof caches.
- Delete static proof counters.
- Delete static proof benchmark output.
- Update tests to assert result correctness through LFTJ.

## Strict Passing Criteria

- No standalone static proof executor remains.
- Empty-query correctness remains exact.
- Global count over empty input remains correct.
- Golden examples pass.
- Full validation gate passes.

## Failure Modes

- Keeping static proof as a sidecar with renamed functions is failure.
- Keeping static proof caches after proof deletion is failure.
- Moving proof into Free Join but keeping duplicate counters is failure.

## Non-Goals

- Do not implement COLT here.
- Do not optimize cover choice here.
