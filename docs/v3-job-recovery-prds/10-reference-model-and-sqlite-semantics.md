# PRD 10: Reference Model And SQLite Semantics

## Goal

Update the in-memory reference model, test support, and SQLite comparisons for the final serial/byte-enum/generic-FK model.

## Explicit Non-Goals

- No backwards compatibility with old reference model identity wrappers.
- No support for old UUID values in tests.
- No support for old `u64` enum fixtures.
- No compatibility SQL for old count-empty behavior.
- No old Datalog/query-text test fixtures.

## Current Code Anchors

- `crates/bumbledb-test-support/src/reference.rs`
- `crates/bumbledb-test-support/src/sqlite.rs`
- `crates/bumbledb-test-support/src/schemas.rs`
- `crates/bumbledb-test-support/src/rows.rs`
- `crates/bumbledb-test-support/src/operations.rs`
- `crates/bumbledb-test-support/tests/property_and_differential.rs`
- `crates/bumbledb-test-support/tests/sqlite_comparison.rs`

## Required Reference Model Changes

Remove identity allocation branches and UUID branches.

Reference value matching should understand:

```text
Value::Serial <-> ValueType::Serial
Value::Enum(u8) <-> ValueType::Enum
```

Reference comparisons should work for all exact equality-key values.

## Required Reference FK Tests

The reference model does not need to enforce write-time FKs unless it already models writes. It must agree with LMDB query results on schemas that include:

- serial FKs
- enum FKs
- compound FKs
- mixed serial+enum FKs

## Required SQLite Semantics

SQLite comparison must account for Bumbledb set semantics.

Rules:

- Projection SQL uses `SELECT DISTINCT` when comparing projected rows.
- Count SQL for global count returns one row containing zero on empty input.
- Enum values are stored as integer bytes in SQLite.
- Serial values are stored as integer IDs in SQLite.

## Required Fixture Updates

Update fixture helpers:

```rust
Value::Identity(IdentityValue::Serial(id)) -> Value::Serial(id)
Value::Enum(840) -> Value::Enum(1)
Value::Enum(978) -> Value::Enum(2)
```

Update schema helpers:

```rust
identity(...) -> serial_type(...)
id_field(...) -> serial_id_field(...)
ref_field(...) -> serial_field(...)
```

Avoid helper names that imply FK semantics.

## Required Property Tests

Add property/differential coverage for:

- duplicate exact insert remains idempotent
- enum FK valid/invalid inserts
- compound FK valid/invalid inserts
- reference query result agrees with LMDB on enum/compound schemas

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test -p bumbledb-test-support`
- `cargo test --workspace --all-features`
- Grep test-support for `IdentityValue`, `IdentityAllocation`, `Uuid`, large enum constants like `840` and `978` in enum contexts.

## Completion Criteria

- Reference evaluator is serial/byte-enum only.
- SQLite comparison matches v3 set/count semantics.
- Test fixtures reflect final value model.
- This PRD is deleted and committed after passing.
