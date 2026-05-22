# 03 Schema And Layout Break

## Purpose

Delete the schema assumptions that force every access path to be a full covering row permutation. The schema model should describe logical constraints and physical access paths separately.

## Current Bad Shape

Current code requires exactly one covering unique constraint and expands every index to include every field:

- `SchemaError::MissingCoveringConstraint`
- `RelationDescriptor::with_covering_unique`
- `ConstraintDescriptor::Unique { covering }`
- `RelationDescriptor::index_components`
- `CurrentIndexLayout.components`

This makes secondary indexes function as duplicate row stores.

## New Schema Concepts

Unique constraint:

```rust
Unique { name, fields }
```

Foreign key:

```rust
ForeignKey { name, fields, target_relation, target_constraint, on_delete: Restrict }
```

Access path:

```rust
AccessLayout {
    relation_id,
    access_id,
    name,
    kind,
    key_fields,
    include_tuple_id,
    payload_fields,
}
```

Canonical tuple membership is implicit for every relation and is not modeled as a covering unique constraint.

## Required Code Changes

- Remove `covering` from `ConstraintDescriptor::Unique`.
- Remove `with_covering_unique` and replace schema fixtures with `with_unique` plus indexes where needed.
- Remove `COVERING_ACCESS_NAME` from public and planner-facing logic.
- Replace `CurrentIndexLayout` with a set-native access layout that does not append all fields.
- Update schema fingerprint canonical bytes with a new version tag.
- Update every benchmark/test schema declaration.

## Acceptance Gates

- No production code references `with_covering_unique`.
- No production code references `unique_covering`.
- No production code references `COVERING_ACCESS_NAME`.
- Schema validation allows relations with zero named unique constraints if no FK targets require them.
- FK targets must still reference named unique constraints.
- Key-size validation applies to declared access key fields only, not full row width.

## Tests Required

- Relation with no unique constraints can store a tuple set.
- Relation with a named unique rejects two different tuples with the same unique key.
- FK to compound unique still validates.
- FK with mismatched field types is rejected.
- Old schema fingerprint differs from new canonical fingerprint.

## Non-Goals

- No compatibility aliases for covering unique.
- No primary-key concept reintroduction.
