# TODO — review findings and Postgres-transfer rocks

Product of a full-repo review (2026-07-09, working tree at `ae4c2af` + in-flight
changes) plus a comparative study of Postgres planner/storage internals against this
engine's axioms. Each item is self-contained: context, current behavior with
citations, the work, acceptance criteria, and the doc amendments rule 5 requires.
Line numbers are as-of the review; verify before relying on them.

Items are **ordered**: correctness first, then contract holes, then hardening, then
the feature transfers, then doc-only notes and a minor sweep. Work top-down.

| # | Item | Kind |
|---|---|---|
| 01 | [Validation caps for hoist paths](01-validation-caps-for-hoist-paths.md) | reachable panic |
| 02 | [Oracle direction divergence](02-oracle-direction-divergence.md) | verify-red latent |
| 03 | [`alloc_dyn` typed error](03-alloc-dyn-typed-error.md) | ETL-surface panic |
| 04 | [Zero-alloc gate: warm growth](04-zero-alloc-gate-warm-growth.md) | contract hole |
| 05 | [Storage hardening](05-storage-hardening.md) | reopen-trust + reader cap |
| 06 | [`verify_store` — the offline sweeper](06-verify-store-sweeper.md) | coherence tooling |
| 07 | [Dependency-driven join elimination](07-dependency-join-elimination.md) | planner feature |
| 08 | [Plan staleness signal](08-plan-staleness-signal.md) | host-facing API |
| 09 | [Skip-scan note on the range-accelerator OPEN item](09-open-item-skip-scan-note.md) | doc-only |
| 10 | [Minor findings sweep](10-minor-findings-sweep.md) | batch of small items |

Review provenance: four independent subsystem readings (storage, executor,
planner/IR, API/commit/oracle). Everything each reading checked and found **sound**
is recorded in the items' "verified sound" notes where load-bearing, so nobody
re-audits it by accident.
