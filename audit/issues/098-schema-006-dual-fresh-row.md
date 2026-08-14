# schema-006: dual coordinate — `KeyStatement.fresh_row` and `Relation.fresh_row_field`

- **Severity:** medium
- **Tree:** schema
- **Status:** FIXED(cc808db7)
- **Source:** audit/storage-schema.md F14
- **Depends on:** schema-002
- **Conflicts with:** schema-001 (ordinary-arm `fresh` field)

## The bug

"This relation's first fresh field is the `F` row id" is stored twice: `Relation.fresh_row_field: Option<FieldId>` (`schema.rs:576-581`) and `KeyStatement.fresh_row: bool` (`schema.rs:417`). Sealing re-derives the bool from the Option (`validate.rs:155-157`). Image cache (`get_or_build.rs:140`) and delta alloc (`delta/accessors.rs:75`) read the relation; judgment and point reads read the key.

## Why it's wrong

Insight 11 — dual coordinates. One fact, two fields, two consumer forests. Dijkstra's off-by-one lived in the numbering; here the special case lives in "the bool on the key, if the Option on the relation agrees."

## The fix

Per schema-002's `KeyForm::FreshRow`. The ordinary `Relation` arm holds at most `Option<KeyId>` to that key (or the key form alone is enough). No second bool, no second `FieldId` slot that must agree.

## Acceptance criteria

- [ ] Gone: `rg -n 'fresh_row_field' crates/bumbledb/src/schema.rs crates/bumbledb/src/schema/relation.rs` → no matches (or only the `KeyForm::FreshRow.field` accessor).
- [ ] Gone: `rg -n 'fresh_row: bool' crates/bumbledb/src/schema.rs`.
- [ ] One site names the mint field; image cache, delta alloc, judgment, and point reads all read that site (or match `KeyForm::FreshRow`).
- [ ] Unchanged tests: R16 fresh-row commit/alloc/point-read tests green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Lands with or immediately after schema-002. R16 semantics identical.
