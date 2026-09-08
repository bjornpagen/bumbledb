# Bumbledb 1.1.0 benchmark results

The complete local suite finished on **2026-09-08T16:32:35.700086+00:00**:
**14 reported lanes, 32 read families, and 34 scenario queries**.
The pre-timing oracle passed **2,879 cases**. This page includes
every current benchmark chart, the full read/scenario tables, and the slower
results as well as the improvements.

## Measurement identity

- Source: `e5d4e4e3d4ba8b865a2dd1805230e2cc19b90335`, crate version **1.1.0**.
- Run: `release-1.1.0-20260908.ozXBJm/full`; fresh isolated corpus, seed 1.
- Frozen executable SHA-256: `ee3d52b4278b5dfa379e1997abeb76e83bfec5f9bc2350789fc27c40ef09018d`.
- Host: Apple M2 Max, 96 GiB RAM, macOS 15.7.7, AC power, 16 KiB pages.
- Toolchain: `nightly-2026-08-15`, ordinary optimized release build.
- Command: `scripts/bench-night.sh <fresh-output> --full --shared --jobs auto --allow-macos-qos`, with
  `BUMBLEDB_BENCH_BIN` selecting the frozen executable and
  `BUMBLEDB_BENCH_DATA` selecting the isolated corpus.

These are **shared-machine observations with up to eight concurrent lane
workers**, capped to this Mac's performance-core count. Each measured process
set and read back user-interactive QoS. macOS steers work toward P-cores but
provides **no hard P-core-only affinity guarantee**. Concurrent lanes contend
for CPU, caches, memory bandwidth and disk; these are not isolated query
latencies. The worker cap does not mean eight cores remain busy after most
lanes finish, and it does not add parallel execution inside a query.

The measurement lock excluded other repository measurements. Every lane used
separate working data; Bumbledb and SQLite within each lane retained the same
sample counts, draws, warmups and durable commit contracts. Raw reports and
the [report index](runs/1.1.0/MANIFEST.json) preserve their source, scheduling
policy, timing boundaries and hashes. Later documentation commits do not
change the measured executable. No new profiles were captured.

The 1.1.0 release evidence archive contains the report index, raw reports,
per-lane logs and measurement provenance. The [1.0.1 release](https://github.com/bjornpagen/bumbledb/releases/tag/v1.0.1)
retains the previous published evidence. Its measured engine source was
`5e83ee60c4e5d88e8ca395daa3de4d92a0c03186`. Nothing here relabels old runs.

## Coverage and limits

All ordinary lanes report `RUN-OK`: storage; warm, cold, large-result and
tenant application lifecycles; hash probes; reads; scenarios; CRUD; lawful
admission; writes; S/M/L scale and warmth curves; heap; and Primer-shaped
work. The complete runner is used, not its compact default subset.

The main read comparison reports **`all_win=true` against SQLite**, while
the informational latency-budget aggregate remains **`budget_ok=false`**.
Neither is a verdict that every workload beats SQLite or every historical
Bumbledb revision. Mutation and admission results below include SQLite wins.
Scenario and scaling timeouts remain caps, never invented latency values.

The manifest's external prerequisites remain `NOTRUN-PREREQ`. Real-S3/IAM,
Graviton performance, Linux Node application timing, and a populated store
larger than available memory were **not run**. Correctness and packaging CI are separate from timing; a benchmark manifest
does not substitute for them. The current cross-platform CI link is in the
[release notes](../release-1.1.md). Raspberry Pi memory/performance remains
unqualified; this is not a 512 MB hardware test. Hash one-shot/streaming
equivalence passed, but known-answer vectors were not supplied: hash KATs
and reused-state hash timing remain **not run**.

## Compared with the last published results

This is a historical comparison with the single run published with 1.0.1,
on the same M2 Max and pinned toolchain. **1.0.1 ran serial lanes; 1.1.0 ran
up to eight concurrent lanes.** That changes contention, so these differences
are **not controlled code speedups or regressions**. Both retain the same
workload settings and scheduler boost.
Read-family medians: **10 lower, 2 unchanged, 20 higher**.
Scenario medians: **15 lower, 3 unchanged, 16 higher**.
Tiny differences can reflect timer granularity or ambient load.

| Selected workload | 1.0.1 median | 1.1.0 median | Median change | Mean change |
| --- | --- | --- | --- | --- |
| Aggregate statistics | 345.792 µs | 359.458 µs | +4.0% | +7.3% |
| Triangle join | 1509.042 µs | 1803.750 µs | +19.5% | +28.6% |
| Slot-booking overlap | 6.042 µs | 10.458 µs | +73.1% | -2.5% |
| Chain join | 247.208 µs | 251.000 µs | +1.5% | -0.3% |
| Closure depth | 13.834 µs | 4.584 µs | -66.9% | +0.3% |
| Store min/max aggregation | 0.522 ms | 0.510 ms | -2.2% | -7.0% |
| Reciprocal-ring query | 0.482 ms | 0.430 ms | -11.0% | -44.4% |
| Temporal overlap join | 64.865 ms | 38.804 ms | -40.2% | -41.7% |
| Construct and deliver 100,000 rows | 63.864 ms | 5.471 ms | -91.4% | -91.4% |
| Small-tenant activation | 0.593 ms | 5.818 ms | +881.8% | +758.3% |

Means and tails accompany medians because the draw rosters mix hits, misses,
and different amounts of result work. Clock flags are retained, not used to
dismiss inconvenient results. Establishing a causal code improvement requires
a separate comparison under matched scheduling and load.

## Native application lifecycle

Protocol 2 measures actual native query/result work, excluding TypeScript
serialization, Effect overhead, network delivery, and hosted activation.
Cold-open is not a flushed operating-system page cache.
All figures in this table are **microseconds**.

| Workload | Median | Mean | p99 |
| --- | --- | --- | --- |
| Warm prepared account projection | 1.750 | 1.767 | 1.958 |
| First query after related delete/insert | 3442.083 | 3463.434 | 4024.167 |
| Open, prepare and query | 4106.209 | 3833.070 | 8583.375 |
| Construct and deliver 100,000 rows | 5470.542 | 5508.315 | 5874.417 |
| Small-tenant activation | 5818.250 | 6715.575 | 21033.500 |

The large-result median is **5.471 ms** end to end. Its separately
recorded execution and delivery medians are **1.535 ms** and
**3.858 ms**; component medians need not sum to the total.
This covers 1,600,000 delivered rows across 16 draws.
Paging completed results is not streaming query execution.

## Read families

Each sample times one call (256 samples per family). Tables use **microseconds**.
Clock flags describe recorded contamination, not proof of isolation when absent.

| Family | Median | Mean | p99 | SQLite median | Clock flag |
| --- | --- | --- | --- | --- | --- |
| point | 0.500 | 1.296 | 20.000 | 1.500 | Bumbledb |
| containment_walk | 2.334 | 156.854 | 954.792 | 77.042 | SQLite |
| chain | 251.000 | 224.229 | 404.500 | 2100.083 | none recorded |
| range | 5.167 | 5.180 | 5.459 | 147.042 | none recorded |
| balance | 1.416 | 9.973 | 36.458 | 389.250 | none recorded |
| stats | 359.458 | 388.453 | 812.959 | 85304.917 | none recorded |
| string | 1.125 | 0.988 | 1.292 | 66.167 | none recorded |
| skew | 1192.541 | 1723.255 | 6241.125 | 10053.584 | none recorded |
| spread | 13073.250 | 14061.101 | 40365.000 | 145274.625 | none recorded |
| triangle | 1803.750 | 1964.477 | 5992.375 | 40210.292 | Bumbledb, SQLite |
| entries_for_account_set | 6.041 | 162.370 | 782.500 | 8.375 | none recorded |
| postings_without_tag | 2.125 | 164.573 | 723.959 | 64.000 | none recorded |
| latest_posting_per_account | 121.292 | 126.145 | 225.833 | 20354.375 | none recorded |
| mandate_at_instant | 0.542 | 0.525 | 0.666 | 8.666 | none recorded |
| mandate_overlap | 12.208 | 11.268 | 17.250 | 434.834 | none recorded |
| deep_chain | 453.416 | 407.179 | 762.000 | 4643.250 | none recorded |
| busy_scan | 3.833 | 3.082 | 4.417 | 3591.791 | none recorded |
| meets_chain | 2.125 | 20.133 | 83.125 | 20.625 | none recorded |
| rsvp_union | 931.542 | 989.443 | 1375.042 | 18978.667 | none recorded |
| conflict_pairs | 22.042 | 29.846 | 75.125 | 3199.667 | none recorded |
| conflict_free | 0.834 | 0.784 | 0.917 | 24.208 | none recorded |
| free_busy | 3.792 | 12.648 | 41.917 | 332.708 | none recorded |
| slot_scan | 13.834 | 10.662 | 14.667 | 2882.083 | none recorded |
| slot_booking_overlap | 10.458 | 16.333 | 37.625 | 5739.792 | none recorded |
| closure_depth | 4.584 | 444.569 | 1345.333 | 55.333 | none recorded |
| closure_fanout | 5.000 | 44.794 | 190.417 | 39.875 | Bumbledb |
| disp_probe | 148019.084 | 149082.430 | 171008.083 | 1122664.667 | SQLite |
| disp_probe_d24 | 173432.167 | 185251.194 | 273375.625 | 978714.459 | none recorded |
| disp_probe_d96 | 159330.250 | 164127.170 | 214089.333 | 1076347.000 | SQLite |
| disp_stream | 172.375 | 175.361 | 195.416 | 40983.125 | none recorded |
| disp_stream_d24 | 177.833 | 181.864 | 197.459 | 41872.667 | none recorded |
| disp_stream_d96 | 184.000 | 185.684 | 201.625 | 41471.167 | none recorded |

![Median read latency against indexed SQLite; lower is better](../../assets/bench-vs-sqlite.svg)

![Read-family median ratios against SQLite, not against historical Bumbledb](../../assets/bench-speedup.svg)

![Read medians and tail latencies for both engines](../../assets/bench-tails.svg)

![The p50, p90 and p99 read distributions remain visible together](../../assets/tails-fan.svg)

![Sorted read and scenario median ratios, with capped comparisons excluded](../../assets/ratio-waterfall.svg)

## Scenario queries

The full six-world roster uses 8 warmups and 64 samples per query.
Times are **microseconds**. A zero answer count for one registered draw does
not mean the entire mixed roster is zero-work. Capped SQLite cells are not
measurements, and these reports have no per-cell clock diagnostics.

| Query | Median | Mean | p99 | SQLite median/outcome |
| --- | --- | --- | --- | --- |
| j1_filmography | 0.500 | 2.952 | 10.458 | 7.875 |
| j2_costars | 1.167 | 6.864 | 25.000 | 13.625 |
| j3_keyword_kind | 1.667 | 5.748 | 19.625 | 15.291 |
| j4_five_way | 1340.500 | 3206.863 | 11213.042 | 5401.125 |
| j5_country_rollup | 4353.291 | 4388.418 | 5194.416 | 38395.834 |
| j6_keyword_neighborhood | 39.583 | 291.363 | 2820.750 | 1705.792 |
| g1_neighbors | 0.334 | 0.629 | 1.625 | 3.167 |
| g2_two_hop | 0.875 | 115.290 | 1681.167 | 19.000 |
| g3_three_hop_count | 1.750 | 776.319 | 3889.041 | 32.750 |
| g4_mutual | 3423.417 | 3454.436 | 4344.792 | 30321.583 |
| g5_triangles_from | 0.792 | 7.247 | 27.708 | 25.042 |
| g6_weighted_hop | 0.500 | 2.580 | 9.125 | 6.750 |
| o1_revenue_by_region | 490.584 | 561.736 | 1562.500 | 287934.958 |
| o2_category_window | 480.000 | 413.358 | 1162.583 | 59659.959 |
| o3_promo_split | 339.542 | 341.190 | 408.042 | 119592.042 |
| o4_segment_category | 28535.625 | 29001.410 | 37624.833 | 433383.916 |
| o5_store_extremes | 510.208 | 534.861 | 869.541 | 221529.958 |
| o6_brand_drill | 1.958 | 1.771 | 2.791 | 599.833 |
| p1_by_id | 0.666 | 0.595 | 0.833 | 1.125 |
| p2_by_key | 1.250 | 1.169 | 1.375 | 1.417 |
| p3_bucket_fetch | 4.042 | 5.880 | 16.625 | 218.792 |
| p4_size_band | 0.416 | 1.505 | 5.000 | 118.334 |
| p5_keyed_get | 1.250 | 1.130 | 1.375 | 1.500 |
| r1_wash_ring | 9004.209 | 7261.195 | 15679.583 | 114384.500 |
| r2_temporal_ring | 26098.500 | 20401.552 | 41869.834 | 167596.292 |
| r3_bomb_t1 | 2650.042 | 2669.577 | 3091.250 | 32670.667 |
| r4_bomb_t2 | 1311644.667 | 1705084.142 | 6161316.666 | exceeded_cap |
| r5_reciprocal | 429.666 | 344.518 | 709.333 | 3404.125 |
| r6_two_path_count | 10409.208 | 10513.190 | 12701.584 | 679207.583 |
| t1_stab | 0.500 | 4.792 | 11.875 | 43.791 |
| t2_overlap_join | 38804.041 | 39324.763 | 52662.208 | exceeded_cap |
| t3_mixed_mask | 13.833 | 1002.890 | 4179.750 | 1190.458 |
| t4_ray_stab | 18.417 | 14.018 | 22.959 | 4242.083 |
| t5_pack_key | 2.250 | 21.463 | 83.958 | not reported |

The canonical table above shows the `sqlite` lane. Additional tuned and
hand-written SQL comparators are retained below, in **microseconds**.

| Query | Additional SQLite lane | Median/outcome | Mean | p99 |
| --- | --- | --- | --- | --- |
| r2_temporal_ring | sqlite-tuned | 119299.875 | 92472.115 | 156417.417 |
| t2_overlap_join | sqlite-tuned | 518005.333 | 522922.678 | 681601.458 |
| t5_pack_key | sqlite-hand | 106.125 | 919.627 | 4052.334 |

![All scenario queries, including the SQLite cap annotations](../../assets/bench-scenarios.svg)

### Multiway joins and country rollups

![Multiway joins and country rollups](../../assets/world-joins.svg)

### Graph traversal, reciprocal edges and triangle counting

![Graph traversal, reciprocal edges and triangle counting](../../assets/world-graph.svg)

### Analytic sums, grouped extrema and dimension joins

![Analytic sums, grouped extrema and dimension joins](../../assets/world-olap.svg)

### Point, key and bucket access

![Point, key and bucket access](../../assets/world-points.svg)

### Cyclic and temporal ring workloads

![Cyclic and temporal ring workloads](../../assets/world-rings.svg)

### Interval stabs, overlap joins and temporal masks

![Interval stabs, overlap joins and temporal masks](../../assets/world-temporal.svg)

![Scenario comparisons where SQLite exceeded its cap; caps are not completed timings](../../assets/adversarial-dnf.svg)

## Durable mutation and admission

Both engine sides retain their documented durable commit contract. On macOS,
the comparisons include media flushes; there is no Bumbledb no-sync lane.
Small durable commits are strongly affected by storage and scheduling.
Times below are **milliseconds** and rates come directly from measured means.

| Operation | Rows/batch | Bumbledb median | SQLite median | Bumbledb rows/s | SQLite rows/s |
| --- | --- | --- | --- | --- | --- |
| commit_b1 | 1 | 8.751 | 11.570 | 108.14 | 79.18 |
| commit_b10 | 10 | 14.779 | 27.175 | 692.5 | 349.26 |
| commit_b100 | 100 | 19.442 | 142.842 | 4,874.39 | 544.43 |
| commit_b1000 | 1,000 | 44.159 | 193.478 | 22,054.3 | 4,383.73 |
| delete_b1 | 1 | 8.322 | 8.676 | 124.48 | 114.42 |
| delete_b10 | 10 | 12.428 | 11.770 | 799.48 | 880.56 |
| delete_b100 | 100 | 21.156 | 34.949 | 4,512.06 | 2,630.78 |
| delete_b1000 | 1,000 | 47.786 | 73.171 | 21,172.03 | 10,649.2 |
| insert_stream | 200,000 | 697.558 | 1342.290 | 260,249.68 | 142,111.41 |

Rates and latency reflect the concurrent lane schedule, including competing
disk work. They are not an across-the-board write-speedup claim.

![Durable writes and first-read costs from the main comparison runner](../../assets/bench-writes.svg)

![Commit, delete and insertion rates for each supported batch size](../../assets/bench-writes-rates.svg)

![Write throughput across the measured commit and delete batch ladders](../../assets/write-throughput.svg)

![Application CRUD comparisons under matched durability contracts](../../assets/world-crud.svg)

![Accepted and refused transactions under declared-law enforcement](../../assets/world-lawful.svg)

CRUD and lawful-admission post-state checks both reported `ok`. Refused-law
samples include validation and abort, not successful commits. Bumbledb emits
the complete violation evidence; SQLite uses its constraint/trigger refusal
and rollback. The charts deliberately retain SQLite's faster refusal cases.

## Storage and scaling

Compacted LMDB file lengths are compared with checkpointed, indexed SQLite
at matching row counts, not RSS or virtual map reservations. Table-only
SQLite has a different indexing contract. Free-page history explains some
raw-versus-compacted growth, not the remaining page and index overhead.

| Scale / world | Facts | Compacted bytes | Indexed SQLite bytes | Ratio |
| --- | --- | --- | --- | --- |
| S / ledger | 253,264 | 33,259,520 | 18,432,000 | 1.804× |
| S / calendar | 192,369 | 31,129,600 | 17,686,528 | 1.760× |
| M / ledger | 2,526,889 | 330,842,112 | 192,147,456 | 1.722× |
| M / calendar | 1,830,369 | 294,043,648 | 176,193,536 | 1.669× |

![Compacted storage against indexed and table-only SQLite](../../assets/bench-storage.svg)

Scale curves exercise S/M/L. Their four-family roster is narrower than the
32-family read panel. SQLite caps remain excluded-and-counted rather than
fabricated results. The raw report records fact counts for every scale.

| Family | S median (µs) | M median (µs) | L median (µs) |
| --- | --- | --- | --- |
| triangle | 1752.625 | 21099.209 | not timed (gate cap) |
| point | 0.458 | 0.500 | 0.541 |
| busy_scan | 4.000 | 37.291 | 668.125 |
| closure_fanout | 0.542 | 1.042 | 79.042 |

The large-scale triangle comparison hit SQLite's cap during the pre-timing
result gate, so neither engine has a timing for that point. At medium scale,
SQLite hit its timing cap; Bumbledb's completed timing remains recorded.

![S/M/L scaling for the four registered curve families](../../assets/bench-curves.svg)

![Reopen-cold, warm and memoized execution at the first requested scale](../../assets/bench-warmth.svg)

Reopen-cold creates a new handle and prepared query in the same process with
a warm OS page cache. Open and prepare are outside its timed first execution.
Large populated corpora here do not establish larger-than-memory performance.


## Heap and Primer-shaped workloads

The heap lane compares a frozen in-memory instance with LMDB access; it is
not an alternative durability promise. Point/access medians are microseconds.

| Operation | Frozen heap | LMDB |
| --- | --- | --- |
| get | 0.167 | 0.583 |
| contains | 0.208 | 0.500 |
| scan | 16.708 | 29.042 |

| Admission facts | Wall time (ms) | ns/fact |
| --- | --- | --- |
| 693 | 0.363 | 523.15 |
| 2,633 | 1.947 | 739.52 |
| 10,392 | 7.014 | 674.98 |
| 41,432 | 28.675 | 692.11 |

Publication took **1132.657 ms**; the 500-row join took
**47.667 µs**. These are individual wall measurements, not percentile distributions.

Primer uses **200,000 facts across 12 relations**. Its phase figures are
single measured wall times, not medians; allocations were not instrumented.

| Phase | Rows | Wall time (ms) |
| --- | --- | --- |
| builder_load | 200,000 | 62.440 |
| builder_admit | 200,000 | 201.220 |
| builder_publish | 200,000 | 613.544 |
| delta_create | 0 | 181.975 |
| delta_seed | 99,996 | 249.188 |
| delta_write | 100,004 | 335.469 |
| scan_decode | 200,000 | 20.969 |

## Hash probes

Fresh-state hashing covers 150 candidate/input combinations,
including unaligned buffers. One-shot/streaming equivalence passed. This is
not a change to the database's chosen hash, a cryptographic review, or a
known-answer-vector pass. Reused-state timing and known-answer vectors were
not run. The [raw hash report](runs/1.1.0/hash-probe/hash-probe.json) contains
every input length, alignment, mean and tail.

Selected aligned inputs, **median nanoseconds**, including state construction
and finalization into a new output vector:

| Candidate | 32 B | 1 KiB | 8 MiB |
| --- | --- | --- | --- |
| blake3-full-32 | 90 | 1,168 | 4,436,166 |
| blake3-trunc-16 | 89 | 1,168 | 4,470,041 |
| blake3-derive-key-16 | 169 | 1,251 | 4,458,375 |
| blake3-row-prefix-rel0-16 | 131 | 1,369 | 4,508,459 |
| aegis-128l-mac-16 | 65 | 113 | 489,584 |

## Synthetic renderer examples

These two repository images are **test fixtures**, not benchmarks or captured
profiles. Their invented microsecond values test the renderer's layout and
color rules. They are shown here to make the entire repository image catalog
visible without presenting synthetic data as release evidence.

![Synthetic miniature flamegraph renderer fixture](../../scripts/flame-fixtures/mini.svg)

The miniature fixture checks nested frame widths and labels.

![Synthetic differential flamegraph renderer fixture](../../scripts/flame-fixtures/diff.svg)

The differential fixture checks red growth, blue shrinkage and unchanged frames.

All **21 measured charts and both synthetic fixtures** are embedded on this
page. See the [measurement runbook](measurement-plan.md) for reproducible
commands and workload boundaries. Compare reports using source, protocol,
draw mix and host—not screenshots alone.
