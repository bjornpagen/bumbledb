# PRD 14 — Query Validation

Authority: `docs/architecture/20-query-ir.md` (the exhaustive validation roster;
comparison type rules; param anchoring; aggregate legality; degenerate shapes).

## Purpose

The single validation boundary: IR in, `ValidatedQuery` witness out. Everything
downstream trusts the witness and re-checks nothing.

## Technical direction

- `ir::validate`. `validate(&Schema, &Query) -> Result<ValidatedQuery, ValidationError>`.
  Implement the doc's roster **as an exhaustive checklist, one error variant each** —
  transcribe the list from the doc into a module-level comment and check items off in
  code order: unknown ids; duplicate FieldId per atom; variable structural-type
  conflicts; literal/param type mismatches (structural equality of `ValueType`); enum
  ordinal range; comparison legality (Eq/Ne all types, order ops integers only, no
  cross-type — a small `fn cmp_legal(op, &ValueType) -> bool`); constant comparisons
  rejected; unbound find vars; comparison-only vars; empty finds; duplicate find
  terms; no atoms; aggregate input types (Sum: integers; Min/Max: integers; Count:
  `over` must be None); aggregate-over-group-key.
- Param typing: infer from anchors (field bindings and comparisons against typed
  terms); no anchor or conflicting anchors → errors; the witness records each param's
  resolved `ValueType` for bind-time checking (PRD 25).
- `ValidatedQuery` (sealed, private fields): the query plus derived tables — per-var
  resolved type, per-param type, per-atom occurrence list, find-var set, aggregate
  descriptors, group-key var set. Var types resolved to the **column word form**
  (PRD 10) where useful downstream.
- Zero-binding atoms and all-aggregate finds validate successfully (legal per doc).

## Non-goals

Normalization (PRD 15). Planning. Any execution knowledge.

## Passing criteria

- Unit tests: **one accepting and one rejecting test per roster item** (the rejecting
  test asserts the specific error variant — the negative corpus seed); the doc's
  example queries validate; a query binding a U64 var against an I64 field rejects;
  param used at two conflicting types rejects; Count with `Some(var)` rejects;
  all-aggregate finds accept; zero-binding atom accepts.
- `ValidatedQuery` unconstructible outside the module.
- Global commands green.
