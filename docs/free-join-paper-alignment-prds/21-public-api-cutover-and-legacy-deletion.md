# PRD 21: Public API Cutover And Legacy Deletion

## Purpose

Cut over to the paper-compliant architecture and delete stale compatibility paths, misleading names, old storage assumptions, and LFTJ-only claims. This PRD prevents the refactor from leaving two half-truth systems behind.

## Dependencies

- PRD 18.
- PRD 19.
- PRD 20.

## Scope

- Public exports from `bumbledb-lmdb`.
- Internal module names.
- Old query image/access code.
- Old storage v4 code paths.
- Old tests that assert rejected behavior.
- Docs and scripts.

## Required Deletions Or Renames

- Delete or quarantine old v4 storage write/read code after v5 passes.
- Delete old access-entry-as-query-source assumptions unless retained as optional accelerators.
- Delete `FreeJoinPlan` types that are not formal paper Free Join.
- Keep LFTJ-only modules/types/counters deleted unless rebuilt as formal singleton-plan internals.
- Delete tests that require singleton Free Join nodes as a formal invariant.
- Delete stale benchmark count-only correctness code.
- Delete stale docs or comments mentioning aggregation, bag semantics, or public SQL support.
- Delete dead counters and stale trace fields.

## Technical Direction

- Prefer deletion over compatibility shims.
- If a legacy adapter is necessary for a short transition, it must be crate-private and documented with a deletion PRD reference.
- Public exports should be minimal and aligned with Rosetta.
- Keep `QueryResultSet` public unless deliberately changed by prior PRDs. Do not revive `QueryOutput` as a hollow wrapper.
- Keep no legacy LFTJ compatibility path.

## Non-Goals

- Do not add v4 compatibility readers.
- Do not preserve old API names for external compatibility unless the user explicitly asks.
- Do not hide stale behavior behind feature flags unless needed for tests during this PRD.

## Acceptance Criteria

- Public API exports no longer expose raw storage cursors, raw query images, GHT/COLT internals, or misleading plan internals.
- No production type named `FreeJoinPlan` lacks subatoms, partitions, and covers.
- No production explain output claims Free Join without formal plan data.
- No production storage path writes old v4 access entries as required query structures.
- Count-only benchmark correctness is gone.
- All docs and scripts reference current counters/fields.
- Worktree passes full validation.

## Required Tests

- Compile-fail or visibility tests for raw internals if existing trybuild pattern supports them.
- Public export inventory test or documentation check.
- Full workspace tests.
- Benchmark renderer tests.
- Fuzz check.

## Validation Commands

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
bash scripts/check-line-counts.sh
rg "Free Join node must bind one variable|free_join_node id=.*bind_vars|BenchmarkComparison|database-allocated|aggregation|multiset" crates docs/ROSETTA_STONE.md scripts
```

The final `rg` must return no stale production/docs claims except in this PRD suite or paper source if the search path is expanded.
