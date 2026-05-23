# PRD 13: Delete Stale Diagnostics And Benchmark Mechanics

## Status

Not started.

## Current State

The engine still returns and renders counters/diagnostics that are either direct-scan leftovers, always-zero placeholders, or mechanics for PRD 11/12 deletion. Examples found during review:

- `cursor_seeks`, `facts_scanned`, and `facts_matched` are legacy direct-scan style counters and are not meaningful for current LFTJ execution.
- `sink_emit_micros` and `decode_micros` are documented as zero until enabled.
- `stats_exact_scans` is exposed but never incremented.
- `QueryImageStats.sorted_trie_bytes` is zero scaffolding.
- benchmark renderer fields still preserve sorted-trie/temp-build internals until PRD 12 removes them.

## Objective

Delete diagnostics that do not describe surviving behavior. Keep only counters/timings that are updated by current code and useful for tracing, benchmarking, or correctness gates.

## Implementation Steps

1. Audit every field in `PlanCounters`, `QueryTimings`, `QueryAllocationStats`, `QueryImageStats`, and `PlannerStatsCacheDiagnostics`.
2. Delete fields that are never updated, always zero, or describe deleted direct/eager mechanics.
3. Remove corresponding explain lines, benchmark JSON/markdown columns, tests, and gate checks.
4. Rename surviving counters to the behavior they actually measure.
5. Prefer actual runtime counters over planned/estimated/placeholder values.
6. Add tests that renderer output does not include deleted mechanics.

## Passing Criteria

- No always-zero diagnostics remain in public query/benchmark output.
- Benchmark renderer contains only surviving runtime/cache/timing/allocation fields.
- `cargo test -p bumbledb-bench --bin bumbledb-bench renderer` passes.
- Full validation passes.

## Failure Modes

- Keeping stale counters because external output used them is failure.
- Replacing deleted counters with renamed zero fields is failure.
- Deleting allocation/timing telemetry that is actually updated is failure.

## Completion

Delete this PRD and commit.
