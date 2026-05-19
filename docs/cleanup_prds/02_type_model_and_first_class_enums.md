# PRD 02: Type Model And First-Class Enums

## Status

Draft. This PRD depends on PRD 01.

## Problem

The current type model is performance-friendly but incomplete. `ValueType::Symbol { name }` is used as an enum-like type, but it is only a named `u64` domain. It has no declared variants, label mapping, validation, or ergonomic query/input representation.

## Goals

- Preserve the fixed-width scalar hot path.
- Replace pseudo-enum `Symbol` with first-class closed enum domains.
- Keep enum encoding as 8-byte sortable `u64` codes.
- Validate enum domains and values at schema and write boundaries.
- Prepare enum literals for future Logica lowering without implementing Logica.
- Clarify ID/ref scalar type semantics ahead of compound FK work.

## Non-Goals

- No vectors.
- No FlatBuffers.
- No floats.
- No nullable values.
- No open-ended dynamic symbol table.
- No ordered text indexes.
- No Logica parser implementation.

## Current Code References

- `ValueType` in `crates/bumbledb-core/src/schema.rs`.
- `Value` in `crates/bumbledb-lmdb/src/storage.rs`.
- `storage_value_matches_type` and `encode_value_for_type` in `storage.rs`.
- `value_matches_type`, `literal_to_value`, and `value_type_name` in `query.rs`.
- `literal_fits_type` and `type_name` in `datalog.rs`.
- Test fixtures using `ValueType::Symbol` in benchmark and test-support schemas.

## Required Type Model

Add schema-level enum declarations:

```rust
pub struct EnumDescriptor {
    pub name: String,
    pub variants: Vec<EnumVariantDescriptor>,
}

pub struct EnumVariantDescriptor {
    pub name: String,
    pub code: u64,
}
```

Extend schema:

```rust
pub struct SchemaDescriptor {
    pub name: String,
    pub enums: Vec<EnumDescriptor>,
    pub relations: Vec<RelationDescriptor>,
}
```

Replace:

```rust
ValueType::Symbol { name }
```

with:

```rust
ValueType::Enum { name: String }
```

Replace or augment:

```rust
Value::Symbol(u64)
```

with one of:

```rust
Value::Enum { name: String, variant: String }
Value::EnumCode(u64)
```

The preferred public input form is label-based. The storage/internal encoded form is code-based.

## Required Enum Semantics

- Enum domains are closed.
- Enum variant names are unique within a domain.
- Enum variant codes are unique within a domain.
- Enum codes are stable within a schema fingerprint.
- Enum values encode as big-endian `u64`.
- Enum value type has encoded width 8.
- Enum equality is supported.
- Enum joins are supported only within the same enum domain.
- Enum comparison ordering is disabled by default unless the enum declares `ordered: true` in a future extension.
- Enum range indexes are forbidden unless ordered semantics are explicitly enabled.

## ID And Ref Cleanup

The current `ValueType::Id { name, relation }` and `ValueType::Ref { name, target_relation }` serve two roles:

- They describe scalar ID domains.
- `Ref` implicitly triggers FK enforcement.

This PRD prepares for PRD 03 by requiring the type system to support scalar ID-domain compatibility independently from FK enforcement.

The target concept is:

```rust
ValueType::Id { name: String }
```

or an equivalent domain type that does not itself enforce FK existence.

PRD 03 decides whether the existing `Ref` variant is removed, renamed, or demoted to a pure scalar alias. FK existence must move to relation-level constraints.

## Field Capabilities

Add a capability API such as:

```rust
impl ValueType {
    pub fn encoded_width(&self) -> usize;
    pub fn is_key_eligible(&self) -> bool;
    pub fn is_orderable(&self) -> bool;
    pub fn supports_range_index(&self) -> bool;
    pub fn supports_equality_index(&self) -> bool;
    pub fn is_interned_placeholder(&self) -> bool;
}
```

Required capability outcomes:

| Type | Key | Equality | Range | Ordered |
| --- | --- | --- | --- | --- |
| Bool | yes | yes | no | no |
| U64/I64 | yes | yes | yes | yes |
| Id | yes | yes | yes | yes |
| Timestamp | yes | yes | yes | yes |
| Decimal | yes | yes | yes | yes |
| Uuid | yes | yes | no | no |
| Enum | yes | yes | no by default | no by default |
| String | yes via intern id | yes | no | no |
| Bytes | yes via intern id | yes | no | no |

## Canonical Fingerprint Requirements

Schema canonical bytes must include:

- Enum declaration count.
- Enum names in declaration order.
- Variant names and codes in declaration order.
- Field `ValueType::Enum` references.

Changing an enum variant name, code, declaration order, or field enum domain must change the fingerprint.

## Implementation Plan

1. Add `EnumDescriptor` and `EnumVariantDescriptor` to `schema.rs`.
2. Extend `SchemaDescriptor` to carry enum descriptors.
3. Add constructors that keep test ergonomics reasonable.
4. Update canonical schema fingerprinting.
5. Replace `ValueType::Symbol` usages with `ValueType::Enum`.
6. Replace `Value::Symbol` usages with enum-aware values or a transitional internal code type.
7. Add schema validation for enum domains and variants.
8. Update storage encode/decode paths.
9. Update query literal/input/typecheck paths.
10. Update benchmark/test-support schemas and rows.

## Strict Passing Criteria

- No remaining `ValueType::Symbol` in production code.
- No remaining `Value::Symbol` in production code unless explicitly renamed as a non-enum open-code type.
- Invalid enum domain references fail schema validation.
- Duplicate enum names fail schema validation.
- Duplicate variant names fail schema validation.
- Duplicate variant codes fail schema validation.
- Insert of unknown enum value fails before index mutation.
- Query input of wrong enum domain fails.
- Join between two different enum domains is rejected by typechecking/IR validation.
- Enum values still occupy 8 bytes in hot keys and query images.

## Verification Commands

```sh
cargo test -p bumbledb-core schema
cargo test -p bumbledb-lmdb storage
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
