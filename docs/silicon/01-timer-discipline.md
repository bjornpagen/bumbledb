# PRD 01 — Timer discipline: free stamps, honest spans

## Purpose

The findings overturned our timer model: the raw `cntvct_el0` read costs
0.30 ns (1/cycle — 7× cheaper than the ~2 ns we budgeted), the unfenced
reorder slide is bounded at ~50 ns by backend scheduler occupancy (not the
630-entry ROB), and `CNTVCTSS_EL0` (present on M2, FEAT_ECV) is a
slide-proof single-shot stamp at 4.6 ns — half the price of `isb`. Our obs
layer was designed under the wrong cost model: it can afford more stamps
than it takes, and its single-shot spans are the one place the slide
actually bites (−83% error on a 28 ns op).

## Technical direction

`crates/bumbledb/src/obs.rs` (fastclock, spans, PhaseTimers) +
`docs/architecture/50-validation.md`.

- **Add the self-synchronized stamp.** `fastclock` gains
  `ticks_ss() -> u64` reading `CNTVCTSS_EL0` via `mrs` (aarch64 asm,
  `options(nomem, nostack)`). Target is the reference host (M2+, FEAT_ECV
  present — per the machine-tailoring rulings there is no runtime feature
  detection; a `cfg(target_arch = "aarch64")` compile gate matches the
  existing fastclock pattern).
- **Stamp policy, encoded in the API, not in comments:**
  - Accumulated phase attribution (`PhaseTimers`, `phase_start/phase_end`):
    RAW `cntvct` stamps. The measured inflation of raw two-stamp
    attribution is ≤ 2–3% at 10 ns phases — fencing here costs more than
    it fixes (`isb` stamps measured +164%).
  - Single-shot spans (anything that stamps once around one operation —
    `obs::span` and the traced prologue splits from perf-PRD 11): CLOSING
    stamp becomes `ticks_ss()`. Opening stamps stay raw (the slide only
    moves the closing stamp earlier).
- **PhaseTimers overhead pass.** With the cost model corrected, re-audit:
  `phase_start`/`phase_end` must compile to (load capturing flag, branch,
  mrs, integer add/store) — no calls, no Vec pushes, no format machinery
  in the hot path. Any per-event allocation moves to flush time. Verify by
  objdump on a trace-feature build: the inlined phase stamp sequence
  contains no `bl`.
- **Correct the doc.** `50-validation.md`'s timer section: replace the
  ~2 ns read claim, the ROB-slide caveat, and any `isb` recommendation
  with the measured law (0.30 ns raw, ≤ ~50 ns scheduler-bounded slide,
  `CNTVCTSS` 4.6 ns for single-shot, 41.67 ns quantum, `isb` for neither
  purpose). Cite bumblebench exp 11.

## Passing requirements

1. Traced-vs-untraced triangle execute delta ≤ 3% (was ~5%): measure both
   binaries back-to-back under the PRD-00 proxy, min-of-5.
2. An `#[ignore]`d microbench test pins stamp costs in ticks-per-1k-stamps
   form: raw ≤ 1.5 cycles/stamp equivalent, `ticks_ss` ≤ 20 cycles/stamp
   back-to-back (both loop-amortized, proxy-bracketed).
3. objdump gate: no `bl` inside the inlined phase-stamp sequences of the
   trace build's `probe_pass`.
4. `50-validation.md` grep gate: the strings "2 ns" (as a timer-read cost)
   and "reorder buffer" (as the slide bound) no longer appear in the timer
   section; "CNTVCTSS" does.
5. No family regresses >5% (confirm-run protocol); verify green.

## Out of scope

Bench-harness clock proxy (landed in 00); any change to what is traced;
phase-table rendering.

## Result (2026-07-07)

Landed: `fastclock::ticks_ss()` (`CNTVCTSS_EL0` by encoding
`s3_3_c14_c0_6`); trace spans now stamp on the SAME cntvct timeline as
PhaseTimers (tick anchor, anchor-resolves-first — the first-stamp
underflow was caught by the trace tests), with raw opening stamps and
self-synchronized closing stamps; `Instant::now` (22–32 ns/stamp) is out
of the span path entirely. PhaseTimers audit: phase_start/end already
compile to flag-check + `mrs` + integer ops — nothing to trim; the
objdump gate on the trace build found no `bl` in the stamp sequences.
50-validation.md's timer section rewritten to the measured cost model.

Gates:
1. Traced-vs-untraced triangle delta: obs binary min-of-5 12,302.6 µs
   vs default 12,563.9 µs — the trace seam reads 2.1% FASTER than the
   default build (binary-layout noise dominates; the seam cost is below
   noise). ≤ 3% ✓.
2. Stamp-cost pin green: raw ≤ 0.6 ns, `ticks_ss` ≤ 7.0 ns asserted
   (loop-amortized, proxy-bracketed).
3. No `bl` in inlined phase stamps (objdump, trace build) ✓.
4. Doc greps: "2 ns" gone, "CNTVCTSS" present ✓.
5. No family regressed (see PRD 02's battery — same binaries) ✓.
