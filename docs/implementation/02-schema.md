# PRD 02 — Schema Descriptors and Declaration Validation

Authority: `docs/architecture/10-data-model.md` (structural types, fields =
type+generation, auto-unique on serial fields, constraint rules, nullary relations).

## Purpose

The in-memory schema model: descriptors the engine consumes, with declaration-time
validation that makes illegal schemas unconstructible.

## Technical direction

- `schema` module. `ValueType` — exactly six structural variants; `Enum` carries
  `Box<[Box<str>]>` ordered variant names; **no name field on any variant**; derive
  `PartialEq/Eq/Hash` (structural equality IS type equality — one derive, no custom
  logic).
- `FieldDescriptor { name, value_type, generation: Generation }`,
  `Generation::{None, Serial}`; `RelationDescriptor { name, fields, constraints }`;
  `ConstraintDescriptor::{Unique { name, fields: Box<[FieldId]> }, ForeignKey { name,
  fields, target_relation: RelationId, target_constraint }}`;
  `SchemaDescriptor { relations }`. Ids (`RelationId(u32)`, `FieldId(u16)`,
  `ConstraintId(u16)`) are declaration-order indices — newtypes over integers, no
  strings carried anywhere post-construction (post-mortem §10: never both).
- Construction is the validation boundary (parse, don't validate): a builder or
  `SchemaDescriptor::new(...) -> Result<Schema, SchemaError>` that returns a **sealed
  `Schema` witness type** whose invariants downstream code trusts. Checks, exhaustive:
  duplicate relation/field/constraint names (per scoping rules); enum with 0 or >256
  variants or duplicate variant names; Serial generation on a non-U64 field; unique
  constraints with empty/duplicate field lists; FK arity/positional structural-type
  equality against the named target unique constraint; FK targets exist; nullary
  relations explicitly allowed.
- **Auto-materialize** a `Unique` constraint per Serial field at construction, named
  after the field, ordinary in every way (visible, FK-targetable).
- `Schema` exposes: per-relation `FactLayout` (via PRD 01), constraint lookups by id,
  and iteration in declaration order. Precompute per-relation: which unique constraints
  exist, which constraints are FK-targeted (for the delete-side Restrict scan set,
  PRD 08).

## Non-goals

Fingerprint (PRD 03). The proc-macro surface (PRD 27) — tests construct descriptors by
hand.

## Passing criteria

- Unit tests: every rejection listed above has a test with a distinct `SchemaError`
  variant; auto-unique appears in the descriptor and is FK-targetable; structural enum
  equality (same variant list, different declaring relations → equal type; different
  order → unequal); a nullary relation constructs.
- The `Schema` type is unconstructible except through validation (private fields,
  no `pub` constructor).
- Global commands green.
