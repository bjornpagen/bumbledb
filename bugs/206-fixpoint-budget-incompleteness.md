# Fixpoint budget makes the engine incomplete versus Lean evalProgram
- id: 206
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: rust
- components: lean/Bumbledb/Exec/Fixpoint.lean, docs/architecture/40-execution.md, crates/bumbledb/src/api/prepared/fixpoint.rs, crates/bumbledb/src/error.rs
- status: open (do not fix)

## Summary
Lean `evalProgram` is a fueled round loop proved equal to the stratified denotation given sufficient fuel (`program_eval_sound`). The engine adds a host-amendable iteration/tuple budget that aborts with `FixpointBudgetExceeded` before the least fixpoint. Docs acknowledge this as a trust-boundary amendment. Under budget exhaustion the engine's answers are a strict subset of the spec denotation — incompleteness, not a wrong tuple.

## Lean spec
`Fixpoint.lean:11-21`: Level 1 is the fueled round loop "proved sound AND complete against Level 0 (`program_eval_sound`)." Budgets are named as mechanism that "stay in the docs, whole" (`:18-21`). The model has no `FixpointBudgetExceeded`; fuel 0 returns the accumulator (`fueledLoop`), which is incomplete only if fuel is chosen too small — the agreement theorems assume enough fuel.

## Normative docs
`40-execution.md:973-985` and `:529-535`: the budget "amends this stance for fixpoints only"; termination is a theorem of the roster (`program_den_finite`) but size is data-shaped; default exists so the boundary is never unguarded; policy is host-owned. This is honest about incompleteness; it is still a spec-vs-engine answer mismatch when the budget trips.

## Rust implementation
`api/prepared/fixpoint.rs:37-47`: "The budget is the one new trust boundary." `Error::FixpointBudgetExceeded { stratum, rounds, tuples }`. Tests: `a_tight_fixpoint_budget_trips_with_the_typed_error`. Bridge row (`Bridge.lean:590-593`) lists that error as the engine-discharge mechanism for `program_eval_sound` — the ledger equates a complete Lean evaluator with a budgeted driver.

## Why this matters
A recursive query whose fixpoint exists (Lean: finite subset of the active domain) can fail at runtime with a typed error and empty/partial answers. Hosts that treat Lean `programAnswers` as the engine contract will see a false abort. The Bridge row papers over the gap by citing both `run_fixpoint` and `FixpointBudgetExceeded` against the completeness theorem.

## Related
- 210 (runtime error roster)
