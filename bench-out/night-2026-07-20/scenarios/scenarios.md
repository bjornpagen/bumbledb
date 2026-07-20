# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## joins (geomean ratio 0.12 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 31 | 2.2 | 9.5 | 0.24 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | sqlite | 301 | 0.9 | 14.2 | 0.06 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | sqlite | 50 | 1.8 | 12.9 | 0.14 | 3-way pinched by string point + year range |
| j4_five_way | sqlite | 10025 | 1243.5 | 4979.8 | 0.25 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | sqlite | 8 | 4879.5 | 28456.5 | 0.17 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | sqlite | 6638 | 31.5 | 1029.8 | 0.03 | fan-out explosion through shared keywords |

## graph (geomean ratio 0.08 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| g1_neighbors | sqlite | 207 | 0.2 | 2.9 | 0.09 | single hop: hub ~1.5k edges, normal ~4 |
| g2_two_hop | sqlite | 9884 | 0.8 | 9.2 | 0.09 | two hops, deduplicated destination set |
| g3_three_hop_count | sqlite | 0 | 1.5 | 26.5 | 0.06 | three-hop reach folded to Count |
| g4_mutual | sqlite | 17 | 3465.3 | 26928.4 | 0.13 | reciprocal-edge 2-cycle over the full graph |
| g5_triangles_from | sqlite | 0 | 1.0 | 17.0 | 0.06 | 3-cycle through a start node, counted |
| g6_weighted_hop | sqlite | 50 | 0.5 | 7.2 | 0.06 | hop + weight range + target-score range |

## olap (geomean ratio 0.02 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| o1_revenue_by_region | sqlite | 6 | 427.2 | 206743.1 | 0.00 | full-fact Sum through one dimension, 6 groups |
| o2_category_window | sqlite | 12 | 403.2 | 21024.2 | 0.02 | Sum+Count by category inside day windows |
| o3_promo_split | sqlite | 2 | 6833.5 | 88652.7 | 0.08 | bool group key, full-scan fold |
| o4_segment_category | sqlite | 64 | 27182.2 | 343691.2 | 0.08 | two-dimension rollup, 64 groups, 3-way join |
| o5_store_extremes | sqlite | 200 | 7743.9 | 144853.1 | 0.05 | Min+Max per store, 200 groups |
| o6_brand_drill | sqlite | 0 | 1.4 | 417.7 | 0.00 | selective brand point + day range, one Sum |

## points (geomean ratio 0.07 over 4 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| p1_by_id | sqlite | 0 | 0.5 | 1.1 | 0.48 | fresh-id point: key probe vs B-tree descent |
| p2_by_key | sqlite | 0 | 0.9 | 1.3 | 0.66 | keyed string point: dictionary + determinant index |
| p3_bucket_fetch | sqlite | 1744 | 10.3 | 208.0 | 0.05 | small fan-out through a dimension + id ceiling |
| p4_size_band | sqlite | 0 | 0.2 | 108.8 | 0.00 | secondary range folded to Count |

## rings (geomean ratio 0.25 over 5 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| r1_wash_ring | sqlite | 0 | 57140.5 | 105851.0 | 0.54 | the equality 3-ring (wash-trade) over power-law hubs — the binary-join exponent, capped |
| r2_temporal_ring | sqlite | 0 | 60179.1 | 146100.0 | 0.41 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r2_temporal_ring | sqlite-tuned | 0 | 60179.1 | 101940.7 | 0.59 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r3_bomb_t1 | sqlite | 1 | 2835.3 | 31126.2 | 0.09 | bipartite-bomb tier 1 (m=48): K_{m,m} + one planted triangle — answer 3 by construction; sized to finish within the cap |
| r4_bomb_t2 | sqlite | 1 | 1454437.8 | DNF>1000ms | — | bipartite-bomb tier 2 (m=384): m^3≈5.7e7 closing probes — the exponent evidence; SQLite predictably exceeds the cap, reported exceeded-cap, excluded and counted |
| r5_reciprocal | sqlite | 1473 | 719.9 | 3152.9 | 0.23 | the reciprocal-pair 2-cycle, kind-filtered |
| r6_two_path_count | sqlite | 1 | 127886.8 | 658591.3 | 0.19 | the denominator story: the distinct 2-path count binary joins must materialize |

## temporal (geomean ratio 0.04 over 4 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| t1_stab | sqlite | 797 | 1.1 | 5.7 | 0.19 | interval stabbing: point-in-span membership probe |
| t2_overlap_join | sqlite | 1 | 157309.6 | DNF>1000ms | — | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t2_overlap_join | sqlite-tuned | 1 | 157309.6 | 489540.5 | 0.32 | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t3_mixed_mask | sqlite | 29277 | 54.6 | 1272.2 | 0.04 | mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data |
| t4_ray_stab | sqlite | 2255 | 38.9 | 4128.7 | 0.01 | open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter |
| t5_pack_key | sqlite-hand | 1 | 2.3 | 98.5 | 0.02 | Pack/coalesce: Snodgrass coalescing per key — SQLite's lane is the hand-written islands SQL (the free_busy precedent) |

Overall geomean ratio across 33 queries: **0.07**; 2 lane(s) DNF > cap (excluded, counted).
