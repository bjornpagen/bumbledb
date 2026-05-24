# Allocation-First Free Join Redesign PRD Suite

## Status

Drafted as the new active implementation map for the next phase. This suite supersedes the remaining optimization order in `docs/free-join-performance-hardening-prds/` for all work after the committed PRD 14 checkpoint.

## Thesis

Bumbledb already uses LMDB correctly for durable storage. The remaining allocation problem is not LMDB. It is the query-time Free Join adaptation layer, especially COLT. The next phase must make the execution layer look like the paper mechanically, not merely semantically: column buffers, offsets, lazy maps, compact source handles, scratch keys, and arena-owned state.

## Non-Negotiable Focus

- Paper-faithful Free Join comes first.
- Allocation reduction comes before planner, source-filter, storage-accelerator, or benchmark-budget sophistication.
- Do not add SQL, bag semantics, public aggregation, nulls, floats, server mode, async, runtime DDL, or alternate storage.
- Do not add x86 SIMD or x86 dispatch.
- Do not add storage accelerators until this suite explicitly reaches the storage PRDs.
- Do not hide allocation regressions behind traced runs. No-trace release allocation measurements are authoritative for allocation budgets.
- Every PRD must preserve exact SQLite `SELECT DISTINCT` correctness for JOB when benchmarked.

## Current Diagnosis

The hot allocation path is COLT execution state, not base-image loading:

- `HashMap<EncodedTuple, Rc<RefCell<ColtNode>>>` forces one heap-shaped node per distinct key.
- `EncodedTuple { bytes: Vec<u8> }` allocates for normal 8-byte and 16-byte keys.
- Child groups are separate offset vectors or singleton ad hoc state instead of one compact offset arena.
- Source handles clone `Rc` and carry heap-owned metadata rather than compact arena IDs.
- Probe key construction still creates owned key objects.
- Tracing amplifies allocation noise, so traced allocation counts are diagnostic only.

## Baseline Reference

Latest useful no-query-tracing release sample, allocation tracking enabled, `--open-limit 100000`:

| query | alloc_calls | allocated_bytes | result_rows |
| --- | ---: | ---: | ---: |
| `job_broad_cast_keyword_company` | 129544 | 9894250 | 3 |
| `job_broad_movie_info_star` | 108015 | 10911630 | 3 |
| `job_q01_top_production` | 13881 | 1331834 | 0 |
| `job_q09_voice_us_actor` | 129133 | 12582494 | 0 |
| `job_q16_character_title_us` | 66623 | 5850916 | 0 |
| `job_q24_voice_keyword_actor` | 98802 | 9413531 | 0 |
| `job_movie_link_bridge` | 100872 | 10071425 | 0 |
| `job_q33_linked_series_companies` | 20967 | 3541245 | 0 |

`BASELINE.md` contains the checked-in no-query-tracing release baseline for this suite.

## Ordered PRDs

| Order | PRD | Purpose |
| --- | --- | --- |
| 07 | `07-lazy-force-builder.md` | Rebuild COLT force around the arena map and offset pool. |
| 08 | `08-streaming-iteration-and-batching.md` | Restore paper-style iterator behavior over the arena without tuple materialization. |
| 09 | `09-probe-and-source-frame-integration.md` | Integrate arena source handles with the executor source frame. |
| 10 | `10-remove-legacy-colt-structures.md` | Delete legacy COLT pointer graph and compatibility remnants. |
| 11 | `11-vectorized-arena-batches.md` | Make vectorized execution use arena batches and scratch state. |
| 12 | `12-colt-allocation-benchmarks.md` | Add focused allocation benchmarks and allocation budgets for COLT. |
| 13 | `13-job-allocation-gates.md` | Convert no-trace JOB allocation budgets into enforced gates. |
| 14 | `14-output-and-plan-allocation-cleanup.md` | Remove remaining non-COLT query allocation hotspots. |
| 15 | `15-source-filter-pruning-after-arena.md` | Revisit source filtering only after arena COLT exists. |
| 16 | `16-storage-accelerators-after-arena.md` | Revisit optional LMDB accelerators only after source filtering is measured. |
| 17 | `17-neon-only-after-arena.md` | Add AArch64 NEON kernels only after arena and contiguous buffers are stable. |
| 18 | `18-cutover-and-api-cleanup.md` | Remove stale APIs, comments, diagnostics, and compatibility leftovers. |
| 19 | `19-performance-ratchet.md` | Ratchet allocation and time budgets after stable improvements. |
| 20 | `20-final-compliance-gate.md` | Prove Rosetta, paper alignment, and allocation goals. |

## Global Definition Of Done

Each PRD is complete only when all are true:

- The PRD file is deleted in the same commit that completes it.
- The README ordered table is updated.
- Any affected gap inventory or baseline file is updated.
- `cargo fmt --all --check` passes.
- `cargo check --workspace --all-targets --all-features` passes.
- `cargo check --workspace --all-targets --features query-tracing` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `cargo test --workspace --all-features` passes.
- `cargo check --manifest-path fuzz/Cargo.toml` passes when query/storage boundary types change.
- `bash scripts/check-line-counts.sh` passes.
- `git diff --check` passes.
- New allocation claims are backed by no-query-tracing release measurements unless the PRD is explicitly about trace output.
- JOB exact SQLite comparison passes when the PRD touches query execution.
