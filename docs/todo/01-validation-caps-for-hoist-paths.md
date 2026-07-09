# 01 — Hoist-path eligibility (reachable, data-dependent panics)

**Kind:** correctness — reachable panic from a valid query. **Severity: highest in
this folder.** **Decided:** eligibility-condition fix, both caps; no validation cap
is born.

## Context

Two executor fast paths use fixed-size scratch arrays sized 8, guarded by runtime
`assert!`s, with nothing making the guarded state unreachable. A cap at the
validation boundary was considered and **rejected**: `MAX_DISTINCT_VARS` and
`MAX_OCCURRENCES` protect *representations* (bitset widths, DP table size); a
projection-width cap would protect one optimization's scratch array — an
implementation constant leaking into the user contract.

## Current behavior

1. **Wide projection over a scannable leaf.** The scan-pushdown hoist in the
   projection sink uses `[ColumnView; 8]` with
   `assert!(sources.len() <= 8, "projection arity cap")`
   (`crates/bumbledb/src/exec/sink/projection/sink.rs:102`). `sources.len()` equals
   the projected word count (`new.rs:20`), which is unbounded. **The root defect is
   one altitude up:** `begin_scan` returns `true` unconditionally
   (`projection/sink.rs:74`) — an eligibility function with no eligibility
   condition. **Data-dependent:** a query projecting >8 words runs fine while every
   leaf run is < `SCAN_HOIST_THRESHOLD` (8), then panics the first time a run
   reaches 8 — works in tests on small data, dies later on real data. Interval
   fields count two words each, so four projected interval fields suffice: an
   ordinary ledger query, not an adversarial one.
2. **Leaf residual cap.** `assert!(self.leaf_scan_residuals.len() <= MAX_LEAF_RESIDUALS)`
   with `MAX_LEAF_RESIDUALS = 8` (`crates/bumbledb/src/exec/run/scan_table.rs:68-71`,
   constant at `exec/run.rs:329`) — same shape: nothing bounds column-touching
   residuals per leaf; the assert fires on the same hoist condition.

## The work

- **Fix at the eligibility altitude.** `begin_scan` (and the leaf-residual hoist's
  eligibility site) gains the condition: hoist iff the width fits the scratch
  constant; otherwise fall through to the always-correct per-position arm. One
  constant per cap, shared by the scratch array size and the eligibility test —
  a single source of truth, so the two cannot drift.
- The `assert!`s become `debug_assert!`s with comments naming the eligibility
  guard that makes them unreachable.
- If fall-through proves impossible for the leaf-residual case (no correct slow
  arm exists), make the bounding invariant explicit at plan validation
  (`plan/fj/validate.rs`) and cite it at the assert — and say so in the commit.

## Acceptance

- Regression tests: a schema/query pair projecting >8 words over a relation with a
  leaf run ≥8 executes correctly; same for a >8-residual leaf if constructible.
- The randomized query generator (`bumbledb-bench` querygen) draws projected-word
  counts past 8, so the differential oracle covers this class permanently.
- No `assert!` on a data-dependent condition remains in the executor hot path
  (grep-verifiable).

## Doc amendments (rule 5)

`40-execution.md` — the scan-fold pushdown paragraph gains the eligibility
condition.
