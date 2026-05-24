# PRD 04: Profiled Execution API

## Purpose

Add a first-class profiled execution path so benchmark and tests can retrieve exact results plus rich trace data from the same execution.

## Required API Direction

Break APIs as needed. The preferred shape is:

```rust
pub struct QueryExecutionOptions {
    pub tracing: TraceMode,
    pub allocation_tracking: bool,
    pub execution_mode: ExecutionModePublic,
}

pub struct ProfiledQueryResult {
    pub result: QueryResultSet,
    pub trace: QueryTrace,
}
```

The exact public/private boundary may change, but benchmark code must not rely on test-only functions.

## Required Behavior

- `ReadTxn` must expose a way to execute with profiling enabled.
- The ordinary execution path may delegate to the profiled path with tracing disabled.
- Profiling must not change query results.
- Profiling must not permit invalid plans or malformed query IR.
- Profiled output must include selected plan family, node count, cover policy, execution mode, output mode, and trace summary.

## Required Deletions

- Delete ad hoc benchmark-only diagnostic fields that duplicate trace data.
- Delete stale explain claims that timings and allocations are unavailable.
- Remove test-only public-ish hooks if the profiled API replaces them.

## Passing Criteria

- A test executes the same query with profiling disabled and enabled and proves identical `QueryResultSet`.
- A test proves profiled execution emits at least `Normalize`, `PlanSelect`, `ExecuteNode`, and `SinkFinish` spans for a non-empty query.
- A test proves invalid query IR still fails before execution.
- Benchmark code can call the profiled path without `cfg(test)` access.
- Global acceptance from PRD 00 passes.
