# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 1 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## rings (geomean ratio 0.16 over 5 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| r1_wash_ring | sqlite | 1 | 10566.9 | 108682.8 | 0.10 | the equality 3-ring (wash-trade) over power-law hubs — the binary-join exponent, capped |
| r2_temporal_ring | sqlite | 1 | 33525.8 | 155481.0 | 0.22 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r2_temporal_ring | sqlite-tuned | 1 | 33525.8 | 107493.5 | 0.31 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r3_bomb_t1 | sqlite | 1 | 3674.6 | 30906.9 | 0.12 | bipartite-bomb tier 1 (m=48): K_{m,m} + one planted triangle — answer 3 by construction; sized to finish within the cap |
| r4_bomb_t2 | sqlite | 1 | 1736849.2 | DNF>1000ms | — | bipartite-bomb tier 2 (m=384): m^3≈5.7e7 closing probes — the exponent evidence; SQLite predictably exceeds the cap, reported exceeded-cap, excluded and counted |
| r5_reciprocal | sqlite | 1808 | 511.7 | 3194.8 | 0.16 | the reciprocal-pair 2-cycle, kind-filtered |
| r6_two_path_count | sqlite | 1 | 185856.7 | 677760.0 | 0.27 | the denominator story: the distinct 2-path count binary joins must materialize |

Overall geomean ratio across 6 queries: **0.16**; 1 lane(s) DNF > cap (excluded, counted).

## Allocations (per query, --alloc)

| query | allocs | alloc bytes | deallocs | dealloc bytes |
|---|---:|---:|---:|---:|
| r1_wash_ring | 2 | 32 | 1 | 24 |
| r2_temporal_ring | 2 | 32 | 1 | 24 |
| r3_bomb_t1 | 1 | 8 | 0 | 0 |
| r4_bomb_t2 | 1 | 8 | 0 | 0 |
| r5_reciprocal | 2 | 32 | 1 | 24 |
| r6_two_path_count | 1 | 8 | 0 | 0 |
