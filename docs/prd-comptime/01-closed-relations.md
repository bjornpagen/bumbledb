# PRD 01 — Closed relations in the theory

**Depends on:** baseline only.
**Modules:** `crates/bumbledb/src/schema.rs` (descriptor + sealed types),
`schema/validate.rs` (roster + `Resolved`), `schema/fingerprint.rs`,
`storage/delta/` + `api/db/` (write refusal), `error.rs`.
**Authority:** `10-data-model.md`, `30-dependencies.md` (the acceptance gate),
this set's README (the staging law, the intrinsic-vs-policy law).
**Representation move:** the theory acquires constants. A schema was
*signature + axioms* where the axioms were universally quantified statements;
a closed relation's rows are **ground axioms** — atomic sentences — so
vocabularies stop being a type (`enum`) and become what they always were
relationally: unary-plus-payload relations with a fixed extension.

## Context (decided shape)

- `SchemaDescriptor` gains extension data:
  `RelationDescriptor { name, fields, extension: Option<Extension> }` where
  `Extension = Box<[Row]>`, `Row = { handle: Box<str>, values: Box<[Value]> }`
  (values in field-declaration order, one per declared column — the handle is
  NOT a column). A relation with `Some(extension)` is **closed**; `None` is
  ordinary. No new relation kind enum — the option *is* the kind.
- **Identity = the handle**: row id = declaration index (u64), exactly the
  declaration-order rule relations/fields/statements already obey. The id
  column is implicit: the sealed `Relation` for a closed relation carries a
  synthetic first field (`id`, U64) so guards, statements, and queries address
  it uniformly; the macro (PRD 02) never lets the user declare it.
- **The auto-key**: closedness materializes `R(id) -> R` exactly as `fresh`
  does (extend `materialized_statements`, `schema.rs` — fresh auto-FDs first,
  then closed auto-FDs, then declared statements; the ORDER is a fingerprint
  input, so it is pinned here and never revisited).
- **Intrinsic columns are value types only**: U64, I64, Bool, FixedBytes,
  Interval. `str` refused (README refusal), `fresh` refused (identity is the
  handle), closed-to-closed reference columns are plain u64 + a declared
  containment like any reference.
- **The intrinsic-vs-policy law, recorded in `10-data-model.md`**: intrinsic
  properties of a vocabulary go on the closed relation (changing one is a new
  theory — fingerprint); policy over a vocabulary lives in ordinary relations
  and changes by witnessed write.
- **Writes refused**: any delta operation naming a closed relation is the
  typed error `ClosedRelationWrite { relation }` — checked at
  `WriteTx::insert/delete` entry (typed path via the `Fact::RELATION` const;
  dyn path via the descriptor), before any encoding runs. `bulk_load` and
  `alloc`/`fresh` likewise.

## Technical direction

1. `schema.rs`: the `Extension`/`Row` types; `materialized_statements` gains
   the closed auto-FD arm; the sealed `Relation` gains `extension:
   Option<Box<[SealedRow]>>` with values already canonically encoded
   (`encoding::encode` per field) at validate — rows are encoded ONCE at open,
   never re-encoded (the staging law applied to the feature itself).
2. `validate.rs` roster additions, each a distinct `SchemaError` variant:
   duplicate handle; extension row arity ≠ declared columns; extension value
   type mismatch (reuse the selection-literal typing machinery,
   `validate.rs`'s `value` checks); >256 rows; empty extension (a closed
   relation with no rows is a vocabulary of nothing — write no relation);
   `str` column on a closed relation; `fresh` on a closed relation; interval
   extension values violating `start < end` (the constructor law holds for
   axioms too — a malformed ground axiom is a schema error, not corruption).
3. `fingerprint.rs`: extension rows hash in declaration order — handle bytes,
   then each value's canonical encoding, length-prefixed like everything else.
   Add the sensitivity tests (row order, value change, handle rename each move
   the hash) alongside the existing fingerprint test family.
4. Write refusal: one check in `WriteTx::insert`/`delete`/`fresh` dyn+typed
   entries and in `bulk_load`'s per-fact loop; `error.rs` gains
   `ClosedRelationWrite` in the write group with the relation id payload.
5. `verify_store` (the sweeper): closed relations are exempt from F/M/U/R
   coherence walks (they have no rows in the store — PRD 03); add the
   assertion that NO `F`/`M`/`U`/`R` entry exists for a closed `RelationId`
   (their presence is corruption).

## Passing criteria

- `[test]` A descriptor with a closed relation validates; each roster
  addition above produces its typed error (one test per variant, fixture
  descriptors built by hand — the macro does not exist yet for this grammar).
- `[test]` Fingerprint sensitivity: row order / value / handle each move it;
  `Resolved`-equivalent data does not (existing invariance test extended).
- `[test]` `insert`/`delete`/`bulk_load` against a closed relation return
  `ClosedRelationWrite`; nothing reaches the delta (delta remains empty).
- `[shape]` The closed auto-FD appears in `materialized_statements` output
  between fresh auto-FDs and declared statements (order test); the sealed
  extension carries pre-encoded values (grep: no `encoding::encode` call on
  extension values outside validate).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`10-data-model.md`: the ground-axioms section, the intrinsic-vs-policy law,
the roster of intrinsic column types. `30-dependencies.md`: closed auto-key
materialization order. `00-product.md`: *enum* moves toward the deleted
vocabulary (completed by PRD 05).
