# Free Join Performance Hardening PRD Suite

## Status

Drafted. This suite is the ordered, breaking-change implementation map for turning Bumbledb from a correct Free Join prototype into a heavily instrumented, paper-aligned, high-performance embedded set engine.

This suite deliberately prioritizes correctness, observability, Free Join paper alignment, Rosetta set semantics, and speed over API compatibility. Compatibility shims are forbidden unless a PRD explicitly allows one.

## Inputs

- Normative product contract: `docs/ROSETTA_STONE.md`.
- Paper source: `docs/free-join-paper/arXiv-2301.10841v2/`.
- Current Free Join implementation: `crates/bumbledb-lmdb/src/query/`, `crates/bumbledb-lmdb/src/colt.rs`, `crates/bumbledb-lmdb/src/base_image.rs`.
- JOB benchmark harness: `crates/bumbledb-bench/src/job/`.
- Observed baseline: `job_q09_voice_us_actor` warm Bumbledb around 41 ms, SQLite around 5-6 ms, exact result rows 0 at scale 115933.

## Non-Negotiable Constraints

- Bumbledb remains a Rust embedded database over real LMDB through `heed`.
- Durable storage remains LMDB only.
- Set semantics are absolute: base facts are sets, solution bindings are sets, projected results are duplicate-free canonical sets.
- SQL remains only an external benchmark oracle using exact `SELECT DISTINCT`.
- No SQL frontend, bag semantics, nulls, floats, runtime DDL, server mode, network protocol, async API, alternate durable storage engine, or public aggregation.
- The implementation may break all internal Rust APIs, benchmark JSON, storage format, and diagnostic formats when doing so improves architecture.
- Public compatibility is not a goal. Old format readers, migration shims, aliases, and compatibility layers are forbidden.
- All vectorization work is AArch64 NEON only. Do not implement x86 SSE, AVX, AVX2, AVX-512, x86 portable-SIMD dispatch, or x86 feature detection. Non-AArch64 platforms may compile and run scalar code only.
- Every optimization must be justified by traces, counters, allocation data, or a direct Free Join paper alignment requirement.
- Every benchmark timing claim must be paired with exact result verification against a reference set.

## Current Paper-Alignment Gaps

- `GhtSource::iter` returns `Vec<EncodedTuple>` rather than an iterator, causing eager materialization not present in the paper interface.
- `iter_batch` is implemented by materializing all tuples first, so it is not true vectorized execution.
- Planner statistics currently call `relation_base_image`, which builds or loads snapshot images during planning.
- Base-image columns are `Vec<Vec<u8>>`, creating one allocation per loaded value instead of a tight column-oriented layout.
- Source filters are applied after full base-image rows are available, so selective predicates still pay broad scan/load cost.
- Recursive execution clones binding maps and source maps along hot paths.
- COLT uses deterministic `BTreeMap` grouping and per-node allocation patterns, not a performance-oriented hash-trie layout.
- Dynamic cover choice often uses offset counts as estimates rather than true key counts.
- The current vectorized mode is scalar batching, not NEON vectorization.
- Explain says timings and allocations are not collected.
- JOB benchmark output does not expose phase timings, allocation deltas, or execution counters.
- Storage has stats and accelerator namespaces but no real value accelerator path for source predicates.
- The planner is deterministic and heuristic, not yet an integrated Free Join optimizer exploring the design space between binary join and Generic Join.

## Ordered PRDs

| Order | PRD | Purpose |
| --- | --- | --- |
| 08 | `08-planner-stats-without-base-images.md` | Stops plan selection from building base images. |
| 09 | `09-base-image-columnar-layout.md` | Breaks base-image storage into tight fixed-width column buffers. |
| 10 | `10-streaming-ght-interface.md` | Replaces eager `Vec` iteration with streaming and real batching. |
| 11 | `11-colt-lazy-paper-alignment.md` | Makes COLT closer to the paper's lazy column-oriented trie. |
| 12 | `12-source-filter-pruning.md` | Turns source predicates into early pruning rather than post-load scans. |
| 13 | `13-recursive-frame-executor.md` | Replaces recursive map/binding cloning with stack/frame execution state. |
| 14 | `14-set-first-encoded-sinks.md` | Makes encoded set deduplication the default sink path. |
| 15 | `15-neon-only-vectorized-execution.md` | Implements real AArch64 NEON-only vectorized kernels and forbids x86 SIMD. |
| 16 | `16-dynamic-cover-costing.md` | Improves dynamic cover selection with measured key counts and selectivity. |
| 17 | `17-factorized-materialization.md` | Deepens factorized output and materialization alignment without public aggregation. |
| 18 | `18-storage-v6-stats-accelerators.md` | Introduces breaking storage v6 stats and optional value accelerators. |
| 19 | `19-job-benchmark-gates.md` | Makes JOB correctness, trace, allocation, and performance gates mandatory. |
| 20 | `20-api-cutover-cleanup.md` | Deletes stale APIs, compatibility remnants, and misleading diagnostics. |
| 21 | `21-performance-ratchet.md` | Adds measured performance budgets and ratchets after each optimization wave. |
| 22 | `22-final-compliance-gate.md` | Final suite acceptance gate for Rosetta, paper alignment, and performance readiness. |

## Global Definition Of Done

Each PRD is complete only when all of these are true:

- The change preserves Rosetta set semantics.
- The change is backed by real tests or benchmark checks, not screenshots or manual inspection.
- All new benchmark metrics are real engine measurements.
- All timing and allocation spans are emitted by the shared trace model.
- No x86 SIMD or x86 vectorization code exists anywhere under `crates/`.
- NEON code is isolated behind `#[cfg(target_arch = "aarch64")]` and scalar fallback is explicit.
- No public API, docs, or benchmark language implies SQL support, bag semantics, nulls, floats, or public aggregation.
- No compatibility reader, format alias, or migration shim is added.
- `cargo fmt --all --check` passes.
- `cargo check --workspace --all-targets --all-features` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `cargo test --workspace --all-features` passes.
- `cargo check --manifest-path fuzz/Cargo.toml` passes when storage/query boundary types change.
- `bash scripts/check-line-counts.sh` passes, or the PRD explicitly requires a file split before completion.
- `git diff --check` passes.

## Completion Discipline

- Execute PRDs in order.
- If a later PRD exposes that an earlier PRD was incomplete, stop and repair the earlier PRD first.
- If the implementation needs a breaking storage or Rust API change, make the breaking change.
- Do not optimize by adding a second query engine.
- Do not add indexes or accelerators before the tracing/allocation harvest proves the bottleneck and the PRD order reaches storage accelerators.
- Do not weaken exact correctness gates to chase speed.
- Do not carry old behavior for compatibility.
