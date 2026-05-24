# PRD 17: Factorized Materialization

## Purpose

Reduce output and intermediate materialization cost while preserving public duplicate-free set results and private sink/fold seams.

## Required Direction

- Keep factorized output internal.
- Keep public `QueryResultSet` as duplicate-free set output.
- Avoid expanding Cartesian products when projection does not require them.
- Do not add public aggregate queries.
- Use encoded set sinks from PRD 14 as the default public materialization path.

## Required Work

- Extend trace counters for factorized expansions avoided.
- Identify when projection variables are already determined by a prefix frame.
- Avoid repeated sink calls for duplicate projected encoded facts when a factorized representation can prove equivalence.
- Preserve exact `QueryResultSet` equality against materialized execution in tests.

## Required Breaking Changes

- Delete or rewrite old factorized sink code if it duplicates the new encoded set sink without adding value.
- Rename internal modes so they do not imply public aggregation or bag semantics.

## Passing Criteria

- Existing factorized output tests pass or are replaced with stricter encoded-set/factorized equivalence tests.
- A Cartesian duplicate projection fixture shows expansions avoided.
- Decode count remains tied to final facts, not witnesses.
- No public API exposes aggregation.
- Global acceptance from PRD 00 passes.
