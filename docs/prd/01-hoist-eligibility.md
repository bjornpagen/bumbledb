# PRD 01 — Hoist-path eligibility (reachable, data-dependent panics)

**Depends on:** nothing. **Severity: highest in the set** — a valid query panics,
data-dependently: it works while every leaf run is short and dies the first time
a run reaches the hoist threshold, i.e. passes tests on small data and crashes on
real data.
**Modules:** `crates/bumbledb/src/exec/sink/projection/sink.rs`,
`crates/bumbledb/src/exec/run/scan_table.rs`, `crates/bumbledb/src/exec/run.rs`,
`crates/bumbledb-bench/src/querygen/`.
**Authority:** `docs/architecture/40-execution.md` (scan-fold pushdown; the
"no data-dependent branching becomes no data-dependent panicking" spirit of the
batching laws).

## Context (decided)

Two executor fast paths use fixed-size scratch arrays sized 8 guarded by runtime
`assert!`s, with nothing making the guarded state unreachable. A validation-
boundary cap was considered and **rejected**: `MAX_DISTINCT_VARS` /
`MAX_OCCURRENCES` protect *representations* (bitset widths, DP tables); a
projection-width cap would protect one optimization's scratch array — an
implementation constant leaking into the user contract. The fix is at the
**eligibility altitude**.

## Current behavior (verify line numbers before relying)

1. The projection sink's scan-pushdown hoist uses `[ColumnView; 8]` with
   `assert!(sources.len() <= 8, "projection arity cap")`
   (`exec/sink/projection/sink.rs:102`); `sources.len()` = projected word count
   (`projection/new.rs:20`), unbounded — four projected interval fields suffice
   (two words each). **Root defect one altitude up:** `begin_scan` returns `true`
   unconditionally (`projection/sink.rs:74`) — an eligibility function with no
   eligibility condition. The safe per-position arm at `projection/sink.rs:126`
   is correct for any arity.
2. `assert!(self.leaf_scan_residuals.len() <= MAX_LEAF_RESIDUALS)` with
   `MAX_LEAF_RESIDUALS = 8` (`exec/run/scan_table.rs:68-71`, constant at
   `exec/run.rs:329`) — same shape: nothing bounds column-touching residuals per
   leaf.

## Technical direction

1. **Projection width:** `begin_scan` gains the condition — hoist iff
   `sources.len()` fits the scratch constant; otherwise return `false` and let
   the per-position arm run. The constant is defined **once** and used by both
   the scratch array type and the eligibility test (an associated `const` or a
   shared `pub(crate) const SCAN_HOIST_MAX_WORDS: usize = 8` — single source of
   truth so the two cannot drift). The `assert!` becomes `debug_assert!` with a
   comment naming the eligibility guard that makes it unreachable.
2. **Leaf residuals:** same shape — the hoist's eligibility site checks
   `leaf_scan_residuals.len() <= MAX_LEAF_RESIDUALS` and falls back to the
   non-hoisted path. If reading the code proves no correct fallback arm exists
   for this case, instead make the bounding invariant explicit at plan
   validation (`plan/fj/validate.rs`) with a comment citing it at the
   (then-`debug_`) assert — and say which branch you took in the commit body.
3. **Generator coverage:** querygen draws projected-word counts past 8 (mix of
   wide scalar projections and ≥4 interval-field projections) so the
   differential oracle covers this class permanently. Extend the coverage
   contract test's assertions accordingly.

## Passing criteria

- `[shape]` `begin_scan` contains an eligibility condition; the shared constant
  has exactly one definition site; no `assert!` on a data-dependent condition
  remains under `crates/bumbledb/src/exec/` (grep for `assert!` and audit each
  survivor — invariant asserts on plan-witness properties are fine and each
  carries a comment naming its guarantor).
- `[test]` A query projecting >8 words over a relation whose leaf runs reach ≥8
  executes and returns correct results (constructed fixture, compared against
  the per-position arm's output or a hand-computed expectation).
- `[test]` The >8-leaf-residual case, if constructible, executes correctly; if
  not constructible, the plan-validation invariant has its own test.
- `[test]` The querygen coverage-contract test asserts wide projections are
  drawn (±30% band like the other shapes).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`40-execution.md`: the scan-fold pushdown paragraph gains the eligibility
condition sentence.
