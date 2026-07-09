# 08 — Plan staleness signal (host-owned, zero hot-path cost)

**Kind:** host-facing API — the Postgres operational lesson applied without
importing its machinery.

## Context

Plans pin their statistics at prepare time and are never invalidated
(`20-query-ir.md`; `api/prepared.rs:8-12`) — correct for the write design point
(≥100 executions per generation) and correctness-safe (stale plans point at
current data via generational rebinding; only optimality drifts). The gap:
**nothing tells the host optimality has drifted.** Postgres's most common
production planner failure is not a bad estimator — it is a good plan pinned
against data that grew past its assumptions, discovered months later as a latency
incident. Postgres answers with invalidation machinery and background ANALYZE;
both are deleted vocabulary here (no threads, no engine-owned policy). The
transferable part is only the *signal*; re-preparation stays an explicit host act.

## The work

Two independent, cheap surfaces; both are reads of state that already exists:

1. **Prepare-time pin record.** `PreparedQuery` records, per occurrence, the row
   count (and measured survivor count where views were built) the plan was costed
   with — data the builder already holds at `api/prepared/build.rs` and then
   drops.
2. **`PreparedQuery::staleness(&snapshot) -> Staleness`** — compares pinned counts
   against the snapshot's live `S` counters (`read::row_count` is an O(1) LMDB
   get per occurrence; ≤20 occurrences by the roster cap). Returns per-occurrence
   drift ratios and a max. **Never called by the engine** — the host decides when
   to ask (per generation, per N executions, on a timer it owns) and what ratio
   means "re-prepare". No thresholds ship in the engine; a suggested convention
   (≥4×, the worst measured est/actual being 3.3× — `40-execution.md` measured
   mechanisms) is documentation, not code.
3. Fold the same numbers into the EXPLAIN/stats surface (`api/stats.rs`): the
   existing est-vs-actual report gains "estimated *from* (pinned rows at prepare)"
   so a drifted plan is visible in one read.

Explicitly rejected shapes, for the record: re-planning on execute (hot-path cost
plus plan instability); engine-side thresholds (policy belongs to the host —
`00-product.md` doctrine); any background anything.

**Decision (to record):** staleness is a pull-based host query. **Alternative:**
generation-count-based auto-replan inside `execute`. **Why it lost:** a warm
execution must not pay even a branch for a policy question, and silent replans
make performance non-reproducible mid-process. **Reverses if:** never — doctrine.

## Acceptance

- Unit test: prepare at N rows, commit until 4N, `staleness` reports the ratio;
  re-prepare resets it.
- Zero measurable change to warm execution (the gate suite unchanged — staleness
  is outside the measured window).
- The allocation contract is untouched (`staleness` may allocate; it is not a
  warm-path call).

## Doc amendments (rule 5)

`20-query-ir.md`'s pin-at-prepare decision paragraph gains the signal as its
compensating control; `70-api.md` documents `staleness` and the suggested
convention.
