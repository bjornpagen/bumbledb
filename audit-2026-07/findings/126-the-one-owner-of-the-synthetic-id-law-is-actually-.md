## "THE one owner of the synthetic-id law" has five owners; Resolver re-derives it in three near-identical scans

category: unification | severity: low | verdict: CONFIRMED | finder: theory

### Summary

`RelationDescriptor::sealed_fields` (crates/bumbledb-theory/src/schema.rs:359-368) documents itself as "THE one owner of the synthetic-id law — … all read the sealed shape through this accessor, never through re-derived offset arithmetic." The law — a closed relation's synthetic (`id`, U64) handle field at sealed ordinal 0, declared fields shifted by one — is in fact independently re-derived at five other sites: three near-identical name→slot scans inside the same crate's `Resolver`, plus the engine's validation seal and the descriptor codec's decode. The three Resolver twins are one judgment (name → sealed slot: FieldId + newtype + declaredness) split into three functions, and the split forces a redundant rescan: `literal()` re-walks the field Vec by name via `field_newtype` immediately after `side()` already resolved the same name to a `FieldId`.

### Evidence (all verified in the working tree)

The doc claim:
- crates/bumbledb-theory/src/schema.rs:363-367 — "THE one owner of the synthetic-id law — the manifest renderer, the materialized-statement ordinals …, and the node bridge's row marshaling all read the sealed shape through this accessor, never through re-derived offset arithmetic."

The three named consumers genuinely do read through the accessor (the positive claims are true): crates/bumbledb/src/schema/manifest.rs:137, crates/bumbledb-theory/src/schema.rs:417 (`materialized_statements`), ts/crate/src/marshal.rs:254 (which cites the accessor by name at marshal.rs:237). Only the exclusivity claim is false:

1. crates/bumbledb-theory/src/schema/spec.rs:514-533 — `Resolver::field`: `if closed && name == "id" { return Some(FieldId(0)); }` … `let sealed = index + usize::from(closed);` — literally re-derived offset arithmetic, in the same crate as the doc.
2. spec.rs:537-547 — `Resolver::field_newtype`: `if relation.extension.is_some() && name == "id" { return relation.newtype.as_deref(); }` then a second linear scan of `relation.fields`.
3. spec.rs:552-556 — `Resolver::declares`: `(relation.extension.is_some() && name == "id") || relation.fields.iter().any(…)` — the doc there even calls itself "the silent twin of `Resolver::field`", acknowledging the duplication.
4. crates/bumbledb/src/schema/validate.rs:1395-1403 — the seal hand-prepends `FieldDescriptor { name: "id".into(), value_type: ValueType::U64, generation: Generation::None }` when `extension.is_some()`.
5. crates/bumbledb/src/schema/descriptor_codec.rs:120-132 — decode hand-splits `let [id_field, declared_fields @ ..] = sealed_fields.as_slice()` and re-checks the id field's name/type/generation shape.

Redundant rescan: spec.rs:667 (`side()`'s selection loop) resolves the field name to a `FieldId`; spec.rs:678 then calls `literal()`, which at spec.rs:625 calls `self.field_newtype(rel_idx, field)` — a fresh by-name linear scan of the same fields Vec for the same name.

The spec this bears on: docs/architecture/10-data-model.md:365-366 states the law once ("the sealed relation opens with a synthetic first field (`id`, U64)"), and docs/design/representation-first.md is the one-owner doctrine this doc comment is invoking. The doctrine is right; the comment's exclusivity claim does not match the code.

### Failure scenario

No runtime failure today — all five sites currently agree. The risk is drift: any change to the sealed shape (e.g. a second synthetic column, or a change to the synthetic id's type) must be replayed by hand at five sites, which is exactly the failure mode the (false) one-owner comment claims is impossible. The three Resolver scans are also authoring-time-only cost, so the redundant rescan is a cleanliness issue, not a hot-path allocation issue.

### Suggested fix

- Spec side: give `Resolver` (or `RelationSpec`) one sealed-slot lookup — name → `Option<SealedSlot { field_id: FieldId, newtype: Option<&str>, declared: bool }>` — that `field`, `field_newtype`, and `declares` become trivial views of, and thread the resolved slot from `side()` into `literal()` so the by-name rescan disappears. (The Resolver cannot literally call `sealed_fields`: it operates on `RelationSpec`, which carries newtypes that `RelationDescriptor` deliberately drops — so this must be sealed_fields' structural peer on the spec type, not a call into it.)
- Doc side: true up the schema.rs:363 comment — name the two unavoidable owned-materialization sites (validation's seal at validate.rs:1395, the codec's decode at descriptor_codec.rs:124) as the law's producer/decoder rather than claiming the accessor is the sole owner, or make those two sites construct through a shared helper so the claim becomes true.
