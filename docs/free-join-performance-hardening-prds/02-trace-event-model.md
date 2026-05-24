# PRD 02: Trace Event Model

## Purpose

Define one canonical trace model for every timing, counter, and allocation measurement before instrumenting the engine.

## Required Types

Add a query trace module, preferably `crates/bumbledb-lmdb/src/query/trace.rs`, with types equivalent to:

```rust
pub struct QueryTrace {
    pub spans: Vec<TraceSpan>,
    pub counters: TraceCounters,
}

pub struct TraceSpan {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub phase: TracePhase,
    pub label: String,
    pub start_nanos: u128,
    pub elapsed_nanos: u128,
    pub allocs: AllocationDelta,
    pub counters: TraceCounters,
}

pub enum TracePhase {
    Normalize,
    PlanSelect,
    PlannerStats,
    BaseImageCacheLookup,
    BaseImageLoad,
    SourceFilterEncode,
    ColtBuild,
    ColtIter,
    ColtForce,
    ColtGet,
    CoverChoice,
    ExecuteNode,
    ProbeSibling,
    BindingExtend,
    SinkConsume,
    SinkFinish,
    DecodeValue,
}
```

Exact field names may differ, but the information content must not be weakened.

## Required Design

- Spans must be hierarchical.
- Span creation must be cheap when profiling is disabled.
- Normal execution must not allocate per span when trace collection is disabled.
- The trace model must not require JSON serialization dependencies.
- Benchmark rendering may manually format JSON until a serialization dependency is explicitly approved.
- Trace labels must include relation name, atom occurrence, node id, field ids, or source id when applicable.
- Every span must carry elapsed time and allocation delta fields, even if allocation tracking is disabled and the delta is zero.
- Counters must be additive and mergeable.

## Required Phases

The trace phase enum must include at least the phases listed in Required Types. Additional phases are allowed only when they are specific and actionable.

## Passing Criteria

- `QueryTrace` can be constructed in tests without opening LMDB.
- A unit test proves nested span parent IDs are preserved.
- A unit test proves disabled tracing adds no spans and returns an empty trace.
- A unit test proves counters merge by addition.
- `explain` no longer contains a hardcoded claim that timings and allocations are not collected once the trace model is wired in a later PRD.
- Global acceptance from PRD 00 passes.
