# PRD 12: Final Ratchet And Compliance Gate

## Purpose

Lock in the new performance baseline and prove the engine remains Rosetta-first and Free-Join-paper aligned.

## Required Final Proof

The final state must prove:

- LMDB is still the only durable backend.
- Base facts remain sets.
- Projected outputs remain duplicate-free sets.
- SQL is still only an external exact `SELECT DISTINCT` oracle.
- No nulls, floats, bag semantics, runtime DDL, server mode, async API, or SQL frontend were added.
- Free Join plans still use nodes, subatoms, partitions, available variables, new variables, and covers.
- GHT/COLT sources still expose iteration and keyed lookup.
- COLT remains lazy.
- Source filters are pushed to base/source access.
- Dynamic cover choice is exact-or-labeled and traced.
- Accelerators are correctness-optional.
- Vectorized mode, if enabled, is exact and NEON-only for explicit SIMD.

## Required Budget File

Create a checked-in budget file outside deleted old PRD directories. Suggested path:

```text
docs/free-join-rosetta-thermonuclear-prds/JOB_FINAL_BUDGETS.tsv
```

Columns:

```text
query	max_elapsed_nanos	max_alloc_calls	max_allocated_bytes	max_base_image_load_nanos	max_colt_offsets_scanned	max_binding_conflicts
```

Budgets must be based on the improved final trace, not the suite baseline in `README.md`.

## Required Script Cleanup

- Delete or rewrite stale PRD-map checks that reference removed docs.
- Move allocation/performance gates to the new budget file.
- Ensure benchmark gates fail on missing budgets.
- Ensure exact correctness gates cannot be disabled.

## Required Final Commands

```bash
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo check --workspace --all-targets --features query-tracing
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
bash scripts/check-cutover.sh
bash scripts/check-line-counts.sh
git diff --check
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 3 --warmup 1 --trace-output file --profile-query-label final-thermonuclear --alloc on
```

If budget enforcement is implemented as a script, run it here too.

## Required Final Trace Targets

The exact numbers should be ratcheted from measured post-PRD results, but the final shape must satisfy all of these:

- `BaseImageLoad` is no longer the majority of root `ExecuteNode` time.
- `binding_copies` remains 0.
- `batches_yielded` is non-zero when vectorized mode is requested.
- Filtered queries show physical row pruning before COLT.
- q09/q16/q24 no longer burn about 32k binding conflicts each before returning zero rows unless the final plan trace proves that is mathematically unavoidable on the loaded data.
- Full JOB sample exact SQLite comparisons pass.

## Passing Criteria

- Every PRD in this suite is complete in order.
- Every final command passes.
- Final benchmark output includes exact correctness, tracing, allocation, and budget status.
- The final budget file is checked in.
- Old deleted PRD directories are not recreated.
- The final trace analysis is documented in this directory as the new baseline.
