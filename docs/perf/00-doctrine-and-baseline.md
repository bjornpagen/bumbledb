# PRD 00 — Doctrine and baseline: the unsafe policy, the kernel law, the denominator

## Purpose

Before any hot-path work: (a) make the architecture docs say what this suite
is about to do, so no later PRD is deviating from the record while executing;
(b) pin the baseline every later PRD measures against. The baseline capture
itself lives in [baseline.md](baseline.md) (committed alongside this suite);
this PRD makes it normative and reproducible.

## Technical direction

- **Amend `docs/architecture/00-product.md` — the unsafe policy.** Today the
  doctrine sanctions `unsafe` in exactly one module (`exec/kernel.rs`) plus
  the trace-only fast clock. Replace with the named-module policy: `unsafe`
  (including `core::arch` intrinsics and inline asm) is sanctioned in an
  explicit allowlist of kernel/hot modules, initially:
  `exec/kernel.rs`, `exec/colt.rs` (gather/probe paths), `exec/wordmap.rs`
  (slab probe paths), `exec/run.rs` (leaf/batch paths), `image.rs` (decode
  kernels), `obs.rs` (fastclock, existing). Every module on the list carries
  `#[allow(unsafe_code)]` at the item or module level with a comment naming
  this policy; the crate keeps `#![deny(unsafe_code)]` as the default
  everywhere else. State the law verbatim: **every unsafe path has a safe
  portable reference implementation, and a property test asserts
  bit-identical results across randomized inputs including boundary shapes
  (empty, single, odd lengths, lane-multiple ±1).** kernel.rs's existing
  module docs are the template.
- **Amend `docs/architecture/30-execution.md` — sanctioned kernel shapes.**
  The current text names two NEON kernel shapes (fixed-width predicate scans;
  survivor compaction). Extend the sanctioned set to: fold/accumulate kernels
  (Sum/Min/Max/Count over batch columns, strided or gathered), gather kernels
  (position-indexed column reads), and software-prefetch passes (`prfm`) in
  two-phase probing. Record the doctrine that fold kernels are
  **scalar-ILP-first**: unrolled multi-accumulator scalar loops are the
  default shape (the M2 runs 6-wide integer ALU; 2-lane NEON on 64-bit data
  wins only for compare-heavy min/max), and NEON is used only where measured
  faster on the reference host. Sum semantics are unchanged and
  non-negotiable: i128 accumulation, one range check at finalization.
- **Amend `docs/architecture/50-validation.md` — the phase seam.** Document
  the per-(node, phase) attribution added ahead of this suite:
  `JoinPhase {iter, hash, probe, residual, descend, force}`,
  `Counters::phase_start/phase_end` (default no-ops), `PhaseTimers` under an
  active obs capture only, `Category::Phase` accumulator events, the
  `jp_*` name registry with the node cap and overflow bucket, the
  `WORDMAP_GROW` event, and the phase table (with `excl_us` derivation:
  descend minus the next node's total) in trace output. State the
  measurement caveats: cntvct granularity (~41.7 ns ticks, unbiased over
  accumulation), no `isb` (OoO slop tolerated), and that phase totals
  carry the stamp overhead of deep small-batch nodes — phase tables direct
  work; the untraced timing tables decide gates.
- **Baseline reproducibility.** Re-run `verify` + `bench --trace` (obs
  build, S/seed 1) and confirm the captured phase tables agree with
  [baseline.md](baseline.md) within run-to-run noise (±10% on rows ≥100 µs).
  If any row disagrees beyond that, investigate before proceeding — a
  moving baseline poisons every later gate. Do not update baseline.md to
  match a drifted run without a written cause.

## Passing requirements

1. All three architecture docs amended as above; the amendments name this
   suite (docs/perf/) as their origin, following the hardening suite's
   amendment style.
2. `baseline.md` confirmed reproducible on the reference host (or the
   discrepancy diagnosed and documented in this PRD's file under a
   `## Result` section).
3. `scripts/check.sh` green.

## Out of scope

Any hot-path code change. This PRD writes doctrine and pins numbers.
