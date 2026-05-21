# PRD 06: Benchmark Cache Mode Separation

## Goal

Make benchmark output honest about cache behavior.

The current JOB numbers are impressive, but some of them measure repeated prepared-result-cache hits rather than recomputation. That is useful, but it must be labeled separately from raw execution.

## Explicit Non-Goals

- No backwards compatibility for benchmark JSON schema.
- No preserving ambiguous `bumbledb_avg` semantics.
- No hiding cache hits in headline numbers.
- No removing caches from the engine.
- No changing SQLite behavior to mimic Bumbledb result caches.

## Required Benchmark Modes

Add explicit benchmark cache modes:

```rust
enum CacheMode {
    Recompute,
    PreparedPlan,
    PreparedResult,
}
```

Exact CLI spelling may be:

```text
--cache-mode recompute|prepared-plan|prepared-result
```

Defaults should be conservative and honest. Recommended default:

```text
prepared-plan
```

Meaning:

- `recompute`: rebuild normalized/prepared state as much as normal API permits, no prepared result cache reuse.
- `prepared-plan`: reuse prepared query/plan/image caches, but do not use prepared result count cache.
- `prepared-result`: allow prepared count/static result caches.

If disabling prepared result cache requires API changes, add explicit controls to `InputBindings` or execution options.

## Required JSON Fields

Add to every benchmark result:

```json
"cache_mode": "prepared-plan",
"prepared_result_cache_allowed": false,
"prepared_result_cache_hit": false,
"static_empty_cache_hit": false,
"query_image_cache_hit": true
```

The exact field names may differ, but the artifact must identify whether a sample is cache-assisted.

## Required Text/Markdown Output

Text output should include:

```text
cache_mode=prepared-plan prepared_result_cache_hits=0 static_empty_cache_hits=N
```

Markdown tables should include cache mode or have a separate cache diagnostics table.

## Required Engine API

Add execution options if needed:

```rust
pub struct QueryExecutionOptions {
    pub allow_prepared_result_cache: bool,
    pub allow_static_empty_fast_cache: bool,
}
```

Do not add a global mutable flag. Options must be explicit per benchmark/API call or encapsulated in benchmark code without changing user-facing API if not needed.

## Required Tests

- Prepared result cache mode reports cache hits.
- Prepared-plan mode does not use prepared result cache.
- Static-empty cache hits are reported separately.
- JSON output includes cache mode fields.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- non-JOB and JOB benchmark JSON include cache mode fields.

## Completion Criteria

- Benchmark users can tell whether numbers are recompute, prepared-plan, or prepared-result-cache timings.
- JOB q09/q16/q24 no longer look mysteriously magical.
- This PRD is deleted and committed after passing.
