# Allocation Redesign Baseline

Captured: 2026-05-24

## Authority

No-query-tracing release allocation counts are the authoritative allocation baseline for this suite. Trace-enabled allocation counts are diagnostic only because trace spans, labels, counters, and JSON rendering allocate substantially more than the query execution hot path.

## Commands

```bash
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 3 --warmup 1 --alloc on
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label allocation_prd00_q09 --alloc on
```

## No-Trace Release Baseline

The allocation counts and byte counts were stable across all three measured repeats. `elapsed_nanos` is the median of the three measured repeats.

| query | alloc_calls | allocated_bytes | elapsed_nanos | result_rows |
| --- | ---: | ---: | ---: | ---: |
| `job_broad_cast_keyword_company` | 129544 | 9894250 | 9555708 | 3 |
| `job_broad_movie_info_star` | 108015 | 10911630 | 7139833 | 3 |
| `job_q01_top_production` | 13881 | 1331834 | 813167 | 0 |
| `job_q09_voice_us_actor` | 129133 | 12582494 | 9862125 | 0 |
| `job_q16_character_title_us` | 66623 | 5850916 | 5485583 | 0 |
| `job_q24_voice_keyword_actor` | 98802 | 9413531 | 7755875 | 0 |
| `job_movie_link_bridge` | 100872 | 10071425 | 7110333 | 0 |
| `job_q33_linked_series_companies` | 20967 | 3541245 | 2110250 | 0 |

## Traced Q09 Diagnostic

Trace file: `data/traces/allocation_prd00_q09-58243-0.json`

Traced release q09 diagnostic result:

| query | alloc_calls | allocated_bytes | elapsed_nanos | result_rows |
| --- | ---: | ---: | ---: | ---: |
| `job_q09_voice_us_actor` | 259886 | 84196468 | 28457708 | 0 |

Selected diagnostic counters:

| counter | value |
| --- | ---: |
| `source_filters_encoded` | 5 |
| `source_filter_rows_tested` | 63410 |
| `source_filter_survivors` | 46489 |
| `colt_nodes_created` | 46242 |
| `colt_nodes_forced` | 9 |
| `colt_offsets_scanned` | 51987 |
| `colt_map_entries_built` | 46234 |
| `tuples_yielded` | 32086 |
| `probe_calls` | 330 |
| `probe_misses` | 52 |
| `binding_copies` | 0 |
| `binding_conflicts` | 32011 |

The traced allocation totals are not used as budgets. They identify phase shape and hot regions only.

## Target Areas

- COLT force currently creates heap-shaped node and map state proportional to distinct forced keys.
- COLT iteration still owns tuple bytes for map keys and bounded batches.
- Probe key construction still creates owned `EncodedTuple` values.
- Source handle state still clones `Rc<RefCell<ColtNode>>` handles and stores heap-owned metadata.

## PRD 05 No-Trace Checkpoint

After replacing COLT map keys with 8-byte and 16-byte inline owned keys, no-query-tracing release allocation calls improved on q09 and both broad queries. Allocated bytes improved versus the PRD 00 baseline for every JOB sample query.

Command:

```bash
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 1 --warmup 1 --alloc on
```

| query | alloc_calls | allocated_bytes | elapsed_nanos | result_rows |
| --- | ---: | ---: | ---: | ---: |
| `job_broad_cast_keyword_company` | 97287 | 9636194 | 8498667 | 3 |
| `job_broad_movie_info_star` | 59550 | 10523910 | 6041708 | 3 |
| `job_q01_top_production` | 8809 | 1250698 | 849542 | 0 |
| `job_q09_voice_us_actor` | 82900 | 12203798 | 8342250 | 0 |
| `job_q16_character_title_us` | 51866 | 5675092 | 4907041 | 0 |
| `job_q24_voice_keyword_actor` | 69045 | 9117707 | 6608208 | 0 |
| `job_movie_link_bridge` | 53824 | 9695041 | 4714875 | 0 |
| `job_q33_linked_series_companies` | 15225 | 3495309 | 1747500 | 0 |

## PRD 14 Non-COLT Allocation Cleanup

Changed hotspot: trace label formatting in hot query execution paths. Before this cleanup, many `format!` labels were constructed even in no-query-tracing release builds. The cleanup gates those labels behind compile-time tracing so release no-trace execution does not pay for diagnostic strings.

Command:

```bash
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_broad_cast_keyword_company --query job_broad_movie_info_star --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --alloc on
```

| query | before alloc_calls | after alloc_calls | before allocated_bytes | after allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_broad_cast_keyword_company` | 65119 | 19336 | 13182458 | 10475760 | 3 |
| `job_broad_movie_info_star` | 11185 | 10070 | 15910962 | 15852662 | 3 |
| `job_q09_voice_us_actor` | 36730 | 4104 | 18184538 | 16040798 | 0 |

## Architecture Contract

Allowed:

- LMDB as the only durable store.
- Snapshot-local base images, GHTs, and COLTs as private execution structures.
- Private sink/fold boundaries for query execution.
- Exact SQLite `SELECT DISTINCT` only as benchmark/reference oracle.
- AArch64 NEON only when the suite reaches the NEON PRD.

Forbidden:

- SQL frontend, server mode, async public API, runtime DDL, alternate durable backends, fake storage, app-level COW storage replacement.
- Bag semantics, nulls, floats, public aggregation, or duplicate-preserving public query output.
- DuckDB as planner/runtime dependency.
- x86 SIMD or x86 runtime dispatch.

## Suite Status

This allocation-first PRD suite supersedes the remaining implementation order in `docs/free-join-performance-hardening-prds/` until the allocation goals are met.
