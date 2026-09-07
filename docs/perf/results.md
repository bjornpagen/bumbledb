# Current benchmark results

## Measurement identity

The full local suite completed on **September 6, 2026, at 20:01 CDT** on an
**Apple M2 Max**. Run: `autoresearch-full86`. Engine revision:
`3ed3303eb226de6c98955ab75c296887eaf7704b`. Frozen binary SHA-256:
`044b15d404f1b08d204355e1fdcfd9e62ee976d2f77ad9a93f552d82dafb6737`.

This engine was measured before the 1.0 version/documentation/package cutover;
its recorded crate version is `0.20.3`, not a separately benchmarked 1.0 binary.
The same engine implementation ships in 1.0. The command was the full serial
suite with a frozen executable, an isolated layout-7 corpus and `--shared`.
The host was in active use; user-interactive scheduler boosting was enabled.
Clock flags, load observations and tails are retained. These are shared-host
observations, not isolated-machine or cross-platform promises.

The [GitHub release](https://github.com/bjornpagen/bumbledb/releases/tag/v1.0.0)
carries the report archive. It includes the complete manifest, JSON reports,
logs and completed comparison blocks. Raw evidence remains unchanged; this
page and the 27 charts in `assets/` replace the prior published chart set.

## Coverage

All 15 ordinary benchmark lanes completed: storage; warm, cold, large-result
and tenant application lifecycles; hash probes; reads; scenarios; CRUD;
lawful admission; writes; scale/warmth curves; churn; heap; Primer-shaped work.
The pre-timing oracle completed **2,879 cases**. The read panel contains
**32 families** and the scenario panel **34 queries**. Steady and delete-heavy
churn each completed **10,000 cycles**.

The manifest's external prerequisites remain `NOTRUN-PREREQ`. Its completion
does not substitute for separate correspondence tests, Miri, installed SDK
tests, real S3, Graviton, Linux application performance or a populated store
larger than its available memory. Release correctness CI is a separate gate.
Hash timing without supplied known-answer vectors is not a hash KAT pass.

The main SQLite read comparison reported `all_win=true`; the informational
latency-budget aggregate reported **`budget_ok=false`**. Neither is an
all-workload historical-engine result. Scenario SQLite caps stay caps, not
latency measurements. Parameter rosters include hits and misses, so a mixed
median can move without the same change in total work.

## Native application lifecycle

These protocol-2 measurements include actual native query/result work,
not TypeScript serialization, Effect overhead, network delivery or hosted
tenant activation. Cold-open is not a flushed operating-system page cache.

| Workload | Median | Mean |
|---|---:|---:|
| Warm prepared account projection | 1.750 µs | 1.755 µs |
| Open, prepare and query | 334.458 µs | 337.518 µs |
| First query after related delete/insert | 3.705 ms | 3.800 ms |
| Construct and deliver 100,000 rows | 69.338 ms | 69.357 ms |
| Small-tenant activation | 455.125 µs | 495.191 µs |

The large-result roster delivers 1,600,000 rows across 16 draws. Separate
candidate/baseline/baseline/candidate controls used the same protocol:

| Block | Median | Mean |
|---|---:|---:|
| Candidate 85a | 65.422 ms | 65.803 ms |
| Previous checkpoint 83a | 800.806 ms | 817.130 ms |
| Previous checkpoint 83b | 796.016 ms | 796.144 ms |
| Candidate 85b | 65.915 ms | 67.054 ms |

This supports an approximately **12×** improvement for that specific native
owned-result path relative to checkpoint 83, not the original engine on every
benchmark. Batched scratch writes and borrowed cursor-page reads replace
per-row transactions and redundant copies.

## Read families

Times below are microseconds. These are ordinary unprofiled full86 results;
the clock column records the report's contamination flags, not a declaration
that unflagged cells were isolated from other work.

| Family | Median | Mean | p99 | SQLite median | Clock flag |
|---|---:|---:|---:|---:|---|
| point | 0.500 | 0.495 | 0.583 | 1.458 | none recorded |
| containment_walk | 2.125 | 145.199 | 833.542 | 61.375 | none recorded |
| chain | 215.375 | 221.969 | 449.750 | 1912.166 | none recorded |
| range | 4.833 | 4.875 | 6.333 | 145.375 | none recorded |
| balance | 1.459 | 10.035 | 37.000 | 323.208 | none recorded |
| stats | 392.167 | 406.883 | 568.042 | 83177.750 | none recorded |
| string | 1.125 | 0.974 | 1.209 | 59.208 | none recorded |
| skew | 1179.334 | 1252.716 | 1741.709 | 7753.291 | none recorded |
| spread | 13770.792 | 13694.131 | 15550.291 | 136264.375 | Bumbledb |
| triangle | 1863.291 | 1888.644 | 2275.042 | 38120.583 | none recorded |
| entries_for_account_set | 4.125 | 174.499 | 845.416 | 11.750 | none recorded |
| postings_without_tag | 2.375 | 197.461 | 915.583 | 55.584 | Bumbledb |
| latest_posting_per_account | 127.416 | 132.868 | 210.375 | 20800.000 | none recorded |
| mandate_at_instant | 0.583 | 0.566 | 0.667 | 8.083 | none recorded |
| mandate_overlap | 14.292 | 13.144 | 15.625 | 426.792 | SQLite |
| deep_chain | 441.708 | 411.878 | 817.333 | 3544.875 | SQLite |
| busy_scan | 3.917 | 3.154 | 4.459 | 3548.625 | none recorded |
| meets_chain | 3.000 | 26.438 | 113.667 | 19.708 | none recorded |
| rsvp_union | 926.708 | 964.939 | 1449.125 | 19189.167 | none recorded |
| conflict_pairs | 25.167 | 31.670 | 89.250 | 2910.792 | none recorded |
| conflict_free | 0.958 | 0.922 | 1.541 | 23.583 | none recorded |
| free_busy | 4.333 | 13.254 | 54.666 | 326.333 | SQLite |
| slot_scan | 13.083 | 11.073 | 24.209 | 2908.084 | none recorded |
| slot_booking_overlap | 17.542 | 24.201 | 67.542 | 705.334 | none recorded |
| closure_depth | 4.458 | 487.854 | 1506.083 | 26.416 | none recorded |
| closure_fanout | 12.208 | 43.710 | 179.250 | 28.708 | SQLite |
| disp_probe | 158511.667 | 158303.736 | 171944.500 | 886098.625 | none recorded |
| disp_probe_d24 | 144380.791 | 143639.600 | 156909.750 | 950707.375 | Bumbledb, SQLite |
| disp_probe_d96 | 136799.458 | 139060.527 | 164426.792 | 810221.041 | none recorded |
| disp_stream | 213.375 | 212.902 | 236.583 | 42236.208 | Bumbledb, SQLite |
| disp_stream_d24 | 189.000 | 191.576 | 211.125 | 41600.667 | SQLite |
| disp_stream_d96 | 193.625 | 198.986 | 220.292 | 40497.917 | none recorded |

![Read latency](../../assets/bench-vs-sqlite.svg)

## Scenario queries

Times are microseconds. These reports do not have per-cell clock diagnostics.
An answer count can be zero for a registered parameter draw; that does not
make the entire mixed roster a zero-work query.

| Query | Median | Mean | p99 | SQLite median/outcome |
|---|---:|---:|---:|---:|
| j1_filmography | 0.667 | 2.501 | 10.709 | 6.333 |
| j2_costars | 1.167 | 6.559 | 23.667 | 11.917 |
| j3_keyword_kind | 1.500 | 5.459 | 27.209 | 14.625 |
| j4_five_way | 990.500 | 2528.128 | 7741.666 | 5424.750 |
| j5_country_rollup | 4588.084 | 4600.409 | 4875.750 | 32738.209 |
| j6_keyword_neighborhood | 38.125 | 220.770 | 1020.833 | 1607.458 |
| g1_neighbors | 0.375 | 0.673 | 1.750 | 2.917 |
| g2_two_hop | 0.834 | 95.089 | 584.792 | 12.541 |
| g3_three_hop_count | 1.959 | 953.560 | 6105.083 | 35.833 |
| g4_mutual | 3228.416 | 3275.019 | 4242.250 | 31525.917 |
| g5_triangles_from | 0.958 | 8.318 | 35.292 | 24.625 |
| g6_weighted_hop | 0.542 | 2.763 | 9.792 | 9.083 |
| o1_revenue_by_region | 527.250 | 554.719 | 1085.458 | 288067.750 |
| o2_category_window | 526.292 | 469.584 | 1537.625 | 26611.250 |
| o3_promo_split | 348.125 | 353.854 | 509.459 | 115507.125 |
| o4_segment_category | 28989.583 | 28912.421 | 32387.834 | 418900.000 |
| o5_store_extremes | 724.667 | 760.083 | 1177.250 | 216031.792 |
| o6_brand_drill | 2.416 | 2.207 | 10.250 | 606.833 |
| p1_by_id | 0.625 | 0.587 | 0.750 | 1.083 |
| p2_by_key | 1.208 | 1.122 | 1.292 | 1.334 |
| p3_bucket_fetch | 4.250 | 6.179 | 20.834 | 211.459 |
| p4_size_band | 0.542 | 5.022 | 18.792 | 120.125 |
| p5_keyed_get | 1.291 | 1.165 | 1.500 | 1.416 |
| r1_wash_ring | 8521.167 | 6620.602 | 10629.834 | 111964.750 |
| r2_temporal_ring | 29018.083 | 26044.503 | 130564.666 | 164293.458 |
| r3_bomb_t1 | 2450.041 | 2479.024 | 2638.833 | 32537.750 |
| r4_bomb_t2 | 1198867.125 | 1201626.554 | 1228082.416 | exceeded_cap |
| r5_reciprocal | 412.250 | 321.485 | 707.875 | 3336.708 |
| r6_two_path_count | 16106.500 | 16117.142 | 17223.708 | 694501.083 |
| t1_stab | 0.666 | 5.004 | 12.375 | 20.333 |
| t2_overlap_join | 50264.917 | 50320.448 | 52581.417 | exceeded_cap |
| t3_mixed_mask | 18.875 | 1168.141 | 5070.167 | 1185.708 |
| t4_ray_stab | 18.334 | 14.438 | 24.333 | 4475.875 |
| t5_pack_key | 2.500 | 24.186 | 102.042 | not reported |

![Scenario comparisons](../../assets/bench-scenarios.svg)

## Storage

Compacted LMDB file lengths are compared against checkpointed, indexed
SQLite with matching row counts. They are not RSS or virtual map reservations.
Table-only SQLite is a different indexing contract. Free-page history explains
raw-versus-compacted growth, not the entire remaining index/page overhead.

| Scale / world | Facts | Compacted bytes | Indexed SQLite bytes | Ratio |
|---|---:|---:|---:|---:|
| S / ledger | 253,264 | 33,259,520 | 18,432,000 | 1.804× |
| S / calendar | 192,369 | 31,129,600 | 17,686,528 | 1.760× |
| M / ledger | 2,526,889 | 330,842,112 | 192,147,456 | 1.722× |
| M / calendar | 1,830,369 | 294,043,648 | 176,193,536 | 1.669× |

These compacted sizes are unchanged from the previous measured checkpoint.
There is no storage-reduction claim for the latest result-delivery changes.

![Storage comparison](../../assets/bench-storage.svg)

## Regressions and unfinished comparisons

The single-pass full83/full86 comparison also contains slower cells:
displaced-probe means at 0/24/96 MiB were approximately 1.76×/1.49×/1.26×
the previous checkpoint; mutual-neighbor and five-way-join scenario means
were approximately 1.31× and 1.24×. Some unflagged clock cells are affected,
so these are not dismissed as noise. Many durable mutation wall times were
also slower, with clock contamination recorded in both runs. These observations
do not establish their mechanism or a quiet-host causal regression.

Owned-result and recent-read ABBA series completed. The additional displaced,
scenario and original-engine comparison series did not finish. The owner
prioritized release preparation after the full suite; the supplemental queue
was stopped during its first displaced block, with all partial evidence
retained. No native86 profiles were captured and no all-family native86 survey
exists. Existing earlier native profiles remain useful diagnostics but are
not advertised as this round's captures.

## Chart catalog

All charts below come from this round's reports. The three obsolete
`*-nosync.svg` churn charts were removed: no such supported engine lane
was run. SQLite's maintenance variants remain separately labeled comparisons,
not additional Bumbledb durability options.

| Area | Charts |
|---|---|
| Reads | [Latency](../../assets/bench-vs-sqlite.svg), [ratios](../../assets/bench-speedup.svg), [tails](../../assets/bench-tails.svg), [ratio waterfall](../../assets/ratio-waterfall.svg), [tail fan](../../assets/tails-fan.svg) |
| Scenarios | [All queries](../../assets/bench-scenarios.svg), [joins](../../assets/world-joins.svg), [graph](../../assets/world-graph.svg), [aggregates](../../assets/world-olap.svg), [points](../../assets/world-points.svg), [rings](../../assets/world-rings.svg), [temporal](../../assets/world-temporal.svg), [DNF view](../../assets/adversarial-dnf.svg) |
| Mutation | [Writes](../../assets/bench-writes.svg), [batch rates](../../assets/bench-writes-rates.svg), [throughput](../../assets/write-throughput.svg), [CRUD](../../assets/world-crud.svg), [admission](../../assets/world-lawful.svg) |
| Scaling | [Curves](../../assets/bench-curves.svg), [warmth](../../assets/bench-warmth.svg), [storage](../../assets/bench-storage.svg) |
| Steady churn | [Latency](../../assets/churn-latency-steady.svg), [size](../../assets/churn-size-steady.svg), [throughput](../../assets/churn-throughput-steady.svg) |
| Delete-heavy churn | [Latency](../../assets/churn-latency-delete-heavy.svg), [size](../../assets/churn-size-delete-heavy.svg), [throughput](../../assets/churn-throughput-delete-heavy.svg) |

See the [measurement runbook](measurement-plan.md) to reproduce the suite,
understand its workload boundaries, or collect native call stacks. Do not
compare screenshots without checking source, protocol, draw mix and host.
