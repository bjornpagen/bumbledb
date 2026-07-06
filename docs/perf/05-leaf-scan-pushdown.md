# PRD 05 — Leaf scan pushdown: fuse the last node into the sink

## Purpose

After 01–04 the leaf still round-trips every position through
`entry_keys`/`children_out` and a `LeafBatch`: gather → copy → survivors →
sink reads the copy. For the two leaf shapes that dominate the ledger and
olap workloads, the copy is pure waste:

- **Suffix-scan leaf** (stats, balance, range shape): the last node's cover
  is an unforced suffix node — a run of positions; the sink needs one or
  two gathered columns.
- **Pinned-row leaf** (spread, fk_walk shape): the last node's cover is
  `Cursor::Row` — batch size one by construction; the whole node-entry
  machinery (iter call, residual pass, batch bookkeeping: baseline
  16 + 10 + 24 ≈ 50 ns/row on spread, ~6.9 ms of its 13.3 ms join) wraps
  a single row that n0's descend loop already had in hand.

Push the leaf into the sink: the executor hands positions (or the single
pinned row) + resolved column slices + residual specs to a fused kernel.

## Technical direction

- **Plan-time classification.** `plan/fj.rs` (`ValidatedPlan`): compute and
  store a `LeafShape` for the last node:
  `Scan { occ, key_columns }` — single subatom, cover will iterate unforced
  suffix positions; `Pinned { occ }` — single subatom whose occurrence is
  fully bound above (its cursor at the leaf is always `Row` or a forced
  singleton); `General` — anything else (multi-subatom leaves keep the
  PRD 01 batch path). The classification must be conservative: when in
  doubt, `General` — correctness never depends on the fast shapes firing,
  only the phase-table gates do.
- **The pushdown seam.** `exec/run.rs`: for `Scan` leaves, do not draw
  key batches at all. Resolve the leaf's key column slices once per node
  entry (as in PRD 04), then drive one of two fused loops:
  - **Aggregate sinks** (`AggregateSink::fold_scan`): constant-group
    (PRD 02 regime) + positions run → gather-fold kernels straight over
    `(column, positions)` (PRD 03's `fold_*_gather`), residuals evaluated
    as position-filter kernels first (`filter_eq/range_u64` from kernel.rs
    where the residual compares a leaf column against a constant/outer
    slot; general residuals fall back to a scalar filter loop). Root-view
    scans (range shape) fold over `0..len` directly — no positions array
    at all, the fully-contiguous case.
  - **Projection sinks** (`ProjectionSink::insert_scan`): per position,
    gather projected words (outer slots constant, hoisted) and
    `seen.insert`. With `stop_on_skip` semantics as in PRD 01.
  For `Pinned` leaves: n0's (more generally, the parent node's) descend
  loop calls a `sink.emit_pinned(...)`-shaped fused path: outer bindings +
  the pinned position's gathered words + leaf residual evaluation inline —
  **no leaf node entry happens at all**. The parent's descend loop still
  runs per survivor (that is PRD 09/10's problem); what dies here is the
  ~50 ns of leaf node machinery per row.
- **Residual placement honesty**: leaf residuals comparing two outer slots
  are batch-constant — evaluate once per node entry, short-circuit the
  whole leaf when false. (The validator already places residuals at the
  earliest node where both sides are bound, so this case exists only when
  one side is a leaf var — but assert it anyway.)
- **Phase attribution**: fused leaves are timed as the leaf node's `Iter`
  phase... no. Add nothing: time lands in the parent's `Descend` for
  pinned leaves and in the leaf's `Descend` for scan leaves (start the
  phase at fused-loop entry). Update the 50-validation amendment (00) if
  the wording needs it. What matters: the baseline comparison rows are
  named in the gates below.
- **Tests**: the randomized differential family (run.rs) extended with
  leaf-shape coverage: scan leaves with/without residuals, pinned leaves
  under projection and aggregation, empty runs, and the D2 leaf case from
  PRD 01 re-asserted through the pushdown path. EXPLAIN `emits` unchanged.

## Passing requirements

1. Functional gates green; leaf-shape classification is conservative
   (a test constructs a multi-subatom leaf and asserts `General`).
2. Measured (vs baseline):
   - stats p50 ≤ 700 µs (join dominated by gather-fold at ≤ ~5 ns/row);
     `jp_iter_n1 + jp_descend_n1` combined ≤ 500 µs (baseline 4,288.6).
   - spread p50 ≤ 8,000 µs (baseline 13,415.1): the pinned-leaf machinery
     (`jp_iter_n1` 3,342.3 + `jp_residual_n1` 1,061.5 + `jp_descend_n1`
     2,453.8) collapses into the n0 descend loop; n0 rows may grow
     accordingly — gate the join total ≤ 8.5 ms in the traced sample.
   - balance p95 ≤ 450 µs.
   - range p50 ≤ 30 µs (contiguous root-scan fold).
   - fk_walk p50 ≤ 10 µs (baseline 12.8; pinned leaf).
   - No family regresses >5%.

## Out of scope

Middle-node batching (09/10), sink map internals (06), finalize (08).
