# PRD 03: Constraint Model And Compound Foreign Keys

## Status

Draft. This PRD depends on PRD 01 and should be implemented after the type cleanup in PRD 02 is underway or complete.

## Problem

Compound primary keys already work structurally. Compound foreign keys do not. Current FK behavior is inferred from scalar `ValueType::Ref` fields and rejects composite target primary keys at write time.

This is not acceptable for a BCNF-first database. If compound keys are supported, compound FKs must be first-class.

## Goals

- Make schema constraints explicit and relation-level.
- Add compound foreign keys to primary keys.
- Move FK enforcement out of `ValueType::Ref`.
- Generate child-side FK indexes automatically.
- Keep constraint enforcement immediate and transaction-local.
- Preserve no-null semantics.
- Keep cascading and deferrable constraints out of scope for now.

## Non-Goals

- No cascade delete.
- No set-null action.
- No deferred constraint checking.
- No partial/null FK semantics.
- No FKs to arbitrary non-unique field groups.
- No Logica implementation.

## Current Code References

- `ConstraintDescriptor::Unique` in `schema.rs`.
- `ValueType::Ref` in `schema.rs`.
- `check_foreign_keys` in `storage.rs`.
- `check_delete_restrictions` in `storage.rs`.
- `unique_guard_key` in `storage.rs`.
- `ConstraintError::ForeignKeyViolation`, `RestrictViolation`, and `UnsupportedCompositeForeignKey` in `error.rs`.

## Required Constraint Model

Replace the single-variant constraint model with:

```rust
pub enum ConstraintDescriptor {
    Unique(UniqueConstraintDescriptor),
    ForeignKey(ForeignKeyConstraintDescriptor),
    Check(CheckConstraintDescriptor),
}
```

`Check` can be modeled but not executed in this PRD. It exists so future Logica/query constraint prep has a stable database-side namespace.

Required descriptors:

```rust
pub struct UniqueConstraintDescriptor {
    pub name: String,
    pub fields: Vec<String>,
}

pub struct ForeignKeyConstraintDescriptor {
    pub name: String,
    pub fields: Vec<String>,
    pub target_relation: String,
    pub target_fields: Vec<String>,
    pub on_delete: ForeignKeyAction,
    pub on_update: ForeignKeyAction,
}

pub enum ForeignKeyAction {
    Restrict,
}

pub struct CheckConstraintDescriptor {
    pub name: String,
    pub expression: CheckExpressionDescriptor,
}
```

`CheckExpressionDescriptor` may initially be a reserved placeholder that rejects execution. It must be fingerprinted and validated enough to avoid namespace collisions.

## FK Rules

- FK source fields are ordered.
- FK target fields are ordered.
- Source and target arity must match.
- Target fields must equal the target primary key for the first implementation.
- Later FKs to unique constraints can be added using the same descriptor.
- Source field types must be compatible with target field types.
- FK constraints are immediate.
- FK constraints are checked inside the write transaction.
- Rows inserted earlier in the same write transaction must satisfy FK lookups.
- Delete of a referenced target row must fail with restrict violation.
- Replace of referenced key fields is forbidden because primary keys are immutable.

## Generated FK Indexes

Every FK must generate a child-side index:

```text
fk_<constraint_name>(source_field_0, source_field_1, ..., primary_key_suffix)
```

The index must support:

- Efficient restrict-delete scans.
- Efficient join planning from parent key to child rows.
- Compound prefix lookup.

Generated FK index names are reserved.

## Storage Enforcement

Insert/replace FK check:

```text
for each FK on source relation:
  encode source fields in FK order
  encode target key in target primary-key order
  check target current row key exists
```

Delete restrict check:

```text
for each FK in any source relation targeting deleted relation:
  if deleted target fields equal target primary key:
    build source FK index prefix from deleted row target-field values
    if any matching source row exists, reject delete
```

The implementation must not scan every source row.

## Error Model

Replace scalar-only FK errors with constraint-aware variants:

```rust
ForeignKeyViolation {
    relation: String,
    constraint: String,
    target_relation: String,
}

RestrictViolation {
    relation: String,
    referenced_by: String,
    constraint: String,
}
```

Remove `UnsupportedCompositeForeignKey` when compound FKs are implemented.

## Implementation Plan

1. Add descriptor structs and enum variants.
2. Add schema validation for FK constraints.
3. Generate FK index candidates from FK descriptors.
4. Stop deriving enforcement from `ValueType::Ref`.
5. Rewrite `check_foreign_keys` to iterate FK constraints.
6. Rewrite `check_delete_restrictions` to use generated FK indexes.
7. Update benchmarks and test schemas to declare FKs explicitly.
8. Update errors and tests.
9. Remove scalar composite-FK rejection.

## Strict Passing Criteria

- Compound FK to compound primary key is accepted by schema validation.
- Compound FK insert succeeds when target row exists.
- Compound FK insert fails atomically when target row is missing.
- Compound FK delete restriction fails when child rows exist.
- Compound FK delete succeeds after all child rows are removed.
- FK enforcement no longer depends on `ValueType::Ref`.
- Generated FK index appears in `StorageSchema::access_paths`.
- Restrict-delete uses the generated FK index and does not row-scan.
- Existing single-field FK behavior is represented using explicit FK constraints.
- `UnsupportedCompositeForeignKey` is removed or unreachable.

## Verification Commands

```sh
cargo test -p bumbledb-core schema
cargo test -p bumbledb-lmdb storage
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
