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

## Result (2026-07-07, runs bench-out/2026-07-07T01-08-48Z + 01-14-06Z confirm)

Landed: runtime leaf classification (single-subatom leaves dispatch on
cursor kind — no plan-time enum needed, strictly more conservative),
the pinned-row elision (gather + precomputed residual sources + one-row
emit through pointer-cached sink shapes; no batch scaffolding), and the
scan pushdown — `Sink::begin_scan/scan_run/end_scan` with positions
flowing from `SuffixRun`s into the PRD 03 kernels for elided
constant-group aggregates, and into direct seen-set inserts for
projections, with executor-side residual filtering (batch-constant
residuals decide the whole leaf; per-position specs precomputed at
construction — a mid-PRD `collect()` per node entry would have broken
the zero-alloc warm contract and was caught and fixed before commit).

Two optimization lessons were measured INTO this PRD and are load-
bearing code comments now: hoisted operand/column tables cost
+48 ns/row at fanout-sized runs (spread regressed 11.5 → 15.6 ms) and
save ~10 µs at scan-sized runs (range 52.6 → 41.3 µs) — resolution is
now run-length-adaptive (`SCAN_HOIST_THRESHOLD` = 32), with both
directions pinned by the ledger.

Gates (vs baseline; confirm-run values):
- balance p95 **26.1 µs** (gate ≤ 450; baseline 1,110.2) ✓ — p50 1.7 µs
  (−86%). The elided scan-fold leaf is ~free: 8 scans, no iter phase.
- fk_walk p50 **7.5 µs** (gate ≤ 10; bimodal band 4.9–7.8 across runs) ✓.
- range p50 **40.8 µs** (gate ≤ 30) ✗ near-miss: the scan now filters
  2,000 survivors through the residuals and inserts them — the
  remaining cost is seen-set inserts (PRD 06) + finalize (PRD 08).
- stats p50 **1,839–1,905 µs** (gate ≤ 700) ✗: the dedup wall, premise-
  corrected in PRD 02's Result — 100k semantically-required seen-set
  inserts; PRD 06 owns it. Still −55% vs baseline.
- spread p50 **10.4–11.7 ms** across three post-adaptive samples (gate
  ≤ 8,000) ✗: the gate's pinned-leaf premise was wrong — spread's leaf
  is fanout-~1.4 *chunked nodes* (transfer pairs), so it takes the scan
  path per node entry, not the pinned elision. The scan removed the
  iter/residual leaf machinery; what remains is n0's per-survivor
  middle-node bookkeeping (~2.3 ms — PRD 09's cross-node batching),
  n0's probes (~2.0 ms — PRD 07), and the seen-set inserts (PRD 06).
- triangle 16.7 ms (−4.7%), chain 151 (−28%), skew 38 (−36%); EXPLAIN
  emits digests byte-identical on every run; ALL-WIN held; verify green.
  One hot sample (triangle 19.2 ms / p95 38 ms) was ruled noise by the
  same-binary confirm run per the PRD 03 protocol.
