# 03 — `alloc_dyn` panics on the data-driven surface

**Kind:** ETL-surface panic — violates the import-path principle ("ETL input is
data, not code — no panics on the import path", the principle `error.rs:234-236`
documents for `FactShape`).

## Current behavior

`WriteTx::alloc_dyn` is documented as untyped serial minting for ETL tooling
(`crates/bumbledb/src/api/db/alloc.rs:28`), but it delegates to
`WriteDelta::alloc`, which **asserts** the field is `Generation::Serial` and panics
otherwise (`crates/bumbledb/src/storage/delta/alloc.rs:24-28`). Every neighboring
dynamic-surface entry point returns typed errors for data-shaped mistakes:
`get_dyn` → `FactShapeError::NotAKeyStatement` (`api/db/get.rs:134-140`), `scan` →
`UnknownRelation`. A tool that reads `(relation, field)` out of an export manifest
and hits a non-serial field crashes the process instead of failing the row.

## The work

- `alloc_dyn` returns a typed error (new `FactShapeError` variant, e.g.
  `NotASerialField { relation, field }`) instead of reaching the assert. Unknown
  relation/field ids on the same path get the existing unknown-id variants.
- The internal typed `alloc` keeps its assert (or `debug_assert`) — the macro-typed
  surface makes a non-serial call unrepresentable there; the assert documents that.
  The fix is a check at the dynamic boundary, not a change to the typed path.
- Sweep the rest of the `_dyn`/ETL surface for the same shape: any `assert!`/
  `unwrap`/`panic!` reachable from values that arrive as data rather than from the
  schema macro. (`insert_dyn`/`delete_dyn`/`bulk_load` were checked and route
  through `FactShape` correctly; re-verify after changes.)

## Acceptance

- `alloc_dyn` on a non-serial field returns the typed error; a test asserts it.
- `grep` audit note in the PR: no `assert!` on data-derived values remains on the
  dynamic surface.

## Doc amendments (rule 5)

`70-api.md` ETL section: `alloc_dyn`'s error contract gains the variant.
