# PRD 01: Measurement And Allocation Contract

## Status

Proposed.

## Motivation

The current benchmark output is useful but easy to misinterpret. The JOB trace analysis showed that `job-results.json` mixes three different ideas:

- A first Bumbledb query execution called `prepare` by the benchmark harness.
- Two warmup executions.
- Thirty measured sample executions.

Allocation telemetry in `QueryPlan.allocations` comes from the first Bumbledb execution whose `QueryOutput` is retained in the result, not from the 30 measured sample executions. That is valid cold-path product data, but it is not a per-sample allocation profile.

Before changing internal data structures, the benchmark contract must make cold/warm/sample metrics explicit. Otherwise future optimization work can appear to improve or regress the wrong phase.

## Evidence

| Evidence | Code or trace anchor |
|---|---|
| Benchmark first executes Bumbledb once and names elapsed time `bumble_prepare` | `crates/bumbledb-bench/src/main.rs:812-831` |
| Then it executes two Bumbledb warmups and thirty Bumbledb samples | `crates/bumbledb-bench/src/main.rs:846-883` |
| SQLite also prepares/counts in each call via `conn.prepare(sql)` | `crates/bumbledb-bench/src/main.rs:967-976` |
| Result JSON serializes `output.plan.timings` and `output.plan.allocations` from the retained first output | `crates/bumbledb-bench/src/main.rs:990-1047`, `1589-1650` |
| q09 allocation telemetry is dominated by first-run `lftj_build`, even though samples hit the cached build | `docs/job-trace-analysis/04-job_q09_voice_us_actor.md:115-143` |
| q33 allocation telemetry includes first-run static-empty proof miss, while samples are cached static-empty hits | `docs/job-trace-analysis/08-job_q33_linked_series_companies.md:89-121` |

## Problem Statement

The benchmark schema must distinguish:

- Correctness execution.
- Cold first execution.
- Warm first execution after caches are populated.
- Measured samples.
- Materialized-output mode.
- Row-count-only mode.
- Allocation telemetry scope.

The current names blur these together and make future PRD gates ambiguous.

## Goals

- Make benchmark output self-describing enough that agents can distinguish cold product cost from warm sample cost.
- Preserve the ability to compare Bumbledb materialized outputs with SQLite row counts for correctness.
- Keep allocation telemetry honest: explicitly name which execution it describes.
- Add optional per-sample allocation summaries when `alloc-profile` is enabled without bloating default runs.
- Update markdown and JSON output fields in a breaking way if needed. No compatibility shims.

## Non-Goals

- Do not add a new external benchmark framework.
- Do not hide cold costs by prewarming unless the output labels them separately.
- Do not optimize query execution in this PRD.
- Do not change query semantics.

## Current Code Map

| Component | Anchor | Current behavior |
|---|---|---|
| Benchmark loop | `crates/bumbledb-bench/src/main.rs:806-920` | Executes materialized correctness run, optional count run, warmups, samples, then renders one result |
| Timing stats | `crates/bumbledb-bench/src/main.rs:638-655` | Computes sample stats from measured sample vector only |
| Result construction | `crates/bumbledb-bench/src/main.rs:987-1065` | Pulls plan/timing/allocation from one retained `QueryOutput` |
| JSON render | `crates/bumbledb-bench/src/main.rs:1589-1668` | Emits `prepare`, `warmup`, `phase_timing`, `allocations` under a single result object |
| Markdown render | `crates/bumbledb-bench/src/main.rs:1380-1476` | Presents phase timing and allocation tables without saying allocation scope is first execution |
| Count-only path | `crates/bumbledb-lmdb/src/query.rs:1597-1761` | Returns `QueryCountOutput`, not materialized rows |
| Count-only result wrapper | `crates/bumbledb-bench/src/main.rs:1068-1077` | Fabricates `QueryOutput` with empty rows and count plan |

## Required Output Model

Replace the ambiguous result vocabulary with explicit sections.

### Per Query JSON Shape

The future JSON may be breaking. Suggested shape:

```json
{
  "dataset": "job",
  "query": "job_q09_voice_us_actor",
  "result": {
    "logical_rows": 1,
    "materialized_rows": 1,
    "materialized_values": 1,
    "output_mode": "materialized"
  },
  "bumbledb": {
    "correctness_execution": {...},
    "cold_execution": {...},
    "warmup": {...},
    "samples": {...}
  },
  "sqlite": {
    "correctness_execution": {...},
    "cold_execution": {...},
    "warmup": {...},
    "samples": {...}
  },
  "allocation_scope": "bumbledb.cold_execution",
  "query_image_scope": "full_schema",
  "compare_mode": "materialized"
}
```

Fields may use Rust structs rather than dynamic maps, but output must communicate the above distinctions.

### Execution Record

Each execution record should include:

- `elapsed_us`
- `plan_family`
- `runtime`
- `query_image_built`
- `query_image_scope`
- `phase_timing`
- `allocations` when profiling is enabled
- `counters`
- `materialized_rows`
- `logical_rows`
- `output_values`

### Sample Stats

Sample stats should retain:

- `samples`
- `total_us`
- `avg_us`
- `min_us`
- `p50_us`
- `p95_us`
- `max_us`

If per-sample allocation tracking is added, it should be a separate `sample_allocation_stats` field with avg/min/max allocation calls and bytes.

## Implementation Plan

### Step 1: Rename The Existing First Execution

Change benchmark data structs so `prepare` becomes `cold_execution` or `first_execution`.

Code anchors:

- Update `QueryTimingSamples` near `crates/bumbledb-bench/src/main.rs:626-635`.
- Update `BenchmarkResult` near `crates/bumbledb-bench/src/main.rs:532-585`.
- Update `benchmark_result` near `crates/bumbledb-bench/src/main.rs:987-1065`.
- Update JSON render near `crates/bumbledb-bench/src/main.rs:1589-1668`.
- Update markdown render near `crates/bumbledb-bench/src/main.rs:1380-1476`.

Do not keep old `prepare` fields unless they are explicitly named aliases in a transition-only local branch. The repo is unstable; prefer breaking JSON now.

### Step 2: Split Correctness From Cold Execution

The current first Bumbledb execution is used both as correctness materialization and cold metric. Keep that if needed, but label it explicitly.

Suggested states:

- `bumbledb_correctness_execution`: materialized query used for row-match validation.
- `bumbledb_cold_execution`: may be the same measurement as correctness in materialized mode, but the output must say so.
- `bumbledb_count_cold_execution`: count-only cold run when `compare_mode=rows`.

For materialized mode, it is acceptable that correctness and cold are the same execution. The output must contain `cold_execution_uses_correctness_output: true`.

For row-count mode, current code first materializes for correctness and then runs count-only as `bumble_prepare` at `crates/bumbledb-bench/src/main.rs:817-829`. Label this clearly:

- correctness materialized execution warms caches.
- count-only cold execution is not truly cold if run after materialized correctness.
- If we want true count-only cold metrics, add a separate benchmark path that does correctness after measuring cold count.

### Step 3: Add Allocation Scope Field

Add a required field:

```text
allocation_scope = "bumbledb.correctness_execution"
allocation_scope = "bumbledb.count_cold_execution"
allocation_scope = "none"
```

This prevents future agents from treating q09 first-run `lftj_build` allocations as per-sample allocations.

### Step 4: Report Query Image Scope

Add a `query_image_scope` string now, even before scoped images exist.

Initial values:

- `full_schema`
- `skipped_for_streaming_dataset_before_query`
- `not_applicable`

Later PRDs will add scoped values such as `relations:Title,MovieCompanies` or a structural scope fingerprint.

### Step 5: Optional Per-Sample Allocation Summary

When `alloc-profile` is enabled, wrap measured sample closures in allocation snapshots and collect a small stats vector.

Do not do this by default if it distorts timing. It can be behind a CLI flag such as:

```text
--sample-allocations
```

The user asked for allocation tracing. Future agents need per-sample allocations when killing hot-path allocation. This PRD should add the measurement hook but may leave it disabled by default.

## Data Structures To Introduce

Suggested new internal structs in `crates/bumbledb-bench/src/main.rs`:

```rust
struct ExecutionMeasurement {
    elapsed: Duration,
    output: Option<QueryOutput>,
    count_output: Option<QueryCountOutput>,
    output_mode: OutputMode,
    allocation_scope: AllocationScope,
}

enum OutputMode {
    Materialized,
    CountOnly,
}

enum AllocationScope {
    CorrectnessExecution,
    ColdExecution,
    CountColdExecution,
    SampleSummary,
    None,
}
```

Avoid overengineering. The structs exist to make result construction explicit.

## Acceptance Criteria

- JSON output no longer uses ambiguous `prepare.bumbledb_us` as the only name for first execution.
- JSON output explicitly states allocation scope for each query.
- Markdown output explicitly separates first execution, warmups, and samples.
- Existing benchmark rows still validate against SQLite.
- Existing CLI presets still work.
- Trace analysis can still map query order as first execution + warmups + samples.
- No source compatibility requirement for old JSON field names.

## Test Plan

### Unit Tests

- Update existing JSON render tests near `crates/bumbledb-bench/src/main.rs:2739-2910`.
- Add a test that JSON contains `allocation_scope`.
- Add a test that materialized mode marks `cold_execution_uses_correctness_output=true` or equivalent.
- Add a test that row mode marks count cold execution as warmed by correctness if retaining current order.

### Command Tests

Run:

```sh
cargo test -p bumbledb-bench
cargo run -p bumbledb-bench --release -- --preset quick --format json
cargo run -p bumbledb-bench --release -- --preset nonjob --query triangle_count --format json
```

### Benchmark Smoke

Run one JOB query:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q33_linked_series_companies
```

## Regression Risks

- Existing scripts that parse old JSON fields will break. This is acceptable; update scripts in the same change if they are in-repo.
- Adding per-sample allocation snapshots can distort timing. Keep disabled unless explicitly requested.
- Splitting correctness and cold execution can change cache state. Label it honestly and do not compare new cold metrics against old ambiguous prepare metrics without explanation.

## Definition Of Done

- Benchmark result schema is explicit about cold/warm/sample and allocation scope.
- Markdown and JSON renderers are updated.
- Existing gates still pass.
- `docs/job-trace-analysis` conclusions remain interpretable under the new naming.
- Future PRDs can refer to cold execution and sample execution without ambiguity.
