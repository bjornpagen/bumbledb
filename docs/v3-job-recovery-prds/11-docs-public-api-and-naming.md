# PRD 11: Docs, Public API, And Naming

## Goal

Update the public-facing architecture contract, exports, helper names, comments, and tests so the final v3 model is unmistakable:

```text
No UUID.
Enums are bytes.
Identities are nominal serials.
Foreign keys are generic exact-key constraints over named unique constraints.
Compound FKs and enum FKs are first-class.
```

## Explicit Non-Goals

- No backwards compatibility with old public names if they obscure v3 semantics.
- No deprecated aliases for removed UUID, identity allocation, or identity wrapper APIs.
- No migration guide that promises in-place upgrades.
- No docs describing old primary/ref/UUID behavior as supported.
- No compatibility exports for deleted internal types.

## Current Docs To Update

- `docs/ROSETTA_STONE.md`
- `crates/bumbledb-core/src/lib.rs`
- `crates/bumbledb-core/src/schema.rs` module docs and type docs
- `crates/bumbledb-core/src/query_builder.rs` docs
- `crates/bumbledb-lmdb/src/lib.rs` public exports
- `crates/bumbledb-lmdb/src/storage.rs` public value docs
- benchmark/test helper comments in `crates/bumbledb-bench` and `crates/bumbledb-test-support`

## Required Rosetta Stone Updates

Update `docs/ROSETTA_STONE.md` to state:

- Persistent UUID is not supported.
- Enums are one-byte closed domains.
- Enum codes must be `0..=255`.
- Nominal identities are serial `u64`s.
- `ValueType::Serial { type_name, owning_relation }` is the only identity type.
- `Value::Serial(u64)` is the runtime identity value.
- Foreign keys are explicit constraints over named unique constraints.
- Foreign keys are not field types.
- Foreign keys can target compound unique constraints.
- Foreign keys can include enum fields.
- FK compatibility is positional exact type equality.
- There is no migration or backwards compatibility promise.

## Required Public API Review

Review exports in:

```text
crates/bumbledb/src/lib.rs
crates/bumbledb-core/src/lib.rs
crates/bumbledb-lmdb/src/lib.rs
```

Remove exports for deleted types:

```text
UuidBytes
IdentityValue
IdentityAllocation
```

Ensure `Value`, `ValueType`, and schema descriptors expose only final v3 names.

## Required Naming Cleanup

Rename or remove helper names that suggest old reference-type semantics.

Examples:

```text
id_field -> serial_id_field or serial_field
ref_field -> serial_field or fk_serial_field, but it must not imply FK generation
identity -> serial_type
IdentityValue -> deleted
IdentityAllocation -> deleted
```

Preferred helper names:

```rust
fn serial_type(type_name: &str, owning_relation: &str) -> ValueType
fn serial_field(name: &str, type_name: &str, owning_relation: &str) -> FieldDescriptor
fn enum_type(name: &str) -> ValueType
fn enum_value(code: u8) -> Value
```

## Required Comments Cleanup

Search for and remove or update comments mentioning:

```text
UUID
identity allocation
application identity
ref field
reference type
primary key
Datalog
u64 enum code
```

Historical docs may retain old terms only if clearly marked historical.

## Required Compile-Time Rejection Search

The final code should not contain active Rust references to:

```text
Uuid
UuidBytes
IdentityValue
IdentityAllocation
ValueType::Identity
Value::Identity
ValueType::Uuid
Value::Uuid
ref_field
id_field
```

If `id_field` remains in benchmarks for readability, it must return `ValueType::Serial` and have no FK implication. Prefer deletion.

## Tests Required

- Public schema examples compile with v3 names.
- Docs examples in comments are updated where tested.
- No exported deleted types remain.
- Grep rejection list is clean.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- Active-code rejection list is clean.

## Completion Criteria

- Docs and public API say one coherent thing.
- Public naming reflects serial identities and generic FKs.
- No deleted v2/v1 concepts leak through active exports or examples.
- This PRD is deleted and committed after passing.
