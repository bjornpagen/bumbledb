# schema-001: sealed `Relation.extension: Option` — closed vs ordinary as a flag

- **Severity:** high
- **Tree:** schema
- **Status:** OPEN
- **Source:** audit/storage-schema.md F1
- **Depends on:** none (foundation; image-001, store-002 land after)
- **Conflicts with:** schema-006, schema-009, image-001 (same sealed `Relation`; land F1 first)

## The bug

`crates/bumbledb/src/schema.rs` — sealed `Relation` carries `extension: Option<Box<[SealedRow]>>`. Ordinary and closed share one product. Kind is restated as a method:

```rust
// schema/relation.rs:26-28
pub fn is_closed(&self) -> bool {
    self.extension.is_some()
}
```

Every consumer reconstitutes the kind: `WriteTx::refuse_closed` (`api/db.rs:493-499`) on every insert/delete/alloc; snapshot/write point reads `if let Some(extension)`; image build `debug_assert!(!is_closed())`; key codec `debug_assert_ordinary`; cache `closed_slots: Box<[Option<u32>]>` (image-001). Two encodings of "this id is closed" before the cache even starts.

## Why it's wrong

Insight 4 — Minsky's nullable field: the Option admits ordinary-with-rows and closed-with-None if a future writer slips; downstream guards the combinations. Insight 6 — validate already knew closedness (`validate_extension`) and stored it as absence of rows. The sealed witness should carry the kind, not a hole.

## The fix

`audit/CONTRACT.md` C1 does **not** freeze this tree. **CONTRACT gap — propose C9** (sealed schema sums: a relation is `Ordinary | Closed`, a key is `KeyForm`, capacity tails live in the weight/bound arms). Descriptor `RelationDescriptor.extension: Option` stays the hostile spelling (schema-010). Witness:

```rust
enum RelationBody {
    Ordinary { fields, layout, keys, outgoing, capacity_sources, capacity_targets,
               fresh: Option<KeyId> },
    Closed   { fields, layout, keys, outgoing, capacity_sources, capacity_targets,
               extension: Box<[SealedRow]> },
}
```

- `is_closed()` / `extension()`-as-Option die; closed accessors exist only on the Closed arm.
- Typed write path: closed relations do not offer `insert`/`delete` (marker or no monomorphization). `ClosedRelationWrite` remains the dyn-surface refusal (ids are data).
- Image cache closed table sized to closed relations only (image-001).

## Acceptance criteria

- [ ] Gone: `rg -n 'fn is_closed' crates/bumbledb/src/schema`; `rg -n 'extension: Option' crates/bumbledb/src/schema.rs` → no sealed-Relation match (descriptor may still have Option — schema-010).
- [ ] Gone: `rg -n 'refuse_closed' crates/bumbledb/src/api/db.rs` either deleted or dyn-only (typed `insert` of a closed `Fact` does not compile, or is uninhabited).
- [ ] Unchanged tests: `cargo test -p bumbledb --lib` and closed-relation API tests pass with zero assertion edits; `ClosedRelationWrite` still raised on dyn writes.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Fingerprint, codec tag 0/1, and `SchemaDescriptor` shape stay the hostile boundary (schema-010). Semantics identical: virtual storage, write-refused, synthetic id at `FieldId(0)`.
- Do not invent a third relation type at the descriptor (schema-010).
