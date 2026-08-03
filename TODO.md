# TODO — trued at the bugbash-perf campaign close (2026-08-03)

The 2026-07-25 bug-bash + perf campaign is **CLOSED**. Its ledger is
`audit-2026-07-25/README.md` (44 findings, 44 fixed, the tally
cross-referenced to its fix commits); its measurement story is
`bench-out/baseline-2026-07-25-post/DELTA.md` (the rebench deltas, the
per-fix-lane verdicts, the 22 flamediffs). The campaign lived on branch
`bugbash-perf` (PR #14); main takes it only as a completed state.

## Owed — the campaign's own residue (data first, no intuition fixes)

- **The storage-lane regression cluster** (the rebench called its own lane
  NOT CASHED): `cold_containment_walk_delete` 3.1–3.4× in all six reps,
  the NOSYNC commit ladder 1.24–1.44 (b100 traced suspects: apply_deletes
  self-time 2.1× under the cursor-fold applier, judgment_source +70% under
  the T8 walk at small batch — the sweep only priced 1k–4k parents),
  windowed ephemeral 1.07–1.17. Attribution first: the delete lane has no
  traced twin (the reps' write set is untraced by protocol) — light it,
  then fix. Flamediffs: `bench-out/baseline-2026-07-25-post/flame/
  writes.durable.delete_b100.diff.svg`, `writes.nosync.commit_b100.diff.svg`.
- **r6_two_path_count 1.46** (ours 131→197 ms) on the sink/plan lane's own
  COUNT-shaped territory — descend now carries essentially the whole query
  (`flame/scenarios.rings.r6_two_path_count.warm.diff.svg`, jp_descend
  51%+45%). Same discipline: trace-reader ranking before any change.
- **Owner ruling owed (surfaced by C17, recorded not ruled)**: the landed
  slot arm refuses a ray-valued Duration weight at WRITE time — strictly
  stronger than C10's judge-time refusal, visible only for a ray child
  under an absent parent (`bench-out/baseline-2026-07-25/capacity-c17/
  SUMMARY.md`).
- **Overlap constants re-pin**: `OVERLAP_CROSSOVER = 16`
  (`exec/run/overlap_leaf.rs`) and `FLAT_SWEEP_CEILING = 128`
  (`interval/overlap.rs`) are rig-pinned provisional — re-pin both from the
  `overlap_profile` sweep on a quiet machine, never by inspection (the
  finding-013 phase attribution now decomposes build vs walk vs residual,
  so the sweep finally has its signal).

## Open items (pre-existing, unchanged)

- **1.0.0 close** — owner-gated, explicitly deferred 2026-07-18. Owner
  ceremony only.
- **crashpoint + image-oracle disposition** — consumer-less test-support
  features (fuzzer deletion); keep-dormant vs delete is an owner ruling.
- **Audit-2026-07 deferred findings** (stamped in `audit-2026-07/findings/`):
  014 (per-parent leaf batch-of-1 — the campaign's pinned-run fold
  `a75d1e65` lands the adjacent mechanism, but o4's lane was not re-benched;
  the stamp stands until it is), 044 (forced-map telescoped distinct Count),
  053 (two FilterPredicate interpreters), the 009 step-2 per-forced-map
  min/max fence, and the R5 tail (TS measure-keyed Arg spelling + the Lean
  denotation's conformance fence, RULINGS.md §R5).
- **Feature-register triggers, recorded and waiting**: C19 balance laws
  (`Sum == Sum` per group — double-entry); temporal capacity (per-instant
  stabbing-set windows — mechanism sketch recorded beside the trigger:
  half-open boundary sweep, 1-D Helly, the overlap index as the judge's
  walk, polarity intact). Min/Max-window refusal trigger likewise.
- **Primer follow-through**: expect their P2.4 cutover questions via
  `docs/handoffs/`; the expressibility test
  (`ts/test/expressibility-operand-views.test.ts`) is the living evidence
  to point at.

## Retired this campaign (previously owed here)

- C17 slot-vs-fetch: measured, slot landed, fetch arm + flag deleted
  (`484c3871`; artifacts `bench-out/baseline-2026-07-25/capacity-c17/`).
- The capacity/windowed/lawful bench lanes + re-pins under the capacity
  spelling (`e511b540`), the calendar capacity twin world.
- The writes-ladder and churn wall-power reruns (the campaign manifest's
  PENDING rows) — landed at the baseline, re-run at the rebench; every
  README chart now regenerates from `bench-out/baseline-2026-07-25-post/`.
- C10 ray-Duration verdict parity engine-vs-naive: the refusal is a
  compared verdict on the differential wall (finding 018, `b0fa69f0`).
- The sweep-commit T8 lane (`0e7d1d42`) and the T8 mechanism (`128e4504`)
  — with the b100 caveat recorded under Owed above.

## Shipped (compressed ledger — detail in git history and the stamped docs)

- **bugbash-perf campaign** (2026-07-25 → 2026-08-03): instrument → gate →
  baseline (`bench-out/baseline-2026-07-25/`, TALLY attribution, 147
  flamegraphs) → hunt → 44/44 findings fixed across seven file-disjoint
  lanes → rebench (`baseline-2026-07-25-post/`: reads 0.87–0.94 on five of
  six reps, scenarios 0.88 with the Hunt cluster reversed, storage lane
  honestly NOT CASHED). Ledger: `audit-2026-07-25/README.md`.
- **0.9.0** (2026-07-25): comparators, bool-order tail, pin injection,
  primer pins. CI green all lanes.
- **0.8.0** (2026-07-25): the capacity cutover whole — `<=[w]{lo..hi}`,
  format v7, fingerprint tag 4/label v5, corpus `judgment-capacity-*`,
  zero-trace gate green. Design + dossier stamped LANDED in
  `docs/design/capacity-laws.md` + `capacity-cutover.md`.
- **0.7.0** (2026-07-24): the audit campaign — 162 findings (158 fixed),
  22 rulings (21 implemented, R5 partial), GJ split, overlap join, point
  path, R16/R17/R18, wall-power re-bench, R21 re-pins. Ledger:
  `audit-2026-07/README.md`.
- Earlier: 0.6.0 destructure, 0.5.0 surface pair, cleanup-0.5.0 (PR #11),
  incremental images (PR #10) — see git history.
