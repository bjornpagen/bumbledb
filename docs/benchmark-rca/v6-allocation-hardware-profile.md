# V6 Allocation And Hardware Profile

## Purpose

This document answers whether the current v6 hotset is dominated by allocation pressure, cache locality, branchy traversal, encoded comparison throughput, dictionary/intern overhead, or bulk index write amplification.

This is a measurement-only PRD result. No engine behavior was changed.

## Artifact Paths

Allocation-profile benchmark artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/allocation-hotset-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/allocation-hotset-job-10k.json
```

Sampling artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/sampling-tag-lookup.trace
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/sampling-red-boat.trace
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/sampling-job-load.trace
```

Counter baseline:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-job-10k.json
```

Trace RCA:

```text
docs/benchmark-rca/current-heavy-trace-analysis.md
docs/benchmark-rca/v6-hotset-baseline.md
```

## Commands

Non-JOB hotset allocation profile:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset nonjob \
  --query tag_lookup_join \
  --query red_boat_sailors \
  --query high_rating_red_boats \
  --query triangle_count \
  --query revenue_by_customer_range \
  --query supplier_nation_orders \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/allocation-hotset-nonjob.json
```

JOB hotset allocation profile:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --query job_q09_voice_us_actor \
  --query job_q16_character_title_us \
  --query job_q24_voice_keyword_actor \
  --query job_movie_link_bridge \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/allocation-hotset-job-10k.json
```

Sampling profiles:

```sh
xctrace record --template "Time Profiler" \
  --output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/sampling-tag-lookup.trace \
  --time-limit 20s \
  --no-prompt \
  --target-stdout - \
  --launch -- /Users/bjorn/Documents/bumbledb/target/release/bumbledb-bench \
  --preset nonjob --query tag_lookup_join --format json
```

```sh
xctrace record --template "Time Profiler" \
  --output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/sampling-red-boat.trace \
  --time-limit 20s \
  --no-prompt \
  --target-stdout - \
  --launch -- /Users/bjorn/Documents/bumbledb/target/release/bumbledb-bench \
  --preset nonjob --query red_boat_sailors --format json
```

```sh
xctrace record --template "Time Profiler" \
  --output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/sampling-job-load.trace \
  --time-limit 30s \
  --no-prompt \
  --target-stdout - \
  --launch -- /Users/bjorn/Documents/bumbledb/target/release/bumbledb-bench \
  --preset job --query job_q09_voice_us_actor \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 --format json
```

## Tooling Notes

Available:

```text
xctrace: /usr/bin/xctrace
sample: /usr/bin/sample
```

Unavailable:

```text
samply: not found
```

`xctrace list templates` reports `Time Profiler`, `CPU Counters`, `Allocations`, and other standard templates. Time Profiler traces were produced for `tag_lookup_join`, `red_boat_sailors`, and JOB load. The JOB-load recording was capped at 30 seconds and ended while SQLite loading was still in progress, but it captured the Bumbledb load phase.

`sample 2 1` was tested as a smoke check and failed because process `2` was not running. This confirms the tool exists but requires a live PID workflow. The PRD used `xctrace` instead.

## Non-JOB Allocation Hotset

| Query | Rows | Values | Alloc calls | Bytes allocated | Peak live bytes | Alloc/row | Bytes/row | Execute alloc calls | Execute bytes | LFTJ exec alloc calls | Sink finish alloc calls | Sink emits | Project seen | Project dupes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ledger/tag_lookup_join | 10000 | 20000 | 31889 | 32145530 | 14043702 | 3.19 | 3214.55 | 21672 | 15721920 | 0 | 10001 | 10000 | 10000 | 0 |
| sailors/red_boat_sailors | 10000 | 20000 | 16112 | 20302951 | 4941009 | 1.61 | 2030.30 | 1833 | 17213180 | 1649 | 10001 | 16660 | 16660 | 6660 |
| sailors/high_rating_red_boats | 6660 | 13320 | 8787 | 3124542 | 1063741 | 1.32 | 469.15 | 1167 | 1821711 | 1065 | 6661 | 6660 | 6660 | 0 |
| joinstress/triangle_count | 1 | 1 | 8123 | 25848608 | 11777396 | 8123.00 | 25848608.00 | 5134 | 16789028 | 0 | 2 | 0 | 0 | 0 |
| tpch/revenue_by_customer_range | 2000 | 4000 | 12322 | 49956607 | 23885415 | 6.16 | 24978.30 | 2550 | 26788570 | 2334 | 2002 | 8000 | 0 | 0 |
| tpch/supplier_nation_orders | 5716 | 11432 | 14678 | 50755332 | 24504617 | 2.57 | 8879.52 | 1332 | 26873775 | 1110 | 5717 | 5716 | 5716 | 0 |

## JOB Allocation Hotset

| Query | Rows | Values | Alloc calls | Bytes allocated | Peak live bytes | Alloc/row | Bytes/row | Execute alloc calls | Execute bytes | LFTJ exec alloc calls | Sink finish alloc calls | Sink emits | Project seen | Project dupes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| job/job_q09_voice_us_actor | 1 | 1 | 804876 | 184605884 | 124997643 | 804876.00 | 184605884.00 | 802514 | 40831382 | 0 | 2 | 0 | 0 | 0 |
| job/job_q16_character_title_us | 1 | 1 | 2885 | 118479269 | 96346768 | 2885.00 | 118479269.00 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| job/job_q24_voice_keyword_actor | 0 | 0 | 1938 | 158855749 | 127118903 | n/a | n/a | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| job/job_movie_link_bridge | 1 | 1 | 17976 | 97730981 | 67235834 | 17976.00 | 97730981.00 | 572 | 32798590 | 0 | 2 | 0 | 0 | 0 |

## Analysis Questions

### Is `EncodedProjectSink` allocation-heavy?

Yes for high-output materialized projections, but the pattern depends on query shape.

Evidence:

- `tag_lookup_join`: `10000` project rows, `10001` sink-finish allocations, `21672` execute allocations.
- `red_boat_sailors`: `16660` project rows seen, `6660` duplicates, `10001` sink-finish allocations.
- `high_rating_red_boats`: `6660` project rows, `6661` sink-finish allocations.
- `supplier_nation_orders`: `5716` project rows, `5717` sink-finish allocations.

The near one-allocation-per-output-row pattern in `sink_finish` strongly supports PRD 03. The current output finalization decodes into public `Vec<Vec<Value>>`, which will still allocate final rows, but PRD 03 should reduce hot execute-path allocation and dedup/set insertion before final decode.

### Are high-output LFTJ queries dominated by allocation, branchy traversal, or set insertion?

They are a combination of LFTJ traversal and projection mechanics.

Evidence:

- `red_boat_sailors`: `34153` LFTJ next, `17491` seek, `105789` key reads, `16660` sink emits, `6660` duplicate projection rows.
- `high_rating_red_boats`: similar LFTJ operation count but no duplicates.
- `supplier_nation_orders`: lower but still significant LFTJ operations and `5716` project rows.

Allocation profile suggests the projection sink/final decode is material, but not the only issue. PRD 03 should come first because it unifies output memory layout and removes per-emit set insertion. PRD 05 should then attack LFTJ iterator/key mechanics.

### Does direct chain materialization allocate per row?

Yes, `tag_lookup_join` strongly suggests per-row direct-chain/output allocation pressure.

Evidence:

```text
rows: 10000
sink emits: 10000
direct chain step rows: 20000
direct chain output rows: 10000
alloc calls: 31889
execute alloc calls: 21672
sink finish alloc calls: 10001
```

This supports PRD 04. The direct path should write encoded projection rows into a reusable batch sink rather than allocating/binding/emitting row-by-row.

### Are dictionary intern lookups allocating or mostly map lookup/cache misses?

The query allocation profile does not measure ingest dictionary intern directly. Heavy trace evidence does show dictionary intern dominates JOB ingest event volume:

```text
dict_intern: 2850668 events, 17.6s busy traced
dictionary value already interned: 2098890 events
dictionary value interned: 751778 events
```

This supports PRD 08. The likely issue is repeated lookup and write-path mechanics, not query-time materialization.

### Are query image/trie builds cache-local enough for JOB?

Not proven. Allocation profiles show large allocations for JOB q16/q24/q09 correctness execution, but those include query image/static proof/direct count precompute behavior.

Evidence:

- q16: `118MB` allocated, `96MB` peak live, zero execute allocations.
- q24: `158MB` allocated, `127MB` peak live, zero execute allocations.
- q09: `184MB` allocated, `125MB` peak live, `802514` execute allocations.

This suggests query image/static proof/direct-count paths allocate heavily. But the most urgent non-JOB latency issue is still projection/direct/LFTJ mechanics. Query image/trie layout should be revisited after PRD 03-06.

### Does width-specialized comparison look likely to move current bottlenecks?

Yes, but not first.

Width-specialization is likely useful for:

- LFTJ key reads and comparisons (`triangle_count`, `red_boat_sailors`, `high_rating_red_boats`).
- Static semijoin proof (`q16`, `q24`).
- Direct predicates/range checks.

However, allocation and sink mechanics are strong enough that PRD 03 and PRD 04 should happen before NEON work. SIMD should target ARM NEON only as specified in PRD 06.

## Recommendation

Priority order remains:

1. PRD 03 batched encoded projection sink.
2. PRD 04 batched direct materialization.
3. PRD 05 LFTJ emit and iterator mechanics.
4. PRD 06 ARM NEON width-specialized encoded operations.
5. PRD 07 query image/trie layout only after the above changes expose layout as the next bottleneck.
6. PRD 08 ingest dictionary/index write layout as a separate load-performance track.

## Hardware/Sampling Conclusion

Sampling artifacts were produced with `xctrace` Time Profiler. This environment can create `.trace` bundles non-interactively. A deeper call-tree export from those bundles was not required for this PRD because allocation telemetry and mechanics counters already identified the dominant next steps.

Hardware counter extraction was not completed in this PRD. `xctrace` reports a `CPU Counters` template, so future layout/vectorization work can use it when necessary.

## Compatibility Statement

No backwards compatibility work. No migrations. No layout or engine behavior changes in this PRD.
