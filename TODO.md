# TODO — the handoff (2026-07-25, owner-directed pause)

The owner paused work mid-campaign. This document is the complete handoff: the
state of the repo, the paused campaign in enough detail to resume or re-author
it cold, and everything else owed. A resumer should read this top to bottom
before touching anything.

## Where things stand right now

- **v0.9.0 is released, published, and CI-green** (tag `v0.9.0` @ `9ecba41b`;
  both npm packages live with the pack-time-injected platform pin — the
  sdk-lane frozen-lockfile bootstrap circle is dead permanently via
  `ts/scripts/pin.ts`). Contents: the zero-key `by()`/`desc()` identity
  comparators over the exact engine-orderable roster, the R3 bool-order tail
  closed on the TS type tier (`NumericVarOk`/`OrderVarOk` split), the primer
  expressibility pins.
- **The primer consumer has its reply**:
  `docs/handoffs/2026-07-25-primer-reply.md` (C33 shipped; C31 ruled with
  running evidence in `ts/test/expressibility-operand-views.test.ts`; the
  whole-jump 0.5.x→0.9.0 runbook). `docs/handoffs/` is now the standing
  convention for consumer asks. Expect primer follow-ups there.
- **The bug-bash + perf campaign is PAUSED mid-Gate** (details below). Its
  Instrument phase is fully landed and committed; its Gate verdict is
  UNRESOLVED — **zero-cost-off is UNPROVEN for the new instrumentation**.
  Treat the instrumented estate as unvalidated until the Gate passes.

## THE PAUSED CAMPAIGN: instrument-first bug bash + perf fanout

Run id `wf_260adbe6-ad4`; script archived at
`~/.claude/projects/-Users-bjorn-Documents-bumbledb/88c4a64e-bc42-45de-813f-9def272bda73/workflows/scripts/bugbash-perf-campaign-wf_260adbe6-ad4.js`.
NOTE: workflow run caches are session-local — a NEW session cannot resume the
run; it re-authors from this section (the script file is readable and is the
authoritative phase spec). Instrument is committed, so a re-launch starts at
Gate.

**Intent (owner's charter):** be excruciatingly data-driven — evaluate all
benchmarks, run a full baseline WITH trace attribution before any perf work,
tally everything, investigate flamegraphs trivially, then fan out on bugs and
on perf targets ranked by measured attribution, not intuition.

**Standing constraints for any resumer:**
- ALL leaf agents (coders, verifiers, finders — every workflow agent) pin
  `model: "opus"` (owner ruling 2026-07-25, supersedes the fable pin).
- Maximal churn, maximal elegance, zero backwards compat — including
  consumers (primer breaks and upgrades; they have no persistent data).
- Benches: strictly sequential, `scripts/measure.sh` mutex, wall-power
  verified (pmset) before/after, nothing else runs during timed windows,
  oracle-gated, DNFs excluded-and-counted.
- Observability doctrine: zero-cost-off (trace feature ZSTs), NO per-row
  spans, no allocation in join loops, alloc×trace are exclusive run modes.

### Phase 1 — Instrument: DONE, committed (7 commits, `d7c111a8..ccd0de8d`)

- **I1 (bench lanes trace)**: scenarios take `--trace` → per-query warm+cold
  Chrome traces + flame embeds; write/commit lanes trace (JUDGMENT_*/
  LMDB_COMMIT finally reach artifacts); every traced artifact lands a
  `.folded` twin. Timed batches stay untraced; the traced sample is separate
  (the measure.rs discipline).
- **I2 (dark subsystems lit)**: spans at batch/pass granularity in the DP
  planner interior + selectivity ladder, columnar batch decode +
  predicate-scan kernels, verify_store sweep, normalization sub-passes.
- **I3 (flamegraphs dead simple)**: `scripts/flame.sh <lane> [query]` — one
  command → `.folded` + self-contained SVG + top-10 table (no external
  flamegraph.pl); `scripts/flamediff.sh <a> <b>` — differential red/blue SVG.

### Phase 2 — Gate: IN FLIGHT WHEN STOPPED (verdict unresolved)

Must pass before anything downstream: (1) release build WITHOUT the trace
feature proves zero-cost-off — `scripts/check-asm.sh`, the alloc gate, hot-
symbol discipline; (2) `cargo test --workspace --all-targets` (default) AND
`-p bumbledb --features trace`; (3) `scripts/check.sh`; (4) ts suite
untouched-check; (5) `scripts/lean.sh`; (6) the I1/I3 smoke + golden tests.
At the stop, both cargo test batteries were mid-run (logs were going to
`/tmp/gate_test_ws.log` and `/tmp/gate_test_trace.log`; the processes were
killed at pause). **Resume = run this gate first, fix-loop until green.**

### Phase 3 — Baseline (never ran): strictly sequential, into `bench-out/baseline-2026-07-25/`

1. **Bench debt first** (this retires every owed-bench item below): the C17
   slot-vs-fetch measurement on the power-budget lane (both
   `measure_children` arms behind `CAPACITY_WEIGHT_SLOT` in
   `storage/commit/judgment.rs`; land the winner, DELETE the loser + flag,
   record numbers at the constant; if slot wins, surface its write-time
   ray-Duration corner for owner ruling — do not rule it); the calendar
   capacity lane (fresh twin world); windowed/lawful re-pins under the
   capacity spelling; the campaign-2026-07-23 stragglers (writes + churn
   wall-power reruns, sweep-commit — needs the obs build — and the unlanded
   `adversarial` subcommand).
2. **Full-suite baseline**: scenarios, report-class durable+ephemeral ×3,
   writes, churn ×3 profiles, crud, curves, lawful, storage — with delta
   tables vs `bench-out/campaign-2026-07-23` (and vs night pins where the
   campaign rode them). MANIFEST with provenance.
3. **Attribution passes**: traced pass (every scenario query warm+cold +
   every write/judgment lane; flamegraphs for ALL via I3 into
   `.../flame/`); separate alloc pass; then **`TALLY.md`** — per lane: the
   number, delta, top-5 span/phase attribution (absolute µs + share), alloc
   footprint, flamegraph path. THE document the perf fanout reads.

### Phase 4 — Hunt (never ran): concurrent once the machine is free

- **4 trace readers** (rings+graph / olap+joins / points+temporal /
  writes+commit lanes): read TALLY + flamegraphs, produce attribution
  rankings — worst absolute µs per lane, mechanism hypotheses that cite
  file:line of the span's source, ranked by (absolute µs × fixability).
- **9 bug-bash finders** over everything written since the 2026-07 audit
  (do not re-report stamped `audit-2026-07/` items): capacity judge
  (plan/judgment/verify_store, dependent bounds, clipped walk, u128, rays,
  R16 interplay); capacity surface (validate/theory/macros/TS builder/FFI/
  pin.ts); GJ-split in production; overlap join + const-operand routing
  (the provisional crossover 16); storage v7 (R17 lockless readers, R18
  wipe, one-allocator); fresh TS surface (by() zero-key, bool tier,
  capacity builder, dispose, explain); Lean capacity drift (Capacity.lean +
  Decide/Oracle vs engine, C11 Admission form, C12 clip lemma); a
  branching/free-feature/unification sweep over all post-audit code; the
  new I1–I3 instrumentation itself (category: observability).

### Phases 5–8 (never ran)

- **Verify**: one adversarial refuter per finding (REFUTED default for
  uncertain bug claims), self-contained report_markdown for survivors.
- **Fix**: a planner turns attribution + verified findings into ≤7
  file-disjoint lanes; RULE: speculative perf mechanisms are skipped —
  data-driven or nothing. Post-fix full gates.
- **Rebench**: exactly the targeted lanes vs the baseline, with
  `flamediff.sh` SVGs embedded; fixes that didn't move their lane are
  called out NOT CASHED.
- **Close**: findings ledger into `audit-2026-07-25/` (reports + README
  tally with outcome stamps), TODO trued, doc re-pins + README graphs where
  headline numbers moved, final gates, push.

## Other open items (pre-existing, unchanged by the pause)

- **1.0.0 close** — owner-gated, explicitly deferred 2026-07-18. Owner
  ceremony only.
- **crashpoint + image-oracle disposition** — consumer-less test-support
  features (fuzzer deletion); keep-dormant vs delete is an owner ruling.
- **C10 ray-Duration verdict parity** (engine vs naive) — one differential
  fixture owed once the engine refusal shape is pinned stable.
- **Audit deferred findings** (stamped in `audit-2026-07/findings/`): 014
  (per-parent leaf batch-of-1), 044 (forced-map telescoped distinct Count),
  053 (two FilterPredicate interpreters), the 009 step-2 per-forced-map
  min/max fence, and the R5 tail (TS measure-keyed Arg spelling + the Lean
  denotation's conformance fence, RULINGS.md §R5).
- **Feature-register triggers, recorded and waiting**: C19 balance laws
  (`Sum == Sum` per group — double-entry); temporal capacity (per-instant
  stabbing-set windows — mechanism sketch recorded beside the trigger:
  half-open boundary sweep, 1-D Helly, the overlap index as the judge's
  walk, polarity intact). Min/Max-window refusal trigger likewise.
- **Primer follow-through**: expect their P2.4 cutover questions via
  `docs/handoffs/`; the expressibility test is the living evidence to point
  at.

## Shipped (compressed ledger — detail in git history and the stamped docs)

- **0.9.0** (2026-07-25): comparators, bool-order tail, pin injection,
  primer pins. CI green all lanes.
- **0.8.0** (2026-07-25): the capacity cutover whole — `<=[w]{lo..hi}`,
  format v7, fingerprint tag 4/label v5, corpus `judgment-capacity-*`,
  zero-trace gate green. Design + dossier stamped LANDED in
  `docs/design/capacity-laws.md` + `capacity-cutover.md`.
- **0.7.0** (2026-07-24): the audit campaign — 162 findings (158 fixed),
  22 rulings (21 implemented, R5 partial), GJ split, overlap join, point
  path, R16/R17/R18, wall-power re-bench (`bench-out/campaign-2026-07-23/
  SUMMARY.md`), R21 re-pins. Ledger: `audit-2026-07/README.md`.
- Earlier: 0.6.0 destructure, 0.5.0 surface pair, cleanup-0.5.0 (PR #11),
  incremental images (PR #10) — see git history.
