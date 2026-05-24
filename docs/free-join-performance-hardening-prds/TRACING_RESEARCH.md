# Tracing Research Decision

## Research Summary

Rust's `tracing` ecosystem is the standard choice for process-wide structured diagnostics. The `tracing` crate records spans and events, and subscribers or `tracing-subscriber` layers collect those spans. The ecosystem supports efficient disabled callsites and compile-time level filters.

That model is not the right primary mechanism for Bumbledb query profiling right now. Bumbledb needs `ReadTxn::execute_query_profiled` to return a per-query `QueryTrace` containing exact phase spans, engine counters, and allocation deltas. Returning this data through a global or thread-local `tracing` subscriber would add dependency weight and would still require a custom collection layer to reconstruct the exact `QueryTrace` shape.

The Rust Reference documents `cfg(debug_assertions)` as enabled by default for unoptimized debug builds. Cargo features are the conventional compile-time switch for crate-specific capabilities. Therefore the selected design is a custom per-query trace recorder compiled in when either `debug_assertions` or the `query-tracing` feature is active.

## Decision

- Do not add `tracing` or `tracing-subscriber` dependencies for the per-query profiler.
- Remove runtime trace modes.
- Enable query tracing at compile time with `cfg(any(debug_assertions, feature = "query-tracing"))`.
- Keep release builds trace-free unless built with `--features query-tracing`.
- Keep benchmark trace output controls limited to where compiled trace data is written, not whether trace instrumentation runs.

## Commands

Release trace harvest must use:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- <benchmark args>
```

Normal release benchmark runs without `--features query-tracing` produce `"trace":{"enabled":false}`.
