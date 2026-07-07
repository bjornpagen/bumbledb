# PRD 14 — Endgame: close the inherited gates, pin the final table

## Purpose

The docs/perf suite ended with two documented misses that this suite was
built to close: triangle p50 ≤ 8,000 µs with `jp_probe_n1` ≤ 1,500 µs
(missed at ~15,100/~5,500 — now attacked by 02/03/04/10), and point
p50 ≤ 0.8 µs (missed at 1.0 — attacked by 12). This PRD measures where
the campaign landed, applies the one remaining named lever if the
triangle gate still misses, and records the suite's final table as the
new committed denominator. Nothing new is invented here; this is where
the suite proves its gates were real.

## Technical direction

`crates/bumbledb/src/exec/run.rs` (only if the segregation lever fires);
measurement and recording otherwise.

- **Full re-measure.** 5 ledger runs + traces with phase tables
  (triangle, chain, stats, spread, skew, range), proxy-bracketed,
  min-of-5 — the PRD-00 protocol exactly.
- **Triangle post-mortem, itemized.** Compare `jp_probe_n1`,
  `jp_hash_n1`, descend excl_us, and seen-set time against the PRD-00
  baseline AND against each intermediate PRD's recorded result — the
  table must attribute the total win across 02 (instruction diet), 03
  (geometry), 04 (hash-ahead), 10 (tiering).
- **The remaining named lever: cover-stable batch segregation.** The
  perf-PRD 10 result recorded that per-entry cover flips fragment probe
  batches (mean ~37 against a 2×BATCH ceiling), and flush-on-cover-change
  is the current rule. If — and only if — `jp_probe_n1` still exceeds its
  gate after re-measure: segregate pending entries by cover choice before
  the probe pass (stable partition into per-cover runs inside the
  existing pending buffers — index-based, zero-alloc, no reordering
  visible to results since set semantics and D2 origins are
  order-independent; the origins/epoch machinery must be carried through
  the partition untouched). Gate the batch-mean improvement (≥ 48) and
  re-verify the full D2 differential corpus — this touches the exact
  machinery the origin-collision bug lived in; the randomized
  subset-projection harness (≥ 200 cases) is the tripwire and must run
  green before the lever counts.
- **Point path check.** If 12 left point p50 above 0.8 µs, itemize the
  prologue again (perf-PRD-11 methodology) and close the remainder within
  this PRD only if a named, sub-100 ns lever exists (e.g. residual
  snapshot-check call shape); otherwise documented-miss with the split.
- **Final recording.** `docs/silicon/final.md`: the complete ledger table
  (all families, p50/p95, ratio vs the SQLite anchors), the store suite
  (commit_batch, cold, footprint), the phase tables, per-PRD attribution
  of the deltas, and the surviving walls (if any) with owners. Update
  `docs/silicon/baseline.md`'s header to point at `final.md` as the new
  denominator for any future suite.

## Passing requirements

1. **Triangle (the wall, hard gates):** p50 ≤ 8,000 µs; `jp_probe_n1`
   self ≤ 1,500 µs. Documented-miss protocol applies but the bar for the
   document is high: full phase attribution plus a quantified statement
   of what instruction class remains and why it is irreducible under the
   current plan shape.
2. **Point:** p50 ≤ 0.8 µs (expected already green from 12 — re-affirmed
   here on the final binary).
3. **Suite-wide:** ledger ALL-WIN preserved (every family beats its
   SQLite anchor); every family's final p50 ≤ its PRD-00 baseline p50
   (bimodal families: p95 ≤ baseline p95) — the suite may not have made
   anything slower, full stop; ledger geomean of ratios improves ≥ 20%
   vs the PRD-00 baseline geomean.
4. If the segregation lever fired: batch-mean gate (`jp_hash_n1`-derived
   mean ≥ 48), D2 differential corpus green (≥ 200 randomized cases),
   emits digests byte-identical.
5. `final.md` committed with everything in "Final recording"; verify
   green; zero-alloc green; clippy green; `check-asm.sh` green on all
   accumulated symbol gates.

## Out of scope

Anything new. Scenario suite, L-scale runs, the performance claim, and
publication of results are human-owned. Follow-ups discovered here are
recorded as named walls in `final.md`, not chased.

## Result (2026-07-07)

Landed: cover-stable batch segregation in `pump` (pass 1 precomputes
each pending entry's dynamic cover choice + exactness; pass 2 processes
entries grouped by cover in first-appearance order, cancellation
re-checked per entry) and the 2×-batch cascade threshold. The
segregation lever's own measurement is the finding: triangle's batch
means moved only 37 → 39 — the fragmentation was never cover flips but
pump-call granularity (each ~one-batch cascade carries ~one batch of
rows), and doubling the accumulation confirmed the volume bound. The
full D2 gauntlet stayed green through the reorder: the randomized
subset-projection differential at **200 cases**, the two-parent
interleave fixtures, batch-size equality, and the 2,468-case verify
oracle — cancellation is origin-keyed and order-independent, exactly as
the origins design promised.

Gates:
- **point p50 0.4 µs ≤ 0.8 ✓** (re-affirmed on the final binary).
- **triangle p50 11,742 µs — MISSED ≤ 8,000**; **jp_probe_n1 3,667 µs —
  MISSED ≤ 1,500** (−22% and −35% from baseline). The high-bar
  documentation lives in `final.md`: full phase attribution
  (probe_n1 5,649 → 3,667; probe_n0 −39%; hash_n0 −67%) and the
  irreducibility statement — ~37 ns/probe against bumblebench's
  17–21 ns faithfully-shaped floor, L2-resident and ROB-overlapped at
  batch 1, so the remaining levers are probe-COUNT reduction (planner
  scope) or SIMD-batched COLT probing (layout redesign); both recorded
  as the next suite's openers.
- **Suite-wide: ALL-WIN on every run ✓; every family's final p50 ≤ its
  baseline p50 (bimodal families' p95 all improved) ✓; ledger geomean
  0.69 (−31%, gate ≥ −20%) ✓.**
- `final.md` committed with the complete table, phase attribution,
  per-PRD delta attribution, prefetch-tier evidence (782 passes on
  triangle/spread, zero elsewhere), and the surviving walls with
  owners; verify green; zero-alloc green; clippy green; check-asm green
  on all accumulated gates.
