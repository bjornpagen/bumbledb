# PRD 22: Final Compliance Gate

## Purpose

Prove the hardened engine is aligned with Rosetta, aligned with the Free Join paper where product constraints allow it, and ready for future performance work without architectural drift.

## Required Rosetta Proof

- Relations are sets of full facts.
- Duplicate insert is idempotent.
- Delete is exact and idempotent for absent facts.
- Projection output is duplicate-free.
- Query solutions are set-based.
- No public API implies bag semantics, SQL support, nulls, floats, or public aggregation.
- LMDB is the only durable backend.

## Required Paper Alignment Proof

- Formal Free Join plans use nodes, subatoms, partitions, available vars, new vars, and covers.
- GHT interface is streaming, not eager vector materialization.
- COLT stores offsets over column-oriented base images and builds maps lazily.
- Binary-derived plans and factored plans are validated.
- Dynamic cover choice is observable and uses exact or labeled-estimated key counts.
- Vectorized execution is real batching and NEON-only where SIMD is used.
- Private sinks consume complete encoded bindings and preserve future fold seams.

## Required Benchmark Proof

- JOB exact SQLite gates pass.
- Trace and allocation output exists for representative queries.
- Performance budgets pass.
- Warm q09 and broad numbers are materially improved from the original baseline or the remaining bottleneck is documented with trace evidence.

## Required Search Gates

```bash
rg "std::arch::x86|std::arch::x86_64|target_feature.*(sse|avx)|\b_mm_|\bavx\b|\bsse\b" crates
rg "SELECT(?! DISTINCT)|COUNT\(|GROUP BY|IS NULL|UNION ALL" crates/bumbledb-bench/src/job
rg "bag semantics|public aggregate|SQL frontend|runtime DDL|null" crates docs/ROSETTA_STONE.md
```

The SQL search may need implementation-specific adjustment because Rust regex support differs from PCRE. The gate must still prove forbidden benchmark oracle features do not appear.

## Final Acceptance Commands

```bash
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
bash scripts/check-cutover.sh
bash scripts/check-line-counts.sh
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 3 --warmup 1 --trace summary --alloc on
```

## Passing Criteria

- Every earlier PRD is completed.
- Every paper gap is either closed or explicitly marked product-rejected with Rosetta justification.
- Every final acceptance command passes.
- Benchmark output includes exact correctness, trace, allocation, and budget status.
- No x86 SIMD exists under `crates/`.
- The final state is documented as the new baseline for subsequent work.
