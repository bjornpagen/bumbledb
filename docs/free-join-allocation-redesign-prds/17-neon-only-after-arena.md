# PRD 17: NEON Only After Arena

## Purpose

Add real AArch64 NEON kernels only after arena COLT and contiguous column buffers are stable.

## Hard Policy

- Explicit SIMD may only use `std::arch::aarch64`.
- SIMD code must be behind `#[cfg(target_arch = "aarch64")]`.
- No x86, x86_64, SSE, AVX, AVX2, AVX-512, or x86 runtime dispatch.
- Non-AArch64 builds must compile and run scalar code.

## Required Targets

- 8-byte equality filter scans.
- 8-byte ordered range filters if measured useful.
- Survivor offset compaction if measured useful.
- Batch key comparison only if arena batches expose contiguous key data.

## Passing Criteria

- Scalar and NEON filter outputs are identical on AArch64 tests.
- Non-AArch64 scalar fallback compiles.
- Trace/counters indicate scalar or NEON kernel use.
- x86 forbidden search returns no matches under `crates/`.
- JOB exact SQLite comparison passes.
- No-trace allocation budgets do not regress.
- Global gates pass.
