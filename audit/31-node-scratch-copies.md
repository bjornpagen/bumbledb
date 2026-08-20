# 31 — `NodeScratch` still copies residual-source metadata beside `NodePrecompute`

- **Status:** **fixed this pass** — residual metadata is plain data on
  `NodePrecompute` (`ResidualSpec` / `WordResidualSpec` /
  `AllenResidualSpec` / `DurationResidualSpec`). Scratch keeps only
  cover-dependent `(Source, Source)` pairs. No `FilterPredicate` clone
  into precompute. Gate: `alloc_law_budgets` plus exec run tests.
- **Severity:** should-fix (representation + allocation).

## Principle

`NodePrecompute` landed (the seven shadow spines are gone); the execution
scratch still copies residual-source lists that are plan facts. Precompute
owns metadata; scratch owns transient state only — the split is the point
of the earlier fix, finished.

## Guardrails (recorded refusals stand — do not re-litigate)

- The kind-grouped `PlanNode` batching lists stay ("the grouping IS the
  batching law").
- No `NodeScratch` extraction into a struct-of-structs ("the grouping buys
  no new invariant").
Only the *copies* go.

## The fix

Move the residual-source metadata reads to `NodePrecompute` (or direct plan
reads); `NodeScratch` retains buffers whose content is per-execution. Any
surviving copy carries a recorded reason at the site.

`NodePrecompute::of` now extracts op/vars/offsets/slots at construction.
Eval (`run_node`, `probe_pass`, `leaf_precompute`) reads those fields —
it does not re-call `compare_sides` / `allen_sides` / `duration_sides` on
a cloned predicate. `bind_allen_masks` is a no-op for literals already
copied into `allen_masks`. Cover-dependent `residual_sources` /
`word_residual_sources` / `allen_sources` / `duration_sources` stay on
scratch (runtime `word_base` → Batch vs Slot).

## Acceptance

- No plan-derivable list is duplicated into scratch; 28's warm-query budget
  covers the allocation half.
- Scenario lanes byte-identical.
- `Executor::execute(plan, colts, bindings, sink, counters)` untouched.
- Kind-grouped `PlanNode` lists untouched.
