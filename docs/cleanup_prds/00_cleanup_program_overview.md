# PRD 00: Cleanup Program Overview

## Status

Draft. This is the ordered execution map for the cleanup pass before any new database feature work resumes.

## Thesis

Bumbledb should be a lightning-fast embedded relational database for BCNF-normalized, typed, join-heavy workloads. The cleanup program must make that core identity sharper, not broader.

The target system is:

```text
typed scalar values
strict schemas
compound primary keys
compound foreign keys
first-class closed enums
small explicit indexes
query IR independent of any surface language
fast direct paths for simple workloads
Free Join/LFTJ for true join-heavy workloads
```

## Hard Non-Goals

- No FlatBuffer support.
- No vector support.
- No vector set support.
- No JSON/document model.
- No nullable values.
- No SQL frontend.
- No migration compatibility.
- No backward-compatible storage layout.
- No dual public query languages.
- No general-purpose database scope creep.

## Current Codebase Facts

- Core schema/types live in `crates/bumbledb-core/src/schema.rs`.
- Current custom query frontend and typed query IR live together in `crates/bumbledb-core/src/datalog.rs`.
- LMDB storage and write constraints live in `crates/bumbledb-lmdb/src/storage.rs`.
- Query normalization, planning, direct kernels, hash probe, LFTJ, and sinks live in `crates/bumbledb-lmdb/src/query.rs`.
- Free Join physical plan types live in `crates/bumbledb-lmdb/src/free_join.rs`.
- Query images, fixed columns, relation images, and caches live in `crates/bumbledb-lmdb/src/query_image.rs`.
- Planner stats live in `crates/bumbledb-lmdb/src/planner_stats.rs`.
- Hash and sorted trie implementations live in `hash_trie.rs` and `sorted_trie.rs`.
- Benchmarks live in `crates/bumbledb-bench/src/main.rs`, `open.rs`, and `crates/bumbledb-lmdb/src/benchmark.rs`.
- Test support includes an independent reference evaluator in `crates/bumbledb-test-support/src/reference.rs`.

## Current Strengths To Preserve

- Fixed-width typed encodings.
- No runtime type tags in hot relation/index keys.
- No null semantics.
- Strong typed ID/ref distinction.
- LMDB sorted byte keys as the physical foundation.
- Query images for immutable snapshot execution.
- Free Join/LFTJ path for cyclic and highly join-heavy queries.
- Hash probe/direct-kernel scaffolding for selective joins.
- Strict workspace lints.
- Good transactional failpoint coverage.

## Current Problems To Fix

- Schema validation is too weak and too late.
- Field-level `ValueType::Ref` conflates scalar type and foreign-key constraint.
- Compound primary keys exist, but compound foreign keys do not.
- `Symbol` is not a first-class enum.
- Generated and explicit index names can collide.
- Index candidates are silently deduped by field vector.
- Every current index covers every field, which is elegant for narrow rows but must become explicit and controlled.
- The internal typed IR is owned by the `datalog` module even though the runtime only needs a language-neutral IR.
- Simple point/range/selective workloads often pay query-image, planner, hash-trie, or LFTJ setup overhead.
- Prepared plan caching is disabled for parameterized queries.
- The benchmark harness checks row counts more often than full result equality.
- Large files hide conceptual boundaries and slow future work.

## Ordered PRDs

1. `01_schema_validation_and_namespaces.md`
2. `02_type_model_and_first_class_enums.md`
3. `03_constraint_model_and_compound_foreign_keys.md`
4. `04_index_layout_and_access_paths.md`
5. `05_storage_layout_and_lifecycle.md`
6. `06_query_ir_and_logica_preparation.md`
7. `07_non_join_fast_paths.md`
8. `08_planner_strategy_and_stats.md`
9. `09_module_boundaries_and_public_api.md`
10. `10_tests_benchmarks_and_acceptance_gates.md`
11. `11_execution_sequence_and_cutover.md`

## Global Design Rules

- Types describe values.
- Schema constraints describe relational invariants.
- Query predicates describe query filters.
- Indexes serve constraints and query planning.
- Surface query languages lower into a typed language-neutral IR.
- Direct execution paths should run before heavyweight join machinery when the query shape allows it.
- LFTJ is for true join-heavy/cyclic workloads, not the default tax on everything.

## Global Passing Criteria

- `cargo fmt --all --check` passes.
- `cargo check --workspace --all-targets --all-features` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `cargo test --workspace --all-features` passes.
- `cargo check --manifest-path fuzz/Cargo.toml` passes.
- Existing benchmark gates pass.
- New schema/constraint/enum/direct-path tests pass.
- No vector or FlatBuffer support appears in schema, planner, docs, or code beyond explicit non-goal statements.
