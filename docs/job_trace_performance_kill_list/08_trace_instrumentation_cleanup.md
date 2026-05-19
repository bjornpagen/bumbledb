# 08 Trace Instrumentation Cleanup

Priority: P1

Primary affected reports:

- Every report that showed high `bumbledb.query.sink.emit` time.
- `job_broad_movie_info_star`: `sink.emit` appears as `97.1%` of steady state but source shows it wraps the whole LFTJ executor.
- `job_broad_cast_keyword_company`: `sink.emit` appears as `85.1%` of steady state for the same reason.

## Problem

The firehose trace is valuable, but some span names are misleading. The most important issue is `bumbledb.query.sink.emit`: it is opened around the entire hash/LFTJ executor body, not around actual sink emission calls. This makes trace reports look sink-bound when the real work is traversal/probing.

Future optimization work will be based on traces. Misattribution here can send work in the wrong direction.

## Current Technical Cause

LFTJ execution opens `sink.emit` before constructing and running the executor:

`crates/bumbledb-lmdb/src/query.rs:2501-2516`

```rust
let result = {
    let _span =
        tracing::debug_span!("bumbledb.query.lftj.execute", variables = query.vars.len())
            .entered();
    let _sink_emit_span = tracing::debug_span!("bumbledb.query.sink.emit").entered();
    let mut executor = LftjExecutor {
        txn,
        query,
        inputs,
        plan,
        runtime,
        binding: EncodedBinding::new(query.vars.len()),
        sink,
    };
    executor.execute(0)
};
```

Hash execution does the same:

`crates/bumbledb-lmdb/src/query.rs:1377-1382`

```rust
let result = {
    let _span =
        tracing::debug_span!("bumbledb.query.hash.execute", variables = query.vars.len())
            .entered();
    let _sink_emit_span = tracing::debug_span!("bumbledb.query.sink.emit").entered();
    let participants_by_variable =
        hash_participants_by_variable(query.vars.len(), &plan.relation_atoms);
```

Actual sink emission happens elsewhere:

- LFTJ leaf: `crates/bumbledb-lmdb/src/query.rs:2560-2566`
- Hash leaf: `crates/bumbledb-lmdb/src/query.rs:2139-2145`
- Direct chain leaf: `crates/bumbledb-lmdb/src/query.rs:1911-1918`

## Desired End State

Trace spans should distinguish:

- Executor body time.
- Candidate iteration/probe time.
- Actual sink emission time.
- Sink finalization time.
- Aggregate state update time.

No span named `sink.emit` should wrap the entire executor.

## Proposed Technical Solution

### Rename Current Wrapper Spans

Replace current broad `sink.emit` wrappers with executor body spans:

```rust
let _body_span = tracing::debug_span!("bumbledb.query.lftj.body").entered();
```

```rust
let _body_span = tracing::debug_span!("bumbledb.query.hash.body").entered();
```

### Add Real Sink Emit Spans

Wrap actual sink calls:

```rust
let _emit_span = tracing::trace_span!("bumbledb.query.sink.emit").entered();
self.sink.emit(...)?;
```

Use `trace_span` rather than `debug_span` if per-binding volume is high. For normal non-firehose runs, it will not emit unless `trace` is enabled.

### Add Aggregate Count Range Span

When count pushdown lands, add:

```rust
tracing::trace_span!("bumbledb.query.aggregate.count_range", count).entered();
```

### Add Node Timing Spans

Per-node spans should be measured inside the executor:

```rust
tracing::trace_span!(
    "bumbledb.query.node.execute",
    node = depth,
    implementation = ?node.implementation,
    variable = %query.vars[variable].name,
).entered();
```

This should make the `QueryNodeTiming` data more meaningful than the current zero-heavy node timings.

### Keep Counters As Ground Truth

Trace spans are inclusive and can be noisy. The reports should continue to rely on counters for exact event counts:

- `bindings_yielded`
- `variable_candidates`
- `trie_key_reads`
- `hash_probe_calls`
- `hash_index_build_rows`
- `materialized_output_values`

## Implementation Plan

1. Rename broad executor-body spans.
2. Add real sink emit spans at actual sink callsites.
3. Add node execution spans.
4. Add trace counters for plan cache hits/misses after prepared plan cache lands.
5. Update benchmark trace summary extraction scripts or docs to use new span names.

## Tests

- Run a tiny traced query with one emitted row and verify exactly one `sink.emit` span.
- Run an empty HashProbe query and verify no `sink.emit` span.
- Run count aggregate with many bindings and verify `sink.emit` count matches `bindings_yielded` until count pushdown changes semantics.

## Acceptance Criteria

- `sink.emit` no longer appears as `85-97%` of broad LFTJ query time unless actual sink emission is truly that expensive.
- Executor body spans carry the previous broad time.
- Node spans identify per-node work in JOB traces.
- Future performance reports can distinguish traversal, probing, aggregation, and output costs.

## Risks

- Per-binding trace spans can explode trace volume. Use `trace_span` and only enable in firehose runs.
- Moving spans changes historical report scripts. Preserve old field names in one transition run if needed.

## Rollout Plan

1. Rename broad spans.
2. Add real emit spans.
3. Update trace report extraction.
4. Rerun one small benchmark trace.
5. Rerun JOB firehose after major runtime changes.
