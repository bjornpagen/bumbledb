# JOB Trace Performance Kill List, Round 2

This folder is the next performance kill list derived from the post-kill-list full-firehose JOB trace run on the real CWI IMDb/JOB data subset.

This replaces the completed first-round specs. The first round shipped prepared physical plan caching, lazy HashProbe index construction, mixed HashProbe/LFTJ execution, equivalent LFTJ atom trie reuse, count range pushdown, hash build-aware costing, static-empty query short-circuiting, and corrected sink emit tracing.

## Trace Artifacts

- Full post-kill trace JSONL: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-trace-after-kill-list-scale10000-r10.jsonl`
- Post-kill results JSON: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-results-after-kill-list-traced-scale10000-r10.json`
- Post-kill per-query summary JSON: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-after-kill-list-per-query-trace-summary.json`
- Post-kill per-query summary CSV: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-after-kill-list-per-query-trace-summary.csv`

Trace command:

```sh
RUST_LOG='bumbledb_lmdb=trace,bumbledb_bench=debug' \
cargo run -p bumbledb-bench --release -- \
  --dataset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --scale 10000 \
  --warmup 2 \
  --repeats 10 \
  --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-trace-after-kill-list-scale10000-r10.jsonl \
  --trace-format json \
  --format json
```

Trace volume:

- `5.4GB`
- `12,504,153` JSONL events
- `104` traced query executions: first execution, two warmups, and ten measured samples for each of eight JOB queries

## Current Results

| Query | Runtime | Rows | BumbleDB avg | SQLite avg | Ratio | Current Main Bottleneck |
|---|---|---:|---:|---:|---:|---|
| `job_broad_cast_keyword_company` | `Lftj` | 1 | `3.85ms` | `5.01ms` | `1.30x` faster | LFTJ key-read/seek churn; cold planner stats |
| `job_broad_movie_info_star` | `Lftj` | 1 | `21.46ms` | `56.60ms` | `2.64x` faster | LFTJ broad count traversal: `507,718` key reads/run |
| `job_q01_top_production` | `Lftj` | 0 | `145us` | `793us` | `5.47x` faster | Fixed overhead and missing join-level static-empty proof |
| `job_q09_voice_us_actor` | `Lftj` | 1 | `4.15ms` | `2.23ms` | `1.86x` slower | Cold planner stats; LFTJ execution under trace; aggregate emits `276` bindings/run |
| `job_q16_character_title_us` | `Lftj` | 0 | `213us` | `3.98ms` | `18.7x` faster | Cold LFTJ scan/filter/copy temp relation construction |
| `job_q24_voice_keyword_actor` | `Lftj` | 0 | `171us` | `9.26ms` | `54.2x` faster | Cold LFTJ scan/filter/copy temp relation construction |
| `job_movie_link_bridge` | `Lftj` | 1 | `249us` | `149us` | `1.67x` slower | Generic LFTJ and aggregate finish overhead for tiny count |
| `job_q33_linked_series_companies` | `StaticEmpty` | 0 | `69us` | `61us` | `1.13x` slower | Static-empty proof/frontend overhead; no executor runs |

## Ordered PRDs

| Priority | Item | Spec | Primary Metric Targets |
|---:|---|---|---|
| P0 | Indexed LFTJ atom builders | [`02_indexed_lftj_atom_builders.md`](02_indexed_lftj_atom_builders.md) | Cut q16/q24 cold `lftj.build` by `>80%` |
| P0 | Factorized star-count kernels | [`03_factorized_star_count_kernels.md`](03_factorized_star_count_kernels.md) | Cut broad star steady LFTJ key reads by `>80%` |
| P0 | Join-level static-empty proof | [`04_join_level_static_empty.md`](04_join_level_static_empty.md) | Make q01 `StaticEmpty`; keep q33 below `40us` |
| P1 | Direct aggregate count kernels | [`05_direct_aggregate_count_kernels.md`](05_direct_aggregate_count_kernels.md) | Make `job_movie_link_bridge` consistently faster than SQLite |
| P1 | LFTJ inner-loop borrowed-key optimization | [`06_lftj_inner_loop_key_reads.md`](06_lftj_inner_loop_key_reads.md) | Cut key-read overhead for remaining LFTJ workloads by `30%+` |
| P1 | Static-empty/frontend cache and instrumentation | [`07_static_empty_frontend_cache.md`](07_static_empty_frontend_cache.md) | Cut q33 from `69us` to `<40us`; expose proof counters |
| P1 | LFTJ build subphase tracing | [`08_lftj_build_subphase_tracing.md`](08_lftj_build_subphase_tracing.md) | Attribute scan/filter/copy vs column build vs sort precisely |

## Cross-Query Priority Map

| Query | Indexed LFTJ Build | Factorized Count | Join Static Empty | Direct Count Kernel | LFTJ Key Reads | Frontend Cache | Build Tracing |
|---|---|---|---|---|---|---|---|
| `job_broad_cast_keyword_company` | Medium | High | Low | Low | High | Low | Medium |
| `job_broad_movie_info_star` | Medium | Critical | Low | Low | Critical | Low | Medium |
| `job_q01_top_production` | Low | Low | Critical | Medium | Low | Medium | Low |
| `job_q09_voice_us_actor` | High | Medium | Low | Low | High | Low | High |
| `job_q16_character_title_us` | Critical | Low | Low | Low | Low | Low | Critical |
| `job_q24_voice_keyword_actor` | Critical | Low | Low | Low | Low | Low | Critical |
| `job_movie_link_bridge` | Low | Medium | Low | Critical | Medium | Low | Low |
| `job_q33_linked_series_companies` | Low | Low | Medium | Low | Low | Critical | Low |

## Shared Source Hotspots

- `crates/bumbledb-lmdb/src/planner_stats.rs:119-159`: exact planner stats build scans fields and builds sorted tries for access paths.
- `crates/bumbledb-lmdb/src/query.rs:3657-3816`: `build_lftj_sorted_trie` scans source rows, filters literals, clones encoded bytes, builds temp columns.
- `crates/bumbledb-lmdb/src/query.rs:3301-3387`: LFTJ recursive execution loop drives trie opens, nexts, seeks, key reads, and recursive calls.
- `crates/bumbledb-lmdb/src/query.rs:3416-3424`: current count suffix pushdown still iterates every final candidate key.
- `crates/bumbledb-lmdb/src/query.rs:3569-3575`: every trie key read clones into `EncodedOwned`.
- `crates/bumbledb-lmdb/src/query.rs:1432-1486`: static-empty proof scans literal atoms but does not prove join-level emptiness or expose counters.
- `crates/bumbledb-lmdb/src/query.rs:1051-1070`: prepared plan instantiation clones the full `ExecutionPlan` on cached execution.
- `crates/bumbledb-lmdb/src/query.rs:5619-5652`: aggregate finish uses generic group iteration and sorting even for single global counts.

## Round 2 Completion Criteria

The round is complete when an untraced and traced JOB rerun at `scale=10000`, `warmup=2`, `repeats=10` shows:

- `job_broad_cast_keyword_company` prepare below `50ms` and steady average below `2.5ms`.
- `job_broad_movie_info_star` steady average below `8ms` and `trie_key_reads` reduced by at least `80%`.
- `job_q09_voice_us_actor` prepare below `60ms` and traced steady average below SQLite.
- `job_q16_character_title_us` cold `lftj.build` below `5ms`.
- `job_q24_voice_keyword_actor` cold `lftj.build` below `3ms`.
- `job_movie_link_bridge` steady average below SQLite with at least `1.5x` margin.
- `job_q33_linked_series_companies` steady average below `40us` and static-empty proof counters visible.
- LFTJ build traces identify scan/filter/copy, column construction, and trie sort separately.
