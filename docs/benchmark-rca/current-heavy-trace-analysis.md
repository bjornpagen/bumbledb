# Current Heavy Trace Analysis

## Purpose

Capture the current post-v5 engine state with full benchmark tracing enabled, after deleting redundant mixed/hash-probe/tiny-project paths.

These traces are intentionally heavy and distort absolute benchmark timings. Use them to identify hot regions and event volume, not to compare product latency against untraced artifacts.

## Cleanup

Old trace-labeled outputs under the temp artifact root were removed before this run:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/*trace.jsonl
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/*trace-summary.txt
```

New trace root:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces
```

## Commands

Non-JOB:

```sh
RUST_LOG="bumbledb_lmdb=trace,bumbledb_bench=debug" \
cargo run -p bumbledb-bench --release -- \
  --preset nonjob \
  --trace \
  --trace-format json \
  --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/nonjob-trace.jsonl \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/nonjob-benchmark.json
```

JOB 10k:

```sh
RUST_LOG="bumbledb_lmdb=trace,bumbledb_bench=debug" \
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --trace \
  --trace-format json \
  --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/job-10k-trace.jsonl \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/job-10k-benchmark.json
```

## Artifacts

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/nonjob-benchmark.json      50K
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/nonjob-trace.jsonl         2.0G
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/job-10k-benchmark.json     40K
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/job-10k-trace.jsonl        3.7G
```

## Benchmark Summary Under Trace

| Suite | Queries | BDB wins | Gate failures |
|---|---:|---:|---:|
| Non-JOB | 10 | 1 | 1 |
| JOB 10k | 8 | 6 | 0 |

The non-JOB gate failure is `joinstress/chain4_from_a` at `86us` against a `75us` gate. This is trace overhead, not an untraced product regression.

## Non-JOB Query Phase Shape

| Query | BDB sample us | Runtime | Execute us | LFTJ exec us | Sink finish us | Materialized values |
|---|---:|---|---:|---:|---:|---:|
| postings_for_holder_range | 208 | Lftj | 3430 | 42 | 20 | 6 |
| balances_by_instrument | 211 | Lftj | 3642 | 47 | 20 | 6 |
| tag_lookup_join | 94455 | IndexNestedLoop | 90407 | 0 | 788 | 20000 |
| red_boat_sailors | 172935 | Lftj | 183320 | 175766 | 994 | 20000 |
| sailor_range_reserves | 72 | DirectKernel | 40 | 0 | 14 | 10 |
| high_rating_red_boats | 73641 | Lftj | 70532 | 70485 | 530 | 13320 |
| chain4_from_a | 86 | IndexNestedLoop | 19 | 0 | 10 | 1 |
| triangle_count | 10232 | Lftj | 15747 | 10368 | 11 | 1 |
| revenue_by_customer_range | 78485 | Lftj | 85239 | 81022 | 190 | 4000 |
| supplier_nation_orders | 57345 | Lftj | 63497 | 57311 | 492 | 11432 |

Under heavy tracing, high-output materialized queries are dominated by LFTJ/direct execution and sink emission volume. The trace contains about `1.55M` `bumbledb.query.sink.emit` close events in non-JOB.

## JOB Query Phase Shape

| Query | BDB sample us | Runtime | Direct count us | Static proof us | LFTJ exec us | Materialized values |
|---|---:|---|---:|---:|---:|---:|
| job_broad_cast_keyword_company | 436 | DirectKernel | 412 | 1 | 0 | 1 |
| job_broad_movie_info_star | 520 | DirectKernel | 446 | 1 | 0 | 1 |
| job_q01_top_production | 271 | StaticEmpty | 0 | 166 | 0 | 0 |
| job_q09_voice_us_actor | 1008 | DirectKernel | 52600 | 922 | 0 | 1 |
| job_q16_character_title_us | 679 | StaticEmpty | 0 | 613 | 0 | 0 |
| job_q24_voice_keyword_actor | 705 | StaticEmpty | 0 | 584 | 0 | 0 |
| job_movie_link_bridge | 258 | Lftj | 0 | 1 | 61 | 1 |
| job_q33_linked_series_companies | 130 | StaticEmpty | 0 | 0 | 0 | 0 |

JOB query time under tracing is dominated by direct count/static proof/query image, not projection materialization.

## Top Span Aggregates

### Non-JOB

Top busy spans:

| Span | Count | Busy |
|---|---:|---:|
| `bumbledb.query.execute_prepared_with_options` | 330 | 16100ms |
| `bumbledb.query.free_join.dispatch` | 231 | 12868ms |
| `bumbledb.query.lftj.execute` | 231 | 12829ms |
| `bumbledb.storage.bulk_load` | 4 | 12140ms |
| `bumbledb.insert` | 402504 | 8317ms |
| `bumbledb.query.sink.emit` | 1552419 | 360ms busy, 6831ms idle |
| `bumbledb.dict_intern` | 42503 | 288ms busy |
| `bumbledb.query.project` | 231 | 89ms |
| `bumbledb.query.sink.finish` | 231 | 73ms |

Important parent/child paths:

| Parent -> Child | Count | Busy |
|---|---:|---:|
| `execute_prepared_with_options -> free_join.dispatch` | 231 | 12868ms |
| `free_join.dispatch -> lftj.execute` | 231 | 12829ms |
| `lftj.execute -> sink.emit` | 1222386 | 281ms |
| `execute_prepared_with_options -> sink.emit` | 330033 | 79ms |
| `sink.finish -> project` | 132 | 63ms |

The important number here is not the traced busy time of `sink.emit`; it is the event volume. Non-JOB materialized workloads produce roughly `1.55M` sink emits in this run.

### JOB 10k

Top busy spans:

| Span | Count | Busy |
|---|---:|---:|
| `bumbledb.write_txn` | 1 | 75700ms |
| `bumbledb.insert` | 792400 | 57821ms |
| `bumbledb.dict_intern` | 2850668 | 17631ms |
| `bumbledb.query.execute_prepared_with_options` | 264 | 287ms |
| `bumbledb.query.image` | 264 | 98ms |
| `bumbledb.query_image.build` | 8 | 95ms |
| `bumbledb.query.static_semijoin.prove` | 132 | 57ms |
| `bumbledb.query.free_join.dispatch` | 33 | 8ms |
| `bumbledb.query.lftj.build` | 33 | 6ms |
| `bumbledb.query.lftj.execute` | 33 | <1ms |

Important parent/child paths:

| Parent -> Child | Count | Busy |
|---|---:|---:|
| `write_txn -> insert` | 792400 | 57821ms |
| `insert -> dict_intern` | 2850668 | 17631ms |
| `execute_prepared_with_options -> query.image` | 264 | 98ms |
| `query.image -> query_image.build` | 8 | 95ms |
| `execute_prepared_with_options -> static_semijoin.prove` | 132 | 57ms |
| `execute_prepared_with_options -> free_join.dispatch` | 33 | 8ms |

JOB query spans do not show the same per-row sink emission explosion. JOB tracing overhead is mostly ingest-time storage/dictionary work and query-image/static-proof work.

## Trace Slowdown Shape

Heavy tracing distorts absolute timings. The distortion pattern is still informative because it correlates with event density.

### Non-JOB Trace Slowdown

| Query | Untraced us | Traced us | Traced/Untraced |
|---|---:|---:|---:|
| postings_for_holder_range | 49 | 208 | 4.24x |
| balances_by_instrument | 50 | 211 | 4.22x |
| tag_lookup_join | 7116 | 94455 | 13.27x |
| red_boat_sailors | 6877 | 172935 | 25.15x |
| sailor_range_reserves | 8 | 72 | 9.00x |
| high_rating_red_boats | 5295 | 73641 | 13.91x |
| chain4_from_a | 16 | 86 | 5.38x |
| triangle_count | 10255 | 10232 | 1.00x |
| revenue_by_customer_range | 2900 | 78485 | 27.06x |
| supplier_nation_orders | 3407 | 57345 | 16.83x |

High-output materialized queries are the ones that explode under tracing. That strongly suggests the hottest mechanics are high-frequency per-binding/per-output operations, not one-time planning or query-image setup.

### JOB Trace Slowdown

| Query | Untraced us | Traced us | Traced/Untraced |
|---|---:|---:|---:|
| job_broad_cast_keyword_company | 371 | 436 | 1.18x |
| job_broad_movie_info_star | 448 | 520 | 1.16x |
| job_q01_top_production | 191 | 271 | 1.42x |
| job_q09_voice_us_actor | 923 | 1008 | 1.09x |
| job_q16_character_title_us | 606 | 679 | 1.12x |
| job_q24_voice_keyword_actor | 634 | 705 | 1.11x |
| job_movie_link_bridge | 126 | 258 | 2.05x |
| job_q33_linked_series_companies | 55 | 130 | 2.36x |

JOB queries mostly do not have the high-output per-row behavior. They are static-proof/direct-count/query-image dominated.

## Event Volume

### Non-JOB

Top non-close message counts:

| Message | Count |
|---|---:|
| `put current index entry` | 1225009 |
| `dictionary value interned` | 42503 |
| `free join query planned` | 231 |
| `free join query executed` | 231 |

### JOB 10k

Top non-close message counts:

| Message | Count |
|---|---:|
| `put current index entry` | 2357403 |
| `dictionary value already interned` | 2098890 |
| `dictionary value interned` | 751778 |
| `free join query planned` | 33 |
| `free join query executed` | 33 |

This separates the two optimization worlds:

- query latency work is mostly non-JOB materialized per-binding mechanics
- ingest work is JOB dictionary/index insertion mechanics

## Interpretation

### Query Runtime

The clearest query-side target is not allocation first. The trace points at execution mechanics:

- non-JOB materialized joins spend time in LFTJ/direct execution loops
- high-output materialized queries produce huge sink emission volume
- sink finish/project itself is not the main wall-clock span, but per-row sink emission is extremely frequent
- JOB is already mostly static proof/direct count/query image work

The strongest evidence is the traced/untraced slowdown split. Non-JOB high-output materialized queries are hit extremely hard by per-event tracing, while JOB is not. That means the next query work should attack per-binding/per-row loops before memory layout rewrites.

Important nuance: q09 prepared-plan samples are around `923us`, but the first/cold correctness path reports about `53ms` of direct-count work. This indicates that factorized/precomputed-count state is still doing useful warm work outside the prepared result cache counters. If we care about first-run q09 latency, direct-count precompute is a separate target. If we care about repeated prepared-plan latency, q09 is already acceptable.

The next query-performance pass should focus on reducing per-row/per-binding overhead:

1. Batch direct chain materialization for high-output direct paths like `tag_lookup_join`.
2. Batch or compress LFTJ emit paths so `sink.emit` is not called once per binding when output is simple.
3. Specialize LFTJ traversal/intersection for encoded width 1 and 8.
4. Keep decoding at final output boundary only.

### Load/Storage Runtime

For load-heavy JOB traces, the largest spans are:

- `bumbledb.insert`
- `bumbledb.dict_intern`
- `bumbledb.write_txn`

If ingest performance matters, the next pass should target dictionary interning and bulk-load write layout, not query algorithms.

### Allocation vs Layout vs Vectorization

This trace does not directly prove allocation cost because allocation profiling was not enabled. It does show where to look:

- For query latency: LFTJ/direct loop mechanics and sink emission volume are the likely highest leverage.
- For ingest: dictionary intern and insert path dominate.
- For vectorization: width-specialized encoded comparisons in LFTJ/static proof/direct kernels are plausible, especially width 1 enums and width 8 serial/timestamp/integer fields.
- For memory layout/cache locality: query image and trie iteration should be inspected with hardware counters or allocation/profile runs before changing layout.

Do not start with bit packing or broad memory-layout rewrites. The trace does not show query-image build as the main repeated query cost for non-JOB materialized workloads. It shows repeated execution and emit mechanics.

## Recommended Next Step

Run a focused profiling suite rather than adding new algorithms:

1. Add cheap non-tracing counters to every benchmark artifact: sink emits, bindings yielded, LFTJ iterator opens/seeks/next/key reads, projection dedup inserts/hits, decoded values, and dictionary reverse lookups.
2. Allocation-profile only the hot query set: `tag_lookup_join`, `red_boat_sailors`, `high_rating_red_boats`, `revenue_by_customer_range`, `supplier_nation_orders`, `job_q09`, `job_q16`, `job_q24`.
3. Implement batched direct-chain materialization for `tag_lookup_join`-like shapes.
4. Implement batched encoded projection emission for LFTJ outputs.
5. Then evaluate width-specialized LFTJ comparison/intersection for width 1 and width 8 fields.

The trace says to optimize the workhorse mechanics we kept: direct kernels, pure Free Join/LFTJ, static proof, and encoded sinks.
