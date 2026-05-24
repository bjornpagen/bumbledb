# Trace Baseline

Collected on the local JOB sample with `--open-limit 100000`, accepted scale `115933`, release build with `--features query-tracing`, `--trace-output file`, and `--alloc on`.

## Commands Run

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 3 --warmup 1 --trace-output file --profile-query-label job_q09_voice_us_actor --alloc on
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_broad_cast_keyword_company --format json --repeats 3 --warmup 1 --trace-output file --profile-query-label job_broad_cast_keyword_company --alloc on
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label job_all --alloc on
```

## Raw Trace Files

Trace files were written under `data/traces/`, which is ignored by git through `/data/`.

Representative files:

- `data/traces/job_q09_voice_us_actor-15829-0.json`
- `data/traces/job_q09_voice_us_actor-15829-1.json`
- `data/traces/job_q09_voice_us_actor-15829-2.json`
- `data/traces/job_broad_cast_keyword_company-15878-0.json`
- `data/traces/job_broad_cast_keyword_company-15878-1.json`
- `data/traces/job_broad_cast_keyword_company-15878-2.json`
- `data/traces/job_all-15921-0.json` through `data/traces/job_all-15921-7.json`

## q09 Voice US Actor

Measured reports:

| repeat | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | net_allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 62.860 | 5.523 | 663879 | 108271453 | 12976243 | 0 |
| 2 | 66.749 | 6.271 | 663879 | 108272251 | 12976247 | 0 |
| 3 | 64.024 | 6.637 | 663879 | 108271747 | 12976243 | 0 |

Top spans by elapsed time, repeat 1:

| rank | phase | label | elapsed_ms | alloc_calls | allocated_bytes |
| --- | --- | --- | ---: | ---: | ---: |
| 1 | ExecuteNode | execute Free Join plan | 29.574 | 433322 | 55804372 |
| 2 | ExecuteNode | node=0 | 24.889 | 433194 | 54876830 |
| 3 | PlanSelect | select Free Join plan | 17.220 | 99632 | 6307680 |
| 4 | PlannerStats | collect planner stats | 17.043 | 96455 | 5995635 |
| 5 | ExecuteNode | node=1 | 9.395 | 138272 | 9983106 |
| 6 | ExecuteNode | node=2 | 9.392 | 138243 | 9981472 |
| 7 | ExecuteNode | node=3 | 9.391 | 138233 | 9980892 |
| 8 | PlannerStats | planner relation=Title atom=AtomOccurrenceId(7) | 6.518 | 35403 | 2289014 |
| 9 | ExecuteNode | node=1 | 6.067 | 92807 | 9933096 |
| 10 | ProbeSibling | node=3 atom=AtomOccurrenceId(5) | 5.134 | 64503 | 4943130 |

Top spans by allocated bytes, repeat 1:

| rank | phase | label | elapsed_ms | alloc_calls | allocated_bytes |
| --- | --- | --- | ---: | ---: | ---: |
| 1 | ExecuteNode | execute Free Join plan | 29.574 | 433322 | 55804372 |
| 2 | ExecuteNode | node=0 | 24.889 | 433194 | 54876830 |
| 3 | ExecuteNode | node=1 | 1.161 | 15571 | 12657760 |
| 4 | ExecuteNode | node=2 | 0.822 | 7745 | 12093698 |
| 5 | ExecuteNode | node=3 | 0.822 | 7735 | 12093118 |
| 6 | ExecuteNode | node=1 | 9.395 | 138272 | 9983106 |
| 7 | ExecuteNode | node=2 | 9.392 | 138243 | 9981472 |
| 8 | ExecuteNode | node=3 | 9.391 | 138233 | 9980892 |
| 9 | ExecuteNode | node=1 | 6.067 | 92807 | 9933096 |
| 10 | ExecuteNode | node=1 | 0.345 | 7803 | 6329826 |

Aggregate counters, repeat 1:

| counter | value |
| --- | ---: |
| base_image_cache_hits | 16 |
| base_image_cache_misses | 0 |
| live_rows_scanned | 0 |
| column_values_loaded | 0 |
| loaded_bytes | 0 |
| source_filters_encoded | 5 |
| source_filter_rows_tested | 63410 |
| source_filter_survivors | 46489 |
| colt_nodes_created | 46242 |
| colt_nodes_forced | 9 |
| colt_offsets_scanned | 51987 |
| tuples_yielded | 32086 |
| cover_choices | 79 |
| probe_calls | 284 |
| probe_misses | 52 |
| recursive_node_entries | 79 |
| binding_copies | 32086 |
| source_frame_changes | 243 |
| sink_consumes | 0 |
| decoded_values | 0 |

## Broad Cast Keyword Company

Measured reports:

| repeat | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | net_allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 64.297 | 4.618 | 715822 | 149108114 | 25125801 | 3 |
| 2 | 65.297 | 5.871 | 715822 | 149109768 | 25125937 | 3 |
| 3 | 66.053 | 5.847 | 715822 | 149109881 | 25125817 | 3 |

Top spans by elapsed time, repeat 1:

| rank | phase | label | elapsed_ms | alloc_calls | allocated_bytes |
| --- | --- | --- | ---: | ---: | ---: |
| 1 | ExecuteNode | execute Free Join plan | 28.836 | 437995 | 77810422 |
| 2 | ExecuteNode | node=0 | 26.286 | 437885 | 76785020 |
| 3 | PlanSelect | select Free Join plan | 15.272 | 93156 | 5844296 |
| 4 | PlannerStats | collect planner stats | 15.137 | 90711 | 5616890 |
| 5 | PlannerStats | planner relation=Title atom=AtomOccurrenceId(0) | 6.164 | 35398 | 2288678 |
| 6 | ExecuteNode | node=1 | 4.999 | 71408 | 5376262 |
| 7 | ExecuteNode | node=2 | 4.997 | 71374 | 5374500 |
| 8 | ExecuteNode | node=3 | 4.996 | 71363 | 5373920 |
| 9 | ExecuteNode | node=1 | 4.767 | 64742 | 4956700 |
| 10 | ExecuteNode | node=2 | 4.762 | 64708 | 4954938 |

Top spans by allocated bytes, repeat 1:

| rank | phase | label | elapsed_ms | alloc_calls | allocated_bytes |
| --- | --- | --- | ---: | ---: | ---: |
| 1 | ExecuteNode | execute Free Join plan | 28.836 | 437995 | 77810422 |
| 2 | ExecuteNode | node=0 | 26.286 | 437885 | 76785020 |
| 3 | PlanSelect | select Free Join plan | 15.272 | 93156 | 5844296 |
| 4 | PlannerStats | collect planner stats | 15.137 | 90711 | 5616890 |
| 5 | ExecuteNode | node=1 | 4.999 | 71408 | 5376262 |
| 6 | ExecuteNode | node=2 | 4.997 | 71374 | 5374500 |
| 7 | ExecuteNode | node=3 | 4.996 | 71363 | 5373920 |
| 8 | ExecuteNode | node=1 | 4.767 | 64742 | 4956700 |
| 9 | ExecuteNode | node=2 | 4.762 | 64708 | 4954938 |
| 10 | ExecuteNode | node=3 | 4.761 | 64697 | 4954358 |

Aggregate counters, repeat 1:

| counter | value |
| --- | ---: |
| base_image_cache_hits | 16 |
| base_image_cache_misses | 0 |
| live_rows_scanned | 0 |
| column_values_loaded | 0 |
| loaded_bytes | 0 |
| source_filters_encoded | 0 |
| source_filter_rows_tested | 52534 |
| source_filter_survivors | 52534 |
| colt_nodes_created | 32265 |
| colt_nodes_forced | 7 |
| colt_offsets_scanned | 37534 |
| tuples_yielded | 15115 |
| cover_choices | 156 |
| probe_calls | 30460 |
| probe_misses | 30014 |
| recursive_node_entries | 178 |
| binding_copies | 15115 |
| source_frame_changes | 15407 |
| sink_consumes | 22 |
| decoded_values | 22 |

## Baseline Interpretation

- Warm runs show base-image cache hits and zero measured base-image loads, so this pass isolates warm planning/execution work rather than cold image construction.
- Planner stats still dominate a large fraction of elapsed time despite warm images, because planner stats traverse cached base-image data and allocate heavily.
- Execution allocates much more than planning, primarily through recursive binding/source-map copies and COLT force/probe paths.
- q09 proves an empty result only after 32,086 yielded tuples, 51,987 COLT offsets scanned, and 32,086 binding copies.
- broad emits 22 sink consumes but returns 3 projected rows, showing duplicate projection witnesses are still decoded and materialized before final public set output.

## Ranked Optimization Targets

1. Remove planner stats dependence on base images and per-query distinct scans.
2. Replace recursive binding and source-map cloning with frame/undo state.
3. Replace eager `GhtSource::iter` tuple materialization with streaming or real batching.
4. Reduce COLT force allocation and use tighter map/offset structures.
5. Push selective source filters earlier and short-circuit empty sources.
6. Deduplicate encoded projected facts before decoding values.
7. Add measured storage stats and optional value accelerators only after the scan/allocation fixes above are complete.

## PRD 08 Follow-Up

Command:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label prd08_q09 --alloc on
```

Measured report after planner stats stopped using base images:

| query | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_q09_voice_us_actor` | 43.244 | 5.307 | 567463 | 102294267 | 0 |

Trace acceptance evidence:

- Trace file: `data/traces/prd08_q09-18280-0.json`.
- `BaseImageCacheLookup` spans under `PlannerStats`: 0.
- `BaseImageLoad` spans under `PlannerStats`: 0.
- `select Free Join plan` elapsed: 0.212 ms.

## PRD 09 Follow-Up

Command:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label prd09_q09 --alloc on
```

Measured report after base-image columns moved from per-cell `Vec<u8>` allocation to contiguous fixed-width buffers:

| query | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_q09_voice_us_actor` | 44.131 | 6.073 | 567463 | 102294197 | 0 |

Trace file: `data/traces/prd09_q09-19538-0.json`.

## PRD 10 Follow-Up

Command:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label prd10_q09 --alloc on
```

Measured report after replacing eager GHT tuple vectors with streaming iteration and bounded batch fill:

| query | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_q09_voice_us_actor` | 42.910 | 4.826 | 535347 | 101016803 | 0 |

Trace file: `data/traces/prd10_q09-21799-0.json`.

## PRD 11 Note

COLT now uses `HashMap` for forced map nodes, suffix iteration streams from base-image offsets without forcing, and trace spans distinguish `ColtBuild`, `ColtIter`, `ColtForce`, and `ColtGet`.

`Rc<RefCell<...>>` remains in the private COLT handle structure. No current trace isolates borrow/refcell overhead as material separate from forced map allocation, so replacing it stays an open trace-backed optimization target under the broader COLT allocation/locality work.

Command:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label prd11_q09 --alloc on
```

Measured report after COLT lazy-alignment and `HashMap` forced nodes:

| query | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_q09_voice_us_actor` | 37.990 | 5.201 | 513072 | 110363704 | 0 |

Trace file: `data/traces/prd11_q09-24120-0.json`.

## PRD 12 Follow-Up

Command:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label prd12_q09 --alloc on
```

Measured report after replacing recursive binding/source-map clones with dense binding slots and source undo records:

| query | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_q09_voice_us_actor` | 38.113 | 5.092 | 352792 | 99905193 | 0 |

Trace evidence:

- Trace file: `data/traces/prd12_q09-34955-0.json`.
- `binding_copies`: 0.
- `frame_pushes`: 79.
- `frame_pops`: 79.
- `source_replacements`: 139.

## PRD 13 Follow-Up

Command:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label prd13_q09 --alloc on
```

Measured report after making the default projection sink deduplicate encoded projected facts before decoding:

| query | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_q09_voice_us_actor` | 38.069 | 4.956 | 352792 | 99901827 | 0 |

Trace file: `data/traces/prd13_q09-38731-0.json`.

Decode-count acceptance is covered by `profiled_projection_decodes_only_final_projected_cells`, because q09 returns zero rows and therefore has zero final projected cells to decode.

## PRD 14 Follow-Up

Command:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label prd14_q09 --alloc on
```

Measured report after skipping already-seen encoded projection subtrees:

| query | bumbledb_ms | sqlite_ms | alloc_calls | allocated_bytes | result_rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_q09_voice_us_actor` | 40.170 | 5.253 | 352850 | 99908089 | 0 |

Trace file: `data/traces/prd14_q09-43376-0.json`.

Factorized-expansion acceptance is covered by `encoded_set_sink_avoids_reexpanding_seen_projection_prefix`, because q09 returns zero rows and therefore has no repeated projected fact to skip.
