# PRD 00 — Baseline re-anchor and harness discipline

## Purpose

Every gate in this suite reads against a committed denominator, and the
findings proved our harness can be lied to: co-tenant builds swing P-cores
2.4–3.5 GHz (manufactured two fake 2× results in the bumblebench campaign
before an interleaved clock proxy caught them), and wall-clock `cntvct`
cannot distinguish "code got slower" from "clock got slower". This PRD
gives bumbledb-bench the same defenses bumblebench ended up with, then
captures the baseline.

## Technical direction

`crates/bumbledb-bench/` (driver, families, trace_out) + `scripts/`.

- **Serial-ALU clock proxy.** Add to the bench harness a calibration
  primitive: a dependent chain of 8 `mul` instructions (24 cycles/iteration
  at latency 3 — the exact discriminator bumblebench used), run for a fixed
  tick budget, converted to an effective GHz estimate. Implementation:
  inline asm on aarch64 (`mul x, x, y` chain; `options(nomem, nostack)`),
  timed with raw `cntvct` over ≥ 4,096 iterations (loop-amortized — the
  41.67 ns quantum makes single-shot meaningless). Run the proxy
  immediately before and after every family measurement block; attach
  `ghz_pre`/`ghz_post` to the emitted sample record.
- **Contamination annotation, not retry-loops.** If `min(ghz_pre, ghz_post)
  < 3.2`, mark the sample `contaminated: true` in the bench output and have
  the report renderer exclude contaminated samples from p50/p95 (print the
  exclusion count). Do NOT silently rerun in a loop — a bounded single
  retry per contaminated sample is allowed; beyond that, report dirty.
- **Cross-run minima support.** The ledger renderer must be able to merge
  N run directories and report per-family min-of-runs p50 alongside the
  per-run numbers (`--merge` flag or equivalent). Throughput-style gates in
  this suite quote min-of-5.
- **DVFS warm-up.** The findings showed opening calibration reads 3.06 GHz
  vs 3.49 steady-state: the driver must run a ≥ 200 ms warm spin before the
  first family and discard the first sample of each family (already done
  via warmup? verify — the gate below checks it).
- **Quantum guard.** Assert (in the driver, not per-sample) that every
  gated metric is either loop-amortized or ≥ 12 ticks (500 ns) — point/
  string p50s at ~1 µs are ~24 ticks and pass; if any family's per-execute
  time falls below 12 ticks, the driver must batch executes per sample for
  that family and divide.
- **Baseline capture.** With the above landed: 5 full ledger runs on a
  quiet machine (measurement lock respected — `scripts/measure.sh`
  conventions from bumblebench apply if a lock script exists here; if not,
  create one), traces with phase tables for triangle, chain, stats, spread,
  skew, range. Commit `docs/silicon/baseline.md` with: per-family p50/p95
  (each run + min-of-5), the triangle/chain/stats/spread phase tables
  (`jp_*` rows with excl_us), proxy GHz bands observed, and the store-suite
  numbers (commit_batch, cold, store footprint).

## Passing requirements

1. Proxy live: every sample record in bench output carries `ghz_pre`/
   `ghz_post`; an `#[ignore]`d test demonstrates the detector fires (spawn
   a spin thread on all cores, observe `contaminated: true` samples).
2. Two consecutive full ledger runs agree within 5% p50 on every
   non-bimodal family (fk_walk/balance/skew exempt, gated on p95 within
   10%) — else the divergence is investigated and documented in
   `baseline.md` before it is committed.
3. `docs/silicon/baseline.md` committed with everything listed above.
4. No engine code changes in this PRD (bench + scripts only); verify green.

## Out of scope

Any engine change; scenario-suite and L-scale runs (human-owned); the
timer changes inside the engine's obs layer (PRD 01).

## Result (2026-07-07)

Landed: the serial-ALU clock proxy (`clockproxy.rs`: 8-mul dependent
chain, 24 cycles/iter, ~200 µs loop-amortized reads), per-family-block
GHz brackets in every report row (`ghz` in JSON, Clock proxy table in
markdown), contamination = min < 3.2 GHz with ONE bounded retry for
idempotent read blocks (`guarded`) and annotate-only for write blocks
(`stamped` — the retry re-ran `Db::create` once and crashed on
`AlreadyInitialized`: writes are not idempotent, and their low GHz is
fsync-DVFS physics anyway), cross-run minima via the new `merge`
subcommand (contaminated blocks excluded, exclusion count printed), a
200 ms warm spin before the first family, and the 500 ns quantum guard
(batch-and-divide; no current family trips it). `measure.sh` created
(mkdir-lock measurement mutex).

Gates: detector-fires ignored test green under 24-way spin load; run
variance investigated and documented in baseline.md (tick-quantized
sub-2 µs families; chain's ±10% run-scoped mode); baseline.md committed
(min-of-5 ledger, six phase tables, GHz bands, 12 contaminated blocks
named). Engine untouched at the baseline commit; verify green (2,468
cases).
