## `fresh` on a non-u64 field emits type-mismatched Fresh/Key impls — raw rustc errors bury the typed teaching path

category: bug | severity: medium | verdict: CONFIRMED | finder: macros:core

### Summary

The `schema!` macro's `fresh` modifier is documented as "legal on u64, validated at the engine's `SchemaDescriptor::validate`, as the macro defers it" (bumbledb-theory/src/schema/spec.rs:86-88). But the macro's deferral can never arrive: `parse_field` accepts `fresh` on any newtyped field (i64, bytes<N>, interval), and `emit_fact_struct` then unconditionally emits u64-shaped `Fresh` and `Key` impls for the newtype. For a non-u64 fresh field the generated code fails typechecking, so the user gets four raw `E0308` errors spanned at the entire macro invocation — the documented `SchemaError::FreshOnNonU64` teaching error is unreachable from the macro surface, and no expansion-time teaching message replaces it.

### Evidence (all verified against the working tree)

- crates/bumbledb-macros/src/lib.rs:438-444 — the `as NewType` gate admits `FieldTy::U64 | FieldTy::I64 | FieldTy::FixedBytes(_) | FieldTy::Interval(..)`, so `id: i64 as RId` parses.
- crates/bumbledb-macros/src/lib.rs:459-472 — the `fresh` arm asserts only `field.newtype.is_some()`; there is no check that the field is u64.
- crates/bumbledb-macros/src/lib.rs:2479-2508 — for every `(fresh, newtype)` field, the macro emits `fn from_fresh(raw: u64) -> Self {{ Self(raw) }}` (line 2490), `fn fresh(self) -> u64 {{ self.0 }}` (line 2491), and two `ValueRef::U64(self.0)` key encodings (lines 2498, 2502) — all assuming the newtype wraps u64.
- Reproduced: compiling `bumbledb::schema! { pub T; relation R { id: i64 as RId, fresh, name: str } }` against the workspace crate yields exactly 4 `error[E0308]: mismatched types ... expected u64, found i64`, each spanned at the whole invocation with `note: this error originates in the macro bumbledb::schema` and a `.try_into().unwrap()` suggestion aimed at the invocation's closing brace.
- crates/bumbledb/src/schema/validate.rs:1442 and src/error.rs:140 — `SchemaError::FreshOnNonU64` exists and is tested, but only via the raw descriptor API (crates/bumbledb/src/schema/tests/reject.rs:53-66 constructs a `FieldDescriptor` directly). No macro user can reach it.
- crates/bumbledb-macros/src/lib.rs:784-793 — the file's own `ParseError` doctrine: grammar mistakes that "carry MEANING worth teaching" get a spanned `compile_error!` at the offending token, because "an expansion panic at the invocation would bury the lesson." This is precisely such a mistake, unhandled.
- crates/bumbledb/tests/schema-compile-fail/ — no fresh-on-non-u64 case exists (the only fresh-related case, `foreign_fresh_witness.rs`, tests the cross-schema witness law).

### Spec cross-check

docs/architecture-level contract lives in bumbledb-theory/src/schema/spec.rs:86-88, which explicitly promises the macro defers the u64 judgment to `SchemaDescriptor::validate`. The code diverges from that spec: validation is unreachable because the program never compiles, so the deferral is a fiction on the macro path. This strengthens the finding rather than refuting it.

### Failure scenario

```rust
bumbledb::schema! {
    pub T;
    relation R { id: i64 as RId, fresh, name: str }
}
```
produces four E0308 errors in invisible generated code, spanned at the whole invocation, instead of either a spanned expansion-time teaching error or the typed `SchemaError::FreshOnNonU64`. The same holds for `bytes<N>` and interval fresh fields. This is a representation-doctrine violation in the finding's sense: the illegal state (fresh on non-u64) is representable in the macro's `Field` AST and dies only as an accident of downstream typechecking.

### Suggested fix

In `parse_field`'s fresh arm (lib.rs:463), add alongside the existing newtype assert:

```rust
assert!(
    matches!(field.ty, FieldTy::U64),
    "schema!: fresh field `{}` must be u64 — fresh is the mint mark and \
     mints are u64 generations (spec: Fresh legal on u64 only)",
    field.name
);
```

or route it through the existing `ParseError` machinery for a token-spanned diagnostic, matching the file's stated doctrine. Add a `fresh_on_non_u64.rs` case to crates/bumbledb/tests/schema-compile-fail/ pinning the teaching message, and amend the spec.rs:86-88 comment (the macro no longer defers — it judges at expansion, as it already does for the newtype requirement).
