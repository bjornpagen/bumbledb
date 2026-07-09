# PRD 03 — `alloc_dyn` typed error (ETL-surface panic)

**Depends on:** nothing.
**Modules:** `crates/bumbledb/src/api/db/alloc.rs`,
`crates/bumbledb/src/storage/delta/alloc.rs`, `crates/bumbledb/src/error.rs`.
**Authority:** the import-path principle already documented at `error.rs:234-236`
("ETL input is data, not code — no panics on the import path");
`docs/architecture/70-api.md` § ETL.

## Current behavior

`WriteTx::alloc_dyn` is documented as untyped serial minting for ETL tooling
(`api/db/alloc.rs:28`) but delegates to `WriteDelta::alloc`, which **asserts**
the field is `Generation::Serial` and panics otherwise
(`storage/delta/alloc.rs:24-28`). Every sibling dynamic entry point returns typed
errors for data-shaped mistakes: `get_dyn` → `FactShapeError::NotAKeyStatement`,
`scan` → `UnknownRelation`. A tool reading `(relation, field)` from an export
manifest and hitting a non-serial field crashes the process instead of failing
the row.

## Technical direction

1. `alloc_dyn` validates at the dynamic boundary and returns typed errors: new
   `FactShapeError::NotASerialField { relation, field }` for a non-serial field;
   the existing unknown-id variants for unknown relation/field. Follow `get_dyn`'s
   validation style exactly (same module, same error-construction idiom).
2. The internal typed path keeps its assert as a `debug_assert!` with a comment:
   the macro-generated `Serial` newtypes make a non-serial typed call
   unrepresentable — the assert documents that invariant; the check lives at the
   dynamic boundary only. Do not double-check on the typed path.
3. Sweep the rest of the `_dyn`/ETL surface for the same shape: any
   `assert!`/`unwrap`/`panic!` reachable from values that arrive as *data*
   (manifest ids, dynamic values) rather than from the schema macro.
   `insert_dyn`/`delete_dyn`/`bulk_load` were audited clean — re-verify and note
   the audit result in the commit body.

## Passing criteria

- `[shape]` No `assert!`/`unwrap`/`panic!` on data-derived values remains on the
  dynamic surface (`api/db/*_dyn` paths, `bulk_load`, `scan`); the commit body
  carries the grep audit note.
- `[test]` `alloc_dyn` on a non-serial field returns
  `FactShapeError::NotASerialField` with the right ids; on an unknown
  relation/field returns the unknown-id variants; on a serial field still mints
  correctly (existing behavior test survives).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`70-api.md` ETL section: `alloc_dyn`'s error contract gains the variant.
