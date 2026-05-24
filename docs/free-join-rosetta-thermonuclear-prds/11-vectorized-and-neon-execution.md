# PRD 11: Vectorized And NEON Execution

## Purpose

Make vectorized execution real after the physical source path is fixed.

## Rosetta Alignment

Vectorization is private execution strategy. It must preserve exact duplicate-free public output.

## Paper Alignment

The paper's vectorized Free Join batches cover iteration and performs probes for a batch before recursing. The current baseline reports `execution_mode = Scalar` and `batches_yielded = 0`, so paper vectorization is not yet part of the benchmark path.

## Required Scope

First make scalar and vectorized modes both operate over the new source variants:

- column-scan sources;
- survivor-view sources;
- accelerator-backed sources;
- empty sources.

Then optimize AArch64 with NEON only where contiguous encoded columns make it valuable.

No x86 SIMD is allowed.

## Required Vectorized Behavior

- Public default may remain scalar until vectorized gates pass.
- Vectorized mode must use bounded `TupleBatch`, not all-tuples materialization.
- Batched sibling probes must preserve source-frame undo semantics.
- Batch survivor compaction must preserve tuple order where deterministic output depends on it.
- Residual predicates and sink semantics must match scalar exactly.

## Required NEON Direction

Allowed kernels:

- fixed-width equality scan for 8-byte encoded values;
- fixed-width range scan for 8-byte ordered encodings;
- survivor bitmap/index compaction when it beats scalar;
- batch key comparison where data is contiguous.

All explicit SIMD must be under:

```rust
#[cfg(target_arch = "aarch64")]
```

Non-AArch64 builds must compile and run scalar fallback.

## Tests Required

- Scalar/vectorized equivalence for all existing query fixtures.
- Scalar/vectorized equivalence for JOB sample queries or a representative JOB mini-fixture.
- Batch sizes 1, 2, 4, 16, 64, 256, and 1024 produce identical results.
- Empty sources and partial final batches work.
- NEON and scalar filter kernels produce identical survivor sets on AArch64.
- Non-AArch64 scalar fallback compiles.

## Search Gates

These must return no production matches:

```bash
rg "std::arch::x86|std::arch::x86_64|target_feature.*(sse|avx)|\b_mm_|\bavx\b|\bsse\b" crates
```

## Benchmark Passing Criteria

Run full traced JOB sample in scalar and vectorized modes.

Required evidence:

- Vectorized mode reports non-zero `batches_yielded`.
- Exact SQLite comparisons pass in both modes.
- Vectorized mode is not slower than scalar by more than 5% on the full JOB sample after warmup.
- Any default-mode switch must be justified by measured improvement.
- No x86 SIMD exists.
