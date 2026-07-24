# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## graph (geomean ratio 0.06 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| g1_neighbors | sqlite | 212 | 0.3 | 2.9 | 0.11 | single hop: hub ~1.5k edges, normal ~4 |
| g2_two_hop | sqlite | 6199 | 0.2 | 10.7 | 0.02 | two hops, deduplicated destination set |
| g3_three_hop_count | sqlite | 0 | 1.3 | 30.0 | 0.04 | three-hop reach folded to Count |
| g4_mutual | sqlite | 15 | 3176.8 | 26806.5 | 0.12 | reciprocal-edge 2-cycle over the full graph |
| g5_triangles_from | sqlite | 0 | 0.7 | 14.2 | 0.05 | 3-cycle through a start node, counted |
| g6_weighted_hop | sqlite | 49 | 0.4 | 8.5 | 0.04 | hop + weight range + target-score range |

Overall geomean ratio across 6 queries: **0.06**.
