# PRD 01: Paper Adaptation And Public Language

## Purpose

Remove current public-language drift and document how Bumbledb adapts the Free Join paper to Rosetta. This must happen before deeper implementation so future code and docs do not preserve false concepts.

## Dependencies

- PRD 00.

## Scope

- Public Rust docs and comments in `crates/bumbledb-core` and `crates/bumbledb-lmdb`.
- Benchmark labels and correctness wording if benchmark code is reintroduced before PRD 20.
- Rosetta-adjacent documentation.
- Count-only benchmark fixtures that can mislead future work.

## Required Changes

- Change all public serial wording to database-generated monotonic nominal `u64` sequence values.
- Remove aggregation wording from `OutputPlan`, counters, benchmark renderers, and test support unless it is clearly rejected as out of scope. If those modules have been deleted, keep them deleted.
- Make benchmark docs say exact value equality is required before timing, not count equality.
- Replace or delete the legacy LMDB benchmark count-only correctness path. If the benchmark crate is absent, document that PRD 20 must recreate only exact-value correctness checks.
- Rename benchmark queries whose names imply aggregation when they return set projections, such as `triangle_count`. If the benchmark crate is absent, ensure no stale names remain in retained docs/scripts.
- Document that paper SQL examples are illustrative only and not a Bumbledb API.
- Document that paper bag-semantics references are rejected in Bumbledb.
- Document that DuckDB appears only in the paper, not as a Bumbledb dependency.
- Document that query images/COLT are snapshot-local implementation structures, not durable public storage APIs.
- Document that a future Logica-like surface is a frontend lowering path, not a change to Rosetta set semantics.
- Document that public aggregation is deferred, while private sink-based execution is required so future aggregation does not require a Free Join rewrite.

## Technical Direction

- Start with `ValueType::Serial` docs in `crates/bumbledb-core/src/schema/descriptors.rs`.
- Search for `aggregation`, `aggregate`, `group`, `count`, `multiset`, `bag`, `database-allocated`, `generated`, and `SELECT DISTINCT` in public-facing source comments and docs.
- Keep `SELECT DISTINCT` wording only for SQLite benchmark reference behavior.
- If the legacy `crates/bumbledb-lmdb/src/benchmark*` path remains, change it to compare exact result values or explicitly mark it non-authoritative and non-correctness-bearing. Prefer deletion if it is stale.
- Do not recreate benchmark or test-support crates here only to satisfy old test wording. PRDs 19 and 20 own recreation of those crates.

## Non-Goals

- Do not implement the new planner or executor here.
- Do not introduce SQL parsing.
- Do not introduce aggregation.
- Do not change storage layout.

## Acceptance Criteria

- Public docs say `Serial` is generated only for explicitly declared serial fields and is never a UUID or application-supplied nominal-id convention.
- No public API docs claim Bumbledb supports aggregation.
- No benchmark correctness path validates only count equality when claiming correctness.
- All retained SQLite references are explicitly reference-oracle-only and use set semantics.
- `docs/ROSETTA_STONE.md` has a short paper-adaptation note or equivalent linked documentation explaining rejected paper assumptions.
- `docs/ROSETTA_STONE.md` distinguishes future Logica-like syntax from current engine semantics and names the private aggregate-ready sink seam.
- Tests or docs prevent future reintroduction of count-only benchmark correctness. Before PRD 20 recreates benchmark code, this may be satisfied by PRD 20 wording plus retained script/docs checks.

## Required Tests

- If a benchmark crate exists, add a benchmark test where Bumbledb and SQLite have equal counts but different values and assert the benchmark runner fails. If the crate is absent, defer this exact test to PRD 20 and keep PRD 20's requirement explicit.
- Add a lint-style test or script check for forbidden public wording if practical.
- Update any affected golden snapshots.

## Validation Commands

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
rg "database-allocated|bag semantics|multiset behavior|aggregation" crates docs/ROSETTA_STONE.md docs/free-join-paper-alignment-prds
```

The final `rg` command must have no matches in `crates`. It may match explicit rejection, deferral, or adaptation language in `docs/ROSETTA_STONE.md` and this PRD suite.
