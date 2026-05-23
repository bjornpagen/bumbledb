# PRD 02: Query Normalization And Atom Occurrences

## Purpose

Define the normalized query model required before formal Free Join planning. The engine must represent relation occurrences, self-joins, field bindings, literals, inputs, wildcards, omitted fields, duplicate fields, and same-atom repeated variables explicitly.

## Dependencies

- PRD 00.
- PRD 01.

## Scope

- `crates/bumbledb-core/src/query_ir.rs`
- `crates/bumbledb-core/src/query_builder.rs`
- `crates/bumbledb-lmdb/src/query/normalize.rs`
- `crates/bumbledb-lmdb/src/query/model.rs`
- Query builder tests and execution-boundary validation tests.

## Required Model

- Every relation atom in a normalized query must become an atom occurrence with a stable `AtomOccurrenceId`.
- Self-joins must be represented by distinct atom occurrences even when they share the same base relation ID.
- Each atom occurrence must know its base relation, source relation name, field bindings, and the ordered variable tuple used for Free Join planning.
- Duplicate field bindings inside one atom must be rejected as invalid IR unless deliberately lowered before validation.
- Same-atom repeated variables must have one explicit product decision.
- The preferred repeated-variable policy is lowering to same-fact equality predicates before planning. If this is too large, reject repeated variables as invalid IR in this PRD and add support later under PRD 15.
- Omitted fields and wildcards must not accidentally introduce variables.
- Literal and input fields must be represented as pushed selection constraints for planning purposes, even if execution initially evaluates them as residual filters.

## Technical Direction

- Add or formalize `AtomOccurrenceId` separately from base `RelationId`.
- Audit `TypedRelationAtom.fields` for duplicate `field_id` and duplicate `field` values in `validate_typed_query`.
- Normalize relation atom fields into a full relation-field view with one of these term kinds per field: variable, input, literal, wildcard, omitted.
- Build a per-atom ordered variable tuple after applying the repeated-variable policy.
- Preserve source variable names for result columns and explain output.
- Convert internal panics or `Error::internal` for malformed query shapes into `Error::invalid_query` or the existing product error category.

## Non-Goals

- Do not add formal Free Join plan nodes here.
- Do not rewrite execution here.
- Do not add SQL aliases. Alias identity is internal atom occurrence identity, not a SQL surface.

## Acceptance Criteria

- Normalized queries expose atom occurrence IDs that are unique and stable in clause order.
- Self-join queries with repeated base relation names produce distinct atom occurrences.
- Duplicate field bindings in one relation atom fail before planning.
- Same-atom repeated variables either lower into explicit equality predicates or fail with a product-level invalid-query error.
- No valid query fails later with an internal LFTJ access-path error because of malformed atom shape.
- Existing typed query builder examples still compile and run.

## Required Tests

- Duplicate field binding in one atom is rejected.
- Same-atom repeated variable follows the chosen policy.
- Two atoms over the same relation have distinct occurrence IDs.
- Self-join projection remains duplicate-free.
- Literal/input/wildcard/omitted field normalization has expected planner-visible terms.
- Invalid public typed IR is rejected at execution boundaries even if manually constructed outside the builder.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-core --all-features
cargo test -p bumbledb-lmdb query --all-features
cargo test -p bumbledb-test-support --test golden_examples --all-features
```
