# PRD 05 — The `schema!` macro: statement grammar

**Depends on:** 01, 02.
**Modules:** `crates/bumbledb-macros/src/lib.rs` (the whole crate).
**Authority:** `docs/architecture/70-api.md` (§ The `schema!` grammar — normative), `30-dependencies.md` (statement semantics).

## Goal

The macro parses relation blocks plus raw dependency statements and generates the
new descriptors. The old field-modifier grammar (`unique`, `fk(...)`,
relation-level `fk`) is deleted — those tokens must fail to parse with a targeted
error, not be silently ignored.

## Technical direction

1. The macro stays hand-rolled over `proc_macro::TokenStream` (no syn/quote —
   dependency policy, `00-product.md`).
2. **Field grammar:** `name: type [as Ident] [, serial]` where type ∈
   `bool | u64 | i64 | str | bytes | enum Ident { Variants } | interval < i64 > | interval < u64 >`.
   `as` is legal on `u64`, `i64`, and both intervals. `serial` legal on `u64` only
   (existing check). Parsing `unique` or `fk` anywhere must produce a compile error
   naming the replacement: "field-level constraints do not exist; write a statement
   — see docs/architecture/30-dependencies.md".
3. **Statement grammar** (top-level items alongside `relation` blocks, each
   terminated by `;`):
   ```
   stmt      := side '->' ident ';'          // Functionality: right ident must equal side's relation
              | side '<=' side ';'           // Containment
              | side '==' side ';'           // lowered to two Containments (A<=B, B<=A)
   side      := ident '(' fieldlist [ '|' sellist ] ')'
   fieldlist := ident (',' ident)*
   sellist   := ident '==' literal (',' ident '==' literal)*
   literal   := int | '-' int | 'true' | 'false' | ident            // bare ident = enum variant name
              | strlit | bytestrlit | int '..' int                  // interval literal, half-open
   ```
   **Interval literals are `start..end`** (Rust's own range tokens; the
   architecture docs' `[start, end)` notation is mathematical, not lexical — amend
   the one literal-syntax line in `70-api.md` to say `start..end` in the same
   change, per README rule 5).
   Enum-variant literals are bare idents resolved against the selected field's
   variant list at *schema validation* (the macro emits the name; `runtime.rs`
   resolves to the ordinal — extend `LiteralValue` construction helpers
   accordingly, or resolve in the macro if the enum is declared in the same
   invocation: prefer resolving in the macro, since all enums are).
   For `->`: emit `StatementDescriptor::Functionality`; error if the right-hand
   ident differs from the side's relation ident, with the message "an FD's right
   side is its own relation: R(X) -> R".
4. **Codegen:** per relation — the fact struct (interval fields typed
   `bumbledb::Interval<i64>` / `<u64>`, or the `as`-newtype wrapping it), the
   scalar/interval newtypes, `Fact` impls (encode paths call the PRD 01 encoders),
   `Serial` newtype impls, and the `schema()` constructor producing
   `SchemaDescriptor { relations, statements }` with statements in source order
   (`==` contributing its two lowered statements adjacently, `A<=B` first).
5. Macro-level diagnostics are parse-shape only; everything semantic (unknown
   fields, type mismatches, key resolution) flows through schema validation
   (PRD 03) as typed errors. Do not duplicate semantic checks in the macro.

## Out of scope

IR/query surface, WriteTx codegen changes (PRD 10 uses existing `Fact` machinery).

## Passing criteria

- `[shape]` The tokens `unique` and `fk` produce compile errors with the
  replacement message; no code path accepts them.
- `[shape]` `70-api.md`'s literal-syntax line says `start..end` (amended in this
  PRD's change).
- `[test]` Macro expansion tests (the crate's existing test style): the
  `30-dependencies.md` example schema expands to the exact expected
  `SchemaDescriptor` — statement order, `==` lowering to two adjacent
  Containments, selection ordinals resolved.
- `[test]` A schema with `active: interval<i64> as ActiveDuring` generates a
  newtype wrapping `Interval<i64>` and a fact struct using it.
- `[test]` Negative expansion tests: `Rel(x) -> Other;` (FD naming a different
  relation), a `sellist` on an FD (parses, then rejected by PRD 03 — assert the
  macro *passes it through* rather than erroring), and stray `unique` (macro
  error).
