# PRD 04: Schema v2 Value Model

## Goal

Replace the old `Id`/`Ref`/`Code` value model with a single typed identity model and a smaller primitive type set.

This PRD changes logical schema types and runtime values. It intentionally breaks all callers and persisted fingerprints that rely on old variants.

## Current State

Old logical variants live at `crates/bumbledb-core/src/schema.rs:1165-1195`:

```rust
pub enum ValueType {
    Bool,
    U64,
    I64,
    Id { name: String, relation: String },
    Ref { name: String, target_relation: String },
    TimestampMicros,
    Decimal { scale: u32 },
    Uuid,
    Enum { name: String },
    Code { name: String },
    String,
    Bytes,
}
```

Old runtime variants live at `crates/bumbledb-lmdb/src/storage.rs:119-146`:

```rust
pub enum Value {
    Bool(bool),
    U64(u64),
    I64(i64),
    Id(u64),
    Ref(u64),
    Timestamp(TimestampMicros),
    Decimal(DecimalRaw),
    Uuid(UuidBytes),
    Enum(u64),
    Code(u64),
    String(String),
    Bytes(Vec<u8>),
}
```

Old compatibility branches exist in:

- `crates/bumbledb-core/src/schema.rs:1197-1242`, `1244-1279`
- `crates/bumbledb-lmdb/src/storage.rs:1597-1629`, `1666-1682`, `1784-1836`, `1838-1852`
- `crates/bumbledb-lmdb/src/query.rs:2410-2445`, `8427-8477`, `9503-9517`
- `crates/bumbledb-test-support/src/reference.rs:407-479`
- `crates/bumbledb-test-support/src/sqlite.rs:89-107`

## Required Type Model

Replace `ValueType` with this shape:

```rust
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueType {
    Bool,
    U64,
    I64,
    TimestampMicros,
    Decimal { scale: u32 },
    Uuid,
    Enum { name: String },
    String,
    Bytes,
    Identity {
        type_name: String,
        owning_relation: String,
        allocation: IdentityAllocation,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentityAllocation {
    Serial,
    Uuid,
    Application,
}
```

Replace runtime identity values with:

```rust
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Bool(bool),
    U64(u64),
    I64(i64),
    Identity(IdentityValue),
    Timestamp(TimestampMicros),
    Decimal(DecimalRaw),
    Uuid(UuidBytes),
    Enum(u64),
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentityValue {
    Serial(u64),
    Uuid(UuidBytes),
    Application(u64),
}
```

If implementation simplicity demands one runtime representation first, this is acceptable:

```rust
pub enum Value {
    Identity(u64),
    // other variants
}
```

But only if `IdentityAllocation::Uuid` is rejected during validation until UUID identity storage is implemented. Do not silently encode UUID identities as `u64`.

## Semantics

### Identity Type Equality

Two identity fields are the same logical type iff all type-defining fields match.

Recommended final rule:

```rust
fn identity_types_compatible(left: &ValueType, right: &ValueType) -> bool {
    match (left, right) {
        (
            ValueType::Identity {
                type_name: left_name,
                owning_relation: left_relation,
                allocation: left_allocation,
            },
            ValueType::Identity {
                type_name: right_name,
                owning_relation: right_relation,
                allocation: right_allocation,
            },
        ) => {
            left_name == right_name
                && left_relation == right_relation
                && left_allocation == right_allocation
        }
        _ => left == right,
    }
}
```

If allocation is considered production strategy rather than logical type, the PRD implementer may choose to ignore allocation for type equality, but must document it and add tests. The default should be strict equality.

### Reference Semantics

There is no `Ref` type.

A referencing field uses the target identity type directly:

```rust
FieldDescriptor::new(
    "holder",
    ValueType::Identity {
        type_name: "HolderId".to_owned(),
        owning_relation: "Holder".to_owned(),
        allocation: IdentityAllocation::Serial,
    },
)
```

The fact that `Account.holder` references `Holder` is expressed only by an explicit `ForeignKey` constraint in PRD 05.

### Code Removal

There is no open code domain type.

Replace current `ValueType::Code { name }` usages with:

- `ValueType::U64` for open numeric domains.
- `ValueType::Enum { name }` for closed domains with known variants.
- `ValueType::Identity` only when the value is a nominal identity.

Known current `Code` anchors:

- `crates/bumbledb-bench/src/main.rs:2274-2316`, `2375`, `2401`, `2525-2533`, `2763-2768`
- `crates/bumbledb-bench/src/open.rs:1309`, `1379`, `1412`, `1436`, `1450-1515`, `1533`, `1545`
- `crates/bumbledb-test-support/src/reference.rs:416`, `460`, `476`, `493`
- `crates/bumbledb-test-support/src/sqlite.rs:103-107`

## Encoding Requirements

Update `ValueType::encoded_width`.

Recommended:

```rust
impl ValueType {
    pub fn encoded_width(&self) -> usize {
        match self {
            ValueType::Bool => 1,
            ValueType::U64
            | ValueType::I64
            | ValueType::TimestampMicros
            | ValueType::Enum { .. }
            | ValueType::String
            | ValueType::Bytes => 8,
            ValueType::Decimal { .. } | ValueType::Uuid => 16,
            ValueType::Identity { allocation, .. } => match allocation {
                IdentityAllocation::Serial | IdentityAllocation::Application => 8,
                IdentityAllocation::Uuid => 16,
            },
        }
    }
}
```

Update `is_orderable`:

```rust
pub fn is_orderable(&self) -> bool {
    matches!(
        self,
        ValueType::U64
            | ValueType::I64
            | ValueType::TimestampMicros
            | ValueType::Decimal { .. }
            | ValueType::Identity {
                allocation: IdentityAllocation::Serial,
                ..
            }
    )
}
```

`IdentityAllocation::Application` is not orderable unless the implementation explicitly commits to ordered application ids.

## Canonical Serialization

Update `ValueType::push_canonical` at `crates/bumbledb-core/src/schema.rs:1244-1279`.

Do not preserve old tags. This is schema v2.

Recommended tags:

```text
1 Bool
2 U64
3 I64
4 TimestampMicros
5 Decimal
6 Uuid
7 Enum
8 String
9 Bytes
10 Identity
```

Identity canonical bytes:

```rust
ValueType::Identity {
    type_name,
    owning_relation,
    allocation,
} => {
    push_u8(out, 10);
    push_str(out, type_name);
    push_str(out, owning_relation);
    allocation.push_canonical(out);
}
```

Allocation tags:

```text
1 Serial
2 Uuid
3 Application
```

## Runtime Encoding Requirements

Update `crates/bumbledb-lmdb/src/storage.rs`:

- `encode_value_for_type`
- `storage_value_matches_type`
- `decode_value`
- `value_type_name`
- `Value::kind_name`

Remove `normalize_value_for_type` bridges in `crates/bumbledb-lmdb/src/query.rs:8448-8454` and `crates/bumbledb-test-support/src/reference.rs:429-435`.

There is no more `Id` versus `Ref` normalization.

## Tests Required

Add or update tests for:

- Identity types with different `owning_relation` are not equal.
- Identity fields with same type unify in query builder.
- Identity fields with different owning relation fail query builder typecheck.
- `ValueType::Code` does not exist.
- `Value::Code` does not exist.
- Closed enum still validates codes.
- Open numeric code fields in benchmarks now use `U64`.
- Identity encoding round trips for supported allocation modes.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- Grep for `ValueType::Id` returns no real code references.
- Grep for `ValueType::Ref` returns no real code references.
- Grep for `ValueType::Code` returns no real code references.
- Grep for `Value::Id` returns no real code references.
- Grep for `Value::Ref` returns no real code references.
- Grep for `Value::Code` returns no real code references.

## Completion Criteria

- The logical type system has one identity concept.
- Reference semantics are no longer encoded in field types.
- Open code domains are plain `U64` or closed `Enum`.
- Runtime storage/query/reference code no longer has Id/Ref/Code branches.
