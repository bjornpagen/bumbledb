# PRD 06: Width-Specialized Encoded Operations

## Goal

Specialize encoded comparison, equality, range, and intersection operations by encoded width to improve direct kernels, static proof, and LFTJ traversal.

This is the vectorization/SIMD PRD, but it must be driven by counters and hot paths from PRD 01-05.

The SIMD target is ARM NEON only. Do not implement x86/x86_64 SIMD paths. Do not add SSE, AVX, AVX2, AVX-512, or runtime x86 feature detection. If the active platform is not ARM with NEON support, the scalar fallback remains the implementation.

## Background

Bumbledb is unusually suitable for width specialization:

- enums are width 1
- serials/integers/timestamps are width 8
- decimals and some composite values are width 16
- query images and trie keys are encoded and order-preserving
- decoding is intentionally delayed

Trace evidence suggests vectorization is promising only after per-binding output mechanics are reduced. This PRD comes after projection/direct/LFTJ mechanics.

## Explicit Non-Goals

- No backwards compatibility.
- No new join algorithm.
- No unsafe SIMD without scalar fallback and tests.
- No x86/x86_64 SIMD.
- No SSE, AVX, AVX2, or AVX-512 code.
- No CPU-specific hard requirement unless guarded by ARM/NEON detection.
- No changing encoded ordering semantics.
- No decoding just to compare values.

## Code Anchors

Expected areas:

```text
EncodedOwned
EncodedRef
SortedTrieIndex
LftjTrieIter
LeapfrogState
static_atom_row_matches
static_atom_entry_matches
encoded_comparison_supported
compare_encoded_values
direct_row_satisfies_atom
query_access.rs
```

## Required Width Abstraction

Introduce a small explicit width dispatch layer:

```rust
enum EncodedWidth {
    W1,
    W8,
    W16,
}
```

or equivalent existing encoded width type.

Provide specialized functions:

```rust
eq_w1(slice, value)
eq_w8(slice, value)
eq_w16(slice, value)
range_w8(slice, lower, upper)
intersect_sorted_w1(...)
intersect_sorted_w8(...)
```

The API should make the hot loop choose width once outside the row loop.

Do not branch on width inside every row if the width is known for the atom/index/column.

## SIMD Policy

Use scalar specialization first if that is simpler. SIMD comes after scalar width dispatch is clean.

Allowed SIMD approaches:

- ARM NEON intrinsics via Rust `std::arch::aarch64` where the target supports them
- architecture-specific NEON modules behind `cfg(target_arch = "aarch64")` or other ARM cfgs when correct
- chunked scalar loops that are vectorizer-friendly

Required fallback:

- scalar path for every specialized operation
- tests must pass without SIMD

Forbidden SIMD approaches:

- `std::arch::x86`
- `std::arch::x86_64`
- `is_x86_feature_detected!`
- SSE/AVX intrinsics
- x86-only dependencies

## Target Operations

### Static Proof

Specialize static literal/input proof over relation images:

- width 1 equality scan
- width 8 equality scan
- width 8 range scan

### Direct Kernels

Specialize direct row predicate checks:

- repeated variable equality on width 8
- input/literal equality on width 1 and 8
- range predicates on width 8 timestamps/integers

### LFTJ

Specialize leapfrog key comparisons:

- compare width 1 keys without generic slice comparison
- compare width 8 keys as `u64`/ordered bytes where safe
- avoid `EncodedOwned` allocation/copy where borrowed key bytes suffice

### Query Image Columns

If column scans are still hot after PRD 03-05:

- use width-specific column scan loops
- avoid decoding during scan

## Required Tests

- Width 1 enum equality matches scalar generic path.
- Width 8 serial equality matches scalar generic path.
- Width 8 timestamp/integer range comparisons preserve encoded ordering.
- Width 16 decimal comparisons preserve encoded ordering or fall back to generic safely.
- LFTJ specialized intersection matches generic intersection on randomized small fixtures.
- Static proof specialized scans match generic proof results.
- Direct predicate specialized checks match generic checks.
- Tests run on machines without NEON features by using the scalar fallback.
- ARM NEON implementations are behind explicit cfg/runtime guards.
- Grep active Rust code to ensure no x86 SIMD path was introduced.

## Required Benchmarks

Run:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-width-specialized-nonjob.json
```

Run JOB:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-width-specialized-job-10k.json
```

Focused expected beneficiaries:

```text
sailors/red_boat_sailors
sailors/high_rating_red_boats
job_q16_character_title_us
job_q24_voice_keyword_actor
```

## Performance Targets

Hard gates:

- all existing gates pass

Optimization targets:

- q16/q24 static proof improves by at least 10%, or RCA explains why not
- at least two non-JOB LFTJ materialized hot queries improve by at least 10%, or RCA explains why not
- no direct kernel query regresses by more than 5%

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB gates pass
- JOB 10k gates pass
- scalar fallback tests pass
- active Rust code contains no `std::arch::x86`, `std::arch::x86_64`, `is_x86_feature_detected`, `avx`, `sse`, or `x86` SIMD implementation
- no decoded comparisons are introduced into hot encoded paths

## Completion Criteria

- Encoded operations have explicit width-specialized ownership.
- SIMD or scalar specialization is measured, not speculative.
- This PRD is deleted and committed after passing.
