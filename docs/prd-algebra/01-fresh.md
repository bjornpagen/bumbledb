# PRD 01 — `fresh`: the generation attribute, renamed and closed

**Depends on:** nothing.
**Modules:** `crates/bumbledb-macros/src/lib.rs` (grammar, emission),
`crates/bumbledb/src/schema.rs` (`Generation`), `crates/bumbledb/src/api/db.rs`
(traits), `crates/bumbledb/src/error.rs`, `crates/bumbledb/src/storage/delta/alloc.rs`,
all docs mentioning serial.
**Authority:** `10-data-model.md`, `30-dependencies.md`, `00-product.md`.
**Representation move:** naming is representation — the last SQL word in the
schema surface dies, and the replacement names what the mechanism *is* in the
theory the engine already speaks.

## Context (decided shape)

`serial` is Postgres's word, and the deleted-vocabulary law killed every other
SQL constraint word while this one survived. The mechanism it names is
chase-theoretic: minting an id is generating a **fresh existential witness** —
exactly what the chase does when a TGD demands a value that does not exist.
`fresh` is the dependency-theory name for the thing the engine does.

Three rulings close the serial question permanently, each already true in code
and none recorded:

1. **u64-only.** Already enforced (`SchemaError::SerialOnNonU64`,
   `schema/validate.rs`). A monotone counter over i64 has no sighting; the
   census law forbids the surface area.
2. **Writable-by-default is load-bearing, not a leak.** Update is
   delete+insert, so re-inserting a fact writes its existing id back; ETL and
   `bulk_load` must preserve ids other facts reference. The SQL-standard
   `GENERATED ALWAYS` shape is incompatible with the engine's own update idiom.
   Explicit writes advance the high-water (`saturating_add`); exhaustion at
   u64::MAX is ~585,000 years at 10⁶ allocs/sec — no guard beyond the existing
   `SerialExhausted`.
3. **Generation attribute, not a type** (standing ruling, now with its reason):
   a type is an encoding and the value's encoding *is* u64; a distinct engine
   type would smuggle nominal typing past the structural-typing law while
   duplicating what host newtypes already provide under rustc.

## Technical direction

Rename everywhere, no aliases, one cut: the macro modifier `serial` → `fresh`;
`Generation::Serial` → `Generation::Fresh`; `SerialField` → `FreshField`;
`SchemaError::SerialOnNonU64` → `FreshOnNonU64`; `Error::SerialExhausted` →
`FreshExhausted`; traits `Serial`/`SerialKeyed` → `Fresh`/`FreshKeyed`;
`from_serial`/`serial()` accessors → `from_fresh`/`fresh()`; `serial_field` →
`fresh_field`; `serial_next`/high-water internals follow. The auto-key
derivation ("`fresh` auto-materializes `R(field) -> R`") is unchanged in
substance and re-stated in the new name. Discharge the idioms chapter
(README of this set) into `10-data-model.md` in the same change.

## Passing criteria

- `[shape]` `grep -ri serial crates/ docs/architecture/` returns nothing
  (bench-crate SQLite fixtures exempt where SQL DDL requires the word — none
  should).
- `[shape]` The three rulings appear as a Decision block in `10-data-model.md`
  with the chase-witness rationale and reversal triggers.
- `[test]` Existing alloc/high-water/exhaustion tests pass under the new names;
  the macro rejects `serial` as an unknown modifier with the standard error.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`10-data-model.md`: the fresh section + Decision block + idioms chapter.
`30-dependencies.md`, `70-api.md`, `50-storage.md`, README example: the rename.
