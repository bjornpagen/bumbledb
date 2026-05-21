# PRD 04: Batched Direct Materialization

## Goal

Make direct materialized paths produce encoded projection batches without per-binding sink overhead.

Primary target: `ledger/tag_lookup_join`.

## Background

`tag_lookup_join` is now correctly routed to direct/index-nested-loop execution, but it remains materially slower than SQLite:

```text
untraced: about 7.1ms BDB vs about 1.3ms SQLite
traced: direct execution dominates
output: 10000 rows, 20000 materialized values
```

The path is simple enough that it should not pay generic binding/sink overhead per row.

## Explicit Non-Goals

- No backwards compatibility.
- No restoring legacy direct-chain APIs.
- No relation-name-specific optimizations.
- No changing set semantics.
- No changing public output row format.
- No bypassing correctness checks.
- No new join algorithm.

## Code Anchors

Expected areas:

```text
try_execute_direct_materialized_kernel
try_direct_chain_kernel
DirectChainExecutor
DirectChainProbePlan
DirectChainStep
DirectExistenceCheck
OutputSink
EncodedProjectSink from PRD 03
PlanCounters direct_* fields
```

## Required Design

Introduce a batch-output direct materialization path for direct chain and direct prefix/range kernels.

Recommended shape:

```rust
trait EncodedRowBatchSink {
    fn push_encoded_row(&mut self, row: &[u8]) -> Result<()>;
    fn push_projected_binding(&mut self, layout: &ProjectionLayout, binding: &EncodedBinding) -> Result<()>;
}
```

Or implement equivalent methods directly on the unified `EncodedProjectSink`.

Direct kernels should be able to avoid:

- constructing transient logical rows
- decoding per row
- calling generic `TupleSink::emit` for every binding when projection is simple
- repeated projection layout lookup

## Projection Layout Precomputation

Before executing a direct materialized kernel, precompute:

```text
output variable IDs
encoded widths
row byte offsets
source relation/field for values when derivable
```

For direct chain steps, if the output variables correspond directly to fields in the current row or bound variables, write their encoded bytes directly into the output row buffer.

If a projection cannot be represented as direct encoded bytes, fall back to generic binding-to-sink emission.

## Direct Chain Batch Algorithm

For chain probes:

1. Build or reuse step access structures.
2. Iterate the driver prefix/range.
3. Carry compact encoded bindings in a reusable buffer.
4. When a full output binding is reached, append encoded projection row into the batch sink.
5. Dedup at finish using the unified encoded projection sink.

Avoid allocating one `EncodedBinding` per result. Reuse the binding buffer or maintain a compact stack-like binding state.

## Direct Storage Batch Algorithm

For direct storage scans:

1. If projection fields map directly to scanned row fields, copy encoded bytes from the storage row/current tuple into batch output.
2. Evaluate predicates on encoded bytes where possible.
3. Fall back only when projection requires decoded values or unsupported comparison.

## Counter Requirements

Expose:

```text
direct_batch_rows
direct_batch_row_bytes
direct_batch_fallback_rows
direct_binding_reuses
direct_chain_step_rows
direct_chain_output_rows
sink_emit_calls
```

The key proof is that `tag_lookup_join` uses fewer generic sink emits than before.

## Required Tests

- `tag_lookup_join` synthetic equivalent uses direct batch output.
- Direct chain output rows match existing interpreted/direct output.
- Direct chain duplicate outputs still deduplicate correctly.
- Direct prefix/range materialized query uses batch path when projection is direct.
- Fallback path is used for unsupported projection and remains correct.
- No per-row logical decode occurs during direct batch emit.
- Materialized projection does not use prepared result cache.
- Counters distinguish batch rows from fallback rows.

## Required Benchmarks

Run:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-direct-batch-nonjob.json
```

Focused:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob \
  --query tag_lookup_join \
  --query chain4_from_a \
  --query sailor_range_reserves \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-direct-batch-focused.json
```

Run JOB 10k to ensure direct count/static proof behavior is unchanged:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-direct-batch-job-10k.json
```

## Performance Targets

Hard gates:

- all existing gates pass

Optimization targets:

- `tag_lookup_join` improves by at least 25% untraced, or RCA explains why not
- `chain4_from_a` does not regress by more than 5%
- `sailor_range_reserves` does not regress by more than 5%

Stretch:

- `tag_lookup_join` approaches 3ms or less at scale 10000

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB gates pass
- JOB 10k gates pass
- direct batch counters visible in JSON
- no relation-name-specific code

## Completion Criteria

- Direct materialized paths are batch-oriented where structurally possible.
- `tag_lookup_join` no longer spends most of its time in generic per-binding output mechanics.
- This PRD is deleted and committed after passing.
