# PRD 02 — The emission: handles, host enums, the manifest

**Depends on:** 01 (the descriptor shape it emits into).
**Modules:** `crates/bumbledb-macros/src/lib.rs` (grammar + emission),
`schema.rs` (manifest surface).
**Authority:** `70-api.md` (schema! grammar), PRD-algebra 20 (id constants,
the manifest), README refusals.
**Representation move:** the host's `match` exhaustiveness is an *emission*,
not a type. The engine's vocabulary is relational; the macro projects it into
a Rust enum so rustc's pattern checking keeps working — one vocabulary, two
checkers, zero drift (the ids are the same declaration-order numbers on both
sides, welded by codegen).

## Context (decided shape)

The grammar (three tiers, all one production):

```
closed relation Status as StatusId = { Open, Frozen, Closed };

closed relation Kind as KindId {
    mastered: bool,
} = {
    DirectPass { mastered: true },
    Failed     { mastered: false },
};
```

- `closed` is a leading keyword on the existing `relation` production; `as
  NewType` is REQUIRED (the handle needs a host type); the column block is
  optional (tier 1); the `= { ... }` extension block is required and
  non-empty; each row is `Handle` or `Handle { field: literal, ... }` with
  every declared column present exactly once (macro panics with the missing/
  extra field name — expansion-time, spanned).
- Row literals reuse the existing selection-literal parsing and typing
  (`value_expr`, macros `lib.rs` — same machine, same errors).

**Emission per closed relation:**

1. `pub enum Kind { DirectPass, Failed }` — the **host enum**: variants in
   declaration order, `#[repr(u64)]`-equivalent mapping via
   `impl Kind { pub const fn id(self) -> KindId }` and
   `pub const fn from_id(KindId) -> Option<Kind>`; derives mirror the
   existing emitted-enum derives.
2. `pub struct KindId(pub u64)` — the newtype, via the existing
   `emit_newtypes` machinery (closed handles are u64-backed, order ops
   refused at the IR as for any reference).
3. **Handle constants**: `impl Kind { pub const DIRECT_PASS: KindId = ... }`
   — wait, no: the enum variants ARE the handles; emit instead
   `Theory::KIND: RelationId` + per-handle `Kind::DirectPass.id()` being
   `const fn` — usable in const contexts. No separate constant namespace; the
   host enum is the constant namespace.
4. **No fact struct, no `Fact` impl** — closed relations are unwritable;
   emitting a writable struct would be a lie the type system tells. Reads go
   through queries and the dyn surface.
5. Descriptor construction: the `Theory::descriptor()` body gains the
   `extension` field with handle strings + values (the literal encoding
   already spliced by `value_expr`).
6. **The manifest** (PRD-algebra 20's name→id surface) gains closed
   extensions: relation → [(handle, id, [(column, value)])] — plain data from
   the descriptor, so foreign surfaces (render, future bindings) see the
   vocabulary without touching Rust.

## Technical direction

1. Parse: extend `parse_relation` with the `closed` prefix and the `= { }`
   tail; rows into the AST as `(handle: String, values: Vec<(String,
   Literal)>)`; expansion-time checks: duplicate handle, unknown/missing/
   duplicate field per row, literal-vs-column type (all reuse existing panic
   paths with the offending token named).
2. Emit: new `emit_closed` alongside `emit_enums` (which PRD 05 later
   deletes); the host enum's `from_id` is a match over declaration indices;
   `id()` is `KindId(self as u64)` semantics without relying on repr — emit
   an explicit match (weaker-model note: do NOT use `as` casts on the enum;
   emit the match arms).
3. Selections/statements referencing handles: `| kind == DirectPass` inside
   the schema block resolves the handle to its declaration index at
   expansion, exactly as enum variants resolve today (`value_expr` — the
   handle namespace is per-closed-relation, resolved via the referenced
   field's newtype → owning closed relation; a handle used on a field whose
   newtype is not a closed relation's is an expansion panic).
4. Manifest: extend the manifest builder in `schema.rs` with the extension
   table; no serde (dependency law).

## Passing criteria

- `[test]` The tier-1 and tier-2 grammars expand; the emitted descriptor
  round-trips through `validate()` (ties to PRD 01's roster).
- `[test]` Host-enum weld: `Kind::from_id(Kind::DirectPass.id()) ==
  Some(Kind::DirectPass)` for every variant, exhaustively, in a generated
  test the macro emits alongside (the weld test is EMITTED per closed
  relation, so it cannot be forgotten for new theories).
- `[test]` Compile-fail suite: duplicate handle; missing column in a row;
  extra column; type-mismatched literal; `closed relation` without `as`;
  handle literal on a non-closed field.
- `[shape]` No `Fact` impl is emitted for closed relations (grep the
  expansion of a fixture theory); the manifest carries the extension.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`70-api.md`: the grammar tiers, the host-enum weld, the no-fact-struct rule.
Repo `README.md` theory-grammar table: the closed-relation row (replacing the
enum row when PRD 05 lands — this PRD adds, 05 deletes).
