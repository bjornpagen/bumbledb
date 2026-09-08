# Bumbledb 1.1.0 benchmark results

The complete local suite finished on **2026-09-08T17:51:59.690205+00:00**:
**13 reported lanes, 32 read families, and 34 scenario queries**.
The pre-timing oracle passed **2,879 cases**. This page includes
every current benchmark chart, the full read/scenario tables, and the slower
results as well as the improvements.

## Measurement identity

- Source: `548193d46f645ff4a4f007517733deb4bd389569`, crate version **1.1.0**.
- Run: `release-1.1.0-final-20260908.Ivqk2w/full`; fresh isolated corpus, seed 1.
- Frozen executable SHA-256: `84b920460fe27b49181a8e8a444e6557971f559e96972fb5e5780f305d47cc74`.
- Host: Apple M2 Max, 96 GiB RAM, macOS 15.7.7, AC power, 16 KiB pages.
- Toolchain: `nightly-2026-08-15`, ordinary optimized release build.
- Command: `scripts/bench-night.sh <fresh-output> --full --shared --jobs 1 --allow-macos-qos`, with
  `BUMBLEDB_BENCH_BIN` selecting the frozen executable and
  `BUMBLEDB_BENCH_DATA` selecting the isolated corpus.

These are **shared-machine measurements with one lane running at a time**.
Each measured process set and read back user-interactive QoS, which steers
macOS toward performance cores but provides **no hard P-core-only affinity
guarantee**. No other repository benchmarks or builds ran alongside the suite.
Ambient operating-system and application activity can still affect timings.

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
admission; writes; S/M/L scale and warmth curves; and heap work. This is the
complete runner, not its compact default subset.

The main read comparison reports **`all_win=true` against SQLite**, while
the informational latency-budget aggregate remains **`budget_ok=false`**.
Neither is a verdict that every workload beats SQLite or every historical
Bumbledb revision. Mutation and admission results below include SQLite wins.
Scenario and scaling timeouts remain caps, never invented latency values.

The manifest's external prerequisites remain `NOTRUN-PREREQ`. Real-S3/IAM,
Graviton performance, Linux Node application timing, and a populated store
larger than available memory were **not run**. Correctness and packaging CI are
separate from timing; a benchmark manifest
does not substitute for them. The current cross-platform CI link is in the
[release notes](../release-1.1.md). Raspberry Pi memory/performance remains
unqualified; this is not a 512 MB hardware test. Hash one-shot/streaming
equivalence passed, but known-answer vectors were not supplied: hash KATs
and reused-state hash timing remain **not run**.

## Compared with the last published results

This is a historical comparison with the single run published with 1.0.1,
on the same M2 Max and pinned toolchain. **Both runs measured lanes serially**,
using the same workload settings and scheduler boost. This removes overlap
between unrelated benchmark workloads; it does not control all background
activity across different days. Treat differences as observations from these
runs, not universal or precisely attributable code speedups.
Read-family medians: **11 lower, 1 unchanged, 20 higher**.
Scenario medians: **15 lower, 5 unchanged, 14 higher**.
Tiny differences can reflect timer granularity or ambient load.

| Selected workload | 1.0.1 median | 1.1.0 median | Median change | Mean change |
| --- | --- | --- | --- | --- |
| Aggregate statistics | 345.792 µs | 348.708 µs | +0.8% | +3.3% |
| Triangle join | 1509.042 µs | 1570.958 µs | +4.1% | +4.1% |
| Slot-booking overlap | 6.042 µs | 25.791 µs | +326.9% | +18.8% |
| Chain join | 247.208 µs | 264.958 µs | +7.2% | +17.4% |
| Closure depth | 13.834 µs | 23.334 µs | +68.7% | +71.4% |
| Store min/max aggregation | 0.522 ms | 0.490 ms | -6.1% | -10.4% |
| Reciprocal-ring query | 0.482 ms | 0.429 ms | -11.0% | -44.5% |
| Temporal overlap join | 64.865 ms | 39.950 ms | -38.4% | -40.9% |
| Construct and deliver 100,000 rows | 63.864 ms | 5.824 ms | -90.9% | -90.8% |
| Tiny tenant open/query/close | 0.593 ms | 0.342 ms | -42.2% | -50.0% |

Means and tails accompany medians because the draw rosters mix hits, misses,
and different amounts of result work. Clock flags are retained, not used to
dismiss inconvenient results. Establishing a causal code improvement requires
a separate comparison under matched scheduling and load.

## Native application lifecycle

Protocol 2 measures actual native query/result work, excluding TypeScript
serialization, Effect overhead, network delivery, and hosted activation.
Tiny-tenant timing includes opening an existing database, preparing a query,
reading its five account rows, and closing the result, plan and database.
It excludes tenant creation and network requests. Cold-open is not a flushed
operating-system page cache.
All figures in this table are **microseconds**.

| Workload | Median | Mean | p99 |
| --- | --- | --- | --- |
| Warm prepared account projection | 1.791 | 1.795 | 2.250 |
| First query after related delete/insert | 3419.875 | 3493.740 | 4368.083 |
| Open, prepare and query | 333.542 | 357.570 | 533.750 |
| Construct and deliver 100,000 rows | 5824.500 | 5931.213 | 7278.458 |
| Tiny tenant open/query/close | 342.333 | 391.169 | 941.792 |

The large-result median is **5.824 ms** end to end. Its separately
recorded execution and delivery medians are **1.775 ms** and
**3.969 ms**; component medians need not sum to the total.
This covers 1,600,000 delivered rows across 16 draws.
Paging completed results is not streaming query execution.

## Read families

Each sample times one call (256 samples per family). Tables use **microseconds**.
Clock flags describe recorded contamination, not proof of isolation when absent.

| Family | Median | Mean | p99 | SQLite median | Clock flag |
| --- | --- | --- | --- | --- | --- |
| point | 0.458 | 0.448 | 0.500 | 1.500 | none recorded |
| containment_walk | 3.834 | 224.710 | 2226.458 | 92.042 | none recorded |
| chain | 264.958 | 263.902 | 756.125 | 2714.375 | none recorded |
| range | 5.167 | 5.864 | 28.417 | 152.125 | SQLite |
| balance | 1.417 | 10.026 | 37.792 | 404.542 | none recorded |
| stats | 348.708 | 374.039 | 616.125 | 83973.083 | SQLite |
| string | 1.125 | 0.999 | 1.500 | 61.459 | none recorded |
| skew | 1165.834 | 1250.457 | 1818.625 | 7955.584 | none recorded |
| spread | 14357.958 | 16299.290 | 84840.000 | 145453.000 | Bumbledb |
| triangle | 1570.958 | 1590.073 | 1968.875 | 40085.667 | none recorded |
| entries_for_account_set | 3.334 | 161.125 | 833.667 | 14.666 | none recorded |
| postings_without_tag | 1.958 | 157.761 | 721.584 | 68.500 | none recorded |
| latest_posting_per_account | 118.042 | 123.499 | 198.667 | 20646.917 | none recorded |
| mandate_at_instant | 0.500 | 0.487 | 0.542 | 8.416 | none recorded |
| mandate_overlap | 12.292 | 12.614 | 68.500 | 434.750 | none recorded |
| deep_chain | 457.583 | 433.098 | 882.667 | 4729.667 | SQLite |
| busy_scan | 3.958 | 3.157 | 4.458 | 3615.209 | SQLite |
| meets_chain | 2.125 | 21.041 | 98.875 | 19.875 | SQLite |
| rsvp_union | 914.041 | 985.855 | 1793.708 | 19504.875 | none recorded |
| conflict_pairs | 24.459 | 29.713 | 79.959 | 9100.500 | SQLite |
| conflict_free | 0.917 | 1.102 | 2.792 | 26.583 | Bumbledb, SQLite |
| free_busy | 4.083 | 15.665 | 115.500 | 449.375 | Bumbledb, SQLite |
| slot_scan | 14.292 | 11.452 | 27.041 | 3598.958 | Bumbledb, SQLite |
| slot_booking_overlap | 25.791 | 19.914 | 98.292 | 8203.000 | Bumbledb, SQLite |
| closure_depth | 23.334 | 759.949 | 3982.833 | 122.167 | Bumbledb, SQLite |
| closure_fanout | 7.042 | 67.189 | 758.708 | 243.292 | Bumbledb, SQLite |
| disp_probe | 178306.542 | 180193.618 | 211951.084 | 1093999.542 | none recorded |
| disp_probe_d24 | 186404.166 | 200395.767 | 245365.375 | 1032432.250 | none recorded |
| disp_probe_d96 | 169126.000 | 167818.257 | 185241.125 | 912323.667 | none recorded |
| disp_stream | 166.959 | 168.385 | 179.875 | 41444.458 | Bumbledb, SQLite |
| disp_stream_d24 | 168.917 | 169.930 | 174.000 | 40598.292 | none recorded |
| disp_stream_d96 | 176.875 | 178.066 | 186.667 | 40556.291 | none recorded |

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
| j1_filmography | 0.459 | 2.793 | 9.917 | 7.042 |
| j2_costars | 1.125 | 6.547 | 23.750 | 12.583 |
| j3_keyword_kind | 1.708 | 7.312 | 77.667 | 17.917 |
| j4_five_way | 1650.958 | 3063.645 | 9834.042 | 6081.458 |
| j5_country_rollup | 4380.792 | 4383.892 | 4707.458 | 33350.542 |
| j6_keyword_neighborhood | 37.042 | 225.317 | 1177.500 | 1580.792 |
| g1_neighbors | 0.333 | 0.616 | 1.542 | 3.083 |
| g2_two_hop | 0.792 | 95.810 | 609.375 | 13.333 |
| g3_three_hop_count | 2.125 | 825.936 | 5107.500 | 78.459 |
| g4_mutual | 4590.334 | 4716.890 | 6874.250 | 33365.667 |
| g5_triangles_from | 0.792 | 7.451 | 36.917 | 28.584 |
| g6_weighted_hop | 0.500 | 2.661 | 9.416 | 9.500 |
| o1_revenue_by_region | 510.917 | 541.138 | 1167.792 | 297331.250 |
| o2_category_window | 444.291 | 366.646 | 1135.709 | 27057.292 |
| o3_promo_split | 326.334 | 335.412 | 422.125 | 116179.208 |
| o4_segment_category | 27660.667 | 28051.746 | 41563.083 | 445643.542 |
| o5_store_extremes | 490.250 | 515.419 | 937.250 | 227471.250 |
| o6_brand_drill | 2.166 | 1.892 | 3.209 | 904.792 |
| p1_by_id | 0.666 | 0.600 | 0.750 | 1.125 |
| p2_by_key | 1.250 | 1.213 | 3.750 | 1.416 |
| p3_bucket_fetch | 4.083 | 5.998 | 17.125 | 223.166 |
| p4_size_band | 0.416 | 1.484 | 4.875 | 114.875 |
| p5_keyed_get | 1.209 | 1.187 | 1.625 | 1.542 |
| r1_wash_ring | 8078.541 | 6387.087 | 10676.834 | 120404.000 |
| r2_temporal_ring | 26400.500 | 20247.580 | 34594.708 | 163773.750 |
| r3_bomb_t1 | 2792.375 | 2862.589 | 3661.375 | 35075.416 |
| r4_bomb_t2 | 1291450.792 | 1559288.667 | 6034197.709 | exceeded_cap |
| r5_reciprocal | 429.250 | 344.272 | 607.542 | 3353.583 |
| r6_two_path_count | 11064.167 | 11129.622 | 13098.333 | 685832.667 |
| t1_stab | 0.583 | 5.542 | 15.958 | 21.709 |
| t2_overlap_join | 39949.916 | 39894.769 | 43033.417 | exceeded_cap |
| t3_mixed_mask | 15.958 | 1026.653 | 4462.125 | 1210.083 |
| t4_ray_stab | 19.625 | 15.166 | 21.833 | 4477.292 |
| t5_pack_key | 2.416 | 24.774 | 133.084 | not reported |

The canonical table above shows the `sqlite` lane. Additional tuned and
hand-written SQL comparators are retained below, in **microseconds**.

| Query | Additional SQLite lane | Median/outcome | Mean | p99 |
| --- | --- | --- | --- | --- |
| r2_temporal_ring | sqlite-tuned | 119027.666 | 97697.977 | 281302.208 |
| t2_overlap_join | sqlite-tuned | 523220.250 | 529930.028 | 660401.334 |
| t5_pack_key | sqlite-hand | 122.417 | 970.005 | 4388.542 |

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
| commit_b1 | 1 | 4.657 | 4.507 | 217.85 | 218.29 |
| commit_b10 | 10 | 5.588 | 5.105 | 1,757.59 | 1,951.08 |
| commit_b100 | 100 | 11.992 | 8.004 | 8,073.68 | 12,336.79 |
| commit_b1000 | 1,000 | 29.497 | 20.106 | 32,691.06 | 49,817.82 |
| delete_b1 | 1 | 4.342 | 4.183 | 220.86 | 237.01 |
| delete_b10 | 10 | 4.989 | 4.872 | 2,023.5 | 2,078.74 |
| delete_b100 | 100 | 13.703 | 8.605 | 6,964.54 | 11,248.51 |
| delete_b1000 | 1,000 | 36.793 | 22.998 | 27,015.47 | 42,691.79 |
| insert_stream | 200,000 | 618.266 | 758.074 | 323,361.24 | 264,018.45 |

Rates and latency reflect durable storage work on this shared host.
They are not an across-the-board write-speedup claim.

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
| triangle | 1581.041 | 21578.792 | not timed (gate cap) |
| point | 0.458 | 0.500 | 0.542 |
| busy_scan | 3.834 | 35.416 | 1099.125 |
| closure_fanout | 0.500 | 3.792 | 68.125 |

An oracle cap before timing leaves neither engine with a timing for that
point. A cap during SQLite timing does not erase Bumbledb's completed sample.
The table and charts preserve these distinctions.

![S/M/L scaling for the four registered curve families](../../assets/bench-curves.svg)

![Reopen-cold, warm and memoized execution at the first requested scale](../../assets/bench-warmth.svg)

Reopen-cold creates a new handle and prepared query in the same process with
a warm OS page cache. Open and prepare are outside its timed first execution.
Large populated corpora here do not establish larger-than-memory performance.


## Heap workloads

The heap lane compares a frozen in-memory instance with LMDB access; it is
not an alternative durability promise. Point/access medians are microseconds.

| Operation | Frozen heap | LMDB |
| --- | --- | --- |
| get | 0.167 | 0.625 |
| contains | 0.208 | 0.500 |
| scan | 16.542 | 29.000 |

| Admission facts | Wall time (ms) | ns/fact |
| --- | --- | --- |
| 693 | 0.419 | 604.32 |
| 2,633 | 1.249 | 474.25 |
| 10,392 | 5.490 | 528.27 |
| 41,432 | 23.888 | 576.57 |

Publication took **624.096 ms**; the 500-row join took
**39.333 µs**. These are individual wall measurements, not percentile distributions.

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
| blake3-full-32 | 86 | 1,131 | 4,385,833 |
| blake3-trunc-16 | 85 | 1,121 | 4,403,000 |
| blake3-derive-key-16 | 162 | 1,201 | 4,427,959 |
| blake3-row-prefix-rel0-16 | 125 | 1,313 | 4,423,542 |
| aegis-128l-mac-16 | 62 | 110 | 472,458 |

## Synthetic renderer examples

These two images are **test fixtures**, not benchmarks or captured profiles.
Their invented microsecond values test the renderer's layout and color rules.

![Synthetic miniature flamegraph renderer fixture](../../scripts/flame-fixtures/mini.svg)

The miniature fixture checks nested frame widths and labels.

![Synthetic differential flamegraph renderer fixture](../../scripts/flame-fixtures/diff.svg)

The differential fixture checks red growth, blue shrinkage and unchanged frames.

All **21 measured charts and both synthetic fixtures** are embedded on this
page. See the [measurement runbook](measurement-plan.md) for reproducible
commands and workload boundaries. Compare reports using source, protocol,
draw mix and host—not screenshots alone.
