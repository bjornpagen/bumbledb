# schema-001: sealed `Relation.extension: Option` — closed vs ordinary as a flag

- **Severity:** high
- **Tree:** schema
- **Status:** FIXED(cc808db7)
- **Source:** audit/storage-schema.md F1
- **Depends on:** none (foundation; image-001, store-002 land after)
- **Conflicts with:** schema-006, image-001 (same sealed `Relation`; land this first). Not schema-009 (`SealedField` is the theory descriptor view).

## The bug

`crates/bumbledb/src/schema.rs:553-581` — sealed `Relation` carries `extension: Option<Box<[SealedRow]>>`. Ordinary and closed share one product. Kind is restated as a method:

```rust
// schema/relation.rs:26-28
pub fn is_closed(&self) -> bool {
    self.extension.is_some()
}
```

Every consumer reconstitutes the kind: `WriteTx::refuse_closed` (`api/db.rs:493-499`) on every insert/delete/alloc; snapshot/write point reads `if rel.is_closed()` / `if let Some(extension)`; image build `debug_assert!(!is_closed())`; key codec `debug_assert_ordinary`; cache `closed_slots: Box<[Option<u32>]>` (image-001). Two encodings of "this id is closed" before the cache even starts.

## Why it's wrong

Insight 4 — Minsky's nullable field: the Option admits ordinary-with-rows and closed-with-None if a future writer slips; downstream guards the combinations. Insight 6 — validate already knew closedness (`validate_extension`) and stored it as absence of rows. The sealed witness should carry the kind, not a hole.

## The fix

Implementable under `audit/CONTRACT.md` C1–C8 (C1 does **not** freeze this tree). Proposed C9 would *pin* this shape later; this issue is not blocked on pinning C9 and must not pretend C9 is law.

Descriptor `RelationDescriptor.extension: Option` stays the hostile spelling (schema-010). Witness:

```rust
enum RelationBody {
    Ordinary { fields, layout, keys, outgoing, capacity_sources, capacity_targets,
               fresh: Option<KeyId> },
    Closed   { fields, layout, keys, outgoing, capacity_sources, capacity_targets,
               extension: Box<[SealedRow]> },
}
```

Shared fields may sit outside the sum; the kind-carrying payloads (`fresh` vs `extension`) must not.

- `is_closed()` / `extension()`-as-Option die; closed accessors exist only on the Closed arm.
- Dyn write path: `ClosedRelationWrite` remains the refusal — ids are data (`insert_dyn` / `delete_dyn` / `bulk_load_dyn` / `alloc_at`). Existing tests are dyn (`api/db/tests.rs:684-708`, `tests/dyn_surface.rs`).
- Typed `Fact::RELATION` is a `const` id, not a sealed kind. A `Writable` marker (macro-emitted) that makes typed `insert` of a closed fact a compile error is a later win, **not** this issue's acceptance. Do not drop `refuse_closed` on the typed path until that marker exists.
- Image cache closed table sized to closed relations only (image-001).

## Acceptance criteria

- [ ] Gone: `rg -n 'fn is_closed' crates/bumbledb/src/schema`; `rg -n 'extension: Option' crates/bumbledb/src/schema.rs` → no sealed-Relation match (descriptor may still have Option — schema-010).
- [ ] `refuse_closed` remains on dyn writes (and typed writes until a marker exists). `ClosedRelationWrite` still raised on dyn writes.
- [ ] Unchanged tests: `cargo test -p bumbledb --lib` and closed-relation API tests pass with zero assertion edits.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Fingerprint, codec tag 0/1, and `SchemaDescriptor` shape stay the hostile boundary (schema-010). Semantics identical: virtual storage, write-refused, synthetic id at `FieldId(0)`.
- Do not invent a third relation type at the descriptor (schema-010).
- Do not put `extension` on the Ordinary arm or a write method on Closed (writable closed relations are a contract violation).
- Do not mix ordinary LMDB storage with closed virtual storage.
