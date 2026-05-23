# PRD 05: Delete Static Proof Caches And Counters

## Status

Not started.

## Severity

High cleanup correctness.

## Prerequisite

PRD 04 must be complete.

## Problem

Even after static proof behavior is moved or deleted, cache and counter residue can preserve the old architecture in diagnostics. This PRD removes that residue.

## Code To Delete

- `StaticProofCacheKey`
- `StaticProofCacheValue`
- `StaticProofKind`
- `static_empty_queries`
- `static_proof_cache`
- `static_empty_fast`
- static proof diagnostics
- static proof benchmark JSON fields
- `StaticSemijoinSkipReason`
- static proof counters in `PlanCounters`

## Required Replacement

If PRD 04 chose Path A, replacement diagnostics must live in Free Join node counters.

If PRD 04 chose Path B, no replacement diagnostics are needed.

## Implementation Steps

1. Remove static proof cache fields from `QueryImage` and `QueryImageCache`.
2. Remove public execution option for static-empty fast cache if no longer used.
3. Remove cache-control tests tied only to static proof.
4. Remove benchmark fields.
5. Update explain output.
6. Run source grep gates.

## Strict Passing Criteria

- Zero Rust matches for `StaticProof`.
- Zero Rust matches for `static_empty_fast`.
- Zero Rust matches for `static_semijoin`.
- Zero Rust matches for `StaticSemijoinSkipReason`.
- Full validation gate passes.

## Failure Modes

- Leaving zero-valued static proof counters is failure.
- Keeping cache options that no longer alter behavior is failure.
- Keeping benchmark fields for deleted proof is failure.

## Non-Goals

- Do not add new proof machinery here.
