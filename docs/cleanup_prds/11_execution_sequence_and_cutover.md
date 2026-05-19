# PRD 11: Execution Sequence And Cutover

## Status

Draft. This document defines the order to execute the cleanup PRDs and the cutover criteria before returning to new database work.

## Execution Principles

- Break storage/schema layouts freely.
- Do not preserve compatibility with old data.
- Keep commits small enough to bisect.
- Do not start Logica implementation until the prep work is complete.
- Do not add vector or FlatBuffer work.
- Keep benchmark artifacts for major performance changes.

## Ordered Work Plan

### Phase 1: Schema And Types

1. Implement PRD 01 schema validation.
2. Implement PRD 02 enum/type model.
3. Update all schemas, rows, tests, and benchmarks to use first-class enums.

Exit criteria:

- All valid existing schemas compile under strict validation.
- Invalid schema tests are comprehensive.
- No pseudo-enum `Symbol` remains in production code.

### Phase 2: Constraints And Indexes

4. Implement PRD 03 compound FK constraints.
5. Implement PRD 04 explicit index layout/access paths.
6. Implement PRD 05 storage layout/lifecycle updates required by new indexes.

Exit criteria:

- Compound FK tests pass.
- Unique guard tests pass.
- Index layout tests pass.
- Non-covering index scans can fetch rows correctly.
- Existing benchmark gates still pass.

### Phase 3: Query IR And Fast Paths

7. Implement PRD 06 language-neutral query IR.
8. Implement PRD 07 non-join fast paths.
9. Implement PRD 08 planner family split and stats improvements.

Exit criteria:

- LMDB runtime no longer imports `bumbledb_core::datalog`.
- Simple point/range queries bypass query image/LFTJ where appropriate.
- Parameterized shape caching works.
- Non-JOB SQLite gap is materially reduced.
- JOB and triangle remain strong.

### Phase 4: Boundaries And Acceptance

10. Implement PRD 09 module/API cleanup.
11. Implement PRD 10 expanded tests/bench gates.
12. Update `ROSETTA_STONE.md` after the implementation decisions are real.

Exit criteria:

- Module boundaries match architecture.
- Public/internal surfaces are intentional.
- Full verification gate passes.
- Cleanup PRD directory can be deleted or archived after all PRDs are complete.

## Final Cutover Criteria

Before returning to feature work, all of the following must be true:

- Schema validation is strict and comprehensive.
- Type model has first-class enums.
- Compound FKs are explicit and enforced.
- FK enforcement is no longer implied by field type alone.
- Generated FK indexes exist and support restrict deletes.
- Index layout policy is explicit and does not blindly full-cover everything.
- Storage read/write paths support the new index layout.
- Query IR is language-neutral.
- Datalog parser is isolated and ready to delete in the later Logica phase.
- Simple non-join-heavy queries have direct execution paths.
- LFTJ is not charged to simple point/range workloads.
- Benchmark gates pass.
- Exact correctness tests exist for affected query shapes.
- No vector or FlatBuffer support was added.

## Suggested Commit Order

1. Add schema validation and tests.
2. Add enum descriptors and convert schemas.
3. Add explicit FK descriptors and convert schemas.
4. Add compound FK enforcement.
5. Add explicit index layout policy.
6. Update storage read/write/index scans.
7. Move query IR out of `datalog.rs`.
8. Add direct primary/unique/prefix/range paths.
9. Add index nested-loop path.
10. Add parameterized prepared shape cache.
11. Split planner families and stats.
12. Split modules and narrow API.
13. Tighten tests/bench gates.

## Rollback Strategy

No storage rollback or compatibility is required. If a step fails, revert the code commit and continue from the previous clean commit. Do not add compatibility shims.

## Verification Gate For Final Cleanup Completion

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
scripts/bench-focused.sh --fail-gates
```

If JOB data is available:

```sh
cargo run -p bumbledb-bench --release -- --job-dir "$JOB_DIR" --dataset job --scale 10000 --warmup 2 --repeats 10 --format markdown --fail-gates
```
