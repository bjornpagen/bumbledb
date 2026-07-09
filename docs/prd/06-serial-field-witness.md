# PRD 06 — `alloc_dyn`: parse, don't validate (ETL-surface panic)

**Depends on:** nothing.
**Modules:** `crates/bumbledb/src/api/db/alloc.rs`,
`crates/bumbledb/src/storage/delta/alloc.rs`, `crates/bumbledb/src/schema.rs`,
`crates/bumbledb/src/error.rs`.
**Authority:** the import-path principle at `error.rs:234-236` ("ETL input is
data, not code — no panics on the import path"); `70-api.md` § ETL;
`00-product.md` (parse, don't validate).

## Current behavior

`WriteTx::alloc_dyn` is documented as untyped serial minting for ETL tooling
(`api/db/alloc.rs:28`) but delegates to `WriteDelta::alloc`, which **asserts**
the field is `Generation::Serial` and panics otherwise
(`storage/delta/alloc.rs:24-28`). A tool reading `(relation, field)` from an
export manifest and hitting a non-serial field crashes the process.

## Context (decided — representation-first)

A per-call typed error inside `alloc_dyn` was considered and **rejected as the
primary shape**: it validates on every call and throws the proof away — every
mint re-checks what the first mint learned. The ETL access pattern is
resolve-once, mint-per-row; give it the proof-carrying shape.

## Technical direction

1. **The witness:** `Schema::serial_field(relation: RelationId, field: FieldId)
   -> Result<SerialField, FactShapeError>` — validates ids and generation once,
   returning a small `Copy` witness (`SerialField { relation, field }`, fields
   private, constructible only through the resolver — the type *is* the proof).
   Errors: the existing unknown-id variants plus new
   `FactShapeError::NotASerialField { relation, field }`.
2. **The mint:** `WriteTx::alloc_at(&mut self, field: SerialField) -> Result<u64>`
   — no generation re-check anywhere on the path (the witness's construction is
   the check); the only fallible part is the existing `SerialExhausted`
   behavior. `alloc_dyn(relation, field)` is deleted, not deprecated (no shims);
   the ETL loop becomes: resolve once per relation, `alloc_at` per row.
3. `WriteDelta::alloc`'s assert becomes a `debug_assert!` with a comment: both
   callers are proof-carrying (the macro-generated `Serial` newtypes on the
   typed path; `SerialField` on the dynamic path) — the assert documents the
   invariant, no boundary re-checks it.
4. Sweep the rest of the `_dyn`/ETL surface for the same shape: any
   `assert!`/`unwrap`/`panic!` reachable from manifest-derived values.
   `insert_dyn`/`delete_dyn`/`bulk_load` were audited clean — re-verify, and
   note in the commit body whether any of them would also read better
   witness-shaped (note only; changing them is not in scope).

## Passing criteria

- `[shape]` `SerialField` has private fields and one construction site; no
  generation check exists on the mint path; `alloc_dyn` is gone (grep).
- `[test]` `serial_field` on a non-serial field returns `NotASerialField`; on
  unknown ids the unknown-id variants; on a serial field the witness mints
  across multiple `alloc_at` calls and interleaves correctly with typed
  `alloc` (same sequence).
- `[shape]` The commit body carries the dynamic-surface panic audit note.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`70-api.md` ETL section: the resolve-once/mint-per-row idiom replaces
`alloc_dyn`'s entry.
