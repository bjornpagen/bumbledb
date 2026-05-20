# PRD 06: Byte Enum Encoding

## Goal

Change enum storage from `u64` to `u8`.

Enums are closed symbolic domains. They should be compact and hot-key friendly. A one-byte encoded enum is enough for v3 and simplifies compound FK keys involving enum fields.

## Explicit Non-Goals

- No backwards compatibility with old `u64` enum encoding.
- No migration from old enum code widths.
- No accepting enum codes larger than `255` for legacy data.
- No dual-width enum decoder.
- No storage compatibility shim for existing v2 enum keys.

## Current State

Current enum values are `u64`:

```rust
Value::Enum(u64)
EnumVariantDescriptor { code: u64 }
```

Current encoded width is 8 bytes via `encode_u64` / `decode_u64`.

## Target Model

```rust
pub enum Value {
    Enum(u8),
    // other variants
}

pub struct EnumVariantDescriptor {
    pub name: String,
    pub code: u8,
}
```

`ValueType::Enum { name }` remains.

## Required Encoding

Add or reuse one-byte encoding:

```rust
pub fn encode_enum(value: u8) -> [u8; 1] {
    [value]
}

pub fn decode_enum(bytes: &[u8]) -> Result<u8, EncodingError> {
    let bytes = exact::<1>(bytes)?;
    Ok(bytes[0])
}
```

It is acceptable to use `encode_bool` style direct `[value]` internally, but named helpers are clearer.

## Required Schema Changes

Update `ValueType::encoded_width`:

```rust
ValueType::Enum { .. } => 1
```

Update enum descriptor APIs:

```rust
EnumDescriptor::codes("Currency", [1, 2, 3])
```

If call sites pass integer literals, infer or cast to `u8` safely.

Validation must reject codes greater than `255`.

Recommended error:

```rust
InvalidEnumCodeWidth { enum_name: String, code: u64 }
```

or reuse `InvalidConstraint`/existing enum error if simpler.

## Required Query Builder Changes

Integer literals for enum fields must fit `0..=255` and be declared in the enum domain.

```rust
(Literal::Integer(value), ValueType::Enum { name }) => {
    *value >= 0 && *value <= u8::MAX as i128 && schema.enum_contains_code(name, *value as u8)
}
```

## Required Storage Changes

Update:

- `encode_value_for_type`
- `decode_value`
- `storage_value_matches_type`
- enum validation
- SQLite helper conversions
- benchmark/test row construction
- reference model literal conversion

## Required FK Coverage

Add tests proving enum FKs work after one-byte encoding:

```text
Currency(code: Enum(Currency)) unique by code
Account(currency: Enum(Currency)) FK -> Currency.by_code
```

Also test compound enum FK:

```text
Policy(country: Enum(Country), currency: Enum(Currency)) unique by both
Account(country, currency) FK -> Policy.by_country_currency
```

## Required Test Updates

Update all `Value::Enum(840)`-style test fixtures to byte-sized domains.

Examples:

```text
USD = 1
EUR = 2
Unknown = 3
```

Do not keep `840` or `978` in enum tests. If ISO numeric codes are desired in user data, model them as `U64`, not `Enum`.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- Grep confirms no enum encode/decode path uses `encode_u64` or `decode_u64` for enum values.
- All enum field widths in generated layouts are 1 byte.

## Completion Criteria

- Enums are one byte in schemas, storage, query images, and access keys.
- Enum FKs and compound enum FKs pass.
- Tests no longer use large enum codes.
- This PRD is deleted and committed after passing.
