# BumbleDB 1.3.0 benchmark results

The full local suite finished on **September 11, 2026 at 18:09 UTC**:
**13 measured lanes, 32 read families, and 34 scenario queries**.
The pre-timing oracle passed **2,879 cases**. All lanes and chart generation
completed successfully. This page includes all **21 measured charts**, the
complete workload tables, and the historical comparison with 1.1.0.

## Measurement identity

- Source: `a022bfacdff3dba6903284767e302ed6eb9f5a3f`, crate version **1.3.0**.
- Run: `release-1.3.0-20260911.pcxa6o0b/full`; fresh isolated corpus, seed 1.
- Frozen executable SHA-256: `1953305d464b4d0cc43fc54fc9333e1caabd44ded90905c33452326b874378b7`.
- Host: Apple M2 Max, 96 GiB RAM, macOS 15.7.7, AC power, 16 KiB pages.
- Toolchain: `nightly-2026-08-15`, ordinary optimized release build.
- Command: `scripts/bench-night.sh <fresh-output> --full --shared --jobs 1 --allow-macos-qos`,
  with `BUMBLEDB_BENCH_BIN` selecting the frozen executable and
  `BUMBLEDB_BENCH_DATA` selecting the isolated corpus.

The build was frozen from a clean checkout before timing. These are
**shared-host measurements with one lane at a time**. Each measured process
set and read back user-interactive QoS; macOS provides no hard P-core affinity.
The measurement lock excluded other participating benchmark processes.
Ambient application and operating-system activity still affected the host;
recorded clock flags and load averages remain in the evidence.

The [report index](runs/1.3.0/MANIFEST.json), [source and host metadata](runs/1.3.0/SOURCE.json),
[verification log](runs/1.3.0/verify.log), and all 13 raw JSON reports are
checked in. Every report hash and the frozen executable hash were verified.
Commands in the index retain their original host paths; artifact paths are
relative to the index. Later documentation commits do not change the measured
source. No new native profiles were captured.

## Coverage and limits

All ordinary lanes report `RUN-OK`: storage; warm, cold, large-result, and
tenant native lifecycles; hash probes; reads; scenarios; CRUD; lawful admission;
durable writes; S/M/L scaling and warmth; and heap work.

The main read panel reports **`all_win=true` against indexed SQLite**.
The informational latency-budget aggregate is **`budget_ok=false`**; budgets
were not release gates. Durable writes and constraint refusals include SQLite
wins. Capped comparisons remain caps, not fabricated timings.

This run does not qualify real S3/IAM deployments, Graviton or Linux Node
application performance, a Raspberry Pi with 512 MB RAM, or a populated store
larger than available memory. The timing driver's standalone correspondence
and conformance-test prerequisites are separate from its completed pre-timing
oracle. Correctness/packaging CI and registry installation checks are separate
from benchmarks. Hash equivalence passed; no known-answer vector file was
supplied, and reused-state hash timing was not run.

## Compared with the previous full run

The [1.1.0 raw reports](runs/1.1.0/README.md) measured source
`548193d46f645ff4a4f007517733deb4bd389569` on the same M2 Max and toolchain.
Both runs use serial lanes, matching workload rosters and scheduler boosting.
They were measured on different days under shared-host load, so these are
observed differences, not controlled causal estimates of code improvements.

Read medians: **27 lower, 3 unchanged, 2 higher**.
Scenario medians: **19 lower, 2 unchanged, 13 higher**.
The full distributions matter: a mixed hit/miss workload can show a much larger
median change than mean change. Small differences also reflect timer granularity.

Selected comparisons below use **microseconds**.

| Workload | 1.1.0 median | 1.3.0 median | Median change | Mean change |
| --- | --- | --- | --- | --- |
| stats | 348.708 | 331.875 | -4.8% | -10.7% |
| triangle | 1,570.958 | 1,583.291 | +0.8% | -0.7% |
| slot_booking_overlap | 25.791 | 4.084 | -84.2% | -21.0% |
| closure_fanout | 7.042 | 0.583 | -91.7% | -37.9% |
| disp_probe_d24 | 186,404.166 | 83,147.291 | -55.4% | -54.6% |
| disp_stream_d24 | 168.917 | 171.667 | +1.6% | +1.2% |
| j4_five_way | 1,650.958 | 684.584 | -58.5% | -29.5% |
| g4_mutual | 4,590.334 | 2,539.250 | -44.7% | -46.0% |
| t2_overlap_join | 39,949.916 | 46,119.708 | +15.4% | +38.2% |
| o5_store_extremes | 490.250 | 524.875 | +7.1% | +1.8% |
| 100,000-row native result | 5,824.500 | 4,805.750 | -17.5% | -12.3% |
| Tiny tenant open/query/close | 342.333 | 405.959 | +18.6% | +85.0% |

The slower temporal-overlap scenario and tenant-lifecycle results remain
visible alongside lower query medians. A further matched comparison would be
needed to attribute either direction to a particular implementation change.

## Native application lifecycle

Protocol 2 measures actual native preparation, execution, and paged delivery.
It excludes TypeScript serialization, Effect overhead, networking, and hosted
activation. Cold-open includes open/prepare/execute/close in one process;
it does not flush the OS page cache. The post-write query observes an actual
related delete/insert. Tiny-tenant work opens an existing database, prepares a
query, reads five account rows, and closes its resources; creation is excluded.

All figures below are **microseconds**.

| Workload | Median | Mean | p99 |
| --- | --- | --- | --- |
| Warm prepared account projection | 1.709 | 1.733 | 2.208 |
| First query after related delete/insert | 3,402.042 | 3,940.419 | 13,114.333 |
| Open, prepare, query, and close | 361.375 | 386.570 | 587.750 |
| Construct and deliver 100,000 rows | 4,805.750 | 5,199.908 | 6,531.542 |
| Tiny tenant open/query/close | 405.959 | 723.771 | 4,754.958 |

The large-result median is **4.806 ms** end to end. Separately
recorded execution and delivery medians are **1.572 ms** and
**3.225 ms**; component medians need not sum to the total.
The lane delivered 1,600,000 rows across 16 draws. Paging a completed
result is not streaming query execution. The 64 tiny-tenant activations across
eight tenants recorded **0 net file-descriptor growth**.

## Read families

Each sample times one call; every family has 256 samples.
Times are **microseconds**. Clock flags identify recorded frequency contamination;
absence of a flag does not establish an isolated measurement.

| Family | Median | Mean | p99 | SQLite median | Clock flag |
| --- | --- | --- | --- | --- | --- |
| point | 0.458 | 0.452 | 0.500 | 1.500 | none recorded |
| containment_walk | 3.334 | 457.039 | 13,690.792 | 61.458 | BumbleDB |
| chain | 206.625 | 217.468 | 365.583 | 2,039.292 | none recorded |
| range | 4.958 | 4.973 | 5.250 | 141.875 | none recorded |
| balance | 1.333 | 9.554 | 36.000 | 292.416 | none recorded |
| stats | 331.875 | 333.913 | 355.208 | 82,030.833 | none recorded |
| string | 1.042 | 0.908 | 1.125 | 59.625 | none recorded |
| skew | 1,060.459 | 1,171.422 | 1,514.584 | 8,046.875 | none recorded |
| spread | 12,482.584 | 13,111.678 | 25,547.458 | 132,487.584 | none recorded |
| triangle | 1,583.291 | 1,578.175 | 1,906.167 | 36,846.666 | SQLite |
| entries_for_account_set | 2.833 | 150.966 | 616.750 | 13.250 | none recorded |
| postings_without_tag | 1.916 | 152.896 | 628.458 | 43.500 | none recorded |
| latest_posting_per_account | 114.917 | 115.377 | 127.125 | 20,846.458 | none recorded |
| mandate_at_instant | 0.500 | 0.484 | 0.542 | 8.166 | none recorded |
| mandate_overlap | 11.708 | 10.694 | 13.042 | 421.542 | none recorded |
| deep_chain | 430.375 | 396.220 | 727.833 | 3,370.250 | none recorded |
| busy_scan | 3.792 | 3.049 | 4.375 | 3,438.542 | none recorded |
| meets_chain | 2.125 | 19.229 | 75.042 | 20.000 | none recorded |
| rsvp_union | 854.375 | 858.627 | 890.917 | 17,955.458 | none recorded |
| conflict_pairs | 16.625 | 28.534 | 73.250 | 2,670.416 | none recorded |
| conflict_free | 0.792 | 0.741 | 0.959 | 15.917 | none recorded |
| free_busy | 2.541 | 10.756 | 36.958 | 298.500 | none recorded |
| slot_scan | 13.334 | 10.287 | 14.208 | 2,794.333 | none recorded |
| slot_booking_overlap | 4.084 | 15.735 | 35.916 | 649.667 | none recorded |
| closure_depth | 6.000 | 409.760 | 1,149.167 | 17.875 | none recorded |
| closure_fanout | 0.583 | 41.735 | 149.000 | 8.250 | none recorded |
| disp_probe | 93,315.208 | 107,101.239 | 143,125.208 | 815,316.625 | none recorded |
| disp_probe_d24 | 83,147.291 | 91,062.951 | 117,636.417 | 727,414.334 | none recorded |
| disp_probe_d96 | 87,321.125 | 88,687.045 | 98,406.708 | 732,792.458 | BumbleDB |
| disp_stream | 166.791 | 167.746 | 178.833 | 39,205.458 | none recorded |
| disp_stream_d24 | 171.667 | 171.934 | 176.834 | 39,366.334 | none recorded |
| disp_stream_d96 | 176.666 | 176.683 | 178.250 | 39,591.750 | none recorded |

![Read latency against indexed SQLite; lower is better](../../assets/bench-vs-sqlite.svg)

![Read median ratios against SQLite, not historical BumbleDB](../../assets/bench-speedup.svg)

![Read medians and tails for both engines](../../assets/bench-tails.svg)

![Read p50, p90, and p99 distributions](../../assets/tails-fan.svg)

![Sorted read and scenario ratios; capped comparisons are excluded](../../assets/ratio-waterfall.svg)

## Scenario queries

All six worlds use 8 warmups and 64 samples per query. Times are
**microseconds**. A zero answer count for one draw is not a zero-work roster.
Scenario reports do not contain per-cell clock diagnostics. The main table
shows the canonical `sqlite` comparator; tuned and hand-written lanes follow.

| Query | Median | Mean | p99 | SQLite median/outcome |
| --- | --- | --- | --- | --- |
| j1_filmography | 0.500 | 2.960 | 10.750 | 6.125 |
| j2_costars | 1.166 | 6.798 | 24.583 | 13.042 |
| j3_keyword_kind | 1.750 | 6.800 | 49.042 | 14.875 |
| j4_five_way | 684.584 | 2,158.419 | 6,221.542 | 5,237.375 |
| j5_country_rollup | 4,077.208 | 4,107.520 | 4,727.417 | 32,876.500 |
| j6_keyword_neighborhood | 39.417 | 226.298 | 1,052.792 | 1,584.083 |
| g1_neighbors | 0.334 | 0.648 | 1.666 | 3.041 |
| g2_two_hop | 0.583 | 88.650 | 367.583 | 11.250 |
| g3_three_hop_count | 1.500 | 734.511 | 4,685.542 | 30.000 |
| g4_mutual | 2,539.250 | 2,547.954 | 2,964.542 | 27,739.041 |
| g5_triangles_from | 0.791 | 7.355 | 30.958 | 17.958 |
| g6_weighted_hop | 0.583 | 2.728 | 11.083 | 7.833 |
| o1_revenue_by_region | 503.542 | 504.908 | 528.166 | 263,329.208 |
| o2_category_window | 386.375 | 330.471 | 492.875 | 24,583.500 |
| o3_promo_split | 338.042 | 339.148 | 391.667 | 116,738.000 |
| o4_segment_category | 24,161.875 | 24,323.324 | 33,906.875 | 396,402.250 |
| o5_store_extremes | 524.875 | 524.510 | 537.083 | 191,369.875 |
| o6_brand_drill | 1.917 | 1.722 | 2.625 | 604.292 |
| p1_by_id | 0.666 | 0.602 | 0.792 | 1.125 |
| p2_by_key | 1.250 | 1.159 | 1.333 | 1.417 |
| p3_bucket_fetch | 4.292 | 6.358 | 18.792 | 220.042 |
| p4_size_band | 0.417 | 1.515 | 4.959 | 116.458 |
| p5_keyed_get | 1.250 | 1.129 | 1.459 | 1.500 |
| r1_wash_ring | 7,754.042 | 6,025.449 | 9,242.083 | 117,817.583 |
| r2_temporal_ring | 26,827.334 | 21,995.202 | 65,308.625 | 167,532.167 |
| r3_bomb_t1 | 2,697.250 | 2,677.315 | 2,786.709 | 32,950.625 |
| r4_bomb_t2 | 1,258,392.458 | 1,293,568.535 | 1,821,050.875 | cap > 1,000 ms |
| r5_reciprocal | 424.125 | 339.669 | 615.750 | 3,413.542 |
| r6_two_path_count | 10,474.584 | 11,512.427 | 45,898.333 | cap > 1,000 ms |
| t1_stab | 0.542 | 5.255 | 13.792 | 65.334 |
| t2_overlap_join | 46,119.708 | 55,134.520 | 195,186.583 | cap > 1,000 ms |
| t3_mixed_mask | 13.459 | 1,022.545 | 4,128.083 | 1,198.625 |
| t4_ray_stab | 19.083 | 14.758 | 24.833 | 4,384.959 |
| t5_pack_key | 2.250 | 21.361 | 86.959 | not reported |

| Query | Additional SQLite lane | Median/outcome | Mean | p99 |
| --- | --- | --- | --- | --- |
| r2_temporal_ring | sqlite-tuned | 116,299.833 | 86,404.467 | 125,104.084 |
| t2_overlap_join | sqlite-tuned | 510,407.917 | 512,603.546 | 613,806.833 |
| t5_pack_key | sqlite-hand | 111.958 | 940.054 | 3,660.125 |

![All scenario queries with SQLite cap annotations](../../assets/bench-scenarios.svg)

### Multiway joins and country rollups

![Multiway joins and country rollups](../../assets/world-joins.svg)

### Graph traversal, reciprocal edges, and triangle counting

![Graph traversal, reciprocal edges, and triangle counting](../../assets/world-graph.svg)

### Analytic sums, grouped extrema, and dimension joins

![Analytic sums, grouped extrema, and dimension joins](../../assets/world-olap.svg)

### Point, key, and bucket access

![Point, key, and bucket access](../../assets/world-points.svg)

### Cyclic and temporal ring workloads

![Cyclic and temporal ring workloads](../../assets/world-rings.svg)

### Interval stabs, overlap joins, and temporal masks

![Interval stabs, overlap joins, and temporal masks](../../assets/world-temporal.svg)

![Scenarios whose canonical SQLite comparator exceeded the cap](../../assets/adversarial-dnf.svg)

## Durable writes and first-read costs

Both engine sides retain their documented durable commit contract. On macOS,
LMDB issues media flushes and SQLite uses WAL with `synchronous=FULL` and
`fullfsync=ON`. There is no BumbleDB no-sync lane. Times below are
**milliseconds**, and rows/s come directly from the reported measured means.

| Operation | Rows/batch | BumbleDB median | SQLite median | BumbleDB rows/s | SQLite rows/s | Clock flag |
| --- | --- | --- | --- | --- | --- | --- |
| commit_b1 | 1 | 4.973 | 4.950 | 200.94 | 202.51 | flagged |
| commit_b10 | 10 | 6.168 | 5.112 | 1,610.88 | 1,873.30 | flagged |
| commit_b100 | 100 | 13.170 | 8.623 | 7,383.51 | 11,316.56 | flagged |
| commit_b1000 | 1,000 | 28.214 | 16.993 | 35,968.16 | 58,353.88 | flagged |
| delete_b1 | 1 | 4.116 | 4.024 | 242.66 | 247.89 | flagged |
| delete_b10 | 10 | 4.766 | 5.134 | 2,120.08 | 1,871.56 | flagged |
| delete_b100 | 100 | 12.090 | 7.913 | 8,068.64 | 12,705.40 | none recorded |
| delete_b1000 | 1,000 | 36.302 | 21.097 | 27,453.46 | 47,823.90 | none recorded |
| insert_stream | 200,000 | 655.228 | 783.708 | 258,876.94 | 177,147.83 | none recorded |

The main read runner also measures durable writes and first reads after a
mutation. These are separate workloads from the batch ladder above;
figures remain **milliseconds**. Writes occur outside the first-read timer.

| Operation | BumbleDB median | BumbleDB mean | BumbleDB p99 | SQLite median | Clock flag |
| --- | --- | --- | --- | --- | --- |
| commit_single | 4.623 | 4.592 | 5.289 | 4.101 | SQLite |
| commit_batch | 21.041 | 22.104 | 30.241 | 12.030 | BumbleDB |
| cold_containment_walk | 0.018 | 0.745 | 9.257 | 0.179 | none recorded |
| cold_containment_walk_delete | 0.082 | 2.096 | 8.913 | 0.088 | BumbleDB, SQLite |
| commit_witnessed | 4.252 | 4.484 | 6.966 | not reported | none recorded |
| commit_window_baseline | 4.532 | 4.528 | 5.280 | not reported | BumbleDB |
| commit_window_admission | 4.142 | 4.219 | 7.196 | not reported | BumbleDB |
| commit_window_exclusion | 4.145 | 4.128 | 4.782 | not reported | none recorded |
| commit_capacity_baseline | 4.405 | 4.732 | 12.140 | not reported | BumbleDB |
| commit_capacity_sum | 4.220 | 4.374 | 5.501 | not reported | BumbleDB |
| commit_capacity_duration | 4.161 | 4.109 | 4.820 | not reported | none recorded |
| insert_stream | 566.698 | 570.996 | 608.179 | 766.367 | none recorded |

`cold_containment_walk` inserts an unrelated Org and does not invalidate
queried relation images. `cold_containment_walk_delete` swaps a Posting,
so the query observes a changed relation. Neither is a cold OS-cache test.

![Durable writes and first-read costs from the main runner](../../assets/bench-writes.svg)

![Commit, delete, and insertion rates by batch size](../../assets/bench-writes-rates.svg)

![Measured write-throughput comparisons](../../assets/write-throughput.svg)

## Application CRUD and declared-law admission

These tables use **microseconds** and include means and tails, since durable
commits and refusal paths have different costs. Both post-state checks reported
`ok`. Refused-law samples include validation and abort, not successful commits.
BumbleDB returns complete violation evidence; SQLite uses its constraint/trigger
refusal and rollback. The faster SQLite cases are retained.

### Application reads and writes

| Operation | Median | Mean | p99 | SQLite median | Clock flag |
| --- | --- | --- | --- | --- | --- |
| crud_read_point | 0.875 | 0.785 | 0.958 | 1.500 | flagged |
| crud_insert | 4,237.667 | 4,376.544 | 5,281.750 | 4,336.042 | flagged |
| crud_insert_10 | 5,006.542 | 4,766.501 | 5,359.625 | 4,214.667 | flagged |
| crud_insert_100 | 5,215.250 | 5,155.377 | 6,256.458 | 4,431.750 | flagged |
| crud_insert_1k | 8,276.584 | 8,802.989 | 11,092.625 | 6,087.791 | flagged |
| crud_update | 5,046.042 | 4,842.612 | 6,897.208 | 4,186.167 | flagged |
| crud_update_hot | 4,329.084 | 4,483.683 | 5,438.541 | 4,181.084 | flagged |
| crud_upsert | 4,172.750 | 4,160.645 | 5,269.416 | 4,139.458 | flagged |
| crud_rmw | 4,170.084 | 4,312.041 | 8,505.791 | 4,175.625 | flagged |
| crud_delete | 4,236.000 | 5,213.640 | 19,545.542 | 4,279.958 | flagged |
| crud_mixed_90_10 | 4,348.167 | 4,562.707 | 8,628.834 | 4,195.750 | flagged |

![Application reads and writes](../../assets/world-crud.svg)

### Accepted and refused transactions

| Operation | Median | Mean | p99 | SQLite median | Clock flag |
| --- | --- | --- | --- | --- | --- |
| law_commit_attempt | 4,341.916 | 4,517.164 | 5,390.792 | 5,166.750 | flagged |
| law_commit_cluster | 5,142.917 | 4,983.781 | 5,439.000 | 4,478.916 | flagged |
| law_reject_key | 29.000 | 29.331 | 32.292 | 7.917 | flagged |
| law_reject_containment | 24.792 | 25.350 | 38.875 | 14.125 | flagged |
| law_reject_window | 15.875 | 16.307 | 22.417 | 3.250 | none recorded |
| law_reject_scope | 24.083 | 24.441 | 30.250 | 3.000 | flagged |

![Accepted and refused transactions](../../assets/world-lawful.svg)

## Storage and scaling

The storage lane compares compacted LMDB file lengths with checkpointed,
indexed SQLite at matching fact counts. These are file sizes, not RSS,
physical page residency, or virtual map reservations. Table-only SQLite
uses a different indexing contract. Raw LMDB files also contain free-page history.

| Scale / world | Facts | Raw bytes | Compacted bytes | Indexed SQLite bytes | Compacted ratio |
| --- | --- | --- | --- | --- | --- |
| S / ledger | 253,264 | 44,990,464 | 33,259,520 | 18,432,000 | 1.804× |
| S / calendar | 192,369 | 59,031,552 | 31,129,600 | 17,686,528 | 1.760× |
| M / ledger | 2,526,889 | 477,773,824 | 330,842,112 | 192,147,456 | 1.722× |
| M / calendar | 1,830,369 | 557,449,216 | 294,043,648 | 176,193,536 | 1.669× |

Compacted sizes match the 1.1.0 measurements for these ledger/calendar fixtures.

![Compacted storage against indexed and table-only SQLite](../../assets/bench-storage.svg)

The scale panel covers four families at S/M/L with 64 samples.
Its roster is narrower than the 32-family read panel. Times below are
**microseconds**. SQLite's cap is 30,000 ms per call. A gate cap before timing
leaves no timing for either engine; a cap during SQLite timing preserves the
completed BumbleDB sample. Every excluded point remains explicit.

| Family | Scale | Facts | BumbleDB median | SQLite median/outcome | Clock flag |
| --- | --- | --- | --- | --- | --- |
| triangle | S | 253,264 | 1,717.250 | 42,620.416 | none recorded |
| triangle | M | 2,526,889 | 21,745.625 | cap > 30,000 ms (timing) | flagged |
| triangle | L | 25,263,139 | 246,528.625 | cap > 30,000 ms (timing) | none recorded |
| point | S | 253,264 | 0.458 | 1.458 | none recorded |
| point | M | 2,526,889 | 0.500 | 1.459 | flagged |
| point | L | 25,263,139 | 0.541 | 1.542 | flagged |
| busy_scan | S | 192,369 | 4.292 | 3,681.542 | flagged |
| busy_scan | M | 1,830,369 | 35.541 | 34,466.666 | none recorded |
| busy_scan | L | 18,210,369 | 539.000 | 338,557.333 | none recorded |
| closure_fanout | S | 17,554 | 0.917 | 52.541 | none recorded |
| closure_fanout | M | 156,818 | 4.291 | 27.833 | flagged |
| closure_fanout | L | 1,418,386 | 1.334 | 47.334 | none recorded |

The hand-tuned SQLite `busy_scan` comparator is also measured at each scale,
with its own line in the chart. Values are **microseconds**.

| Family | Scale | Hand-tuned SQLite median | Mean | p99 |
| --- | --- | --- | --- | --- |
| busy_scan | S | 1,324.208 | 1,032.975 | 1,800.375 |
| busy_scan | M | 13,415.042 | 10,313.016 | 15,154.709 |
| busy_scan | L | 127,848.458 | 96,668.901 | 143,568.375 |

The sampled medians can be nonmonotonic across scales; these curves do not
establish asymptotic bounds.

![S/M/L scaling across the four registered families](../../assets/bench-curves.svg)

### Reopen-cold, warm, and memoized execution

The warmth panel uses the first requested scale, S. Reopen-cold makes a new
handle and preparation in the same process with an unflushed OS page cache;
open and prepare are outside its first-execution timer. The warm and memoized
labels describe the runner's execution modes, not proof that every query takes
a result-cache hit. All values are **microseconds**.

| Family | BumbleDB cold | Warm | Memoized | SQLite cold | Warm | Memoized | Clock flag |
| --- | --- | --- | --- | --- | --- | --- | --- |
| triangle | 12,162.791 | 1,670.458 | 1,680.750 | 49,399.291 | 47,360.000 | 39,832.875 | none recorded |
| point | 2.583 | 0.500 | 0.458 | 10.167 | 1.875 | 1.458 | none recorded |
| busy_scan | 1,424.375 | 4.250 | 4.000 | 3,739.125 | 3,445.291 | 3,507.458 | none recorded |
| closure_fanout | 14.958 | 0.958 | 7.333 | 22.917 | 12.833 | 12.459 | none recorded |

![Reopen-cold, warm, and memoized execution at scale S](../../assets/bench-warmth.svg)

Large populated corpora here do not establish larger-than-memory performance.

## Heap workloads

The heap lane compares a frozen in-memory instance with LMDB access, with
32 samples per point/access workload. It is not an alternative durability
contract. Times below are **microseconds**.

| Operation | Heap median | Heap mean | Heap p99 | LMDB median | LMDB mean | LMDB p99 |
| --- | --- | --- | --- | --- | --- | --- |
| get | 0.167 | 0.169 | 0.209 | 0.667 | 0.667 | 0.792 |
| contains | 0.250 | 0.242 | 0.333 | 0.584 | 0.589 | 0.709 |
| scan | 17.875 | 17.970 | 19.875 | 31.625 | 31.214 | 31.875 |

| Admission facts | Wall time (ms) | ns/fact |
| --- | --- | --- |
| 693 | 0.329 | 474.75 |
| 2,633 | 1.326 | 503.78 |
| 10,392 | 5.709 | 549.32 |
| 41,432 | 22.967 | 554.34 |

Publication took **549.292 ms**; the 500-row join took
**35.041 µs**. These are individual wall measurements, not
percentile distributions.

## Hash probes

Fresh-state hashing covers 150 candidate/input combinations, including
unaligned buffers. One-shot/streaming equivalence passed. This is not a
change to the database's chosen hash or a known-answer-vector pass.
Reused-state timing and known-answer vectors were not run.
The [raw hash report](runs/1.3.0/hash-probe/hash-probe.json) contains every
input length, alignment, mean, and tail.

Selected aligned inputs below are **median nanoseconds**, including state
construction and finalization into a new output vector.

| Candidate | 32 B | 1 KiB | 8 MiB |
| --- | --- | --- | --- |
| blake3-full-32 | 87 | 1,168 | 4,455,167 |
| blake3-trunc-16 | 89 | 1,168 | 4,498,416 |
| blake3-derive-key-16 | 162 | 1,252 | 4,565,208 |
| blake3-row-prefix-rel0-16 | 127 | 1,371 | 4,531,167 |
| aegis-128l-mac-16 | 62 | 113 | 482,500 |

## Structural query measurements

The separate [interval and exact-arithmetic measurements](structural-algebra-20260911.md)
retain their own earlier source inventory and SDK timing boundary. They are
not part of the native full-suite figures above.

## Synthetic renderer examples

These two images are **renderer test fixtures**, not measurements or captured
profiles. They remain separate from the 21 regenerated benchmark charts.

![Synthetic miniature flamegraph fixture](../../scripts/flame-fixtures/mini.svg)

![Synthetic differential flamegraph fixture](../../scripts/flame-fixtures/diff.svg)

See the [measurement guide](measurement-plan.md) for commands, workload
boundaries, and profiling procedures. The [raw report directory](runs/1.3.0/README.md)
contains the inputs needed to regenerate these measured charts.
