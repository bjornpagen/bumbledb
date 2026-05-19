# JOB Trace Performance Kill List

This folder is the performance kill list derived from the full-firehose JOB trace run on the real CWI IMDb/JOB data subset.

Trace artifacts:

- Full trace JSONL: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-trace-scale10000-r10.jsonl`
- Results JSON: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-results-scale10000-r10.json`
- Per-query summary JSON: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-per-query-trace-summary.json`
- Per-query summary CSV: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-per-query-trace-summary.csv`

Benchmark command:

```sh
RUST_LOG='bumbledb_lmdb=trace,bumbledb_bench=debug' \
cargo run -p bumbledb-bench --release -- \
  --dataset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --scale 10000 \
  --warmup 2 \
  --repeats 10 \
  --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-trace-scale10000-r10.jsonl \
  --trace-format json \
  --format json
```

Trace volume:

- `5.4GB`
- `12,498,205` JSONL events
- `104` traced query executions: first execution, two warmups, and ten measured samples for each of eight JOB queries

## Query Results

| Query | Runtime | Rows | BumbleDB avg | SQLite avg | Ratio | Primary Bottleneck |
|---|---|---:|---:|---:|---:|---|
| `job_broad_cast_keyword_company` | `MixedFallback` | 1 | `5.40ms` | `5.03ms` | `1.07x` slower | Mixed plan falls back to LFTJ; count enumerates `11,009` bindings |
| `job_broad_movie_info_star` | `Lftj` | 1 | `25.83ms` | `55.90ms` | `2.16x` faster | Count aggregate enumerates `47,034` bindings |
| `job_q01_top_production` | `Lftj` | 0 | `0.344ms` | `0.787ms` | `2.29x` faster | Cold LFTJ builds all atom tries before empty literal branch is known |
| `job_q09_voice_us_actor` | `MixedFallback` | 1 | `1.62ms` | `1.97ms` | `1.22x` faster | Mixed plan falls back to LFTJ; planner stats cold cost |
| `job_q16_character_title_us` | `HashProbe` | 0 | `0.711ms` | `3.67ms` | `5.16x` faster | Cold eager hash trie builds over `501,066` rows |
| `job_q24_voice_keyword_actor` | `HashProbe` | 0 | `0.778ms` | `8.77ms` | `11.27x` faster | Cold eager hash trie builds over `317,617` rows |
| `job_movie_link_bridge` | `Lftj` | 1 | `0.773ms` | `0.153ms` | `5.05x` slower | Steady-state replanning; cold LFTJ temp trie construction |
| `job_q33_linked_series_companies` | `HashProbe` | 0 | `0.809ms` | `0.071ms` | `11.39x` slower | Steady-state replanning; cold hash build over `220,043` rows |

## Kill Items

| Priority | Item | Spec |
|---:|---|---|
| P0 | LFTJ atom index reuse and lazy construction | [`04_lftj_atom_index_reuse.md`](04_lftj_atom_index_reuse.md) |
| P0 | True count aggregate pushdown | [`05_count_aggregate_pushdown.md`](05_count_aggregate_pushdown.md) |
| P1 | Optimizer build-cost model and stats fixes | [`06_optimizer_cost_model.md`](06_optimizer_cost_model.md) |
| P1 | Static atom pre-resolution and FK atom elimination | [`07_static_atom_simplification.md`](07_static_atom_simplification.md) |
| P1 | Trace instrumentation cleanup | [`08_trace_instrumentation_cleanup.md`](08_trace_instrumentation_cleanup.md) |

## Cross-Query Priority Map

| Query | LFTJ Reuse/Lazy Build | Count Pushdown | Cost Model | Static Simplification | Trace Cleanup |
|---|---|---|---|---|---|
| `job_broad_cast_keyword_company` | Medium | High | High | Medium | Medium |
| `job_broad_movie_info_star` | Medium | Critical | Medium | High | Medium |
| `job_q01_top_production` | Critical | Low | Medium | Critical | Medium |
| `job_q09_voice_us_actor` | Medium | Medium | High | Medium | Medium |
| `job_q16_character_title_us` | Low | Low | Critical | Medium | Medium |
| `job_q24_voice_keyword_actor` | Low | Low | Critical | Medium | Medium |
| `job_movie_link_bridge` | Critical | Medium | Medium | Medium | Medium |
| `job_q33_linked_series_companies` | Low | Low | Critical | High | Medium |

## Shared Source Hotspots

The same code paths appear in most reports:

- `crates/bumbledb-lmdb/src/query.rs:1197-1302`: `execute_query` validates, normalizes, gets `QueryImage`, replans, executes, and finishes on every call.
- `crates/bumbledb-lmdb/src/query.rs:3001-3107`: `plan_query` recollects planner stats, recomputes variable order, and optimizes Free Join every execution.
- `crates/bumbledb-lmdb/src/query.rs:1313-1347`: runtime dispatch only uses HashProbe if every node is hash-probe eligible; mixed plans fall back to LFTJ.
- `crates/bumbledb-lmdb/src/query.rs:1349-1535`: HashProbe builds every hash atom index before probing.
- `crates/bumbledb-lmdb/src/query.rs:2443-2524`: LFTJ builds all atom plans before executing.
- `crates/bumbledb-lmdb/src/query.rs:2775-2999`: LFTJ atom plan construction scans source relations, clones encoded bytes, builds temporary relation images, then builds sorted tries.
- `crates/bumbledb-lmdb/src/query.rs:2550-2611`: LFTJ enumerates complete bindings before aggregate sinks count them.
- `crates/bumbledb-lmdb/src/query_image.rs:180-249`: sorted and hash trie caches help after the first build, but miss handling still performs full synchronous builds.
- `crates/bumbledb-lmdb/src/planner_stats.rs:119-159`: optimizer stats build exact field stats and sorted tries for access paths.
- `crates/bumbledb-lmdb/src/hash_trie.rs:31-71`: hash trie build scans every row and stores row IDs by default.
- `crates/bumbledb-lmdb/src/sorted_trie.rs:81-118`: sorted trie build allocates and sorts a full row order.

## Completion Criteria For The Whole Kill List

The kill list is complete when a traced JOB rerun at `scale=10000`, `warmup=2`, `repeats=10` shows:

- Steady-state planning below `10%` of `bumbledb.query.execute` for no-input repeated queries.
- HashProbe cold index build rows reduced by at least `80%` on empty/selective hash queries.
- `MixedFallback` eliminated from the two currently mixed JOB queries or made into a real mixed executor with nonzero hash counters.
- LFTJ cold build time reduced by at least `70%` on `job_movie_link_bridge` and `job_q01_top_production`.
- Count-only broad joins reduce `bindings_yielded` by at least `80%` or replace leaf enumeration with range/factorized counts.
- Trace spans attribute sink time accurately and no longer wrap entire executor bodies under `sink.emit`.
