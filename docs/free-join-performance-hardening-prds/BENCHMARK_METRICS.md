# Benchmark Metrics

This document defines the current `bumbledb-bench` JSON report fields. Every field must come from a real benchmark input, exact correctness check, timer, allocator snapshot, or query trace.

| field | source |
| --- | --- |
| `scale` | Number of loaded facts for JOB, or fixed fixture scale for quick benchmarks. |
| `dataset` | Benchmark dataset identifier. |
| `query` | Query fixture or JOB query name. |
| `engine` | Current Bumbledb engine label, expected to be `free_join`. |
| `sqlite_reference` | Exact reference description. JOB must use exact `SELECT DISTINCT`. |
| `git_commit` | Compile-time `GIT_COMMIT` environment value or `unknown`. |
| `hardware` | CLI-provided hardware label or `unspecified`. |
| `correctness_fingerprint` | BLAKE3 fingerprint over sorted exact result facts. |
| `gate_status` | Correctness gate status. Current successful runs use `passed`. |
| `elapsed_nanos` | Wall-clock nanoseconds around Bumbledb query execution. |
| `sqlite_elapsed_nanos` | Wall-clock nanoseconds around SQLite reference execution, or zero for embedded quick fixtures. |
| `load_nanos` | Wall-clock nanoseconds for Bumbledb benchmark data load, or zero for quick fixtures. |
| `result_rows` | Final duplicate-free projected result fact count. |
| `allocation_tracking` | Whether allocator counters were enabled for the measured Bumbledb execution. |
| `alloc_calls` | Allocation calls measured between allocator snapshots around Bumbledb query execution. |
| `allocated_bytes` | Allocated bytes measured between allocator snapshots around Bumbledb query execution. |
| `deallocated_bytes` | Deallocated bytes measured between allocator snapshots around Bumbledb query execution. |
| `net_allocated_bytes` | `allocated_bytes - deallocated_bytes` for the measured Bumbledb execution. |
| `trace` | Query trace JSON. Release builds without `query-tracing` report `{"enabled":false}`. Debug builds and release builds with `query-tracing` report measured spans, counters, and metadata. |

Trace counter fields are defined in `TraceCounters` and are rendered only from the query trace object. Enabled trace rendering is required to contain at least one span or non-zero counter group.
