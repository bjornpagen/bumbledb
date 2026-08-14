# schema-009: `SealedField.declared: Option` — synthetic id as absence of a descriptor

- **Severity:** medium
- **Tree:** schema
- **Status:** FIXED(0d6108f2)
- **Source:** audit/storage-schema.md F18
- **Depends on:** none
- **Conflicts with:** none (descriptor-side accessor; schema-001 is the sealed relation)

## The bug

`bumbledb-theory/src/schema.rs:427-437`:

```rust
pub struct SealedField<'a> {
    pub name: &'a str,
    pub value_type: &'a ValueType,
    /// The declared descriptor — `None` exactly at the synthetic id.
    pub declared: Option<&'a FieldDescriptor>,
}
```

`materialized_statements` tests `declared.is_some_and(|field| field.generation == Generation::Fresh)`. Synthetic vs declared is a sum, not a missing descriptor.

## Why it's wrong

Insight 4 — Option admits "a declared field with no descriptor" and "a synthetic id with a descriptor." Callers guard with `is_some_and`. Insight 3 — the synthetic-id law is a coordinate; Option is the Cartesian leftover.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree (theory schema is in this campaign's schema scope as the declaration vocabulary).

```rust
enum SealedField<'a> {
    SyntheticId,
    Declared(&'a FieldDescriptor),
}
```

`sealed_fields()` / `materialized_statements` match. Name `"id"` and type `U64` are inherent on `SyntheticId`.

## Acceptance criteria

- [ ] Gone: `rg -n 'declared: Option' crates/bumbledb-theory/src/schema.rs`.
- [ ] Gone: `rg -n 'declared.is_some_and' crates/bumbledb-theory/src/schema.rs`.
- [ ] Unchanged tests: closed auto-key materialization and manifest field-id tests green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb -p bumbledb-theory`; `./scripts/check.sh`.

## Constraints

- Synthetic id remains sealed ordinal 0, handle not a column. Fingerprint inputs unchanged. Spec resolver stays the structural peer (`spec.rs: Resolver::slot`).
