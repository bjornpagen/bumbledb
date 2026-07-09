# PRD 02 — Kill the decl layer

**Depends on:** 01.
**Modules:** `crates/bumbledb-macros/src/lib.rs`,
`crates/bumbledb/src/schema/runtime.rs` (dies or shrinks to the materialization
entry), the `__private` re-export surface, macro expansion tests.
**Authority:** `70-api.md` (the macro generates descriptors — nothing says
"via decl tables"), `00-product.md` (representation over control flow; every
mechanism names its reader).

## Context (decided)

Between the macro surface and `SchemaDescriptor` sits a third representation:
the decl layer (`StatementDecl`, `SideDecl`, `LiteralDecl` in `runtime.rs`,
re-exported through `__private`) — static tables the macro emits, which
`build_schema` then walks resolving *names to ids*. The layer is a mirror world:
`LiteralDecl` is `Value` again with `Str(&'static str)` (runtime.rs:50), and the
name→id resolution it exists for is work the macro can already do — it resolves
enum variant names to ordinals *in the macro* today (`LiteralDecl::Enum(u8)` is
proof), and it sees the whole schema, so it knows every relation's and field's
declaration index. The decl layer has no reader that couldn't read descriptors
directly. Delete the middle representation: surface → descriptors.

## Technical direction

1. **The macro emits `SchemaDescriptor` construction directly**: a `schema()`
   function building `RelationDescriptor`s and `StatementDescriptor`s at
   runtime (plain `vec![]`/`Box<[...]>` construction) with **ids resolved at
   expansion time** — `RelationId(n)`/`FieldId(n)` from declaration order, enum
   ordinals as today, selection literals as shared `Value` (string literals
   become `Value::String(Box::from(*b"..."))` — runtime construction, so no
   const-context type is needed, which is the entire reason `LiteralDecl`
   existed).
2. **`runtime.rs` shrinks to whatever genuinely remains** — if
   `materialized_statements` (the serial auto-key ordering rule) is its only
   survivor, move that next to the descriptor types and delete the file. Every
   decl type dies; the `__private` surface shrinks to what codegen actually
   references.
3. **Name resolution errors move, they don't vanish:** an unknown field name in
   a statement is today a `build_schema`-time panic or error — it becomes a
   macro-expansion error with a span (strictly better diagnostics; targeted
   message naming the relation and field). Semantic validation beyond
   name-to-id (types, roster, acceptance gate) stays exactly where it is —
   `SchemaDescriptor::validate` — per `70-api.md`'s parse-shape-only rule for
   the macro.
4. **Behavior-preservation proof is the fingerprint:** the same schema source
   must produce a byte-identical fingerprint before and after this PRD — the
   fingerprint covers everything semantic about the declaration, so equality
   pins the whole refactor with one assertion.

## Passing criteria

- `[shape]` No decl types exist (`StatementDecl`, `SideDecl`, `LiteralDecl` —
  grep); the macro's emitted `schema()` constructs descriptors directly with
  pre-resolved ids; `__private` exports only what expansion references.
- `[test]` The macro expansion tests assert descriptor contents directly (they
  largely already do — update construction, not assertions).
- `[test]` Fingerprint equality: the bench ledger schema's fingerprint is
  byte-identical across this PRD (golden pinned before, asserted after).
- `[test]` An unknown field name in a statement produces a compile error with
  the targeted message (compile_fail doctest, matching the macro's existing
  negative-test style).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`70-api.md`: if the grammar section mentions the decl/lowering pipeline, it now
says the macro emits descriptors directly; otherwise no change.
