# PRD 04: JOB q09/q24 Regression Gates

## Goal

Add explicit regression gates so q09 and q24 cannot silently fall back to broad LFTJ traversal again.

PRDs 02 and 03 recover performance. This PRD turns the recovery into a permanent guardrail.

## Explicit Non-Goals

- No backwards-compatible weak gates for old slow v2 behavior.
- No accepting old `pure_lftj` fallback for q09/q24 when structural fast paths apply.
- No preserving old benchmark thresholds if they tolerate regressions.
- No migration path for old result artifacts.
- No hidden allowlist for known regressions.

## Required Benchmark Gates

Update benchmark gate definitions in `crates/bumbledb-bench/src/main.rs` or wherever gates are currently defined.

Add gates for JOB 10k/open-limit 10000 behavior.

Recommended thresholds:

```text
job_q09_voice_us_actor: max_bumbledb_avg_micros = 3000
job_q24_voice_keyword_actor: max_bumbledb_avg_micros = 1000
```

These are intentionally looser than old best numbers to avoid machine-noise failures while still catching the current 200ms blowups.

## Required Plan-Family Gates

Add one or both:

```text
q09 allowed runtime: DirectKernel or equivalent CountOnly factorized path
q24 allowed runtime: StaticEmpty or equivalent static semijoin empty path
```

If the benchmark gate framework cannot express this directly, add assertions to unit/integration tests against query plan/explain output.

## Required Counter Gates

For q09:

```text
factorized_counted_bindings > 0
direct_kernel_probes > 0
```

For q24:

```text
static_empty_cache_misses > 0 OR static_semijoin_* counters indicate proof path
lftj_execute_us should be near 0
```

If exact counter names differ after PRDs 02/03, use the new names.

## Required Tests

Add focused tests that do not need the CWI dataset:

- Synthetic q09-like count query with literals and range picks factorized count.
- Synthetic q24-like empty semijoin query picks static empty.
- Both tests compare against materialized execution.

Add benchmark gate test:

- gate definitions include q09 and q24.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- JOB 10k benchmark with `--fail-gates` passes.
- q09 and q24 cannot use broad `pure_lftj` when the structural fast path applies.

## Completion Criteria

- Performance regression is guarded by tests and benchmark gates.
- Any future cleanup that loses the structural fast path fails loudly.
- This PRD is deleted and committed after passing.
