# PRD 05: Trace And Acceptance Gates

## Status

Draft.

## Problem

The benchmark suite can pass gates while still hiding waste. The previous trace was 304 MB and required manual interpretation. We need a repeatable trace workflow and stricter benchmark reports so future changes can prove they reduced the intended waste instead of shifting it.

## Root Cause Analysis

The trace generated many `new` and `close` span events:

```text
805,770 JSONL lines
304 MB
```

Useful hotspots were discoverable, but not automatically summarized:

```text
bumbledb.query.lftj.build
bumbledb.query.lftj.execute
bumbledb.query.lftj.build.scan_filter_copy
bumbledb.sorted_trie.build
bumbledb.hash_trie.build
bumbledb.query.project
bumbledb.query.aggregate
```

The benchmark harness now reports plan family and query-image build behavior, but it still needs targeted trace summaries for non-JOB performance work.

## Goal

Add a repeatable non-JOB tracing protocol and benchmark acceptance gates that can answer:

- Which runtime family ran?
- Did the query build a query image?
- Did it build hash or sorted tries?
- How much time was spent in setup versus execution?
- How much time was spent materializing output?
- Did the change preserve JOB and triangle performance?

## Non-Goals

- No permanent heavy tracing by default.
- No benchmark data checked into git.
- No external profiler dependency requirement.
- No general telemetry backend.

## Required Trace Protocol

Add scripts or documented commands for:

```sh
cargo run -p bumbledb-bench --release -- \
  --scale 10000 \
  --warmup 0 \
  --repeats 1 \
  --format json \
  --trace \
  --trace-format json \
  --trace-output "$TRACE_PATH" \
  --dataset ledger \
  --dataset sailors \
  --dataset joinstress \
  --dataset tpch \
  > "$RESULT_PATH"
```

Add a script such as:

```text
scripts/bench-trace-nonjob.sh
```

The script should write artifacts under a user-specified directory or temp path and print the paths.

## Required Trace Summary

Add a lightweight script or documented `jq` pipeline that summarizes:

- busy time by span name,
- count by span name,
- per-query benchmark result row,
- phase timing per query,
- counters per query,
- runtime family per query.

The summary must not require manually opening a 300 MB trace.

## Required Benchmark JSON Fields

These fields must remain present:

- `runtime`
- `plan_family`
- `query_image_built_during_query`
- `phase_timing`
- `allocations`
- `counters`
- `gate`

Add fields if missing after earlier PRDs:

- `compare_mode`
- `bumbledb_materialized_rows`
- `sqlite_materialized_rows`
- `count_only_supported`
- `count_only_fallback_reason`

The count-only fields are required after PRD 03 in this folder, not necessarily before it.

## Required Gates

Maintain universal gates:

```text
cursor_seeks == 0
rows_scanned == 0
dictionary_reverse_lookups == 0 unless output contains String/Bytes
```

Maintain runtime family gates:

```text
joinstress/triangle_count -> FreeJoinLftj
sailors/sailor_range_reserves -> Direct
joinstress/chain4_from_a -> Direct or IndexNestedLoop
```

Add setup gates after the relevant PRDs:

```text
sailors/sailor_range_reserves -> hash_index_builds == 0, sorted_trie_builds == 0
joinstress/chain4_from_a -> hash_index_builds == 0, sorted_trie_builds == 0
ledger/tag_lookup_join -> hash_index_builds == 0 after PRD 02
```

Add trace regression gates:

```text
no target direct query may spend > 10% of total time in lftj_build
no target direct query may spend > 10% of total time in sorted_trie.build
no target index-nested-loop query may build hash tries or sorted tries
```

## Required Tests

Add tests for:

- `--format both` emits markdown and JSON.
- JSON includes `plan_family`.
- JSON includes `query_image_built_during_query`.
- Runtime family gates are present.
- Trace script exists and is executable.
- Trace summary script handles at least one JSONL close event fixture.

## Strict Passing Criteria

- A developer can run one command to generate non-JOB trace artifacts.
- A developer can run one command to summarize trace hotspots.
- Benchmark JSON contains all required fields.
- Benchmark markdown contains plan family and image-build behavior.
- Runtime-family gates fail if target queries regress to the wrong family.
- Setup gates fail if direct targets build hash/sorted tries after their implementation PRDs.
- Full workspace test/clippy/fuzz gates pass.
- Focused benchmark gates pass at `scale=10000`, `warmup=2`, `repeats=30`.

## Verification Commands

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
scripts/bench-focused.sh --fail-gates
scripts/bench-trace-nonjob.sh
```
