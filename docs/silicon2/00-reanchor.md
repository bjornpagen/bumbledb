# PRD 00 — Re-anchor on final.md and arm the round-two hazards

## Purpose

Every gate in this suite reads against `docs/silicon/final.md`. This
PRD confirms that table still holds on the current tree (nothing has
moved since the endgame commit), and arms the two round-two measurement
hazards that could otherwise poison the suite's own gates: positional
process-start DVFS contamination and the co-tenant contamination class
that survives min-of-reps (exp 15's phantom-finding machinery).

## Technical direction

`crates/bumbledb-bench/` only — zero engine changes.

- **Confirmation runs.** Three full ledger runs + one traced run
  (triangle, chain, stats, spread, skew, range, point), the
  silicon-suite protocol exactly (`scripts/measure.sh`, min-of-3,
  proxy-bracketed). If any family's min p50 deviates > 10% from
  final.md, investigate before proceeding (confirm-run protocol) and
  record the re-anchored number in this PRD's `## Result`.
- **Per-rep proxy stamps (the exp-15 defense).** The harness currently
  brackets a family BLOCK with two proxy readings. Add OPTIONAL per-rep
  bracketing to `measure_batched`: a `proxy_per_rep: bool` mode (off by
  default — it costs ~400 µs/rep) that records `ghz` per sample and
  lets the report renderer print a normalized column
  (`p50 × ghz_sample / ghz_block_max`). This mode exists for
  confirm-runs on suspicious findings, not for routine gating; wire a
  `--proxy-per-rep` bench flag through to it.
- **Warm-start discipline (exp 18's positional hazard).** The driver
  already runs a 200 ms warm spin before the first family. Extend the
  defense: the FIRST family measured in any process is additionally
  susceptible to the 1.45–1.97 GHz process-start band — add one
  discarded dummy family iteration (execute the first selected family's
  closure ~32 times untimed, beyond its own warmups) before the first
  measured block, and note it in the driver.
- **No sleeps audit.** grep the bench crate for `sleep`: any sleep
  between measurement blocks is replaced with `clockproxy::warm_up`
  spins (exp 17's law — a sleep hands the thread to the E-core
  lottery). If none exist, record that.

## Passing requirements

1. Three-run confirmation table committed in `## Result`; every family
   within 10% of final.md (else investigated + re-anchored, recorded).
2. `--proxy-per-rep` mode works end to end: a bench run with it emits
   per-sample GHz in report.json and the normalized column renders; an
   `#[ignore]`d test demonstrates a synthetic slow-clock sample being
   flagged by normalization where the block bracket missed it.
3. Dummy-iteration warm-start in the driver; no-sleeps grep clean (or
   fixed); verify green; no engine diffs in this PRD.

## Out of scope

Engine changes (01+); the fsync settle protocol for cold families
(PRD 09 — it changes a measurand and belongs with the final recording).

## Result

**Verify**: 2,468 cases green on the frozen binary (`/tmp/bb-s2-00`),
stamp `3d383c9c7da5…`. No engine diffs in this PRD (bench crate only).

**Three-run confirmation** (min-of-3, `scripts/measure.sh`,
proxy-bracketed; `bench-out/s2anchor{1,2,3}`), vs final.md p50:

| family | final.md p50 | re-anchor min p50 | Δ | ruling |
|---|---|---|---|---|
| point | 0.4 | 0.4 | 0% | holds |
| string | 0.8 | 0.8 | 0% | holds |
| balance | 0.7 | 0.7 | 0% | holds (bimodal: p95 25.4 vs 25.0 ✓) |
| fk_walk | 2.9 | 6.9 | bimodal | p95 gates: 928.5 vs 889.0 = +4.4% ✓ |
| skew | 35.8 | 39.8 | bimodal | p95 gates: 932.2 vs 924.5 = +0.8% ✓ |
| range | 28.5 | 28.1 | −1.4% | holds |
| chain | 104.0 | 114.5 | +10.1% | investigated ↓, re-anchored 114.5 |
| stats | 1,872.5 | 1,877.8 | +0.3% | holds |
| spread | 10,725.8 | 11,843.4 | +10.4% | investigated ↓, holds (clock) |
| triangle | 11,742.5 | 12,135.2 | +3.3% | holds |
| cold_fk_walk | ~4,018 | 3,544.5 | −12% | holds (better) |
| bulk | ~912–920k | 990,677 | +7.7% | holds |

commit_single/commit_batch were proxy-flagged contaminated in all three
runs (fsync-DVFS, the expected class) and stay on final.md's physics
band.

**The chain/spread investigation** (confirm-run protocol, 3 further
runs, `--proxy-per-rep`, `bench-out/s2confirm{1,2,3}`): both deltas are
clock state, not code. Spread's normalized p50s: 10,782 / 10,731 /
10,508 — at final.md's 10,726 (min raw p50 10,835 = +1.0%; the anchor
battery's 11,843 was a slow-clock artifact). Chain's normalized p50s:
111.9 / 114.6 / 123.6 — min 111.9 = +7.6%, within the 10% band.
**Chain re-anchors at 114.5 raw (111.9 normalized)**: final.md's 104.0
was the min over the whole endgame campaign's many more runs; suite
gates that read chain use 114.5.

**Per-rep proxy mode, end to end** (requirement 2): the confirm runs
above ran `--proxy-per-rep`; report.json carries `"p50_norm"` per
family (chain `111851` ns, spread `10782250` ns in s2confirm1) and the
markdown Clock proxy table renders the "norm p50 (us)" column. Live
catch: s2confirm2's spread block was flagged CONTAMINATED by the block
bracket (3.11 → 3.21 GHz) while its normalized p50 (10,730.6) agrees
with the clean runs to 0.5% — the per-rep stamps rescued a block the
bracket could only discard. Tests: the synthetic slow-clock case is
pinned un-ignored (`normalization_corrects_slow_clock_samples_and_keeps_
real_ones`: raw p50 175 → normalized 100 with real slowness surviving);
the e2e mode test (`per_rep_proxy_mode_populates_the_normalized_p50`,
`#[ignore]`d) runs green.

**Warm-start discipline** (requirement 3): the driver executes the
first selected family's closure 32 extra untimed iterations before the
first measured block (`first_family_warmed`), on top of the 200 ms
clock warm spin — the process-start 1.45–1.97 GHz band (exp 18) never
touches a measured sample. **No-sleeps audit**: `grep -rn "sleep"
crates/bumbledb-bench/src` — zero hits between measurement blocks
(clean at audit time; exp 17's law needs no fixes here).
