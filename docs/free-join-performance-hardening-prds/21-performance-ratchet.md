# PRD 21: Performance Ratchet

## Purpose

Turn performance improvements into enforced budgets so future work cannot silently undo hard-won speedups.

## Required Baseline Process

- Use PRD 06 trace harvest as initial baseline.
- After each optimization PRD, rerun the JOB gate from PRD 19.
- Record p50 and best-of-N timings for q09, broad, and all eight JOB queries.
- Record top spans and allocation totals.
- Update budgets only when correctness is unchanged and improvement is stable.

## Required Budget Types

- elapsed time budget per query;
- allocation byte budget per query;
- allocation call budget per query;
- base-image loaded bytes budget;
- COLT offsets scanned budget;
- tuple materialization budget;
- clone/copy budget;
- sink decode budget.

## Required Ratchet Rule

Budgets may move down after improvement. Budgets may move up only with a written explanation and a trace proving the regression buys a larger required architectural win.

## Passing Criteria

- Budget file exists under `docs/free-join-performance-hardening-prds/PERFORMANCE_BUDGETS.md` or a checked-in benchmark config.
- JOB gate enforces the budget.
- A test or dry-run mode proves regression detection works.
- Budget updates include before/after numbers.
- Global acceptance from PRD 00 passes.
