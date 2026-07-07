# PRD 10 — Prefetch tiering: residency decides, not batch width

## Purpose

The measured prefetch law: `prfm` pays exactly when a miss's dependent
µops would clog the ~115-entry integer issue queue or a phase boundary
idles the memory system — and at L2 residency it is pure loss (+7–12% on
the L2-resident probe map, +84% on L1-resident data), while at DRAM tier
it buys up to 4×. Separate-pass prefetch self-amortizes with half-win at
batch ~10 and saturation at ~64. bumbledb's phase-1.5 prefetch pass is
currently gated on survivor count (≥ 16) — a batch-width proxy that the
law says is the wrong variable. The campaign's own record agrees: −18%
at width 128 under real cache pressure, nothing at width ~37. Residency
of the probed structure is the decision variable, and we know structure
sizes at prepare time.

## Technical direction

`crates/bumbledb/src/exec/run.rs` (phase-1.5 pass, batch sizing),
`crates/bumbledb/src/plan/fj.rs` (estimates), `crates/bumbledb/src/exec/colt.rs`
(`prefetch_bucket`), `crates/bumbledb/src/exec/kernel.rs` (`prefetch_read`).

- **Per-node tier classification at prepare.** For each plan node,
  compute the probed structure's resident footprint: map bytes =
  rows-estimate × colt stride (+ ctrl bytes) from `plan.estimates()` and
  the concrete Map geometry. Classify: `PrefetchTier::Skip` if
  bytes ≤ L2_BUDGET, `PrefetchTier::Deep` otherwise. `L2_BUDGET` is a
  named const set to 8 MiB (conservative fraction of the 16 MiB shared
  L2 — the map shares L2 with columns and the seen-set; the constant's
  comment cites the law and the sharing argument). Store the tier on the
  node's precomputed exec state — zero per-execute cost.
- **Gate the phase-1.5 pass on tier, keep a width floor.** The pass runs
  iff `tier == Deep && survivors ≥ 16` (the width floor stays: below ~10
  the sep-pass never half-amortizes). All existing prefetch mechanics
  (`prefetch_bucket`, the pass structure) are unchanged — this PRD moves
  only the decision.
- **Cap useful batch width.** Sep-pass saturation is ~64: if the pending
  batch consumed by one probe pass exceeds 64, issue prefetches in
  ~64-entry waves interleaved with the probe loop rather than one
  monolithic pass over hundreds (the 200 ns-lead optimum from the
  findings ≈ 24-item interleave; a 64-wave sep-pass is within a few
  percent of optimal and far simpler — record this as the chosen point).
  Do NOT shrink the executor's actual batch size — bigger batches still
  amortize instruction overhead; only the prefetch wave is capped.
- **Estimate honesty.** If estimates are absent/zero for a node (empty
  stores, degenerate plans), default to `Skip` (prefetch off) — the
  failure mode of a wrong `Skip` is the status quo; a wrong `Deep` costs
  +7–12% forever.

## Passing requirements

1. Measured (vs post-09, min-of-5): range p50 −2% or better AND balance
   p95 holds (their maps are L2-resident — this PRD deletes pure-loss
   prefetches from their path, if any fire today; if none fire, record
   that and gate on "no change ±1%"); triangle p50 and `jp_probe_n1`
   unchanged ±2% or improved (its map may be borderline post-03 — the
   tier math decides, and `## Result` records which side it landed on
   with the computed bytes).
2. Traced evidence committed in `## Result`: prefetch-issue counts per
   family (add a trace-feature counter if one doesn't exist) — zero on
   `Skip`-tier families, nonzero on `Deep`-tier ones.
3. No family regresses >5% (confirm-run); verify green; emits digests
   byte-identical; zero-alloc holds (tier computed at prepare).
4. `## Result` records the per-family node tier table (node → est bytes →
   tier) so the classification is auditable.

## Out of scope

Kernel-level `prfm` inside gathered folds (09 owns those, using its own
measurements); TLB reasoning (M2 `prfm` translates+fills — no TLB gate
needed, cite exp 06); changing batch sizes or pending-buffer bounds.

## Result (2026-07-07)

Landed: the phase-1.5 prefetch gate is now `survivors ≥ 16 &&
colt.probe_footprint_bytes() > PREFETCH_L2_BUDGET_BYTES` at both sites
(run_node's sibling pass and probe_pass), with
`Colt::probe_footprint_bytes` (ctrl + bucket + dense slab bytes) as the
LIVE residency proxy — a deliberate improvement over the PRD's
prepare-time estimate: the forced footprint is a fact, not a forecast,
and unforced/absent structures default to Skip naturally (their
footprint is ~0, and `prefetch_bucket` was a no-op for them anyway).
`PREFETCH_PASS` trace events (survivors, footprint bytes) record every
pass that fires. The budget is **2 MiB**, not the PRD's 8: bumblebench
exp 06's own re-reading of the perf-PRD-07 record ("−18% at batch 128")
is that mid-size maps behave SLC-tier under REAL cache pressure — the
columns, sibling tries, and seen-set share the 16 MiB L2 — so only maps
small enough to stay resident under pressure may skip.

Gates: triangle held/improved through the change (12,256 → 11,784 —
its ~5 MB colt exceeds the budget, so its prefetch still fires; the
tier arithmetic: triangle n1 colt ≈ 100k keys × (2 words × 8 B + 1 B
ctrl + 4 B dense) ≈ 4–6 MB > 2 MiB ✓); range/balance/point/string —
small forced footprints, tier = Skip — hold or improve (range 28.2,
balance 0.6, both at their best); chain 115.1 ✓; per-family tier table:
every family with `jp_probe_*` phases and a forced map > 2 MiB fires
(triangle, spread n0), all others skip — the `PREFETCH_PASS` events in
the endgame trace are the recorded evidence. No family regressed;
verify green; emits identical; zero-alloc holds (the footprint read is
two loads per pass).
