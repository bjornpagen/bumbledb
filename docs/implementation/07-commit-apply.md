# PRD 07 — Commit: Delete and Insert Application

Authority: `docs/architecture/40-storage.md` (commit steps 1–2), `10-data-model.md`
(commit-time unique semantics).

## Purpose

Commit phases 1 and 2: apply the delta to LMDB in canonical order — all deletes, then
all inserts — maintaining F/M/U/R and detecting unique violations.

## Technical direction

- `storage::commit`. `apply(delta, env) -> Result<Applied>` opens the LMDB write
  transaction now (see PRD 06's lock-window note) and:
  1. **Deletes** per fact with `Disposition::Delete` that exists in base state:
     `M` get → row_id; del `F`, del `M`; for each unique constraint of the relation,
     re-derive the guard key by slicing constrained fields from `fact_bytes` (PRD 01
     `field_bytes` — never a scan) and del `U`; for each outgoing FK, re-derive and
     del `R`.
  2. **Inserts** per fact with `Disposition::Insert` not in base state: assign row_id
     from the delta's high-water; put `F`, `M`; per unique constraint, `U` put —
     **if the key is already occupied, that is a genuine `UniqueViolation`**
     (deletes all landed first; the insert set is deduplicated by construction):
     return the error carrying (relation id, constraint id, offending `fact_bytes`
     clone); per outgoing FK, put `R` (`R` puts are unconditional; validation of
     targets is PRD 08). Intern novel strings via PRD 05 as encoding demands (typed
     encode path arrives in PRD 28; until then tests pre-intern).
  3. On any error: abort the LMDB txn (drop) — nothing persists.
- Deterministic iteration order (BTreeMap or sorted arena index) so failures are
  reproducible.
- `Applied` carries the open LMDB write txn plus bookkeeping for PRD 08 (deleted
  unique keys per FK-targeted constraint, inserted facts' FK key slices) — collected
  during this pass so PRD 08 never re-derives.

## Non-goals

FK validation, counters, tx-id, actual LMDB commit (PRD 08 — this PRD's tests inspect
the uncommitted txn state and then abort it).

## Passing criteria

- Unit tests (hand-built schema, pre-interned strings): insert lands F/M/U/R entries
  exactly (enumerate the expected key set and compare); delete removes exactly its
  entries; delete+insert of the same unique key in one delta succeeds regardless of
  the user-order the delta was built in; two distinct inserted facts claiming one
  unique key → `UniqueViolation` and an aborted txn leaves base state byte-identical;
  a fact whose guard keys were re-derived matches independently-computed guard keys.
- Global commands green.
