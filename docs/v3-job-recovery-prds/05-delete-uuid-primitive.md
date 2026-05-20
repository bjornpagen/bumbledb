# PRD 05: Delete UUID Primitive

## Goal

Delete UUID as a persistent primitive and remove all UUID-specific encoding, schema, query, storage, reference, and test branches.

This is an intentional simplification. Bumbledb v3 identities are nominal serials, and application data that needs UUID-like values should store them outside Bumbledb or as explicitly interned bytes only if the product later chooses to support that as opaque data. There is no first-class UUID key type.

## Explicit Non-Goals

- No backwards compatibility for schemas containing `ValueType::Uuid`.
- No migration from UUID fields to bytes or strings.
- No deprecated UUID aliases.
- No feature flag to keep UUID support.
- No storage reader for old UUID-encoded data.
- No UUID identity allocation compatibility path.

## Current Code Anchors

- `crates/bumbledb-core/src/encoding.rs`
- `UuidBytes`
- `encode_uuid`
- `decode_uuid`
- `crates/bumbledb-core/src/schema.rs`
- `ValueType::Uuid`
- `IdentityAllocation::Uuid`
- `crates/bumbledb-lmdb/src/storage.rs`
- `Value::Uuid`
- `IdentityValue::Uuid`
- `crates/bumbledb-lmdb/src/query.rs`
- UUID query hashing/type matching/display
- `crates/bumbledb-test-support/src/reference.rs`
- UUID reference matching/display

## Required Deletions

Delete all of these from active Rust code:

```text
UuidBytes
ValueType::Uuid
Value::Uuid
IdentityAllocation::Uuid
IdentityValue::Uuid
encode_uuid
decode_uuid
uuid width invalid
identity uuid width invalid
```

Remove UUID tests from `encoding.rs`.

Remove UUID key-width tests or convert them to other 16-byte types such as decimal if the test is about LMDB key size.

## Required Schema Changes

`ValueType` must no longer include `Uuid`.

Before:

```rust
ValueType::Uuid
```

After:

```rust
// no UUID primitive
```

Update `ValueType::encoded_width` to remove UUID.

Update canonical serialization to remove UUID tag.

Because backwards compatibility is out of scope, do not preserve UUID canonical tags.

## Required Storage Changes

Remove UUID runtime variant:

```rust
Value::Uuid(UuidBytes)
```

Remove UUID encode/decode branches.

Remove UUID type-name branches.

Remove UUID from any public exports.

## Required Query Changes

Remove UUID from:

- query shape hashing
- value type display
- input validation
- literal conversion, if any
- reference model comparison support

## Required Test Updates

Replace UUID schema tests with another fixed-width wide type if needed.

Example for key-size tests:

```rust
FieldDescriptor::new(format!("f{index}"), ValueType::Decimal { scale: 0 })
```

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- Grep for `Uuid`, `uuid`, `UuidBytes`, `encode_uuid`, `decode_uuid` returns no active Rust code hits.

## Completion Criteria

- UUID is not a schema type.
- UUID is not a runtime value.
- UUID is not an identity allocation mode.
- UUID has no sortable encoding helpers.
- This PRD is deleted and committed after passing.
