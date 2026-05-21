# PRD 03: Unified Batched Encoded Projection Sink

## Goal

Replace incremental per-row projection set insertion with a batch-oriented encoded projection sink that appends encoded rows during execution and deduplicates in one compact finalization pass.

This is the highest-priority query optimization from the trace analysis.

## Background

Heavy tracing showed high-output materialized non-JOB queries generate massive sink emission volume:

```text
lftj.execute -> sink.emit: about 1.22M events
execute_prepared_with_options -> sink.emit: about 330K events
```

`sink_finish_us` is small, while `execute_us` and `lftj_execute_us` are high. That means materialized output cost is being paid during per-binding emission.

The current generic encoded projection sink uses set-like behavior during emit. That likely causes:

- per-row comparisons
- pointer chasing
- potential allocation churn
- branchy duplicate handling
- poor cache locality

## Explicit Non-Goals

- No backwards compatibility.
- No preserving current sink internals.
- No changing public query output semantics.
- No changing set semantics.
- No decoding before the final output boundary.
- No reintroducing tiny project sink.
- No benchmark-only shortcuts.

## Code Anchors

Expected areas:

```text
OutputSink
EncodedProjectSink
TupleSink
SmallEncodedRow
decode_output_value
PlanCounters
query_tests.rs projection tests
benchmark JSON counters from PRD 01
```

## Required Design

Introduce a single projection sink design:

```rust
struct EncodedProjectSink {
    vars: Vec<VarId>,
    width: usize,
    row_width_bytes: usize,
    rows: Vec<u8>,
    row_count: usize,
    mode: DedupMode,
}
```

Exact fields may differ, but the sink must store projection rows in a contiguous encoded-row layout.

Recommended row layout:

```text
[value0 bytes][value1 bytes]...[valueN bytes]
```

For mixed encoded widths, use a precomputed offset table:

```rust
struct EncodedProjectionLayout {
    vars: Vec<VarId>,
    offsets: Vec<usize>,
    widths: Vec<usize>,
    row_width: usize,
}
```

Do not allocate `Vec<EncodedOwned>` per row in the hot path.

Do not use `BTreeSet<SmallEncodedRow>` during emit.

## Dedup Strategy

Default strategy:

1. Append encoded row bytes to `rows` during `emit`.
2. At `finish`, sort row chunks lexicographically.
3. Dedup adjacent equal chunks.
4. Decode only unique chunks into public `Value` rows.

Implementation options:

- Use `Vec<usize>` row indices sorted by comparator over `rows` chunks to avoid moving row bytes repeatedly.
- Or sort fixed-size row chunks in-place if implementation is clean and safe.
- For small row widths, consider stack arrays only if they do not reintroduce a separate sink regime.

The first implementation should prioritize correctness and simple contiguous layout over clever unsafe sorting.

## Distinctness Fast Path

Add a conservative optional fast path only when distinctness is proven by exact conditions.

Allowed proof examples:

- projection includes a relation covering-unique key from a single direct relation scan
- direct chain emits a unique final variable by construction
- global count/aggregate paths do not use projection sink

If proof is not exact, use sort/dedup.

Do not add heuristic distinctness.

## Counter Requirements

Expose in benchmark JSON:

```text
encoded_project_rows_seen
encoded_project_rows_inserted
encoded_project_duplicate_rows
encoded_project_row_bytes
encoded_project_sort_us
encoded_project_decode_us
sink_emit_calls
```

Update counters from PRD 01 if they already exist.

## Required Tests

- Projection deduplicates duplicate logical rows.
- Projection with mixed width fields round-trips correctly.
- Projection with width 1 enum fields round-trips correctly.
- Projection with width 8 serial/integer/timestamp fields round-trips correctly.
- Projection with width 16 decimal fields round-trips correctly.
- Direct materialized projection still does not use prepared result cache.
- Count-only query does not use projection sink.
- `decoded_values` increments only during finish/decode, not emit.
- Duplicate-heavy query reports duplicate rows in counters.
- Unique query reports zero duplicates.

## Required Benchmarks

Run:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-batched-project-nonjob.json
```

Run focused query filters if supported:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob \
  --query tag_lookup_join \
  --query red_boat_sailors \
  --query high_rating_red_boats \
  --query revenue_by_customer_range \
  --query supplier_nation_orders \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-batched-project-hot-nonjob.json
```

Run JOB 10k:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-batched-project-job-10k.json
```

## Performance Targets

Hard gates:

- all existing non-JOB gates pass
- all existing JOB gates pass
- no correctness regression

Optimization targets:

- `red_boat_sailors` improves by at least 15% untraced, or RCA explains why not
- `high_rating_red_boats` improves by at least 15% untraced, or RCA explains why not
- `supplier_nation_orders` improves by at least 10% untraced, or RCA explains why not
- `tag_lookup_join` must not regress by more than 5%
- `triangle_count` must not regress by more than 5%

Do not fail the PRD solely because a stretch target is missed if hard gates pass and counters prove the new layout is cleaner.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB benchmark gates pass
- JOB 10k benchmark gates pass
- projection counters are visible in JSON
- no `TinyProjectSink` or equivalent alternate tiny sink is introduced

## Completion Criteria

- Projection sink is unified and batch-oriented.
- Per-row set insertion during emit is gone.
- Output semantics remain set-based and correct.
- This PRD is deleted and committed after passing.
