# PRD 09: Generic FK Storage Prefix Engine

## Goal

Make storage FK enforcement purely generic over encoded unique-key prefixes.

After PRD 08, schema validation supports compound enum/serial FKs. This PRD makes insert/delete enforcement use the same generic mechanism for all FK types.

## Explicit Non-Goals

- No backwards compatibility with old FK storage enforcement paths.
- No unique guard compatibility path.
- No current-row payload compatibility path.
- No special serial-reference branch kept for old schemas.
- No migration for old FK/index key layouts.
- No cascade/deferred compatibility behavior.

## Current Code Anchors

- `crates/bumbledb-lmdb/src/storage.rs`
- `check_foreign_keys`
- `check_delete_restrictions`
- `access_path_prefix_from_fields`
- `unique_constraint_layout`
- `target_unique_constraint`
- `scan_prefix`
- `current_index_prefix`

## Required Runtime Model

The storage engine should compile or derive FK enforcement plans:

```rust
struct CompiledForeignKey {
    source_relation: RelationId,
    source_fields: Vec<FieldId>,
    target_relation: RelationId,
    target_unique_access: AccessId,
    target_fields: Vec<FieldId>,
}
```

The exact location may be `StorageSchema` or internal helpers. The important behavior is generic encoded prefix construction.

## Insert FK Check

For each FK on the inserted relation:

```text
1. Read source field encoded bytes from inserted tuple in FK field order.
2. Build target access prefix: current_index_prefix(target_relation, target_unique_access) + source bytes.
3. Probe LMDB prefix iterator.
4. If no row exists, return ForeignKeyViolation.
```

This must work for:

- one-byte enums
- eight-byte serials
- mixed compound tuples
- string/bytes intern IDs if allowed by schema

No branch should ask whether a field is serial or enum.

## Delete Restrict Check

For deleting a target tuple:

```text
1. For each FK targeting this relation and target constraint, read target unique fields from old target tuple.
2. Build source FK access prefix using those bytes.
3. Probe source relation's FK access path.
4. If any referencing source tuple exists, return RestrictViolation.
```

This must support compound target constraints.

## Required StorageSchema Support

Add compiled maps if not already present:

```rust
unique constraint -> access layout
foreign key constraint -> access layout
target relation + target constraint -> target unique fields
```

Avoid repeated scans over relation descriptors in hot write paths.

## Required Tests

Add LMDB storage tests for:

- single serial FK insert success/failure
- single enum FK insert success/failure
- compound enum FK insert success/failure
- compound serial+enum FK insert success/failure
- restrict delete through enum FK
- restrict delete through compound enum FK
- exact delete after removing referencing tuple succeeds
- FK field order is positional and enforced

## Required Negative Tests

At runtime, ensure FK check fails when any one component of a compound FK does not match.

Example:

```text
Policy(US, USD) exists
Account(US, EUR) insert fails
Account(CA, USD) insert fails
Account(US, USD) insert succeeds
```

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Grep storage FK enforcement for identity-specific branches. None should remain.

## Completion Criteria

- FK enforcement is generic over encoded key prefixes.
- Compound FKs work natively.
- Enum FKs work natively.
- Delete restrict works through compound FKs.
- This PRD is deleted and committed after passing.
