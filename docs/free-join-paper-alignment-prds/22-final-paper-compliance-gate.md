# PRD 22: Final Paper Compliance Gate

## Purpose

Prove the refactor is complete. This PRD is the final acceptance gate for direct alignment with the Free Join paper as adapted by Rosetta.

## Dependencies

- PRD 21.

## Scope

- Full repository audit.
- Full validation run.
- Documentation finalization.
- Benchmark smoke and ablation verification.
- Deletion of stale PRD leftovers only if the project policy calls for it.

## Required Compliance Claims

Bumbledb may claim paper-aligned Free Join only if all are true:

- Formal Free Join plans contain subatoms, atom partitions, nodes, covers, and validation.
- The engine can represent binary-like, Generic Join-like, and mixed/factored Free Join plans.
- `binary2fj` exists and is tested.
- Conservative factorization exists and is tested.
- GHT tuple-key interface exists and is tested.
- COLT exists over snapshot-local relation base images and is tested for laziness.
- Scalar Free Join node/cover/probe execution exists and is tested.
- Dynamic cover selection exists and is tested.
- Vectorized execution exists and is tested against scalar execution.
- Public result output remains duplicate-free and canonicalized.
- Benchmarks validate exact projected values before timing.
- Rosetta constraints remain intact.

## Required Audit Checks

- Read `docs/ROSETTA_STONE.md` and confirm no contradiction with implemented behavior.
- Read the paper source sections used by this suite and confirm every adapted feature is documented.
- Read all investigator reports and confirm every P0 and P1 violation is resolved or explicitly rejected by Rosetta with documentation.
- Search for stale terms and deleted mechanics.
- Inspect public exports from `bumbledb-lmdb`.
- Inspect explain output for representative queries.
- Inspect benchmark JSON for representative queries.

## Required Validation Commands

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p bumbledb-test-support --test golden_examples --all-features
cargo test -p bumbledb-test-support --test property_and_differential --all-features
cargo test -p bumbledb-test-support --test sqlite_comparison --all-features
cargo test -p bumbledb-bench --bin bumbledb-bench renderer --all-features
cargo check --manifest-path fuzz/Cargo.toml
bash scripts/check-line-counts.sh
git diff --check
```

## Required Benchmark Smoke

Run the strictest available quick benchmark command after PRD 20. It must include:

- Exact SQLite `SELECT DISTINCT` value comparison.
- At least one singleton/GJ mode query.
- At least one binary-derived Free Join query.
- At least one factored Free Join query.
- At least one dynamic-cover query.
- At least one vectorized query with batch size greater than 1.
- At least one COLT laziness counter check.
- At least one factorized-output mode check if PRD 17 retained that mode.

## Final Acceptance Criteria

- Every PRD 00 through 21 is marked complete in the suite status or has been deleted according to project policy after completion.
- All required validation commands pass.
- Benchmark smoke passes with exact correctness before timing.
- No stale public claims remain.
- No old v4 compatibility path remains unless explicitly documented as a mismatch failure path.
- No SQL, bag, aggregation, DuckDB, null, generated-ID, or non-LMDB feature was introduced.
- The final architecture is documented in Rosetta or a Rosetta-linked architecture note.
- Worktree is clean after the final commit if a commit is requested.

## Stop Conditions

- If any P0/P1 audit violation remains unresolved, do not declare final compliance.
- If any benchmark correctness path compares only counts, do not declare final compliance.
- If explain output cannot show formal Free Join plan structure, do not declare final compliance.
- If COLT correctness depends on a predeclared physical index, do not declare final compliance.
- If public output can expose duplicate projected facts, do not declare final compliance.
