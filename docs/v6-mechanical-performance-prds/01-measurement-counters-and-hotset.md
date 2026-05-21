# PRD 01: Measurement Counters And Hotset

## Goal

Add cheap, always-available benchmark counters that explain retained execution mechanics without requiring multi-gigabyte trace files.

This PRD does not optimize behavior. It creates the measurement substrate for the rest of v6.

## Background

Heavy tracing showed the next performance problem is high-frequency per-binding/per-row mechanics, especially in materialized non-JOB queries. However, tracing changes runtime behavior too much to be a normal performance tool.

We need cheap counters in normal benchmark JSON so future PRDs can make decisions from untraced runs.

## Explicit Non-Goals

- No backwards compatibility for benchmark JSON.
- No preserving old counter names if clearer names are better.
- No permanent feature flags.
- No trace-format dependency.
- No optimizer changes.
- No algorithm changes.

## Code Anchors

Expected areas:

```text
crates/bumbledb-lmdb/src/query.rs
PlanCounters
OutputSink
EncodedProjectSink
AggregateSink
execute_direct_kernel
execute_lftj
LeapfrogState
crates/bumbledb-bench/src/main.rs
BenchmarkRunResult
render_json_results
render_markdown_results
```

## Required Counter Additions

Add counters for output and binding mechanics:

```rust
sink_emit_calls
sink_emit_count_range_calls
bindings_completed
encoded_project_rows_seen
encoded_project_rows_inserted
encoded_project_duplicate_rows
encoded_project_row_bytes
project_decode_values
aggregate_emit_calls
aggregate_count_range_calls
```

Add counters for direct materialization:

```rust
direct_bind_attempts
direct_bind_successes
direct_chain_steps
direct_chain_step_rows
direct_chain_output_rows
direct_chain_output_values
direct_storage_output_rows
```

Add counters for LFTJ iterator mechanics:

```rust
lftj_open_calls
lftj_up_calls
lftj_next_calls
lftj_seek_calls
lftj_key_reads
lftj_candidate_values
lftj_bind_successes
lftj_bind_rejects
lftj_completed_bindings
```

Existing trie counters may already cover some of these. If so, normalize names and ensure benchmark JSON exposes them consistently.

Add query image/build counters if not already visible:

```rust
query_image_relations_loaded
query_image_columns_loaded
query_image_indexes_loaded
query_image_encoded_bytes
sorted_trie_bytes
hash_trie_bytes
```

Do not add counters that require heap allocation in the hot path. Counters must be integer increments only.

## Required Benchmark JSON

Expose all new counters under a stable `counters` object.

Every benchmark row must include:

```json
"counters": {
  "sink_emit_calls": 123,
  "encoded_project_rows_seen": 123,
  "lftj_next_calls": 456
}
```

Names can differ if they are clearer, but the JSON must let a future agent answer:

- how many bindings were completed
- how many rows were emitted to the sink
- how many projection rows were duplicates
- how many encoded row bytes were touched
- how many LFTJ iterator operations occurred
- how many direct chain rows/steps occurred

## Required Markdown/Text Output

Add a compact `Mechanics Counters` markdown table containing at least:

```text
dataset
query
runtime
sink_emit_calls
encoded_project_rows_seen
encoded_project_duplicate_rows
lftj_next_calls
lftj_seek_calls
direct_chain_step_rows
direct_chain_output_rows
```

Text mode should print the cache mode and the top mechanics counters for each query.

## Required Hotset Artifact

Create a generated RCA note:

```text
docs/benchmark-rca/v6-hotset-baseline.md
```

It must include:

- exact commands
- non-JOB mechanics table
- JOB mechanics table
- top 5 queries by sink emits
- top 5 queries by LFTJ next/seek/key operations
- top 5 queries by direct chain rows
- top 5 queries by encoded project duplicate rows
- initial hypothesis for PRD 03 and PRD 04

## Required Benchmark Commands

Run and store:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-nonjob.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-job-10k.json
```

## Required Tests

- Counter defaults are zero.
- Direct storage query increments direct output counters and not LFTJ counters.
- Direct chain query increments direct chain counters.
- LFTJ materialized query increments LFTJ iterator and sink counters.
- Projection dedup query increments seen/inserted/duplicate counters correctly.
- Count-only aggregate does not increment encoded projection counters.
- Benchmark JSON includes new counter keys.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB benchmark JSON includes mechanics counters.
- JOB benchmark JSON includes mechanics counters.
- `docs/benchmark-rca/v6-hotset-baseline.md` exists and identifies hot queries.

## Completion Criteria

- Future PRDs can use untraced JSON to explain per-query mechanics.
- No permanent trace-only branch exists.
- This PRD is deleted and committed after passing.
