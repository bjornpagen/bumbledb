# 13 Fuzz Crash And Property Validation

## Purpose

Add the validation depth required for a storage-format and semantic rewrite. Encoding fuzzing is not enough.

## Required Validation Families

Operation sequence properties:

- random insert/delete exact tuples
- duplicate insert no-op
- absent delete no-op
- unique/FK/restrict behavior
- relation cardinality after every operation
- access cardinality after every operation

Query equivalence properties:

- generated small schemas
- generated positive conjunctive queries
- generated projection and aggregate-domain queries
- compare engine result set to in-memory set evaluator

Crash/failpoint properties:

- fail after canonical tuple write
- fail after unique write
- fail after reverse-FK write
- fail after access write
- fail before cardinality update
- fail before commit
- reopen and verify no partial committed logical state

Cache properties:

- prepared plan cache scoped by schema and snapshot
- result cache scoped by snapshot, inputs, and aggregate domain
- query image cache invalidates after writes

## Required Code Changes

- Add fuzz target for storage operation sequences.
- Add fuzz target or proptest module for query-vs-reference equivalence.
- Expand failpoints around new namespace write stages.
- Add invariant scanner for canonical/unique/reverse-FK/access consistency.

## Acceptance Gates

- `cargo check --manifest-path fuzz/Cargo.toml` passes.
- Fuzz targets compile and can run for a short smoke duration.
- Property tests cover at least insert/delete, constraints, projection, and aggregate domains.
- Failpoint tests prove failed transactions leave no partial logical state.
- Invariant scanner passes after every golden mutation sequence.

## Non-Goals

- No long-running fuzz campaign required for PRD completion.
- No randomized SQL compatibility testing.
