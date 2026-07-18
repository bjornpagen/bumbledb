# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines) before timing. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## joins (geomean ratio 0.12)

| query | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---:|---:|---:|---:|---|
| j1_filmography | 31 | 2.4 | 10.1 | 0.24 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | 301 | 0.9 | 15.0 | 0.06 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | 50 | 2.0 | 13.7 | 0.14 | 3-way pinched by string point + year range |
| j4_five_way | 10025 | 1693.9 | 5239.1 | 0.32 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | 8 | 5083.6 | 30250.8 | 0.17 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | 6638 | 31.9 | 1129.2 | 0.03 | fan-out explosion through shared keywords |

## graph (geomean ratio 0.08)

| query | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---:|---:|---:|---:|---|
| g1_neighbors | 207 | 0.2 | 3.1 | 0.08 | single hop: hub ~1.5k edges, normal ~4 |
| g2_two_hop | 9884 | 1.4 | 11.4 | 0.12 | two hops, deduplicated destination set |
| g3_three_hop_count | 0 | 1.9 | 32.8 | 0.06 | three-hop reach folded to Count |
| g4_mutual | 17 | 3885.3 | 29589.5 | 0.13 | reciprocal-edge 2-cycle over the full graph |
| g5_triangles_from | 0 | 1.0 | 19.9 | 0.05 | 3-cycle through a start node, counted |
| g6_weighted_hop | 50 | 0.6 | 7.8 | 0.07 | hop + weight range + target-score range |

## olap (geomean ratio 0.02)

| query | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---:|---:|---:|---:|---|
| o1_revenue_by_region | 6 | 454.5 | 231989.9 | 0.00 | full-fact Sum through one dimension, 6 groups |
| o2_category_window | 12 | 430.1 | 26675.5 | 0.02 | Sum+Count by category inside day windows |
| o3_promo_split | 2 | 7411.4 | 94349.1 | 0.08 | bool group key, full-scan fold |
| o4_segment_category | 64 | 30730.8 | 375992.3 | 0.08 | two-dimension rollup, 64 groups, 3-way join |
| o5_store_extremes | 200 | 8811.3 | 158912.2 | 0.06 | Min+Max per store, 200 groups |
| o6_brand_drill | 0 | 1.5 | 447.1 | 0.00 | selective brand point + day range, one Sum |

## points (geomean ratio 0.07)

| query | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---:|---:|---:|---:|---|
| p1_by_id | 0 | 0.5 | 1.1 | 0.48 | fresh-id point: key probe vs B-tree descent |
| p2_by_key | 0 | 0.9 | 1.4 | 0.64 | keyed string point: dictionary + determinant index |
| p3_bucket_fetch | 1744 | 10.8 | 213.6 | 0.05 | small fan-out through a dimension + id ceiling |
| p4_size_band | 0 | 0.2 | 116.1 | 0.00 | secondary range folded to Count |

Overall geomean ratio across 22 queries: **0.06**.
