# PRD 09 — Point Reads and Relation Scan

Authority: `docs/architecture/40-storage.md` (namespace readers), `30-execution.md`
(guard-probe access path — storage half), `60-api.md` (export scan).

## Purpose

The storage read primitives: membership probe, unique-guard probe, fact fetch, and the
sequential relation scan that feeds images and export.

## Technical direction

- `storage::read`. All functions take `&ReadTxn` (PRD 04) and are allocation-free
  (borrowed returns).
- `fact_row(rel, fact_bytes) -> Option<u64>` (`M` probe);
  `unique_row(rel, constraint, key_bytes) -> Option<u64>` (`U` probe);
  `fetch(rel, row_id) -> Result<&[u8]>` (`F` get; missing row for a row_id obtained
  from M/U in the same snapshot = `Corruption`).
- `scan(rel) -> impl Iterator<Item = Result<(u64, &[u8])>>` — one `F` prefix cursor in
  row_id order; every yielded fact's length is checked against the schema fact width —
  mismatch yields `Err(Corruption)` and iteration is expected to stop at the caller
  (hard error, never a skip).
- `row_count(rel) -> u64` (`S` get) — the planner's statistic.

## Non-goals

Decode to values (callers use PRD 01/05). Images (PRD 10). Planner integration
(PRD 23).

## Passing criteria

- Unit tests over a committed fixture (PRDs 06–08): membership hit/miss; unique probe
  hit/miss; fetch round-trips inserted bytes; scan yields exactly the live facts in
  row_id order with holes (delete one, scan skips it — holes are absent keys, not
  tombstones); a manually corrupted `F` value (wrong length, written via a raw heed
  handle in the test) yields `Corruption`, not a skip; row_count equals scan count
  after mixed insert/delete commits.
- Global commands green.
