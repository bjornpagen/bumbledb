# PRD 09: JOB Performance Honesty Gates

## Goal

Keep JOB fast while making the performance story honest.

JOB q09/q16/q24 should stay recovered, but gates must distinguish cached-result performance from recompute performance.

## Explicit Non-Goals

- No backwards compatibility with old benchmark gate semantics.
- No accepting q09/q16/q24 regressions.
- No hiding prepared result-cache hits.
- No relation-name-specific engine code.
- No SQLite comparison tricks that change row semantics.

## Required Gates

Add separate gates for benchmark cache modes from PRD 06.

### Prepared Result Mode

These may be very fast:

```text
q09 < 3000us
q16 < 1000us
q24 < 1000us
```

### Prepared Plan/Recompute Mode

These should be honest about first-run/precompute cost:

```text
q09 recompute/prepared-plan target documented from actual artifact
q16 recompute/prepared-plan target documented from actual artifact
q24 recompute/prepared-plan target documented from actual artifact
```

Set thresholds only after measuring. Do not make up thresholds.

## Required Benchmark JSON

For each JOB query, report:

```text
cache_mode
prepared_result_cache_hits
static_empty_cache_hits
static_semijoin_proof_us
direct_count_us
```

## Required Tests

- q09 gate fails if runtime is broad LFTJ.
- q09 gate identifies if result cache was used.
- q16/q24 gates fail if static proof does not run in prepared-result mode.
- q16/q24 recompute measurements report proof time honestly.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- JOB 10k benchmark passes in configured gate mode.
- JOB benchmark artifact clearly labels cache behavior.

## Completion Criteria

- JOB remains fast.
- JOB benchmark output is not misleading.
- This PRD is deleted and committed after passing.
