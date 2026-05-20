# PRD 08: Native Compound FK Key Model

## Goal

Make the schema-level FK model explicitly generic over exact unique-key tuples, including serial identities, enums, and compound combinations.

## Explicit Non-Goals

- No backwards compatibility with old reference-type FK assumptions.
- No implicit FK generation from field names or field types.
- No compatibility mode for serial-only FKs.
- No migration of old FK descriptors.
- No accepting loosely compatible enum/serial types.
- No positional field reordering to rescue old schemas.

This PRD is a schema/model PRD. Runtime enforcement is PRD 09.

## Core Principle

Foreign keys are not references as a field type. Foreign keys are constraints over key tuples.

The same FK mechanism must support:

```text
Serial -> Serial
Enum -> Enum
Serial + Enum -> Serial + Enum
Enum + Enum -> Enum + Enum
U64 + Enum -> U64 + Enum
```

The schema decides what is legal. The storage engine enforces encoded tuple-prefix existence.

## Target Schema Semantics

An FK is valid if:

```text
target relation exists
target constraint exists
target constraint is Unique
source field count == target unique field count
source field i ValueType == target field i ValueType for every i
```

This means enum FKs must match the same enum domain:

```rust
Enum { name: "Currency" } == Enum { name: "Currency" }
Enum { name: "Currency" } != Enum { name: "Country" }
```

This means serial FKs must match the same nominal serial type:

```rust
Serial { type_name: "AccountId", owning_relation: "Account" }
==
Serial { type_name: "AccountId", owning_relation: "Account" }
```

## Required Schema Tests

Add these tests to `crates/bumbledb-core/src/schema.rs` or a schema test module:

### Single Enum FK

```text
Currency(code: Enum(Currency)) unique covering by code
Account(currency: Enum(Currency)) FK -> Currency.by_code
```

Validation passes.

### Compound Enum FK

```text
Policy(country: Enum(Country), currency: Enum(Currency)) unique covering by country,currency
Account(country: Enum(Country), currency: Enum(Currency)) FK -> Policy.by_country_currency
```

Validation passes.

### Compound Serial Plus Enum FK

```text
AccountCurrency(account: Serial(AccountId), currency: Enum(Currency)) unique covering by account,currency
Posting(account: Serial(AccountId), currency: Enum(Currency)) FK -> AccountCurrency.by_account_currency
```

Validation passes.

### Enum Domain Mismatch

```text
source: Enum(Country)
```

Validation fails with `ForeignKeyTypeMismatch`.

### Field Order Mismatch

```text
source fields: [currency, country]
```

Validation fails unless the source fields are declared in the same semantic order as the target unique constraint.

FK order is explicit and positional.

## Required Error Quality

The existing `ForeignKeyTypeMismatch` should include enough detail:

```rust
ForeignKeyTypeMismatch {
    relation,
    constraint,
    source_field,
    target_field,
    source_type,
    target_type,
}
```

If the current error already does this, keep it. If not, improve it.

## Required API Helpers

Add test/schema helper constructors if useful:

```rust
fn serial_type(type_name: &str, owning_relation: &str) -> ValueType
fn enum_type(name: &str) -> ValueType
fn unique_covering(name: &str, fields: impl IntoIterator<Item = &str>) -> ConstraintDescriptor
fn fk(name: &str, fields: impl IntoIterator<Item = &str>, target: &str, target_unique: &str) -> ConstraintDescriptor
```

Do not hide FK generation behind field types.

## Non-Goals

- Do not implement runtime enforcement here. That is PRD 09.
- Do not add nullable or optional FK behavior.
- Do not add cascade actions.
- Do not add SQL-style deferred constraints.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test -p bumbledb-core`
- All schema tests listed above pass.
- Schema validation still rejects unknown target constraints.
- Schema validation still rejects FK arity mismatch.

## Completion Criteria

- The schema model explicitly supports enum and compound FKs.
- The type checker is exact and positional.
- There is no reference-type shortcut.
- This PRD is deleted and committed after passing.
