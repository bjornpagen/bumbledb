# TODO — the plan of record

## Open

- **The 1.0.0 close (owner-gated, explicitly deferred 2026-07-18)** — crate
  version `1.0.0` + the annotated `v1.0.0` tag. Owner ceremony only; no agent
  bumps, tags, or publishes. Planned fresh when the owner calls it (the old
  packet was deleted 2026-07-20 with every completed PRD packet — its C2 fuzz
  hunt was mooted by the fuzzer deletion; history in git).
- **The crashpoint + image-oracle disposition (owner call, flagged
  2026-07-20):** the `crashpoint` and `image-oracle` test-support features
  lost their only consumers when the fuzzing apparatus was hard-deleted
  (`crates/bumbledb/Cargo.toml` records both as currently
  consumer-less; `docs/architecture/60-validation.md` § the deletion
  record). They still compile in every gate (`check.sh`'s
  `--all-features` co-compile lane) but no lane executes them. Keep as
  dormant instruments or delete — an owner ruling, not yet made; until
  it is, they stay.
- **The campaign's owed bench lanes** (`bench-out/campaign-2026-07-23/
  MANIFEST.txt`, 11 of 14 landable lanes COMPLETE): the wall-power
  reruns of **writes** and **churn** (their battery-era RUN 1 outputs
  retired whole at `f474202a`; the README's ladder and churn sections
  ride the committed night pins and say so), **sweep-commit** (needs the
  obs build), and the unlanded `adversarial` subcommand (the chart
  derives from scenarios' capped lanes meanwhile). All six suite reps
  are landed — min-of-3 per store kind, the like-for-like NOSYNC
  ephemeral pairing (never min-merge them with the night's ephemeral
  rows; the merge refuses).
- **The audit's three deferred findings** (stamps in
  `audit-2026-07/findings/`): 014 (the leaf still runs per parent —
  batch-of-1 `run_node` on fanout-1 lookups), 044 (the forced-map
  telescoped distinct Count; r6 honestly flat), 053 (the two
  FilterPredicate interpreters, view vs key-probe). Plus the 009 step-2
  per-forced-map min/max key fence (COLT force time) and the R5 tail:
  the TS surface cannot yet utter the measure-keyed Arg and the Lean
  denotation keeps the conformance fence (RULINGS.md § R5).

## Everything else: shipped

**The 2026-07 deep audit closed at campaign end (2026-07-24):** 162
findings — 158 fixed, 3 deferred with stamped reasons, 1 superseded by
ruling (089 → R19); every finding carries an `outcome:` stamp and the
tally rides `audit-2026-07/README.md`. The 22 rulings are statused in
`audit-2026-07/RULINGS.md` (21 IMPLEMENTED, R5 PARTIAL with its owed TS
+ Lean tail). The R20 corpus regeneration re-ran every published number
on wall power (the battery-era RUN 1 retired whole at `f474202a`;
`bench-out/campaign-2026-07-23/SUMMARY.md`: scenarios geomean
0.0835 → 0.0554, reads 21.2× gated / 24.8× all-33 durable min-of-3,
crud loss at 0.59×, `closure_fanout`'s 30× honestly down to 13.3× —
the SQLite twin is that family's volatile side), and R21 re-pinned
every doc citation + regenerated every README graph from the campaign
artifacts (`4de40efd`, re-trued whole against the wall-power estate at
campaign close).

**The primer 0.6.0 cutover landed with the destructure run (2026-07-20):**
paradigm C is live in both repos, the primer estate is cleaned, and the
worktrees are gone (the 0.6.0 run record; primer #94 merged).

**The 0.6.0 destructure release is published and tagged `v0.6.0`
(2026-07-20):** vars become values (`v(relation)` mints class-typed variable
records; identity is object reference — reuse IS the join), `select` died
into `find({...})`, `r.var` is dead with no shim; zero fingerprint pins moved
(`ts/test/fixtures/cookbook-fingerprints.txt` byte-identical).
`@bjornpagen/bumbledb@0.6.0` + `-darwin-arm64@0.6.0` are in the registry and
the post-publish lockfile regeneration landed (4b2b3a0c), closing the
documented bootstrap gap (the recurring gap and its remedy live in
`ts/PUBLISHING.md` § post-publish, step one).

**Cleanup-0.5.0 landed via PR #11 (merged):** ruling 1 (one lazy 32 GiB map;
WRITEMAP and the eager capacity contract retired, retractions recorded at
`MAP_SIZE` and in `50-storage.md`), the engine kills (U2: cfg duals into type
twins), the SDK kills + wire tags (U3), CI reshaped (U4a: main+PR scope,
ubuntu matrix, miri cron stub) and the FFI lint regime + re-trued unsafe
allowlist (U4b), lean reconciliation (U5: 26 judgment cases / 272 total), and
the architecture docs swept to the tree's present tense (U6). Its Measure
phase closed RULED 2026-07-19 (run dirs under `bench-out/`; the one owed
`NOSYNC` statistical kill lane is moot — the kill harness died with the
fuzzing apparatus). PR #10 (incremental images, copy-on-append 2.54×) merged
and reconciled 2026-07-19 with every inherited obligation executed. The
cleanup packet was deleted at wave close per its own survival checklist (all
PRD packets removed 2026-07-20; history in git).

`@bjornpagen/bumbledb@0.5.0` (+ `-darwin-arm64@0.5.0`) is published and
tagged `v0.5.0` — the surface-pair SDK (keyed get + host-side ordering, the
plural mint removed) on the 0.3.0 law-typed core; the post-publish lockfile
regeneration landed (81ceb89b) and primer main is cut over (`^0.5.0`).
**The bench pin is healed (2026-07-19):** the
README's read-family numbers (18.7× durable over clean min-of-2 with
`mandate_overlap` excluded-and-counted at rev `adac4010` 2026-07-16; 21.2×
ephemeral over all 22, ALL-WIN ×3, re-earned `NOSYNC`-only 2026-07-19 on the
post-cleanup tree) derive from the committed `bench-out/` artifacts, charts
regenerated from the durable runs; the orphaned
mixed-rev run1 is deleted; the tails sentence names its one honest exception
(`meets_chain` p99). The shipped packets live at their tags. History lives
in git; this document is not an archive.
