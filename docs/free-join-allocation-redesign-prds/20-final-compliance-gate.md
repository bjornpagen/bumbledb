# PRD 20: Final Compliance Gate

## Purpose

Prove the allocation-first Free Join redesign is complete, paper-aligned, and Rosetta-compliant.

## Required Proofs

- Relations remain sets of full facts.
- Duplicate insert remains idempotent.
- Delete remains exact and idempotent for absent facts.
- Projection output remains duplicate-free and canonical.
- Query solutions remain set-based.
- LMDB remains the only durable backend.
- No public API implies SQL support, bag semantics, nulls, floats, or public aggregation.
- COLT uses arena-backed nodes and offset/key storage.
- No legacy heap-shaped COLT pointer graph remains.
- No x86 SIMD exists.

## Required Commands

```bash
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo check --workspace --all-targets --features query-tracing
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
bash scripts/check-line-counts.sh
git diff --check
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 3 --warmup 1 --alloc on
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label allocation_final_q09 --alloc on
```

## Passing Criteria

- Every earlier PRD in this suite is deleted as completed.
- Every required command passes.
- JOB exact SQLite comparison passes.
- Allocation budgets pass.
- Paper gap inventory is updated for all closed/rejected gaps.
- Final baseline documents the new post-redesign allocation state.
