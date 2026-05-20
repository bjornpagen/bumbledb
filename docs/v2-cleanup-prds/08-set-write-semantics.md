# PRD 08: Set Write Semantics

## Goal

Make write behavior match set semantics precisely.

Every relation is a set of full tuples. Inserting the same tuple twice is a no-op. Inserting a different tuple that violates a named unique constraint is an error.

## Current State

Current write behavior in `crates/bumbledb-lmdb/src/storage.rs:428-554`:

- `insert` rejects duplicate primary key with `DuplicateTuple`.
- `insert_tuple` delegates to `insert_inner`.
- `replace` replaces by primary key.
- `delete` deletes by `KeyValues` primary key.
- `delete_tuple` extracts primary-key fields from a row.

Current unique behavior:

- Unique guards live under `NS_UNIQUE_GUARD` at `storage.rs:20`.
- Unique guard value stores encoded primary bytes.
- Unique checks are anchored at `storage.rs:836-855`.
- Unique guard insert/delete at `storage.rs:944-976`.

## Required Public Write API

Simplify public write operations.

Recommended final API:

```rust
impl WriteTxn<'_> {
    pub fn insert(&mut self, schema: &StorageSchema, row: Row) -> Result<InsertOutcome>;
    pub fn delete(&mut self, schema: &StorageSchema, row: Row) -> Result<DeleteOutcome>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted,
    AlreadyPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    Absent,
}
```

If changing return types causes too much test churn, an intermediate API may return `Result<()>`, but duplicate exact inserts must still be no-ops. The final API should expose outcomes because set writes are not exceptional.

Delete:

- `insert_tuple`
- `replace`
- `delete_tuple`
- `KeyValues`
- `get_row` by primary key, unless replaced with exact-row existence by row.

## Required Insert Semantics

Insert algorithm:

```text
1. Validate row relation and fields.
2. Encode row to EncodedTuple.
3. Compute covering access key.
4. If covering key exists: return AlreadyPresent and do nothing else.
5. Check all unique constraints by probing matching unique prefixes.
6. Check all foreign keys by probing target unique prefixes.
7. Insert all access path keys.
8. Increment relation/index counts.
9. Append insert history with new tuple.
10. Publish relation segment.
11. Return Inserted.
```

Important: exact duplicate insert must not append history, increment counts, republish segments, or intern duplicate dictionary values beyond unavoidable lookup. If dictionary interning happens before duplicate detection, it may create dictionary entries for duplicate rows. Prefer detecting exact duplicate after encoding, accepting intern lookups for strings/bytes. Dictionary entries are idempotent by raw value.

## Required Unique Semantics

For a unique constraint with fields `[holder, currency]`, the unique prefix is the encoded values of those fields in that order.

Conflict detection:

```text
If no existing access key has that unique prefix: OK.
If an existing access key with that unique prefix equals the new covering tuple: AlreadyPresent should have already fired.
If an existing access key with that unique prefix decodes to a different full tuple: UniqueViolation.
```

Because the unique access path is full-covering, no separate unique guard namespace is needed.

Delete `NS_UNIQUE_GUARD`.

Delete unique guard functions.

Recommended helper:

```rust
fn unique_prefix_exists_for_different_tuple(
    &self,
    schema: &StorageSchema,
    relation: &CompiledRelationLayout,
    unique_layout: &CurrentIndexLayout,
    tuple: &EncodedTuple,
) -> Result<bool>;
```

## Required FK Semantics

For an FK:

```rust
ConstraintDescriptor::ForeignKey {
    fields: ["holder"],
    target_relation: "Holder",
    target_constraint: "by_id",
    ..
}
```

The source prefix is encoded from `Account.holder`.

The target prefix is probed against the target unique access path for `Holder.by_id`.

FK passes iff at least one target tuple exists with that unique prefix.

Recommended helper:

```rust
fn foreign_key_target_exists(
    &self,
    schema: &StorageSchema,
    fk: &CompiledForeignKey,
    source_tuple: &EncodedTuple,
) -> Result<bool>;
```

## Required Delete Semantics

Delete is exact-row deletion.

Algorithm:

```text
1. Validate and encode row to EncodedTuple using existing dictionary entries.
2. Compute covering access key.
3. If covering key does not exist: return Absent.
4. Check restrict constraints from other relations.
5. Delete all access path keys for the tuple.
6. Decrement relation/index counts.
7. Append delete history with old tuple.
8. Publish relation segment.
9. Return Deleted.
```

Do not delete by unique key. If callers want to delete by unique key later, they must first resolve the row through an explicit lookup.

## Required Restrict Semantics

To delete a target tuple, inspect every FK whose target is this relation and target constraint.

For each source relation, build the source FK access prefix using the target tuple's target unique fields.

If any source tuple exists with that prefix, return restrict violation.

This replaces old primary-key-specific logic at `storage.rs:857-909`.

## Error Model

Update `ConstraintError` in `crates/bumbledb-lmdb/src/error.rs`.

Recommended:

```rust
pub enum ConstraintError {
    UnknownField { relation: String, field: String },
    MissingField { relation: String, field: String },
    TypeMismatch { relation: String, field: String, expected: String, actual: &'static str },
    UniqueViolation { relation: String, constraint: String },
    ForeignKeyViolation { relation: String, constraint: String, target_relation: String },
    RestrictViolation { relation: String, referenced_by: String, constraint: String },
}
```

Remove or stop using:

- `DuplicateTuple`
- `NotFound` for set deletes, unless retained for non-set APIs.

## Bulk Load Semantics

`bulk_load` and streaming bulk load should count only `Inserted`, not `AlreadyPresent`.

Update `BulkLoadReport.rows_inserted` docs if necessary.

## Tests Required

- Exact duplicate insert returns `AlreadyPresent` and row count stays unchanged.
- Exact duplicate insert does not append history.
- Exact duplicate insert does not increment index counts.
- Same unique prefix with different tuple returns `UniqueViolation`.
- FK insert succeeds when target unique prefix exists.
- FK insert fails when target unique prefix is absent.
- Delete exact row returns `Deleted`.
- Delete absent exact row returns `Absent`.
- Delete target row with referencing source returns restrict violation.
- Bulk load with duplicate exact rows counts only distinct inserted rows.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- Grep for `insert_tuple` returns no real code references.
- Grep for `delete_tuple` returns no real code references.
- Grep for `replace(` in write API contexts returns no real API references.
- Grep for `KeyValues` returns no real code references.
- Grep for `NS_UNIQUE_GUARD` returns no real code references.

## Completion Criteria

- Write APIs match relation-as-set semantics.
- Duplicate facts are idempotent.
- Uniqueness and FK enforcement use access paths, not guard namespaces or primary keys.
- Deletion is exact fact deletion.
