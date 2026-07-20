# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## joins (geomean ratio 0.13 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 31 | 2.3 | 9.5 | 0.24 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | sqlite | 301 | 0.9 | 14.6 | 0.06 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | sqlite | 50 | 3.8 | 13.0 | 0.29 | 3-way pinched by string point + year range |
| j4_five_way | sqlite | 10025 | 1278.2 | 5245.2 | 0.24 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | sqlite | 8 | 5271.8 | 29481.3 | 0.18 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | sqlite | 6638 | 32.0 | 1088.7 | 0.03 | fan-out explosion through shared keywords |

## graph (geomean ratio 0.07 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| g1_neighbors | sqlite | 207 | 0.2 | 3.0 | 0.07 | single hop: hub ~1.5k edges, normal ~4 |
| g2_two_hop | sqlite | 9884 | 1.3 | 9.6 | 0.14 | two hops, deduplicated destination set |
| g3_three_hop_count | sqlite | 0 | 1.3 | 33.0 | 0.04 | three-hop reach folded to Count |
| g4_mutual | sqlite | 17 | 3609.5 | 27772.3 | 0.13 | reciprocal-edge 2-cycle over the full graph |
| g5_triangles_from | sqlite | 0 | 1.0 | 21.7 | 0.04 | 3-cycle through a start node, counted |
| g6_weighted_hop | sqlite | 50 | 0.6 | 7.7 | 0.08 | hop + weight range + target-score range |

## olap (geomean ratio 0.02 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| o1_revenue_by_region | sqlite | 6 | 445.9 | 232071.8 | 0.00 | full-fact Sum through one dimension, 6 groups |
| o2_category_window | sqlite | 12 | 427.0 | 21175.1 | 0.02 | Sum+Count by category inside day windows |
| o3_promo_split | sqlite | 2 | 7132.0 | 93156.0 | 0.08 | bool group key, full-scan fold |
| o4_segment_category | sqlite | 64 | 29701.8 | 379868.0 | 0.08 | two-dimension rollup, 64 groups, 3-way join |
| o5_store_extremes | sqlite | 200 | 8151.6 | 163659.6 | 0.05 | Min+Max per store, 200 groups |
| o6_brand_drill | sqlite | 0 | 1.5 | 721.0 | 0.00 | selective brand point + day range, one Sum |

## points (geomean ratio 0.13 over 5 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| p1_by_id | sqlite | 0 | 0.5 | 1.1 | 0.50 | fresh-id point: key probe vs B-tree descent |
| p2_by_key | sqlite | 0 | 0.9 | 1.4 | 0.67 | keyed string point: dictionary + determinant index |
| p3_bucket_fetch | sqlite | 1744 | 10.8 | 209.0 | 0.05 | small fan-out through a dimension + id ceiling |
| p4_size_band | sqlite | 0 | 0.2 | 111.9 | 0.00 | secondary range folded to Count |
| p5_keyed_get | sqlite | 0 | 1.4 | 1.4 | 1.00 | keyed get (0.5.0): the point read through Doc(key) -> Doc — determinant probe, no query machinery |

## rings (geomean ratio 0.25 over 5 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| r1_wash_ring | sqlite | 0 | 62523.5 | 111221.9 | 0.56 | the equality 3-ring (wash-trade) over power-law hubs — the binary-join exponent, capped |
| r2_temporal_ring | sqlite | 0 | 62857.3 | 149166.5 | 0.42 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r2_temporal_ring | sqlite-tuned | 0 | 62857.3 | 104417.5 | 0.60 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r3_bomb_t1 | sqlite | 1 | 3000.8 | 32407.1 | 0.09 | bipartite-bomb tier 1 (m=48): K_{m,m} + one planted triangle — answer 3 by construction; sized to finish within the cap |
| r4_bomb_t2 | sqlite | 1 | 1576952.3 | DNF>1000ms | — | bipartite-bomb tier 2 (m=384): m^3≈5.7e7 closing probes — the exponent evidence; SQLite predictably exceeds the cap, reported exceeded-cap, excluded and counted |
| r5_reciprocal | sqlite | 1473 | 715.2 | 3240.7 | 0.22 | the reciprocal-pair 2-cycle, kind-filtered |
| r6_two_path_count | sqlite | 1 | 135264.0 | 674157.7 | 0.20 | the denominator story: the distinct 2-path count binary joins must materialize |

## temporal (geomean ratio 0.04 over 4 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| t1_stab | sqlite | 797 | 1.1 | 11.3 | 0.10 | interval stabbing: point-in-span membership probe |
| t2_overlap_join | sqlite | 1 | 162794.7 | DNF>1000ms | — | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t2_overlap_join | sqlite-tuned | 1 | 162794.7 | 509756.0 | 0.32 | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t3_mixed_mask | sqlite | 29277 | 67.9 | 1309.7 | 0.05 | mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data |
| t4_ray_stab | sqlite | 2255 | 40.2 | 4182.0 | 0.01 | open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter |
| t5_pack_key | sqlite-hand | 1 | 3.3 | 102.0 | 0.03 | Pack/coalesce: Snodgrass coalescing per key — SQLite's lane is the hand-written islands SQL (the free_busy precedent) |

Overall geomean ratio across 34 queries: **0.08**; 2 lane(s) DNF > cap (excluded, counted).
