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
