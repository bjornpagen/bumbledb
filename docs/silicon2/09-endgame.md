# PRD 09 — Endgame: bench clock protocol, doctrine sweep, final2.md

## Purpose

Close the suite: apply the round-two measurement laws to the bench
harness itself (the fsync-DVFS settle rule and the write-block
placement), sweep the doctrine docs to the round-two state, re-measure
everything, and pin `final2.md` as the new denominator — with the
twice-inherited triangle gates now HARD (the stack that reaches them
shipped in 01/06/07).

## Technical direction

`crates/bumbledb-bench/` (driver, writebench, harness),
`docs/architecture/`, measurement.

- **Spin-settle after commits (exp 17).** The cold protocol
  (`measure_cold`, `tag_touch`) times a rebuild immediately after a
  commit — on a core the fsync just down-clocked (floor 1.05–1.46 GHz,
  demand-driven recovery). Add a bounded spin-settle between touch and
  the timed sample: `clockproxy::warm_up(Duration::from_millis(2))`
  (exp 17: ~10 µs of demand reaches 2.9 GHz; 2 ms of spin reaches the
  ramp's knee without burning the protocol's budget) — the measurand
  becomes "cold cache at working clock", which is the honest number
  (the old one conflated cache cold with clock cold). Record the
  before/after cold_fk_walk delta in `## Result` and annotate
  final2.md's cold row with the protocol change. NEVER sleep — the
  E-core wake lottery (25–40% at ≥ 5 ms sleeps) is exp 17's sharpest
  trap.
- **Write-block placement**: verify (and pin with a driver comment +
  ordering assertion) that write families run AFTER all read families
  in a full run, so no read family measures in a post-fsync clock
  shadow; the `bulk` family (seconds of fsync) must be last of all.
  It already is — the gate is the recorded assertion.
- **Doctrine sweep**:
  - `docs/architecture/50-validation.md`: the attribution section
    gains exp 20's surface — the slide bound generalized to
    min(remaining payload latency, scheduler drain), the −99.6%
    latency-bound-span case, the sub-µs health warning ("attribution
    claims under ~1 µs require CNTVCTSS closes AND repetition; a
    latency-bound span's raw-stamped attribution is presumed wrong"),
    and the commpage kind-3 note (libsystem clocks are slide-proof on
    M2 — `Instant` spans never slid; the campaign's 2× error was stamp
    cost + tick noise + ablation scope).
  - `docs/architecture/30-execution.md`: the probe-shape law (in-cache
    branchless group probing / key-ahead `prfm` / bucketized sweep —
    exps 13/16/18) and the layer split of the instruction doctrine
    (retire-bound: diet; flush-bound: buy instructions to delete
    branches).
  - `docs/silicon/README.md`: appendix line pointing at silicon2.
- **Full re-measure**: 3 ledger runs + traced run (all seven traced
  families), proxy-bracketed min-of-3; the triangle waterfall vs the
  exp 16/19 predictions; `PREFETCH_PASS` coverage counts; the
  per-PRD attribution table (01 vs 06 vs 07 deltas from each PRD's own
  Result).
- **`docs/silicon2/final2.md`**: the complete table vs BOTH
  denominators (silicon final.md and the original baseline), phase
  tables, per-PRD attribution, footprint changes (05), deletion
  ledger (02/08 lines removed), and surviving walls with owners
  (expected: probe COUNT is now planner-owned; stats' residual floor;
  fsync physics).

## Passing requirements

1. **Triangle, hard gates, no documented-miss escape this time unless
   the waterfall proves a new mechanism**: p50 **≤ 8,000 µs** and
   `jp_probe_n1` **≤ 1,100 µs** (staged through 01's ≤ 1,500 and 06's
   ≤ 1,100 — this PRD re-affirms them on the final binary).
2. **stats ≤ 1,200 µs** re-affirmed; point ≤ 0.5 µs holds; every
   family's final2 p50 ≤ its final.md p50 (bimodal: p95); ALL-WIN
   preserved on every run; suite geomean vs final.md improves ≥ 10%.
3. cold_fk_walk re-recorded under the settle protocol with the
   before/after split; no-sleep grep still clean; write-order assertion
   in the driver.
4. Doc greps: 50-validation contains "payload latency" and the sub-µs
   warning; 30-execution contains the layer-split law; no doc still
   states the batch-mean lever as live.
5. `final2.md` committed with everything above; verify green;
   zero-alloc green; clippy green; `check-asm.sh` green on the full
   accumulated gate set.

## Out of scope

Anything new. Scenario suite, L-scale, and the performance claim stay
human-owned. The planner-side probe-count levers (semijoin
pre-filters, degree-aware covers) and wordmap bucketization are the
recorded openers for a THIRD suite, not late additions to this one.
