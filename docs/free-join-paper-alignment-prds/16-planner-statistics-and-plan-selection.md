# PRD 16: Planner Statistics And Plan Selection

## Purpose

Build a real internal optimizer for Bumbledb's typed set engine. It must choose among Generic Join-like singleton plans, binary-like Free Join plans, factored Free Join plans, static/dynamic covers, and optional accelerator usage without SQL or DuckDB.

## Dependencies

- PRD 05.
- PRD 06.
- PRD 09.
- PRD 13.

## Scope

- Planner statistics over v5 storage/base images.
- Candidate plan generation.
- Candidate plan validation.
- Cost/scoring model.
- Plan mode controls for benchmarks/tests.

## Required Candidate Modes

- Deterministic atom-order binary plan converted through `binary2fj`.
- Factored binary-derived Free Join plan.
- Singleton Generic Join-like Free Join plan.
- Optional manually injected binary plan for paper examples and robustness tests.
- Optional static-cover and dynamic-cover variants.

## Required Statistics

- Relation fact count.
- Loaded base image row count.
- Per-field distinct estimate or exact count for small relations.
- Prefix distinct/fanout estimates for tuple schemas used by candidate plans.
- Optional accelerator entry counts and prefix estimates.
- Skew indicators sufficient to identify clover/sand-dollar style bad binary intermediates.
- Materialization/output estimates for projection-heavy queries.

## Technical Direction

- Keep planner deterministic for equal costs.
- All candidate plans must validate under PRD 03 before costing or execution.
- Start with simple cost formulas if needed, but expose enough counters to replace them.
- Do not make query correctness depend on the chosen plan.
- Plan mode overrides should be internal/test/benchmark configuration, not stable public API.
- Keep current variable-order scoring only as one candidate generator if useful.

## Non-Goals

- Do not add SQL optimizer integration.
- Do not call DuckDB.
- Do not attempt global optimality proof in this PRD.

## Acceptance Criteria

- Planner can generate at least three candidate plan families: singleton, binary-derived, and factored.
- Every candidate validates before execution.
- Default planner chooses a plan deterministically.
- Tests can force plan mode for equivalence and benchmarks.
- Basic skew fixtures choose factored or singleton plans over naive binary-like plans.
- Explain or debug output can show candidate costs after PRD 18, or temporary test access exists here.

## Required Tests

- Candidate generation for chain, star, triangle, clover.
- Candidate validation for self-join aliases.
- Forced plan modes return same result set.
- Skewed clover chooses a non-naive plan under default scoring.
- Deterministic tie-break.
- Planner rejects invalid candidate and falls back to valid candidate.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb planner --all-features
cargo test --workspace --all-features
```
