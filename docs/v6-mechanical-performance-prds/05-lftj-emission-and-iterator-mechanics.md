# PRD 05: LFTJ Emission And Iterator Mechanics

## Goal

Reduce pure Free Join/LFTJ overhead for high-output materialized queries by improving iterator mechanics and output emission.

Pure Free Join/LFTJ is the database backbone. This PRD optimizes it, not replaces it.

## Background

The retained general join engine is pure LFTJ. Non-JOB hot queries show high LFTJ execute time:

```text
red_boat_sailors: high LFTJ execute, 20000 output values
high_rating_red_boats: high LFTJ execute, 13320 output values
revenue_by_customer_range: high LFTJ execute, 4000 output values
supplier_nation_orders: high LFTJ execute, 11432 output values
triangle_count: cyclic count workload, LFTJ backbone win
```

Heavy tracing showed millions of per-binding sink events for non-JOB materialized workloads. PRD 03 handles sink layout. This PRD handles LFTJ traversal and emit integration.

## Explicit Non-Goals

- No backwards compatibility.
- No new join algorithm.
- No mixed/hash-probe revival.
- No relation-name-specific logic.
- No changing set semantics.
- No changing logical query semantics.

## Code Anchors

Expected areas:

```text
execute_lftj
LftjExecutor
LftjRuntime
LftjTrieIter
LeapfrogState
build_lftj_atom_plans
SortedTrieIndex
PrefixRows
OutputSink / EncodedProjectSink
PlanCounters trie/lftj counters
```

## Required Investigation From PRD 01 Counters

Before editing LFTJ mechanics, identify for each hot query:

```text
lftj_open_calls
lftj_up_calls
lftj_next_calls
lftj_seek_calls
lftj_key_reads
lftj_candidate_values
lftj_completed_bindings
sink_emit_calls
encoded_project_rows_seen
```

Use this to classify each query:

```text
iterator traversal dominated
sink emission dominated
dedup dominated
decode dominated
build dominated
```

## Required Mechanical Improvements

Implement only improvements justified by counters. Candidate improvements:

### 1. Emit Integration

If `sink_emit_calls` is high, integrate LFTJ full-binding output with batched encoded projection from PRD 03:

- precompute projection layout once
- avoid virtual/enum dispatch on every emit where possible
- append encoded row bytes directly when output is `Project`
- keep generic `TupleSink` only for aggregate/count/fallback

### 2. Iterator Operation Reduction

If `lftj_key_reads` or `lftj_next_calls` dominate:

- cache current key per iterator frame when safe
- avoid repeated owned key construction when borrowed bytes suffice
- avoid copying `EncodedOwned` until a value survives intersection
- specialize `LeapfrogState` for common arity 2 and 3 if clean

### 3. Binding State Reuse

If binding/unbinding dominates:

- reuse a compact binding array
- avoid clone-heavy `EncodedOwned` paths
- use fixed-size small arrays for common variable counts
- preserve correctness for repeated-variable atoms

### 4. Predicate Timing

If predicate checks are repeated too often:

- evaluate encoded predicates as early as possible
- precompute predicate readiness depth
- avoid decoded comparisons when encoded comparison is supported

## Required Tests

- Existing LFTJ correctness tests pass.
- Cyclic triangle count remains correct.
- High-output materialized LFTJ query remains correct and deduplicated.
- Repeated-variable atoms still enforce equality.
- Static existence atoms are still checked.
- Predicate readiness remains deterministic.
- LFTJ counters are accurate enough to distinguish open/next/seek/key reads.
- Batched projection integration does not decode during emit.

## Required Benchmarks

Run:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-lftj-mechanics-nonjob.json
```

Focused:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob \
  --query red_boat_sailors \
  --query high_rating_red_boats \
  --query triangle_count \
  --query revenue_by_customer_range \
  --query supplier_nation_orders \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-lftj-mechanics-focused.json
```

Run JOB 10k:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-lftj-mechanics-job-10k.json
```

## Performance Targets

Hard gates:

- all existing gates pass

Optimization targets:

- `red_boat_sailors` improves by at least 20% over v6 PRD 01 baseline, or RCA explains why not
- `high_rating_red_boats` improves by at least 20%, or RCA explains why not
- `revenue_by_customer_range` improves by at least 15%, or RCA explains why not
- `supplier_nation_orders` improves by at least 15%, or RCA explains why not
- `triangle_count` does not regress by more than 5%

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB gates pass
- JOB 10k gates pass
- LFTJ mechanics counters are visible in JSON
- no mixed/hash-probe code returns

## Completion Criteria

- Pure LFTJ remains the backbone and becomes mechanically faster.
- Hot materialized joins show lower iterator/emit overhead or have documented blockers.
- This PRD is deleted and committed after passing.
