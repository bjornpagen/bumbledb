# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 1 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## graph (geomean ratio 0.05 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| g1_neighbors | sqlite | 842 | 5.4 | 30.8 | 0.18 | single hop: hub ~1.5k edges, normal ~4 |
| g2_two_hop | sqlite | 24786 | 361.5 | 7214.8 | 0.05 | two hops, deduplicated destination set |
| g3_three_hop_count | sqlite | 1 | 2804.2 | 99837.5 | 0.03 | three-hop reach folded to Count |
| g4_mutual | sqlite | 11 | 3039.0 | 25605.7 | 0.12 | reciprocal-edge 2-cycle over the full graph |
| g5_triangles_from | sqlite | 1 | 39.6 | 5804.4 | 0.01 | 3-cycle through a start node, counted |
| g6_weighted_hop | sqlite | 193 | 14.1 | 310.4 | 0.05 | hop + weight range + target-score range |

Overall geomean ratio across 6 queries: **0.05**.

## Allocations (per query, --alloc)

| query | allocs | alloc bytes | deallocs | dealloc bytes |
|---|---:|---:|---:|---:|
| g1_neighbors | 2 | 32 | 1 | 24 |
| g2_two_hop | 2 | 32 | 1 | 24 |
| g3_three_hop_count | 2 | 32 | 1 | 24 |
| g4_mutual | 2 | 32 | 1 | 24 |
| g5_triangles_from | 2 | 32 | 1 | 24 |
| g6_weighted_hop | 2 | 80 | 1 | 72 |
