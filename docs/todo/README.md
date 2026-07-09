# TODO — review findings and Postgres-transfer rocks

Product of a full-repo review (2026-07-09, four independent subsystem readings
plus a comparative study of Postgres planner/storage internals against this
engine's axioms), then a second-pass audit that pinned every open option to a
decision, discharged the doc-only items, and added the elegance pass. Each item is
self-contained: context, current behavior with citations, the decided work,
acceptance criteria, and the rule-5 doc amendments. Line numbers are as-of the
review; verify before relying on them.

Items are **ordered**: correctness first, then contract holes, then hardening,
then the feature transfers, then the sweep and the refactor. Work top-down. A
finished item is deleted from this folder in its landing commit.

Two sequencing constraints beyond the ordering: **10 runs after 02** (G's test
uses the applied-inserts machinery 02 introduces), and **07 is the only item with
real regression risk** — the full two-oracle verify runs green immediately after
it lands, before anything stacks on top. The whole pass ends with re-earned
benchmarks (hot paths move) and regenerated charts.

| # | Item | Kind |
|---|---|---|
| 01 | [Hoist-path eligibility](01-validation-caps-for-hoist-paths.md) | reachable panic |
| 02 | [Oracle direction divergence](02-oracle-direction-divergence.md) | verify-red latent |
| 03 | [`alloc_dyn` typed error](03-alloc-dyn-typed-error.md) | ETL-surface panic |
| 04 | [Zero-alloc gate: high-water contract](04-zero-alloc-gate-warm-growth.md) | contract hole |
| 05 | [Storage hardening](05-storage-hardening.md) | reopen-trust + reader cap |
| 06 | [`Db::verify_store` — the offline sweeper](06-verify-store-sweeper.md) | coherence tooling |
| 07 | [Dependency-driven join elimination](07-dependency-join-elimination.md) | planner feature |
| 08 | [Plan staleness signal](08-plan-staleness-signal.md) | host-facing API |
| 10 | [Minor findings sweep](10-minor-findings-sweep.md) | batch of small items |
| 11 | [The elegance pass](11-elegance-pass.md) | rebuild-seam refactor |

Discharged during the audit (resolutions live in the architecture docs, not
here): the skip-scan note (was 09 — now on the range/stabbing OPEN item and in
`40-execution.md`), and the dictionary-leak honesty amendment (was 10-F — the
accepted-leak sentence in `10-data-model.md` and the GC trigger metric now count
never-referenced ids; filtering was rejected as machinery against an accepted
leak).

Review provenance: everything each reading checked and found **sound** is
recorded in item 10's "verified sound" notes, so nobody re-audits it by accident.
