# The layer-law campaign — eight falsifier twins, two instruments, one day

2026-07-16, main @ 6ef4ed14 (the 1.0.0-candidate tree), Apple M2 Max, the
frozen surface. The campaign law: implementation and layout only; every
optimization an A/B twin judged by interleaved same-session measurement
under `scripts/measure.sh`; predictions written down before the runs
(`docs/reference/apple-silicon-performance.md` is the judgment layer);
losses recorded as gravestones with their falsifier harnesses committed,
never silently dropped. Adversarial review re-ran every claimed win
independently before merge.

## The verdict ledger

| Twin | Verdict | The number (interleaved A/B, regime-labeled) |
|---|---|---|
| T1 predicate-scan reshape (4-lane + `to_bitmask`, `kernel/filter.rs`) | **WIN** | kernel 1.31–1.51x at L1, L2, the 24–50 MiB band, AND DRAM; family: range +3%, rest neutral |
| T2 gather scalar twin (`kernel/gather.rs`) | LOSS | incumbent portable gather wins; the CONTRADICTS-LAYOUT read refuted; pins in `kernel/tests.rs` |
| T3 probe flag-free compare (`colt/probe.rs`) | LOSS | shipped shape wins — the probe batch's cross-element independence already saturates the miss lanes; the 1.2–1.7x flag-strand prediction does not bind here; pins in `colt/tests/pins.rs` |
| T4 stride-padder band (`image/stride.rs`) | LOSS | the >=3x poison band appears nowhere at image pitches (max 1.5x on a synthetic tight kernel at 128 B residue; real family surface retire-bound 1.00x; 2 KiB padding INVERTS 0.85–0.93x). PAD_TOLERANCE stays 384. **Re-open trigger recorded:** re-run `image/tests/stride_ab.rs` now that T1's tighter multi-column kernels have landed; pure pow-2 pitches measured 1.25–1.8x on tight kernels (family-invisible today) |
| T5 probe-hash const-arity dispatch (`run/probe_pass.rs`, `colt.rs`) | **WIN** | triangle +5.5–6.1% (review re-run 1.0614x), chain +2.2%, spread/skew ~+2%, M-scale displaced +4.3%; neutral elsewhere |
| T6 compaction triad diet (`kernel/compact.rs`) | **WIN** (kernel proven 1.50–1.59x flat across selectivity; family neutrality re-run on the quiet machine post-campaign — see the merge commit) | B arm ~0.30 ns/item = the 1.00 cy/item branchless pin; 0/1 mask contract verified at all 14 call sites |
| T7 Allen counter spill (`kernel/neon.rs`) | **WIN** | kernel 3.39–3.62x in BOTH regimes (13.6 → 48.6 codes/ns at L1) — the `black_box` stack slot replaced by a register-pinned empty `asm!`; family strictly neutral, as the phase fraction predicts |
| T8 judgment probe order (`storage/commit`) | LOSS | arrival vs sorted order indistinguishable on the judgment span at bench commit sizes; gravestone in `docs/reports/judgment-probe-order-gravestone.md` |
| T9 displaced lanes (`bumbledb-bench/src/displaced.rs`) | **LANDED** (instrument) | the displacement regime now a standing roster lane: disp_stream 1.19–1.22x at 96 MiB foreign mass, disp_probe correctly ~1.00 (already DRAM-tier) |

Prediction scorecard (written before the runs): 4–5 of 8 landing,
predicted; T3/T8 gravestones called; T2/T4 misses (intuition said land),
T5 underestimated, T7's magnitude underestimated 10x. The ledger's thesis
about intuition versus measurement, self-applied.

## The wall-time budget (the campaign's ranking function, now standing)

Full per-family phase table in the instrument run (obs `JOIN_PHASE`
accumulators + flame self-time, scale S, traced fractions). The top
sinks, fraction x family-count:

1. **`lmdb_commit` (fsync)** — 54–99.5% of every write row. Physics
   (`m2max.clock.fsync-floor`); closed.
2. **Leaf emit + dedup + per-survivor routing** — 35–74% of every join
   family. The leaf runs per parent (batch=1) with full slot-row copies
   per survivor (`probe_pass.rs:446–468`, `:481–495`).
3. **Aggregate gather/fold** — 85–92% of the fold families; the per-row
   suffix-column gather in `AggregateSink::emit_batch` dominates, the
   SIMD fold itself is <10% of it.
4. **Answer encode (`finalize`)** — 39–62% of containment_walk, range,
   rsvp_union, free_busy: per row x per column `match column.ty`
   (`api/prepared/finalize.rs:62–112`).
5. **Interval/Allen residual scans** — meets_chain 94%,
   slot_booking_overlap 73% — real kernel work (T1/T7 landed here).
6. **Fixpoint round machinery** — 95.6% of closure_depth (see W1).
7. Probe walk; 8. cover batch draw; 9. `apply_inserts`; 10. hash
   (max 7.4% anywhere — which is why T5's win is triangle-shaped).

## The waste list (the next campaign's queue, measured not guessed)

- **W1 — quadratic accumulated-image rebuild per fixpoint round**
  (`api/prepared/fixpoint.rs:435–439`): every round re-transposes the
  FULL seen-set (`answers_since(0)`) — O(n²/2) row-copies over n rounds;
  measured 95.6% of closure_depth's wall (21.1 of 22.1 ms). An
  incremental accumulator is the single biggest lever in the read
  roster. **Estimated: most of a 20x on deep closures.**
- **W2 — leaf runs per parent; slot-row copies per survivor**
  (`probe_pass.rs:446–468`, `:481–495`): inside the bucket that owns
  35–90% of every join family. Per-batch leaf grouping / copy-only-
  changed-slots. Estimated 10–25% of spread/skew/chain-class families.
- **W3 — per-row per-cell type dispatch in finalize**
  (`api/prepared/finalize.rs:62–112`): 12–24 ns/row; hoist the column
  dispatch out of the row loop (column-major fill). Estimated 30–60% of
  finalize's share where it dominates.
- **W4 — redundant zero-fill before full overwrite**
  (`probe_pass.rs` mask/gather `resize(n, 0)` x5 sites,
  `kernel/allen.rs:105–118`): `_platform_memset` measured 3.7% of
  meets_chain. Trivial.

## The allocation census (harness landed: `tests/alloc_census.rs`)

- **Warm execute: 0 events / 0 bytes for EVERY family** — including the
  ungated ones (aggregates, Pack, calendar, windowed, recursion at every
  cap). The gate's floor holds census-wide.
- Prepare: linear on every axis (~160–220 events/rule; no superlinear
  pipeline blowup) — the dark-horse suspicion refuted.
- Commit anatomy: ~5.5–6.4 events/fact at batch, dominated by plan-side
  edge/determinant boxes; the determinant clone discipline holds at
  census level (exactly one clone per distinct tuple).
- Open: ~18 events + 1.4 KB per relation; the ephemeral probe battery
  costs 156 events / 13.3 KB once per open.
- Hoistable candidates recorded in the census report (plan-side tiny
  boxes, ops-vec growth) — none load-bearing at current scales.

## What the campaign changes about the next one

The kernels are largely done: T2/T3 prove the probe and gather shapes
optimal under the ledger's own mechanisms, and T1/T6/T7 collected what
the kernels still owed. The remaining money is STRUCTURAL — W1 (fixpoint
rebuild), W2 (leaf batching), W3 (finalize dispatch) — plus the write
side is fsync physics forever. The displaced lanes stand ready to judge
any future DRAM-tier claim, and the budget table turns the next
argument about "where does the time go" into a lookup.

## The post-merge session (the campaign's numbers on the shipped tree)

Full stamped session on the merged tree (verify 2,862 →
`f4a9d094…`, three durable + three ephemeral runs + scenarios, one
mutex hold, quiet machine): **ALL-WIN on all three durable runs**;
the five README charts regenerate from exactly these runs.

Min-of-3 p50 against the pre-campaign re-earn: **triangle +7.2%**
(and +8.3% median in a same-corpus interleaved old-vs-new binary A/B
— the campaign's headline, T5+T6 compounding), chain +3.2%,
postings_without_tag +20% (min-of-3; the family is strongly bimodal
— see below), **free_busy +15%** (interleaved-confirmed, 3 of 4
pairs — an unforecast beneficiary, most plausibly T1's scan reshape
under its Pack), meets_chain/mandate_overlap +1–3%, everything else
within the band.

Two families refused a stable verdict and are recorded as BIMODAL,
not regressed: slot_booking_overlap (interleaved per-pair ratios
0.50–1.15 on identical binaries) and postings_without_tag
(0.34–2.01). Both flip between two performance modes across whole
bench processes; the chart rule (min-of-3) selects the fast mode for
both engines symmetrically. A mode-flip mechanism hunt (store page
state vs the 35% code-placement lottery) is queued behind W1–W4.
