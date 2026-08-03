# bumbledb bug-bash + perf campaign — 2026-07-25

The instrument-first campaign: authored 2026-07-25 (run `wf_260adbe6-ad4`,
paused mid-Gate the same day), resumed 2026-08-01 under the v2 trust posture
(`docs/handoffs/2026-08-01-bugbash-perf-workflow-v2.js` — the landed opus
Instrument estate re-reviewed from scratch as an untrusted submission, every
agent re-pinned to fable), closed 2026-08-03. All work on branch
`bugbash-perf` (PR #14); code rev at close `3b31cd84`, engine at v0.9.0
post-capacity-cutover, post-GJ-split.

Everything here was data-driven by charter: full baseline WITH trace
attribution before any perf work, fixes ranked by measured µs, a rebench
that names what cashed and what did not — no intuition fixes anywhere.

## Findings tally

44 findings published, every one CONFIRMED at the adversarial verify wall
(REFUTED claims died at the wall and were never published; uncertain bug
claims defaulted to REFUTED by policy). All 44 are fixed on this branch —
nothing open, nothing deferred.

### By verdict × severity

| Verdict   | High | Medium | Low | Total |
|-----------|-----:|-------:|----:|------:|
| Confirmed | 5    | 20     | 19  | 44    |

### By category × severity

| Category                | High | Medium | Low | Total |
|-------------------------|-----:|-------:|----:|------:|
| bug                     | 3    | 6      | 3   | 12    |
| incoherence             | 1    | 4      | 6   | 11    |
| unification             | 0    | 3      | 3   | 6     |
| observability           | 1    | 2      | 2   | 5     |
| perf                    | 0    | 2      | 2   | 4     |
| missing-free-feature    | 0    | 2      | 0   | 2     |
| inappropriate-branching | 0    | 0      | 2   | 2     |
| lean-rust-drift         | 0    | 1      | 0   | 1     |
| inelegance              | 0    | 0      | 1   | 1     |

### By outcome (campaign close, 2026-08-03)

| Outcome | Count | |
|---------|------:|---|
| fixed   | 44    | commit refs per row below; per-finding stamps carry the detail |
| open    | 0     | |

## The findings

| # | Title | Category | Severity | Finder lane | Outcome |
|---|-------|----------|----------|-------------|---------|
| [001](./findings/001-interval-measure-underflows-on-an-inverted-general.md) | interval_measure underflows on an inverted general-tail interval — panic in debug, garbage measure in release | bug | high | capacity-judge | fixed `c9a78a67` |
| [002](./findings/002-c18-dimension-gate-enforces-one-of-three-mixing-di.md) | C18 dimension gate enforces one of three mixing directions while four doctrine sites state the full pairing law | incoherence | high | capacity-surface | fixed `a2c555d8` |
| [003](./findings/003-a-stale-ephemeral-dirty-marker-makes-db-ephemeral-.md) | A stale `ephemeral.dirty` marker makes `Db::ephemeral` silently destroy a committed DURABLE store | bug | high | storage-v7 | fixed `15342f00` |
| [004](./findings/004-order-pointin-type-tier-judges-each-side-in-isolat.md) | Order/pointIn type tier judges each side in isolation — every cross-domain pairing compiles | bug | high | ts-surface-fresh | fixed `9241feb3` |
| [005](./findings/005-explain-breaks-the-r13-execute-symmetry-for-every-.md) | explain() breaks the R13 execute-symmetry for every query carrying a set param or literal membership array | observability | high | ts-surface-fresh | fixed `1bb35923` |
| [006](./findings/006-50-storage-md-documents-a-ceiling-walk-early-exit-.md) | 50-storage.md documents a ceiling-walk early exit the C14 ruling and the code both reject | incoherence | medium | capacity-judge | fixed `f720787f` |
| [007](./findings/007-ts-wall-has-no-c18-twin-at-either-tier-unit-capaci.md) | TS wall has no C18 twin at either tier — unit capacity with a duration() bound dies only at Db.create | missing-free-feature | medium | capacity-surface | fixed `4239208e` |
| [008](./findings/008-the-measure-law-weight-apply-bound-resolve-has-two.md) | The measure law has two engine definitions: validate's closed-constant arm re-implements judgment.rs inline | unification | medium | capacity-surface | fixed `d831280a` |
| [009](./findings/009-fold-split-strands-group-only-lookups-in-the-fold-.md) | fold_split strands group-only lookups in the fold-domain node | perf | medium | gj-split-live | fixed `3e7b8920` |
| [010](./findings/010-overlap-crossover-gates-on-group-size-alone-the-in.md) | OVERLAP_CROSSOVER gates on group size alone; once-probed groups pay the sort-and-tree build for a single query | perf | medium | overlap-join-live | fixed `8d17ac59` |
| [011](./findings/011-multiple-const-side-connected-allen-residuals-coul.md) | Multiple const-side connected Allen residuals could conjoin into one tighter overlap query for free | missing-free-feature | medium | overlap-join-live | fixed `ead48b79` |
| [012](./findings/012-the-architecture-estate-still-declares-interval-ov.md) | The architecture estate still declares interval-overlap joins O(n)-decided; the shipped max-end index is recorded nowhere | incoherence | medium | overlap-join-live | fixed `6914471b` |
| [013](./findings/013-the-overlap-index-is-invisible-to-the-entire-obser.md) | The overlap index is invisible to the entire observability estate | observability | medium | overlap-join-live | fixed `b05046cc` |
| [014](./findings/014-exhume-never-reads-the-r18-dirty-marker-a-crashed-.md) | exhume never reads the R18 dirty marker: a crashed ephemeral store opens through the archival lane | incoherence | medium | storage-v7 | fixed `15342f00` |
| [015](./findings/015-a-failed-ephemeral-reopen-leaves-the-dirty-marker-.md) | A failed ephemeral open leaves the dirty marker armed over a cleanly-synced store | bug | medium | storage-v7 | fixed `15342f00` |
| [016](./findings/016-c18-dimension-mixing-unit-window-against-a-duratio.md) | C18 dimension mixing (unit window against a Duration bound) passes BOTH TS tiers | bug | medium | ts-surface-fresh | fixed `4239208e` |
| [017](./findings/017-naive-twin-s-capacity-witness-tie-break-order-dive.md) | Naive twin's capacity witness tie-break order diverges from the engine's for permuted or str-typed parent keys | bug | medium | lean-capacity-drift | fixed `808406eb` |
| [018](./findings/018-engine-s-write-time-capacityraymeasure-refusal-has.md) | Engine's write-time CapacityRayMeasure refusal has no verdict on the differential wall | incoherence | medium | lean-capacity-drift | fixed `b0fa69f0` |
| [019](./findings/019-capacity-lean-and-oracle-lean-docstrings-claim-an-.md) | Capacity.lean and Oracle.lean docstrings claim an engine ceiling early exit that C14 deleted | lean-rust-drift | medium | lean-capacity-drift | fixed `09e300a7` |
| [020](./findings/020-fresh-ratchet-sweep-s-exhaustion-exemption-is-keye.md) | Fresh-ratchet sweep's exhaustion exemption keys on max_fresh alone, masking regressed Q next-values | bug | medium | cross-branching-new | fixed `d3356737` |
| [021](./findings/021-scenarios-trace-rs-hand-rolls-the-cold-capture-and.md) | scenarios/trace.rs hand-rolls the cold capture and leaks the live capture on every error path | unification | medium | cross-branching-new | fixed `ea3d6792` |
| [022](./findings/022-the-span-containment-sweep-filter-start-end-sort-s.md) | The span containment sweep is duplicated verbatim between fold_stacks and FlameSummary::compute | unification | medium | cross-branching-new | fixed `ce8e12b8` |
| [023](./findings/023-keyed-get-traced-lane-ships-empty-engine-captures-.md) | Keyed-get traced lane ships empty engine captures: the snapshot point-read path is wholly dark | observability | medium | obs-estate | fixed `e6d6f877` |
| [024](./findings/024-flame-fold-summary-containment-inverts-parent-chil.md) | Flame fold/summary containment inverts parent/child on equal-tick span pairs | bug | medium | obs-estate | fixed `ce8e12b8` |
| [025](./findings/025-phasetimers-overflow-bucket-clobbers-the-open-desc.md) | PhaseTimers overflow bucket clobbers the open Descend stamp when the last two plan nodes both land in slot 8 | bug | medium | obs-estate | fixed `5b0a3a94` |
| [026](./findings/026-check-capacity-fetches-the-parent-f-row-eagerly-ev.md) | check_capacity fetches the parent F row eagerly even when ψ is empty and the bound is literal | perf | low | capacity-judge | fixed `2b1e87b0` |
| [027](./findings/027-delete-side-capacity-edges-derive-a-fallible-weigh.md) | Delete-side capacity edges derive a fallible weight that is never read — the one repair path for a corrupt ray row refuses | inelegance | low | capacity-judge | fixed `2b1e87b0` |
| [028](./findings/028-the-fresh-row-r16-8-byte-determinant-is-the-f-row-.md) | The fresh-row (R16) determinant-is-the-F-row-id probe is spelled three times in judgment.rs | unification | low | capacity-judge | fixed `2b1e87b0` |
| [029](./findings/029-verify-store-s-marks-pass-closed-parent-capacity-r.md) | verify_store's marks pass (closed-parent capacity re-check) has zero test coverage | incoherence | low | capacity-judge | fixed `2b1e87b0` |
| [030](./findings/030-dotted-dependent-bound-names-get-three-different-v.md) | Dotted dependent-bound names get three different verdicts across the three authoring walls | incoherence | low | capacity-surface | fixed `5eb216de` + `cb8af25f` |
| [031](./findings/031-pump-counts-zero-yield-batch-draws-that-its-line-p.md) | pump counts zero-yield batch draws its leaf twin never counts, skewing the batches observable | observability | low | gj-split-live | fixed `6c32f1f0` |
| [032](./findings/032-the-production-lowering-composition-fold-split-gj-.md) | The production lowering composition fold_split→gj_split is exercised by zero tests in its non-identity form | incoherence | low | gj-split-live | fixed `fa06d11b` |
| [033](./findings/033-40-execution-md-states-the-gj-split-fires-on-varia.md) | 40-execution.md's GJ-split condition says "different earlier nodes" — the split also fires at the probe's own node | incoherence | low | gj-split-live | fixed `6914471b` |
| [034](./findings/034-the-r18-wipe-the-storage-layer-s-only-destructive-.md) | The R18 crash-wipe is the storage layer's only silent lifecycle branch | observability | low | storage-v7 | fixed `15342f00` |
| [035](./findings/035-asserttermside-admits-param-only-comparisons-the-e.md) | assertTermSide admits param-only comparisons the engine convicts as ConstantComparison | incoherence | low | ts-surface-fresh | fixed `9241feb3` |
| [036](./findings/036-interval-measure-trusts-start-end-a-corrupted-inve.md) | interval_measure trusts start <= end: an inverted general interval tail underflows instead of convicting | bug | low | lean-capacity-drift | fixed `c9a78a67` |
| [037](./findings/037-validate-capacity-s-closed-both-sides-arm-re-imple.md) | validate_capacity's closed-both-sides arm re-implements the measure reading inline — already drifted on the ray arm | unification | low | lean-capacity-drift | fixed `d831280a` |
| [038](./findings/038-tick-granularity-ties-invert-parent-child-in-both-.md) | Tick-granularity ties invert parent/child in both containment sweeps | bug | low | cross-branching-new | fixed `ce8e12b8` |
| [039](./findings/039-the-commit-plan-derives-and-carries-the-capacity-s.md) | The commit plan derives the capacity slot weight for DELETE ops the applier never reads | perf | low | cross-branching-new | fixed `2b1e87b0` |
| [040](./findings/040-parse-bound-commits-to-the-duration-form-on-the-ba.md) | parse_bound commits to the Duration-measure form without peeking for the paren group | inappropriate-branching | low | cross-branching-new | fixed `5eb216de` |
| [041](./findings/041-scenario-cold-trace-path-leaks-a-live-capture-on-p.md) | Scenario cold-trace path leaks a live capture on prepare/execute error | bug | low | obs-estate | fixed `ea3d6792` |
| [042](./findings/042-join-phase-name-table-and-phase-node-cap-are-not-c.md) | JOIN_PHASE name table and PHASE_NODE_CAP are not compile-time pinned together | incoherence | low | obs-estate | fixed `915afe6e` |
| [043](./findings/043-emit-pair-claims-to-be-the-one-shared-traced-artif.md) | emit_pair claims to be the one shared traced-artifact fold, but read_family and half of cmd_trace hand-roll its body | unification | low | obs-estate | fixed `5d1f7e68` + `713c9b88` |
| [044](./findings/044-one-unregistered-phase-category-event-silently-sup.md) | One unregistered Phase-category event silently suppresses the entire phase table | inappropriate-branching | low | obs-estate | fixed `94ec57f4` |

## The perf campaign, phase by phase

### Review (2026-08-01): the untrusted Instrument estate, re-earned

The 2026-07-25 Instrument phase (`d7c111a8..ccd0de8d`, opus-authored) was
re-reviewed from scratch against its original lane specs under the v2 trust
ruling, and fixed where short (`21706061..1bc6ac25`): the flamegraph tooltip
percent recharged to the profile total, the golden folded→SVG selftest
became a gate, `flame.sh` gained the lane form, the dead traced_*/alloc
smoke tests joined check.sh, and four lanes that claimed to trace but did
not reach artifacts (crud, lawful, writes, capacity/windowed judgment)
learned to — plus the two dark interiors the review found still unlit
(validation, Allen dense scans).

### Gate: zero-cost-off proven

Release build without the trace feature through `scripts/check-asm.sh`, the
allocation gate, both cargo test batteries (default and `--features
trace`), `scripts/check.sh`, `scripts/lean.sh`, the ts suite. Verdict:
green — introspection stays a representation, not a mode.

### Baseline (2026-08-01, `bench-out/baseline-2026-07-25/`)

Bench debt first: **the C17 measurement** (below), the calendar capacity
lane, windowed/lawful re-pins under the capacity spelling (`e511b540`), the
campaign's writes/churn wall-power stragglers, and sweep-commit (the T8
pending lane: key-sorted probes 0.85x at 1k–4k touched parents,
`0e7d1d42`). Then the full suite — six report reps, scenarios, writes,
churn ×3, crud, curves, lawful, storage — strictly sequential on wall
power, oracle-gated (2889 cases, zero mismatches), flat-to-noise vs the
campaign except one flag: **the graph-world regression cluster**
(g6_weighted_hop 2.20, g3_three_hop_count 1.88, t1_stab 1.86,
g5_triangles_from 1.32). Then the attribution passes: every scenario query
warm+cold traced, 25 read families, every write/judgment lane, 147
flamegraphs, a separate alloc pass, and `TALLY.md` — per lane the number,
the delta, the top-5 span attribution, the alloc footprint, the flamegraph
path. THE document the Hunt read.

### Hunt: attribution readers + bug-bash finders

Four trace readers ranked mechanisms by (absolute µs × fixability) citing
span sources by file:line; nine finders swept everything written since the
2026-07 audit (capacity judge, capacity surface, GJ-split, overlap join,
storage v7, fresh TS surface, Lean capacity drift, a cross-cutting
branching sweep, and the new instrumentation itself). Every claim went to
an adversarial verifier; the 44 survivors above are what held.

### Fix (2026-08-02/03): seven file-disjoint lanes, every change with its test

- **obs/exec FIX lane** (`915afe6e..66fb0281`): findings F4/F5/F6 + the Gap
  B gather phase — the join pipeline's unattributed half got a name
  (`c07907e6`) — plus the P1 closing-probe copy diet and P3 batched
  membership probes.
- **sink/plan lane** (`02024e3c..a75d1e65`): generation-stamped seen-set
  clear, COUNT-shaped dedup without a survivor list, fold_split prefix
  routing (finding 009), the pinned-run leaf fold, Pack's start-word
  finalize sort.
- **OVERLAP lane** (`b2735c55..8d17ac59`): abutment components join the
  index window, const-side residual conjoin (finding 011), amortization
  gating + the flat sweep (finding 010), phase attribution (finding 013),
  the docs re-true (findings 012/033).
- **Items 1–6** (`3e7babbf..e6d6f877`): the six attribution-ranked reads
  mechanisms — shared bound-trie clones, image distinct counts, the
  persistent finalize intern→text cache, the pooled keyed-get lane
  (finding 023), the per-slot param-word memo, the fold_split→gj_split
  composition pin (finding 032).
- **storage lane** (`136f73eb..128e4504`): applier folds, flat delta
  staging, the T8 shared-cursor walk, the R18 lifecycle package
  (findings 003/014/015/034).
- **bench tooling** (`ce8e12b8..7a56a186`): the containment-sweep
  unification (findings 022/024/038), phase-fold synthetic frames,
  unregistered-event row drop (finding 044), emit_pair unification
  (finding 043), the traced warm draw as median-cost param set.
- **R13/theory lane** (`9241feb3..5eb216de`): the TS pairwise comparison
  tier (findings 004/035), explain symmetry (finding 005), the dotted
  dependent-bound single verdict (findings 030/040).

Gate round 2 after the lanes: `7231d2e8` (ts), `d37968c5` (fmt),
`cb8af25f` + `3b31cd84` (clippy -D warnings) — all green.

### Rebench (2026-08-03, `bench-out/baseline-2026-07-25-post/`)

Exactly the targeted lanes rerun at `3b31cd84` under the exact baseline
protocol (wall power pmset-asserted per lane, `scripts/measure.sh` mutex,
`BUMBLEDB_BENCH_BOOST=1`, the oracle re-earned per binary — three times,
because the verify stamp is binary-fingerprint-keyed). THE document:
[`bench-out/baseline-2026-07-25-post/DELTA.md`](../bench-out/baseline-2026-07-25-post/DELTA.md);
raw per-cell tables in `delta-tables.md`; 22 differential flamegraphs under
`bench-out/baseline-2026-07-25-post/flame/` (red = grew).

**CASHED** (post/baseline ratio, <1 = the fixes paid):

- The read estate: reps 0.87–0.94 on five of six (e2's 1.08 is two sub-µs
  cells); scenarios 0.88 overall, the whole lane's wall time halved
  (16m49s → 7m11s).
- The baseline's Hunt cluster fully reversed: g6_weighted_hop 0.107→0.035
  (`flame/scenarios.graph.g6_weighted_hop.warm.diff.svg` — the join body
  collapsed), t1_stab 0.202→0.085
  (`flame/scenarios.temporal.t1_stab.warm.diff.svg`), g3_three_hop_count
  0.068→0.042 (the COUNT fold dies in the leaf now), g5_triangles_from
  0.046→0.026.
- j3_keyword_kind 0.35 — the 56 µs finalize evicted by the intern cache
  (`flame/scenarios.joins.j3_keyword_kind.warm.diff.svg`); t2/r4 our-side
  walls down 19%/6%; containment_walk 0.17–0.26 and postings_without_tag
  0.34–0.38 (the P3 membership batching,
  `flame/postings_without_tag.warm.diff.svg`).
- The obs FIX lane's jp_* phase table (gather/descend/probe/iter/residual)
  now attributes the join's former dark half — every attribution row in
  DELTA.md reads it, and zero-cost-off held (the timed suite improved, not
  paid).

**NOT CASHED / REGRESSED**, called as measured:

- **The storage lane is NOT CASHED net**: writes suite 1.10 (nosync commit
  ladder 1.24–1.44, `flame/writes.nosync.commit_b100.diff.svg` —
  judgment_source +70% at b100, the T8 walk paying where the sweep only
  priced 1k–4k parents), durable delete_b100 1.15 with apply_deletes
  self-time 2.1× (`flame/writes.durable.delete_b100.diff.svg`),
  **cold_containment_walk_delete 3.1–3.4× in all six reps**, windowed
  ephemeral 1.07–1.17. Its stated targets did not move (crud 1.02 flat).
- **r6_two_path_count 1.46** on the sink/plan lane's own COUNT territory —
  descend now carries essentially the whole query
  (`flame/scenarios.rings.r6_two_path_count.warm.diff.svg`).

These are the campaign's standing Hunt items, recorded in TODO.md:
attribution first, no intuition fixes.

## The C17 resolution (slot vs fetch)

The 0.8.0 close owed a measured choice for the weighted-capacity judge's
`measure_children`: empty `R` values + one child F get per walked edge
(fetch) vs the child's u64 weight LE carried in the R slot, paid at write
time (slot). Both arms were built behind `CAPACITY_WEIGHT_SLOT`, each
oracle-gated independently (full capacity battery + 13 capacity/3 windowed
differential suites + a per-binary verify stamp over 2889 cases), then run
on the power-budget lane on wall power: **the slot arm won every weighted
row in every rep** — judged surface −17% (sum) and −21% (duration) on the
discriminating ephemeral lane, direction agreeing on the fsync-shadowed
durable lane. Landed `484c3871`: the value slot is the only form, the fetch
arm and the flag are DELETED, the numbers are the CONSTRAINT comment at the
walk (`crates/bumbledb/src/storage/commit/judgment.rs`). Artifacts:
`bench-out/baseline-2026-07-25/capacity-c17/SUMMARY.md` (eight cells, two
oracle-stamped arms). One owner ruling was surfaced, not ruled: the slot
arm refuses a ray-valued Duration weight at WRITE time — strictly stronger
than C10's judge-time refusal.

## Observability upgrades shipped

All zero-cost-off (trace-feature ZSTs; no per-tuple labels, no always-on
counters, no diagnostics allocation in the join loops — the 40-execution
doctrine held at both gates):

- **Every bench lane traces**: scenarios `--trace` (per-query warm+cold
  Chrome traces + flame embeds), crud/lawful/writes/judgment lanes reach
  per-lane JUDGMENT_*/LMDB_COMMIT artifacts, every traced artifact lands a
  `.folded` twin; timed batches stay untraced (the measure.rs discipline).
- **The dark subsystems are lit** at batch/pass granularity: DP planner
  interior, selectivity ladder, columnar batch decode, predicate-scan
  kernels, verify_store sweep, normalization sub-passes, validation
  interior, Allen dense scans, the overlap index (finding 013), the
  snapshot point-read surface (finding 023), the R18 wipe (finding 034).
- **Flamegraphs are one command**: `scripts/flame.sh <lane> [query]` —
  self-contained SVG renderer, no external flamegraph.pl, top-10 self-time
  table, golden selftest gated; `scripts/flamediff.sh <a> <b>` — the
  red/blue differential SVG the rebench's 22 diffs are drawn with.
- **The join pipeline attributes whole**: the jp_* per-(node,phase) table
  gained the Gather phase (Gap B — the former dark half), the overflow
  bucket merges instead of clobbering (finding 025), zero-yield draws are
  not batches (finding 031), the phase-name table is compile-time pinned
  (finding 042), an unregistered event drops its row, never the table
  (finding 044), and equal-tick containment no longer inverts parent/child
  (findings 024/038).

## Where the numbers live

- Baseline: `bench-out/baseline-2026-07-25/` (`SUMMARY.md`, `TALLY.md`,
  `MANIFEST.txt`, 147 flamegraphs under `flame/`).
- Rebench: `bench-out/baseline-2026-07-25-post/` (`DELTA.md`,
  `delta-tables.md`, `MANIFEST.txt`, 22 flamediffs under `flame/`,
  `run-post-suite.sh` — the driver).
- The README headline graphs regenerate from the post run
  (`scripts/bench_viz.py --night bench-out/baseline-2026-07-25-post`).
