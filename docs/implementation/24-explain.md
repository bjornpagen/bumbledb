# PRD 24 — EXPLAIN

Authority: `docs/architecture/30-execution.md` (observability — monomorphized
Counters, ANALYZE semantics, no release-path instrumentation).

## Purpose

The debugging surface: an instrumented execution of the same plan, via the `Counters`
seam — never a runtime mode.

## Technical direction

- `exec::explain`. `CountingCounters` implementing PRD 19's trait: per-node entry
  counts, per-node cover-choice histogram (chosen subatom × Exact/Estimate label),
  probe/hit counts per subatom, residual pass/fail counts, sink emits, skip counts.
  Fixed-size arrays indexed by (node, subatom) — sized from the plan, arena-allocated
  once; **counter methods are plain increments, no formatting, no allocation**.
- `explain(prepared, txn, params) -> Explain` executes the query with
  `CountingCounters` (ANALYZE semantics — it runs the real thing, including sinks;
  results are discarded or returned alongside, implementer's choice — return them:
  `Explain { rows, report }`).
- `Report`: plan rendering (nodes, subatoms, covers, residual placement, trie
  schemas), per-step planner estimates (PRD 16 retained them) vs measured
  cardinalities, guard-probe classification when applicable. `Display` impl producing
  the human text — format is OPEN per README; keep it plain and stable-ish, no
  self-congratulatory narrative (post-mortem §32).
- The normal execution path continues to instantiate `NoopCounters`; verify zero cost
  by the absence of any counter state in the prepared query's normal-path types (type
  system proof, not measurement).

## Non-goals

Tracing/span systems. Persistent profiles. Per-entry (unaggregated) cover logs.

## Passing criteria

- Unit tests: estimates vs actuals populate for a join fixture; the skew fixture from
  PRD 19 shows the expected cover choice in the histogram (the correct-but-slow
  regression detector the validation doc names); guard-probe queries report their
  classification; Display renders without panicking on every fixture; a compile-time
  assertion (type test) that `NoopCounters` is zero-sized and the normal path carries
  no counter fields.
- Global commands green.
