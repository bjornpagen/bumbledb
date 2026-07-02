# PRD 06 — The Write Transaction Delta Core

Authority: `docs/architecture/40-storage.md` (the transaction is a delta),
`10-data-model.md` (idempotence, serial semantics, alloc pattern).

## Purpose

The in-memory write transaction: delta accumulation, changed-state reporting, and
serial allocation. No LMDB data writes happen in this PRD.

## Technical direction

- `storage::delta`. `WriteDelta`: per relation, a map from `fact_bytes` (arena-interned
  `Box<[u8]>` or arena slices) → `Disposition::{Insert, Delete}` — **last disposition
  wins**; plus in-memory counters: per-(relation,field) serial next-values (lazily
  initialized from `Q` on first `alloc`/explicit-insert), per-relation row-count
  deltas, per-relation row_id high-water (lazily initialized from `S`/scan or a stored
  high-water — store next-row_id in `_meta` per relation? No: keep a `Q`-style
  high-water entry per relation in `S` per `40-storage.md`'s counter list; extend the
  `S` codec accordingly and note it in the doc if missing).
- `insert(&mut self, txn: &ReadView, rel, fact_bytes) -> Result<bool>`: encode is the
  caller's job (PRD 07 wires the typed path); membership = delta disposition if
  present, else `M` probe (read-only get). Returns whether the final state changes.
  `delete(...) -> Result<bool>` symmetric.
- `alloc(&mut self, rel, field) -> Result<u64>`: Serial-generation fields only
  (schema-checked); reads `Q` once per (rel, field) per transaction, then increments in
  memory. Explicit inserts advance the in-memory mark past any serial field value they
  carry (scan the fact's serial fields on insert — the layout knows offsets).
  `SerialExhausted` on u64 wrap.
- The delta is arena-backed: one bump arena per transaction (a minimal internal bump
  arena over `Vec<u8>` chunks is written here — `exec` reuses it later; keep it in a
  `arena` module; no external crate).
- Abort = drop. Nothing here holds an LMDB write transaction yet; the delta borrows a
  read view for probes (the LMDB write txn opens at commit, PRD 07 — this keeps the
  write lock window to the commit step; document this choice inline; single-writer
  serialization across app threads is a mutex in PRD 28).

## Non-goals

Commit application (PRD 07–08). Constraint checking of any kind. Typed fact structs.

## Passing criteria

- Unit tests: insert-then-delete of an absent fact nets to no-op and reports
  (true, true); delete-then-insert of a present fact nets to present; idempotent
  double-insert reports (true, false); disposition-last-wins across long sequences;
  alloc returns strictly increasing values within a txn and initializes from `Q` once;
  explicit value above the mark advances generated successors past it; mixed
  explicit/generated tracks the running maximum; drop leaves LMDB untouched (byte-equal
  `_data` before/after).
- Global commands green.
