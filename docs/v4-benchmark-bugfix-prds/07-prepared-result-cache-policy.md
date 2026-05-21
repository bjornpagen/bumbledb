# PRD 07: Prepared Result Cache Policy

## Goal

Define and enforce when prepared result caches are allowed.

Prepared result caching is semantically valid for immutable snapshots, but it should not contaminate recompute benchmarks or broad query semantics accidentally.

## Explicit Non-Goals

- No backwards compatibility for old cache behavior.
- No silent prepared result cache use in benchmark modes that disallow it.
- No global process-wide cache policy switch.
- No persistent result cache across database opens.
- No cache entries that survive snapshot changes.

## Current Code Anchors

- `PreparedQuery`
- `count_cache`
- `prepared_count_cache_key`
- `cached_prepared_count_output`
- `static_empty_fast_cached`
- `QueryImageCache`

## Required Policy

Prepared result caches may be used only when all are true:

- caller explicitly allows result cache
- query is deterministic under current snapshot
- cache key includes schema fingerprint
- cache key includes storage tx id
- cache key includes normalized query shape
- cache key includes encoded inputs

Prepared result caches must be bypassed when benchmark mode is `recompute` or `prepared-plan`.

## Required Diagnostics

Add counters:

```rust
prepared_result_cache_hits
prepared_result_cache_misses
prepared_result_cache_inserts
prepared_result_cache_bypasses
```

These can live in `PlanCounters`, `PreparedPlanCacheDiagnostics`, or a new diagnostics struct.

## Required Tests

- Cache hit on repeated prepared result when allowed.
- Cache bypass when disabled.
- Cache miss after write transaction changes tx id.
- Different input binding misses.
- Diagnostics distinguish hit, miss, insert, and bypass.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- Benchmark prepared-plan mode proves q09 is not using prepared result cache.
- Benchmark prepared-result mode proves q09 is using prepared result cache.

## Completion Criteria

- Prepared result caching is explicit and observable.
- Benchmark modes can choose whether to include it.
- This PRD is deleted and committed after passing.
