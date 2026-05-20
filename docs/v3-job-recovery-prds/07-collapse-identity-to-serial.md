# PRD 07: Collapse Identity To Serial

## Goal

Replace the current identity/allocation model with a single serial identity concept.

This deletes allocation branching and runtime identity wrappers while preserving nominal type safety at the schema/query-builder level.

## Explicit Non-Goals

- No backwards compatibility with `ValueType::Identity`.
- No backwards compatibility with `Value::Identity`.
- No `IdentityAllocation` compatibility aliases.
- No migration from application/UUID identity allocation strategies.
- No dual runtime representation for serial values.
- No storage reader for old identity-encoded values.

## Current State

Current model:

```rust
ValueType::Identity {
    type_name: String,
    owning_relation: String,
    allocation: IdentityAllocation,
}

pub enum IdentityAllocation {
    Serial,
    Uuid,
    Application,
}

pub enum IdentityValue {
    Serial(u64),
    Uuid(UuidBytes),
    Application(u64),
}

Value::Identity(IdentityValue)
```

Target model:

```rust
ValueType::Serial {
    type_name: String,
    owning_relation: String,
}

Value::Serial(u64)
```

## Required Schema Changes

Replace `ValueType::Identity` with `ValueType::Serial`.

Remove `IdentityAllocation` entirely.

Update canonical serialization:

```rust
ValueType::Serial { type_name, owning_relation } => {
    push_u8(out, SERIAL_TAG);
    push_str(out, type_name);
    push_str(out, owning_relation);
}
```

Do not preserve old identity allocation tags.

## Required Runtime Changes

Replace:

```rust
Value::Identity(IdentityValue::Serial(id))
```

with:

```rust
Value::Serial(id)
```

Delete `IdentityValue` and its public export.

## Required Helper Naming

Rename helper functions to match the new model:

```rust
fn serial_type(type_name: &str, owning_relation: &str) -> ValueType
fn serial_field(type_name: &str, relation: &str) -> FieldDescriptor
fn serial(value: u64) -> Value
```

Do not use `id_field` or `ref_field` names if they hide the new model. If helper names remain for ergonomics, they must return serial types and be clearly documented.

## Type Safety Rule

Two serial fields unify only if both `type_name` and `owning_relation` match.

```rust
ValueType::Serial { type_name: "AccountId", owning_relation: "Account" }
!=
ValueType::Serial { type_name: "InstrumentId", owning_relation: "Instrument" }
```

## FK Rule Reminder

Serial does not imply FK.

This is valid as a value type only:

```rust
FieldDescriptor::new("holder", serial_type("HolderId", "Holder"))
```

It becomes an FK only through:

```rust
ConstraintDescriptor::foreign_key("holder_fk", ["holder"], "Holder", "by_id")
```

## Required Tests

- Serial type distinguishability.
- Query builder rejects cross-serial variable unification.
- Query builder accepts matching serial variable unification.
- Runtime input type checks accept `Value::Serial` for `ValueType::Serial`.
- Runtime input type checks reject `Value::U64` for `ValueType::Serial`.
- All former identity tests are updated.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- Grep for `IdentityAllocation`, `IdentityValue`, `ValueType::Identity`, `Value::Identity` returns no active Rust code hits.
- Grep for `ValueType::Serial` and `Value::Serial` confirms the new model is used.

## Completion Criteria

- Identities are nominal serials.
- Allocation strategy no longer exists in schema or runtime.
- Reference/FK code is not hidden in identity types.
- This PRD is deleted and committed after passing.
