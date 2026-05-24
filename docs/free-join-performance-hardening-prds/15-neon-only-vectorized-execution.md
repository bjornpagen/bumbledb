# PRD 15: NEON-Only Vectorized Execution

## Purpose

Implement real vectorized execution using AArch64 NEON only. No x86 vectorization is allowed.

## Required Policy

- Explicit SIMD code may only use `std::arch::aarch64`.
- Explicit SIMD code must be behind `#[cfg(target_arch = "aarch64")]`.
- Non-AArch64 builds must compile and run scalar code.
- Do not use `std::arch::x86`, `std::arch::x86_64`, SSE, AVX, AVX2, AVX-512, x86 runtime dispatch, or x86-specific portable SIMD wrappers.
- Do not introduce a dependency that silently enables x86 SIMD kernels.

## Required Targets

Implement NEON acceleration only where traces prove value:

- fixed-width equality filter scans for 8-byte values;
- fixed-width range comparisons for 8-byte ordered encodings if beneficial;
- batch key comparison/probe preparation where contiguous columns allow it;
- survivor bitmap or index compaction for filter results.

## Required Design

- NEON kernels must operate over contiguous base-image column buffers from PRD 09.
- Kernels must produce identical survivor offsets as scalar code.
- Scalar and NEON paths must be tested against each other on AArch64.
- NEON paths must expose counters: SIMD lanes processed, scalar tail processed, survivors.
- Vectorized executor batching must use real batches from PRD 10, not materialized all-tuples chunks.

## Required Benchmarking

Run on Apple Silicon or another AArch64 NEON host:

```bash
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 5 --warmup 2 --trace summary --alloc on
```

## Passing Criteria

- The x86 forbidden search from PRD 00 returns no matches under `crates/`.
- NEON code compiles on AArch64.
- Scalar fallback compiles on non-AArch64 in CI or by explicit check if available.
- NEON and scalar filter tests produce identical survivor offsets.
- Trace indicates whether scalar or NEON filter kernels ran.
- JOB exact SQLite comparison passes.
- Global acceptance from PRD 00 passes.
