# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).

## joins (geomean ratio 0.10 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 32 | 0.2 | 7.0 | 0.04 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | sqlite | 317 | 0.9 | 12.1 | 0.07 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | sqlite | 53 | 3.6 | 16.5 | 0.22 | 3-way pinched by string point + year range |
| j4_five_way | sqlite | 10171 | 1481.6 | 4520.6 | 0.33 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | sqlite | 8 | 5074.0 | 29343.9 | 0.17 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | sqlite | 6807 | 30.5 | 1314.2 | 0.02 | fan-out explosion through shared keywords |

## graph (geomean ratio 0.06 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| g1_neighbors | sqlite | 212 | 0.3 | 2.9 | 0.11 | single hop: hub ~1.5k edges, normal ~4 |
| g2_two_hop | sqlite | 6199 | 0.2 | 10.7 | 0.02 | two hops, deduplicated destination set |
| g3_three_hop_count | sqlite | 0 | 1.3 | 30.0 | 0.04 | three-hop reach folded to Count |
| g4_mutual | sqlite | 15 | 3176.8 | 26806.5 | 0.12 | reciprocal-edge 2-cycle over the full graph |
| g5_triangles_from | sqlite | 0 | 0.7 | 14.2 | 0.05 | 3-cycle through a start node, counted |
| g6_weighted_hop | sqlite | 49 | 0.4 | 8.5 | 0.04 | hop + weight range + target-score range |

## olap (geomean ratio 0.01 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| o1_revenue_by_region | sqlite | 6 | 458.2 | 229139.1 | 0.00 | full-fact Sum through one dimension, 6 groups |
| o2_category_window | sqlite | 12 | 457.8 | 22007.8 | 0.02 | Sum+Count by category inside day windows |
| o3_promo_split | sqlite | 2 | 336.7 | 95263.1 | 0.00 | bool group key, full-scan fold |
| o4_segment_category | sqlite | 64 | 27849.0 | 371225.1 | 0.08 | two-dimension rollup, 64 groups, 3-way join |
| o5_store_extremes | sqlite | 200 | 704.8 | 149807.1 | 0.00 | Min+Max per store, 200 groups |
| o6_brand_drill | sqlite | 0 | 1.8 | 562.9 | 0.00 | selective brand point + day range, one Sum |

## points (geomean ratio 0.11 over 5 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| p1_by_id | sqlite | 0 | 0.3 | 1.1 | 0.31 | fresh-id point: key probe vs B-tree descent |
| p2_by_key | sqlite | 0 | 1.0 | 1.3 | 0.75 | keyed string point: dictionary + determinant index |
| p3_bucket_fetch | sqlite | 1744 | 11.7 | 213.6 | 0.05 | small fan-out through a dimension + id ceiling |
| p4_size_band | sqlite | 0 | 0.2 | 112.2 | 0.00 | secondary range folded to Count |
| p5_keyed_get | sqlite | 0 | 1.1 | 1.4 | 0.79 | keyed get (0.5.0): the point read through Doc(key) -> Doc — determinant probe, no query machinery |

## rings (geomean ratio 0.15 over 5 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| r1_wash_ring | sqlite | 0 | 10395.8 | 111719.9 | 0.09 | the equality 3-ring (wash-trade) over power-law hubs — the binary-join exponent, capped |
| r2_temporal_ring | sqlite | 0 | 32687.7 | 160205.4 | 0.20 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r2_temporal_ring | sqlite-tuned | 0 | 32687.7 | 103719.2 | 0.32 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r3_bomb_t1 | sqlite | 1 | 3722.9 | 31530.9 | 0.12 | bipartite-bomb tier 1 (m=48): K_{m,m} + one planted triangle — answer 3 by construction; sized to finish within the cap |
| r4_bomb_t2 | sqlite | 1 | 1737880.8 | DNF>1000ms | — | bipartite-bomb tier 2 (m=384): m^3≈5.7e7 closing probes — the exponent evidence; SQLite predictably exceeds the cap, reported exceeded-cap, excluded and counted |
| r5_reciprocal | sqlite | 1368 | 513.5 | 3302.2 | 0.16 | the reciprocal-pair 2-cycle, kind-filtered |
| r6_two_path_count | sqlite | 1 | 135227.6 | 664714.5 | 0.20 | the denominator story: the distinct 2-path count binary joins must materialize |

## temporal (geomean ratio 0.04 over 4 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| t1_stab | sqlite | 796 | 1.1 | 5.4 | 0.21 | interval stabbing: point-in-span membership probe |
| t2_overlap_join | sqlite | 1 | 56831.6 | DNF>1000ms | — | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t2_overlap_join | sqlite-tuned | 1 | 56831.6 | 487174.6 | 0.12 | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t3_mixed_mask | sqlite | 26543 | 42.0 | 1108.8 | 0.04 | mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data |
| t4_ray_stab | sqlite | 2247 | 42.2 | 4221.2 | 0.01 | open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter |
| t5_pack_key | sqlite-hand | 7 | 2.2 | 94.7 | 0.02 | Pack/coalesce: Snodgrass coalescing per key — SQLite's lane is the hand-written islands SQL (the free_busy precedent) |

Overall geomean ratio across 34 queries: **0.05**; 2 lane(s) DNF > cap (excluded, counted).
