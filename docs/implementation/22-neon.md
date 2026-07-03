# PRD 22 — NEON Kernels

Authority: `docs/architecture/30-execution.md` (D4 — the two kernel shapes, scalar
parity), `00-product.md` (aarch64-only SIMD, scalar fallback compiles everywhere).

## Purpose

The only explicit SIMD in the system: fixed-width predicate scans and survivor
compaction, NEON 128-bit, behind scalar-identical signatures.

## Technical direction

- `exec::kernel` module — **the one module where `unsafe` is sanctioned** (NEON
  intrinsics via `core::arch::aarch64`), every block `// SAFETY:`-commented; the whole
  module `#[cfg(target_arch = "aarch64")]` with the scalar implementations (from
  PRDs 12/21) as the `#[cfg(not(...))]` fallback *and* as the reference the tests
  compare against on aarch64 (compiled wherever it has a reader —
  `cfg(any(not(aarch64), test))` — since an aarch64 release build would hold it as
  dead code, which this suite forbids).
- Kernel 1 — predicate scan: `filter_eq_u64 / filter_range_u64(col: &[u64], lo, hi,
  out_positions: &mut ...)` over 2×u64 lanes (and a `u8` variant for enum/bool
  columns, 16 lanes); writes survivor positions branchlessly (compare → mask →
  extract).
- Kernel 2 — survivor compaction: compact position/NodeRef arrays by mask.
- Wire into PRD 12's `apply` and PRD 21's compaction behind the existing signatures —
  call sites do not change (representation seam, no branches at callers beyond the
  cfg).
- x86 grep-guard honored: no `x86` intrinsics anywhere (manual check, not a CI gate).

## Non-goals

Any other kernel (hash mixing, decode — scalar is the doctrine). Autovectorization
tuning.

## Passing criteria

- On aarch64: property tests comparing every kernel against its scalar reference over
  randomized columns (lengths 0, 1, odd, lane-multiples ±1; extreme values; empty and
  full survivor sets) — bit-identical outputs, order preserved.
- PRD 12/21 test suites green with kernels wired in.
- Cross-compile check for a non-aarch64 64-bit target compiles the fallback
  (`cargo check --target x86_64-unknown-linux-gnu` documented as the verification
  command).
- Global commands green; `unsafe` confined to this module.
