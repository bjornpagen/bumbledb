# PRD 01 — Hoist-path scratch: delete the caps, not guard them

**Depends on:** nothing. **Severity: highest in the set** — a valid query panics,
data-dependently: it works while every leaf run is short and dies the first time
a run reaches the hoist threshold, i.e. passes tests on small data and crashes on
real data.
**Modules:** `crates/bumbledb/src/exec/sink/projection/sink.rs`,
`crates/bumbledb/src/exec/run/scan_table.rs`, `crates/bumbledb/src/exec/run.rs`,
`crates/bumbledb-bench/src/querygen/`.
**Authority:** `docs/architecture/40-execution.md` (scan-fold pushdown;
column-hoisted gather is the existing idiom), `00-product.md` (representation
over control flow).

## Context (decided — representation-first)

Two executor fast paths use fixed-size scratch arrays sized 8 guarded by runtime
`assert!`s. Two fixes were considered and **rejected**: a validation-boundary cap
(an implementation constant leaking into the user contract) and an eligibility
condition in `begin_scan` (a guard protecting a representation that shouldn't
exist). The root defect is the representation: **both widths are statically known
at plan time**, so a fixed-size runtime scratch guarded by a runtime branch is a
patch on the trace. Delete the scratch; the cap, the assert, and the would-be
eligibility branch all stop existing.

## Current behavior (verify line numbers before relying)

1. The projection sink's scan-pushdown hoist collects `[ColumnView; 8]` with
   `assert!(sources.len() <= 8)` (`exec/sink/projection/sink.rs:102`);
   `sources.len()` = projected word count (`projection/new.rs:20`), unbounded —
   four projected interval fields suffice. `begin_scan` returns `true`
   unconditionally (`projection/sink.rs:74`).
2. `assert!(self.leaf_scan_residuals.len() <= MAX_LEAF_RESIDUALS)` with
   `MAX_LEAF_RESIDUALS = 8` (`exec/run/scan_table.rs:68-71`, constant at
   `exec/run.rs:329`) — a fixed-size copy of a plan-known list.

## Technical direction

1. **Projection scan hoist — flip the loop nesting.** The views array exists to
   pre-resolve columns for a row-outer loop. The codebase's own idiom is the
   inverse: `gather_segment`/`gather_identity` are **column-hoisted** (columns
   outer, rows inner) with no views array at all, and the `ResultBuffer` is
   columnar, so column-wise emit is the native write order. Restructure the
   hoisted scan emit column-outer: for each projected source column, resolve
   `ColumnView` once and copy the run's span into the result buffer's column;
   no scratch, no width limit, any arity. The per-position fallback arm remains
   for non-scannable cursors only (its existing role), not as a width fallback.
2. **Leaf residuals — iterate the plan's own list.** The `[...; 8]` copy in
   `scan_table.rs` duplicates a list the plan witness already owns with a
   plan-time-known length. Evaluate directly off the witness slice (or, if the
   copy exists to pre-resolve column views, apply the same column-hoisted
   restructure). No fixed-size copy survives.
3. **Perf honesty:** both restructures target the hoisted (measured) paths. The
   column-hoisted shape is the same one the gather kernels already won with, so
   a regression is not expected — but flag both hunks for the campaign-closing
   re-bench, and if the ledger families regress, the recorded fallback is the
   eligibility condition (hoist iff width fits; documented as the runner-up,
   only reachable on measured evidence).
4. **Generator coverage:** querygen draws projected-word counts past 8 (wide
   scalar projections and ≥4 interval-field projections) so the differential
   oracle covers this class permanently; coverage-contract assertions extended.

## Passing criteria

- `[shape]` No fixed-size scratch array sized by a width cap exists on either
  hoist path; `SCAN_HOIST_THRESHOLD` (a cost threshold, legitimately a branch)
  is the only constant left; no `assert!` on a data-dependent condition remains
  under `crates/bumbledb/src/exec/` (audit survivors — plan-witness invariant
  asserts are fine with a comment naming their guarantor).
- `[test]` A query projecting >8 words over a relation whose leaf runs reach ≥8
  executes correctly (fixture compared against the per-position arm's output).
- `[test]` The >8-residual leaf case, if constructible, executes correctly.
- `[test]` Batch/scan-path equality: hoisted and non-hoisted results identical
  across the fixture set (the existing equality-harness style).
- `[test]` Querygen coverage-contract asserts wide projections are drawn.
- `[gate]` Workspace gates green; both hunks flagged for re-bench.

## Doc amendments (rule 5)

`40-execution.md`: the scan-fold pushdown paragraph states the column-hoisted
shape and that width is unbounded by construction.
