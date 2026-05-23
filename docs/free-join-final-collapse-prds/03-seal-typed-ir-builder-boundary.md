# PRD 03: Seal Typed IR Builder Boundary

## Status

Not started.

## Objective

Prevent public `TypedQuery` construction from bypassing schema validation. The builder should be the only stable construction API for executable typed queries.

## Problem

The typed IR has public mutable fields. Hand-built `TypedQuery` values can use out-of-range variable IDs, wrong relation IDs, wrong field names, wrong value types, and invalid projection terms. Execution currently catches some errors but still trusts too much metadata.

## Required Direction

Make executable typed query construction go through `QueryBuilder` or a checked constructor. Do not trust raw public structs at execution boundaries.

## Implementation Options

Preferred: make `TypedQuery` fields private and provide read-only accessors.

Acceptable: keep structs public for tests but add `TypedQuery::validate(schema)` and call it at every execution/prepare boundary before normalization.

The preferred option wins unless it breaks too much at once.

## Validation Rules

- variable IDs are dense and in range
- input IDs are dense and in range
- relation IDs match names
- field IDs match names and relation bounds
- field value types match schema
- relation atom field terms reference valid variables/inputs
- comparison operands reference valid variables/inputs
- comparison value types match operands
- projection variables are bound by relation atoms
- no invalid aggregate terms remain after PRD 04

## Tests

Add execution-boundary tests for hand-built invalid IR:

- bad variable ID in projection
- bad variable ID in relation field
- bad input ID
- relation ID/name mismatch
- field ID/name mismatch
- comparison type mismatch

## Passing Criteria

- Builder and execution reject the same invalid query shape classes.
- Invalid public IR returns query/schema errors, not panics or internal invariant errors.
- No test constructs invalid IR except explicit validation tests.

## Failure Modes

- Relying only on builder checks is failure.
- Widening execution to accept malformed IR is failure.
- Adding compatibility constructors is failure.

## Completion

Delete this PRD and commit.
