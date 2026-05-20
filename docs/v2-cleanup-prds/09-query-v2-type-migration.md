# PRD 09: Query v2 Type Migration

## Goal

Update query normalization, hashing, input validation, literal conversion, output decoding, reference execution, and benchmark/test helpers for the v2 value model.

This PRD is not the large query architecture cleanup. It is the correctness migration that removes old type branches and makes queries compile against schema v2.

## Current State

Old query type/value anchors:

- Query hash value type: `crates/bumbledb-lmdb/src/query.rs:2410-2445`
- Query input type validation: `query.rs:8427-8446`
- Id/ref normalization bridge: `query.rs:8448-8454`
- Literal conversion: `query.rs:8456-8477`
- Value type display: `query.rs:9503-9517`
- Reference literal conversion: `crates/bumbledb-test-support/src/reference.rs:407-427`
- Reference id/ref normalization: `reference.rs:429-435`
- Reference value matching: `reference.rs:448-464`
- Reference value type names: `reference.rs:466-480`
- SQLite value helpers: `crates/bumbledb-test-support/src/sqlite.rs:89-107`

## Required Changes

### Query Hashing

Update `hash_value_type` for the new v2 `ValueType`.

Target:

```rust
fn hash_value_type(hasher: &mut blake3::Hasher, value_type: &ValueType) {
    match value_type {
        ValueType::Bool => hash_u8(hasher, 1),
        ValueType::U64 => hash_u8(hasher, 2),
        ValueType::I64 => hash_u8(hasher, 3),
        ValueType::TimestampMicros => hash_u8(hasher, 4),
        ValueType::Decimal { scale } => {
            hash_u8(hasher, 5);
            hash_u32(hasher, *scale);
        }
        ValueType::Uuid => hash_u8(hasher, 6),
        ValueType::Enum { name } => {
            hash_u8(hasher, 7);
            hash_bytes_len_prefixed(hasher, name.as_bytes());
        }
        ValueType::String => hash_u8(hasher, 8),
        ValueType::Bytes => hash_u8(hasher, 9),
        ValueType::Identity {
            type_name,
            owning_relation,
            allocation,
        } => {
            hash_u8(hasher, 10);
            hash_bytes_len_prefixed(hasher, type_name.as_bytes());
            hash_bytes_len_prefixed(hasher, owning_relation.as_bytes());
            hash_identity_allocation(hasher, *allocation);
        }
    }
}
```

### Input Value Validation

Replace old matches with identity-aware matching.

Target:

```rust
fn value_matches_type(schema: &StorageSchema, value: &Value, value_type: &ValueType) -> bool {
    if let (Value::Enum(code), ValueType::Enum { name }) = (value, value_type) {
        return schema.descriptor().enum_contains_code(name, *code);
    }
    matches!(
        (value, value_type),
        (Value::Bool(_), ValueType::Bool)
            | (Value::U64(_), ValueType::U64)
            | (Value::I64(_), ValueType::I64)
            | (Value::Identity(_), ValueType::Identity { .. })
            | (Value::Timestamp(_), ValueType::TimestampMicros)
            | (Value::Decimal(_), ValueType::Decimal { .. })
            | (Value::Uuid(_), ValueType::Uuid)
            | (Value::Enum(_), ValueType::Enum { .. })
            | (Value::String(_), ValueType::String)
            | (Value::Bytes(_), ValueType::Bytes)
    )
}
```

Delete `normalize_value_for_type`. There is no `Id`/`Ref` conversion.

### Literal Conversion

Integer literals may build identity values only for 8-byte identity allocations.

Recommended:

```rust
fn literal_to_value(literal: &TypedLiteral) -> Result<Value> {
    match (&literal.literal, &literal.value_type) {
        (Literal::Bool(value), ValueType::Bool) => Ok(Value::Bool(*value)),
        (Literal::String(value), ValueType::String) => Ok(Value::String(value.clone())),
        (Literal::Integer(value), ValueType::U64) => Ok(Value::U64(*value as u64)),
        (Literal::Integer(value), ValueType::I64) => Ok(Value::I64(*value as i64)),
        (
            Literal::Integer(value),
            ValueType::Identity {
                allocation: IdentityAllocation::Serial,
                ..
            },
        ) => Ok(Value::Identity(IdentityValue::Serial(*value as u64))),
        (
            Literal::Integer(value),
            ValueType::Identity {
                allocation: IdentityAllocation::Application,
                ..
            },
        ) => Ok(Value::Identity(IdentityValue::Application(*value as u64))),
        (Literal::Integer(value), ValueType::Enum { .. }) => Ok(Value::Enum(*value as u64)),
        (Literal::Integer(value), ValueType::TimestampMicros) => {
            Ok(Value::Timestamp(TimestampMicros(*value as i64)))
        }
        (Literal::Integer(value), ValueType::Decimal { .. }) => {
            Ok(Value::Decimal(DecimalRaw(*value)))
        }
        _ => Err(Error::internal("typed literal does not match literal value")),
    }
}
```

If UUID identity literals are needed, add a typed UUID literal to `query_ir::Literal`. Do not use integer literals for UUID identities.

### Value Type Names

Recommended display:

```rust
fn value_type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Uuid => "uuid".to_owned(),
        ValueType::Enum { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
        ValueType::Identity {
            type_name,
            owning_relation,
            ..
        } => format!("{type_name}@{owning_relation}"),
    }
}
```

### Query IR Cleanup

In `crates/bumbledb-core/src/query_ir.rs`, remove unused future scaffolding if not used:

- `TypedPredicate`
- `TypedExpr`

The current executor only uses `TypedClause::Relation` and `TypedClause::Comparison`. If no call sites need `TypedPredicate`/`TypedExpr`, delete them now to reduce branchiness.

Do not remove aggregate support.

## Aggregate Empty Semantics Decision

Before changing sink behavior in PRD 10, lock the semantics here.

Recommended semantic contract:

- Projection over empty relation returns zero rows.
- Grouped aggregate over empty input returns zero rows.
- Global `count` over empty input returns one row containing `0`.
- Global `sum`, `min`, `max` over empty input are unsupported unless explicitly defined later, because nulls do not exist.

Current `GlobalCountSink::finish` at `query.rs:8919-8930` returns no row when count is zero. If the recommended contract is accepted, update it here or in PRD 10 and add tests.

## Tests Required

- Query with identity input validates.
- Query with wrong identity value kind fails input validation.
- Query literal identity serial converts correctly.
- Query builder rejects UUID identity integer literal unless UUID literal support exists.
- Query hash changes when identity type name changes.
- Query hash changes when identity owning relation changes.
- Reference executor matches runtime executor for identity queries.
- Code branches are absent from query/reference/sqlite helpers.
- Empty global count behavior is explicitly tested.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- Grep for `normalize_value_for_type` returns no real code references.
- Grep for `ValueType::Code` returns no real code references.
- Grep for `Value::Code` returns no real code references.
- Grep for `ValueType::Id` and `ValueType::Ref` returns no real code references.

## Completion Criteria

- Query execution compiles against schema v2 value types.
- No old Id/Ref/Code runtime compatibility remains.
- Aggregate empty semantics are tested and documented.
