# PRD 15: Source Filter Pruning After Arena

## Purpose

Revisit source filter pruning only after arena COLT exists, so pruning shrinks arena offset ranges before map construction without adding another allocation-heavy path.

## Required Preconditions

- PRDs 00-14 complete.
- COLT arena path is the only production COLT path.
- JOB allocation gates exist.

## Required Work

- Represent source filters in encoded typed form before source construction.
- Apply filters into arena offset ranges or singleton/range node states.
- Avoid loading unrelated columns for impossible dictionary literals.
- Emit rows-tested and survivor counts per filter and per atom.
- Short-circuit execution on zero-source atoms.
- Keep residual cross-variable comparisons exact.

## Passing Criteria

- Missing string dictionary literal creates a zero source without scanning unrelated columns.
- Pushed range filters and residual-only test mode produce identical results on fixtures.
- q09 trace reports survivors for `country_code = '[us]'`, `gender = 'm'`, and `role = 'actor'`.
- Empty-source short-circuit emits a dedicated counter/span.
- No-trace allocation budgets do not regress except with documented survivor-count benefit.
- Global gates pass.
