# TODO — the plan of record

## Open

- **The 1.0.0 close (owner-gated, explicitly deferred 2026-07-18)** — R2 of
  `docs/structural-1.0.0/`: crate version `1.0.0` + the annotated `v1.0.0`
  tag. Owner ceremony only; no agent bumps, tags, or publishes.
- **PR #10 (incremental images)** — complete on its
  branch, gates green, measured (copy-on-append 2.54× on the cold lineage
  family; the mask fork refuted by the decider twin). Held open by owner
  order; merge is the owner's call. The merge inherits cleanup-0.5.0
  obligations: its per-kind ceiling split (durable 32 GiB / ephemeral
  4 GiB) is superseded by ruling 1's ONE lazy 32 GiB map — reconcile at
  merge; the `lineage-off` A/B knob and its bench twin die in the merge
  commit (the gravestone in `crates/bumbledb/Cargo.toml`); the aborted
  scan≡scan_from kill (U2 — the site is a PR #10 artifact, absent here)
  re-runs on the merged tree; the incremental-images packet dies at merge
  (its ruling record is durable in `50-storage.md` § eviction and
  `40-execution.md` D1); its README re-true and waveM `report.json` land
  with it.
- **The Measure phase (cleanup-0.5.0 prd-M) — one lane still owed:** a
  `NOSYNC`-only ≥2,000-round statistical kill session (the recorded ones
  are 2026-07-16, WRITEMAP-era; the deterministic sweep and the kill
  smoke re-ran green at the flip). Everything else RULED 2026-07-19,
  committed run dirs `bench-out/measure-twins/` +
  `bench-out/measure-ephemeral-r6/` + `bench-out/eph-nosync-{1,2,3}`:
  twins — leaf elision LAW (1.69–1.71×), permuted-identity determinant
  LAW (1.23–1.25×), all-words finalize MERGED (0.996–1.005, `AnswerHeap`
  and both word fills deleted, oracle re-refereed on a fresh 2,862-case
  stamp); ephemeral re-earn — README 21.2× over all 22 (ALL-WIN ×3),
  R6 band 43–70x staging / 27–52x ssd dividend / 3.1–3.5x ramdisk
  dividend / 1.1–1.6x device tax; every planted re-earn mark is closed
  (prd-M's close-out grep comes back empty).
- **CI dispatch proof (U4a) — remedies landed, run ids still owed:** the
  miri `.S` stub and the ubuntu lanes want their green run ids recorded
  in PR #11. The only PR run (29697582864) tested `de1bac14` and is green
  everywhere EXCEPT `check (ubuntu-latest)`:
  `clockproxy::tests::the_estimate_is_a_plausible_core_frequency`
  measured 0.18 GHz on the shared runner — U4a's named watch item fired.
  Remedy landed (2026-07-19, owner-visible here): the probe joined the
  host-pinned falsifier set with an arch-conditioned ignore — the
  plausibility band transcribes the aarch64 asm chain's by-construction
  cycle count, and the portable fallback is documented indicative-only —
  the band itself is untouched (no widen), and the macos lane still runs
  the probe. The run gap is DIAGNOSED: PR #11 is CONFLICTING against
  main, GitHub builds no merge ref for a conflicted PR and creates no
  runs for its synchronize events — hence zero runs for the seven
  commits after `de1bac14` (one consequence already surfaced: U5's
  `conformance/judgment.rs` landed with a rustfmt violation no gate ever
  saw; fixed in the U6 working tree). Remedy landed: ci.yml's push
  trigger lists `worktree-cleanup-050` explicitly (drop at PR close);
  pull_request runs stay unavailable until the committer reconciles the
  branch with main. Remaining act (serial committer): push, then
  `gh workflow run ci --ref worktree-cleanup-050` (workflow_dispatch
  runs ALL lanes including miri), record the green run ids in PR #11.
- **Ruling 13 — the 0.5.0 version bump is PENDING, deliberately last:**
  `ts/package.json` + `ts/npm/darwin-arm64/package.json` sit at 0.4.0
  and no crate bumps are staged. U4 item 6 sequences the bump LAST in
  the wave because it pins a platform package that does not exist on npm
  yet — the sdk lane is expected-red between bump and owner publish — so
  it cannot precede the green-dispatch proof above. Stage it (lockstep
  gate green, no tag, no publish, say "last in the wave" in the commit)
  once the run ids are recorded; then the post-publish lockfile
  regeneration commit per the release-flow note below.
- **For the owner (U4a census):** five stale remote worktree branches
  deleted; `worktree-structural-sdk` was closed-not-merged but proven
  subsumed — flagged per census.
- **Optional, unscheduled:** a fresh one-rev seven-run bench session would
  restore min-of-3 durable sampling and re-clean `mandate_overlap` (excluded
  from the current pin as contaminated-in-both). The current README numbers
  are fully derivable from the committed artifacts and need nothing.
- **Release-flow note (recurs every version):** the version-bump commit
  pins the exact platform optional-dep before the package exists, so the
  CI sdk lane's `--frozen-lockfile` fails between bump and publish. The
  post-publish step is a lockfile regeneration commit
  (`cd ts && pnpm install --no-frozen-lockfile`).

## Everything else: shipped

**Cleanup-0.5.0 is landed on `worktree-cleanup-050` (PR #11, stays open;
nobody merges):** ruling 1 (one lazy 32 GiB map; WRITEMAP and the eager
capacity contract retired, retractions recorded at `MAP_SIZE` and in
`50-storage.md`), the engine kills (U2: cfg duals into type twins), the
SDK kills + wire tags (U3), CI reshaped (U4a: main+PR scope, ubuntu
matrix, miri cron stub) and the FFI lint regime + re-trued unsafe
allowlist (U4b), lean reconciliation (U5: 26 judgment cases / 272
total), and the architecture docs swept to the tree's present tense
(U6). The packet at `docs/prds/cleanup-0.5.0/` is deletion-eligible at
wave close per its own survival checklist (serial committer's act).

`@bjornpagen/bumbledb@0.4.0` (+ `-darwin-arm64@0.4.0`) is published and
tagged `v0.4.0` — the host-idiom SDK on the law-typed 0.3.0 core; primer is
cut over and merged (PR #85). **The bench pin is healed (2026-07-19):** the
README's read-family numbers (18.7× durable over clean min-of-2 with
`mandate_overlap` excluded-and-counted at rev `adac4010` 2026-07-16; 21.2×
ephemeral over all 22, ALL-WIN ×3, re-earned `NOSYNC`-only 2026-07-19 on the
post-cleanup tree) derive from the committed `bench-out/` artifacts, charts
regenerated from the durable runs; the orphaned
mixed-rev run1 is deleted; the tails sentence names its one honest exception
(`meets_chain` p99). The shipped packets live at their tags. History lives
in git; this document is not an archive.
