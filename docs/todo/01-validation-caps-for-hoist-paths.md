# 01 — Validation caps for the hoist paths (reachable, data-dependent panics)

**Kind:** correctness — reachable panic from a valid query. **Severity: highest in
this folder.** Violates the declaration law ("rejected at declaration, never
discovered at write time", `30-dependencies.md` acceptance gate — the same law
applies to query admission).

## Context

Two executor fast paths use fixed-size scratch arrays sized 8, guarded by runtime
`assert!`s, but nothing at the validation boundary enforces the corresponding caps.
Validation caps distinct variables at 128 and occurrences at 20
(`ir/validate` roster; `MAX_DISTINCT_VARS`, `MAX_OCCURRENCES`) — projected **word
count** and **per-leaf residual count** are unbounded.

## Current behavior

1. **Wide projection over a scannable leaf.** The scan-pushdown hoist in the
   projection sink uses `[ColumnView; 8]` with
   `assert!(sources.len() <= 8, "projection arity cap")`
   (`crates/bumbledb/src/exec/sink/projection/sink.rs:102`). `sources.len()` equals
   the projected word count (`new.rs:20` sets `arity = slots.len()`), which is
   unbounded. `begin_scan` returns `true` unconditionally (`projection/sink.rs:74`),
   so the hoist path is taken whenever an unforced suffix leaf has a run
   `>= SCAN_HOIST_THRESHOLD` (8). **Data-dependent:** a query projecting >8 words
   runs fine while every leaf run is <8 (the safe per-position arm at
   `projection/sink.rs:126`), then panics the first time a run reaches 8 — i.e. it
   works in tests on small data and dies later on real data.
2. **Leaf residual cap.** `assert!(self.leaf_scan_residuals.len() <= MAX_LEAF_RESIDUALS)`
   with `MAX_LEAF_RESIDUALS = 8` (`crates/bumbledb/src/exec/run/scan_table.rs:68-71`,
   constant at `exec/run.rs:329`) fires only on the same hoist condition. No
   validation cap bounds how many column-touching residuals can attach to a leaf
   node.

Interval fields count two words per projected field, so "8 words" is as few as
four projected interval fields — this is reachable by an ordinary ledger query, not
an adversarial one.

## The work

Pick one per cap, and the choice is per-cap:

- **Projection width:** either (a) add a roster item at the validation boundary —
  `MAX_PROJECTED_WORDS` with a typed `ValidationError` variant, documented next to
  `MAX_DISTINCT_VARS` — or (b) make the hoist ineligible when `sources.len() > 8`
  (fall through to the per-position arm, which is correct for any arity). Option (b)
  is preferable if wide projections are legitimate: the cap becomes a fast-path
  eligibility condition instead of a query rejection, and no user-visible limit is
  born. Either way the `assert!` becomes unreachable-by-construction and should be
  demoted to a `debug_assert!` with a comment naming the guard.
- **Leaf residuals:** same decision shape. If >8 residuals on one leaf is a real
  plan shape, gate the hoist eligibility; if it provably cannot happen after some
  invariant, make that invariant explicit at plan validation (`plan/fj/validate.rs`)
  and cite it at the assert.

## Acceptance

- A regression test: a schema/query pair projecting >8 words over a relation with a
  leaf run ≥8 executes correctly (or is rejected at prepare with the typed error,
  per the option chosen). Same for a >8-residual leaf if constructible.
- The randomized query generator (`bumbledb-bench`) gains coverage: projected-word
  count drawn past 8, so the differential oracle would have caught this class.
- No `assert!` on a data-dependent condition remains in the executor hot path.

## Doc amendments (rule 5)

`40-execution.md` — the scan-fold pushdown paragraph gains the eligibility
condition (or `20-query-ir.md` validation roster gains the cap, per choice).
