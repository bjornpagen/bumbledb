# 03 Query Observability Data Model Results

**Completed Data Model**
- Added `QueryTimings` to `QueryPlan` with coarse phase timings in microseconds.
- Added `QueryRuntimeKind` with `Unknown`, `Lftj`, `HashProbe`, `MixedFallback`, and reserved `DirectKernel` variants.
- Added `QueryAllocationStats` to `QueryPlan`, disabled and zero by default until allocation profiling lands.
- Added `QueryNodeTiming` node summaries with node ID, implementation, bound variables, estimated rows, actual rows, and reserved node execution micros.

**Completed Instrumentation**
- Timed `validate_inputs`, normalization, input encoding, QueryImage acquisition, planning, Free Join execution, LFTJ build, hash index build/lookup, LFTJ execution, hash execution, sink finish, and total query execution.
- Runtime kind is set by Free Join dispatch: all-hash plans report `HashProbe`, pure sorted plans report `Lftj`, and mixed fallbacks report `MixedFallback`.
- `sink_emit_micros` and `decode_micros` are intentionally zero in this PRD to avoid per-row/per-value `Instant` overhead. PRD 04/05 can add opt-in finer profiling.
- Node timing summaries currently carry actual row/candidate counts and reserve `execute_micros=0` until node-level coarse timers are added without recursion overhead.

**Completed Spans**
- `bumbledb.query.validate_inputs`
- `bumbledb.query.normalize`
- `bumbledb.query.encode_inputs`
- `bumbledb.query.image`
- `bumbledb.query.plan`
- `bumbledb.query.plan.stats`
- `bumbledb.query.plan.variable_order`
- `bumbledb.query.plan.optimize_free_join`
- `bumbledb.query.free_join.dispatch`
- `bumbledb.query.hash.build_indexes`
- `bumbledb.query.hash.execute`
- `bumbledb.query.lftj.build`
- `bumbledb.query.lftj.execute`
- `bumbledb.query.sink.emit`
- `bumbledb.query.sink.finish`
- `bumbledb.sorted_trie.build`
- `bumbledb.hash_trie.build`

No span includes raw input values, literal bytes, row payloads, interned text, or result rows.

**Explain And Benchmark Output**
- `QueryPlan::explain()` renders `runtime_kind`, `timings:`, `query_timing`, `allocations:`, `allocation_summary`, and `node_timing` lines.
- Benchmark markdown now includes runtime kind in the main results table.
- Benchmark markdown now includes `## Phase Timing` and `## Allocation Summary` sections.
- Allocation summary fields render disabled/zero values until allocation profiling lands.

**Release Benchmark Smoke**
Command:

```sh
cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown
```

Latest run:

| Dataset | Query | Runtime | Rows | Bumbledb Avg Us | Gate |
|---|---|---|---:|---:|---|
| ledger | postings_for_holder_range | HashProbe | 3 | 166 | pass |
| ledger | balances_by_instrument | HashProbe | 3 | 220 | pass |
| ledger | tag_lookup_join | HashProbe | 10000 | 13849 | pass |
| sailors | red_boat_sailors | HashProbe | 10000 | 40856 | pass |
| sailors | sailor_range_reserves | HashProbe | 5 | 49 | pass |
| sailors | high_rating_red_boats | MixedFallback | 6660 | 10847 | pass |
| joinstress | chain4_from_a | HashProbe | 1 | 109 | pass |
| joinstress | triangle_count | Lftj | 1 | 16840 | pass |
| tpch | revenue_by_customer_range | HashProbe | 2000 | 62573 | pass |
| tpch | supplier_nation_orders | HashProbe | 5716 | 49062 | pass |

Structural counters remain clean: `cursor_seeks=0`, `rows_scanned=0`, and `dictionary_reverse_lookups=0` for every generated query.

**Tests Added Or Updated**
- Default observability structs have zero/default values.
- Executed queries populate non-unknown runtime kind, total timing, execution timing, node summaries, and disabled allocation summary.
- All-hash plans assert `HashProbe` runtime kind.
- Pure LFTJ triangle plan asserts `Lftj` runtime kind.
- Explain output includes timing, allocation, and node timing sections.
- Benchmark markdown renderer emits phase timing and allocation sections.

**Verification**
- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- `scripts/check-cutover.sh`
- `scripts/check-prd-map.sh`
- `scripts/check-performance-kill-list.sh`
- `cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown`

**Next PRD**
- `docs/todos/observability_lints_allocation_hardening/04_tracing_and_profiling_ux.md`
