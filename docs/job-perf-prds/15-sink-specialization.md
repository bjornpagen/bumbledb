# PRD 15: Sink Specialization

## Status

Proposed.

## Motivation

The generic output sink is correct but too heavyweight for common JOB benchmark result shapes:

- Global `count` aggregate with one output row.
- Static-empty zero-row output.
- Tiny distinct projection output, e.g. q24 emits 219 bindings for 12 rows.

Current sinks use `BTreeSet<SmallEncodedRow>` and `BTreeMap<SmallEncodedRow, Vec<AggregateState>>` even when a scalar or tiny vector would do.

This PRD specializes sinks after the larger image/LFTJ/direct/static changes are in place.

## Evidence

| Current behavior | Anchor |
|---|---|
| `OutputSink::new` only distinguishes project vs aggregate | `crates/bumbledb-lmdb/src/query.rs:7557-7568` |
| Project sink stores rows in `BTreeSet<SmallEncodedRow>` | `crates/bumbledb-lmdb/src/query.rs:7617-7665` |
| Count-only sink uses two `BTreeSet`s | `crates/bumbledb-lmdb/src/query.rs:7668-7748` |
| Aggregate sink stores `BTreeMap<SmallEncodedRow, Vec<AggregateState>>` | `crates/bumbledb-lmdb/src/query.rs:7750-7881` |
| Global count direct kernels still emit via aggregate sink | `crates/bumbledb-lmdb/src/query.rs:2776-2782`, `2876-2884` |
| q24 emits 219 bindings per execution for 12 output rows | `docs/job-trace-analysis/06-job_q24_voice_keyword_actor.md:70-78` |
| q09 emits 712,620 sink events across 30 samples, under 1% but visible | `docs/job-trace-analysis/04-job_q09_voice_us_actor.md:68-75` |

## Goals

- Add specialized sink variants for global count, static empty, and tiny project/dedup.
- Avoid `BTreeMap` and `BTreeSet` for single-group count.
- Avoid per-count loop in `TupleSink::emit_count_range` fallback for aggregate count.
- Keep Datalog set semantics.
- Preserve materialized output ordering.
- Preserve aggregate overflow behavior.

## Non-Goals

- Do not change join traversal algorithms.
- Do not change logical aggregate semantics.
- Do not remove generic sinks; they remain fallback for complex grouped aggregates and large distinct projections.

## Current Sink Problems

### Global Count

For `find count(?x)` with no group vars:

- There is exactly one aggregate state.
- The group key is empty.
- Generic `AggregateSink` still creates a `BTreeMap` entry with empty `SmallEncodedRow` and a `Vec<AggregateState>`.

### Project Dedup

For tiny projected outputs:

- `EncodedProjectSink` uses `BTreeSet` from the first row.
- q24 has only 12 output rows after 219 emits; a small vector with linear dedup would likely be cheaper.

### Count-Only

`CountOnlySink` does not materialize values, but it still uses sets for project/group counting. That is required for distinct semantics but can be specialized for global count.

## Proposed Sink Enum

Replace `OutputSink` variants with:

```rust
enum OutputSink {
    StaticEmpty,
    GlobalCount(GlobalCountSink),
    TinyProject(TinyProjectSink),
    Project(EncodedProjectSink),
    GroupedCount(GroupedCountSink),
    Aggregate(AggregateSink),
}
```

Construction rules:

- `StaticEmpty` only from static-empty path, not generic output plan.
- `GlobalCount` when output is aggregate, no group vars, one `count` aggregate.
- `GroupedCount` when all aggregate terms are count and group vars exist.
- `TinyProject` when projected var count is small and expected output is small or unknown but can upgrade.
- `Aggregate` fallback for sum/min/max or mixed aggregate terms.
- `Project` fallback for large/tiny overflow cases.

## GlobalCountSink

```rust
struct GlobalCountSink {
    count: u64,
}
```

Behavior:

- `emit` increments count by 1 with overflow check.
- `emit_count_range` increments count by `count` with overflow check.
- `finish` returns one row containing encoded/decoded count if count > 0, matching current `HAVING COUNT(*) > 0` semantics in benchmark queries.

Important: Current query semantics appear to return zero rows when count is zero for these benchmark aggregate queries. Preserve current behavior unless the logical aggregate semantics are explicitly changed. Static-empty currently returns zero rows.

## TinyProjectSink

Use a small vector first:

```rust
struct TinyProjectSink {
    vars: Box<[VarId]>,
    rows: SmallVec<[SmallEncodedRow; 16]>,
    overflow: Option<BTreeSet<SmallEncodedRow>>,
}
```

Behavior:

- For each emitted row, linear scan existing small rows.
- If row count exceeds threshold, move to `BTreeSet` fallback.
- Finish sorts rows if still small vector.

This preserves set semantics while reducing tree allocation for tiny outputs.

## GroupedCountSink

For grouped count-only aggregates:

- Store `BTreeMap<SmallEncodedRow, u64>` instead of `BTreeMap<SmallEncodedRow, Vec<AggregateState>>`.
- `emit_count_range` updates one group count.
- Finish decodes group key plus count.

## CountOnlySink Specialization

Change count-only sink construction similarly:

- Global count should store only `rows = if count > 0 { 1 } else { 0 }` or track count if needed for counters.
- Project count-only can use tiny set with fallback.
- Grouped count-only can use group key set or map depending semantics.

## Implementation Plan

### Step 1: Add Output Shape Helpers

Add helpers:

```rust
fn is_global_count(output: &OutputPlan) -> bool;
fn is_grouped_count(output: &OutputPlan) -> bool;
fn is_tiny_project_candidate(output: &OutputPlan) -> bool;
```

### Step 2: Implement GlobalCountSink

Add first because direct count kernels and many JOB queries use it.

Update `OutputSink::new` to select `GlobalCount`.

Update tests for zero-count behavior.

### Step 3: Implement TinyProjectSink

Add threshold constant, e.g.:

```rust
const TINY_PROJECT_THRESHOLD: usize = 32;
```

Use fallback `BTreeSet` after threshold.

### Step 4: Implement GroupedCountSink

Add for count-only grouped aggregate paths.

### Step 5: Update CountOnlySink Or Replace With OutputSink Mode

Current count-only uses separate `CountOnlySink`. Consider merging materialized/count-only sinks under one generic trait with output mode. If that is too large, specialize `CountOnlySink` separately.

Given no-tech-debt instruction, prefer unifying sink selection so count-only and materialized share output shape classification.

## Acceptance Criteria

- Global count does not allocate a `BTreeMap` or `Vec<AggregateState>`.
- Direct count kernels use `emit_count_range` into `GlobalCountSink` without looping.
- Tiny q24 projection uses small-vector dedup until threshold.
- Generic aggregate fallback remains correct.
- Count-only mode remains correct.

## Tests

### Unit Tests

- Global count emits one row for positive count.
- Global count emits zero rows for zero count if preserving current semantics.
- Global count overflow returns integer overflow error.
- Tiny project dedups duplicate rows.
- Tiny project falls back to `BTreeSet` after threshold and preserves sorted output.
- Grouped count returns correct grouped rows.
- Count-only global count returns correct row count.

### Integration Tests

Run:

```sh
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Gates

Run:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_broad_movie_info_star \
  --query job_movie_link_bridge \
  --query job_q24_voice_keyword_actor
```

Expected:

- Direct count sink finish allocation drops.
- q24 sink/project overhead drops modestly.
- q09 sink emit overhead may drop if global count uses scalar updates.
- No query regresses materially.

## Risks

- Aggregate zero-row semantics must match current behavior and SQLite comparison harness.
- Tiny project threshold tuning can affect performance; keep threshold simple and tested.
- Count-only and materialized semantics can diverge if implemented separately. Prefer shared classification helpers.

## Definition Of Done

- Global count is scalar, not map/vector based.
- Tiny project/dedup avoids tree allocation for small outputs.
- Grouped count avoids vector aggregate states when all terms are count.
- Materialized and count-only tests pass.
