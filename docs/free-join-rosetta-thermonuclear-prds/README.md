# Free Join Rosetta Thermonuclear PRD Suite

## Purpose

This is the ordered remaining work plan for making Bumbledb's Free Join engine fast enough to justify its architecture.

The current traced JOB sample proves that the next bottleneck is not abstract query theory. It is physical read shape. The engine spends most traced execution time building snapshot-local base images before Free Join can do useful work.

Current traced baseline, collected from `current-full-trace-7934-*.json`:

| metric | value |
| --- | ---: |
| JOB sample queries | 8 |
| Exact SQLite comparisons | 8 passed |
| Total root `ExecuteNode` time | 291.91 ms |
| Total `BaseImageLoad` time under execution | 235.06 ms |
| Base-image share of execution | about 80.5% |
| Total column values loaded | 749,510 |
| Total loaded column bytes | 5,996,080 |
| Total COLT offsets scanned | 284,209 |
| Total tuples yielded | 112,330 |
| Total binding conflicts | 96,033 |
| Vectorized batches yielded | 0 |

## Alignment Order

Every PRD must pass these in order:

1. Rosetta Stone product contract.
2. Free Join paper algorithmic contract.
3. Existing correctness tests and exact SQLite benchmark oracle.
4. Performance and allocation trace evidence.

If an implementation idea conflicts with Rosetta, reject it even if the paper or a benchmark would like it.

If an implementation idea conflicts with formal Free Join, GHT, COLT, dynamic covers, or paper-described laziness, either reject it or explicitly prove that it is a storage adaptation that preserves the same abstract interfaces.

## Non-Negotiables

- LMDB through `heed` remains the only durable backend.
- Bumbledb remains embedded Rust, not a server.
- No SQL frontend.
- SQL exists only as an external exact `SELECT DISTINCT` benchmark oracle.
- Base facts are sets.
- Query solutions and public projections are sets.
- No bag semantics.
- No nulls.
- No floats.
- No runtime DDL.
- No async API.
- No fake storage, in-memory shadow database, or application-level COW substitute.
- No old format compatibility readers unless a PRD explicitly says the old format is rejected with a hard error.
- No public aggregation until Rosetta admits it.
- No x86 SIMD. Explicit SIMD is AArch64 NEON only.
- Do not add caches that cross LMDB snapshot correctness boundaries.

## Current Diagnosis

The trace says the physical base-image loader is the first fire.

Current `load_relation_base_image` does this shape:

```text
scan live row handles
for each requested field:
  for each live handle:
    LMDB point get C | relation | field | handle
```

That is fatal for JOB. It turns a few megabytes of encoded column data into hundreds of thousands of LMDB lookups.

The Free Join paper assumes raw data is already stored column-wise and that selections are pushed down to base tables. The paper also warns that disk-backed COLT can be inefficient because repeated random access hurts. Bumbledb's adaptation must therefore make LMDB reads columnar, sequential, filtered, and lazy enough that COLT receives only useful source images.

## Ordered PRDs

| order | PRD | purpose |
| ---: | --- | --- |
| 03 | `03-query-local-column-cache-and-image-views.md` | Stop reloading the same relation columns for different atom occurrences and scopes. |
| 04 | `04-source-handle-sets-and-survivor-views.md` | Make filtered source images handle/offset views instead of copied full-column images. |
| 05 | `05-storage-v6-columnar-read-layout.md` | Break storage format for query-native columnar read layout. |
| 06 | `06-durable-value-accelerators.md` | Add correctness-optional durable value accelerators maintained by writes. |
| 07 | `07-index-backed-ght-colt-sources.md` | Back GHT/COLT sources directly from accelerators and survivor handle sets. |
| 08 | `08-selectivity-aware-free-join-planner.md` | Stop choosing plans that burn 32k binding conflicts before proving emptiness. |
| 09 | `09-dynamic-cover-and-access-costing.md` | Make dynamic cover choice exact-or-labeled and access-path aware. |
| 10 | `10-colt-force-and-iteration-cleanup.md` | Attack the post-base-image runtime: COLT force, iteration, and probe allocation. |
| 11 | `11-vectorized-and-neon-execution.md` | Make vectorized execution real and AArch64 NEON-only after the physical path is fixed. |
| 12 | `12-final-ratchet-and-compliance-gate.md` | Ratchet exact correctness, trace, allocation, and performance budgets. |

## Global Acceptance Commands

Every PRD that changes code must run the relevant subset during development and all of these before being called complete:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo check --workspace --all-targets --features query-tracing
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
bash scripts/check-line-counts.sh
git diff --check
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label thermonuclear --alloc on
```

If a script still references removed PRD docs, that is not a reason to restore old docs. Fix the script or delete the stale gate.

## Final Target

The final state must make the traced JOB sample show that base-image loading is no longer the dominant execution cost, that source filters reduce physical work before COLT, that COLT remains lazy and paper-aligned, and that exact SQLite comparisons still pass.

The suite is complete only when the bottleneck moves from physical read shape to a legitimate Free Join algorithmic choice, and that new bottleneck is documented with trace evidence.
