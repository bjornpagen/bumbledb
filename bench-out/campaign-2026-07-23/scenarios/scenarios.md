# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## joins (geomean ratio 0.10 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 32 | 0.2 | 6.8 | 0.04 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | sqlite | 317 | 1.2 | 12.2 | 0.10 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | sqlite | 53 | 3.5 | 17.5 | 0.20 | 3-way pinched by string point + year range |
| j4_five_way | sqlite | 10171 | 1992.5 | 6131.2 | 0.32 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | sqlite | 8 | 5056.8 | 30320.6 | 0.17 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | sqlite | 6807 | 39.9 | 1449.2 | 0.03 | fan-out explosion through shared keywords |

## graph (geomean ratio 0.05 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| g1_neighbors | sqlite | 212 | 0.3 | 2.9 | 0.10 | single hop: hub ~1.5k edges, normal ~4 |
| g2_two_hop | sqlite | 6199 | 0.3 | 13.4 | 0.02 | two hops, deduplicated destination set |
| g3_three_hop_count | sqlite | 0 | 1.5 | 42.5 | 0.04 | three-hop reach folded to Count |
| g4_mutual | sqlite | 15 | 4511.3 | 31654.2 | 0.14 | reciprocal-edge 2-cycle over the full graph |
| g5_triangles_from | sqlite | 0 | 0.8 | 21.5 | 0.03 | 3-cycle through a start node, counted |
| g6_weighted_hop | sqlite | 49 | 0.3 | 6.9 | 0.05 | hop + weight range + target-score range |

## olap (geomean ratio 0.01 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| o1_revenue_by_region | sqlite | 6 | 457.8 | 233691.4 | 0.00 | full-fact Sum through one dimension, 6 groups |
| o2_category_window | sqlite | 12 | 441.2 | 23156.3 | 0.02 | Sum+Count by category inside day windows |
| o3_promo_split | sqlite | 2 | 318.2 | 92742.9 | 0.00 | bool group key, full-scan fold |
| o4_segment_category | sqlite | 64 | 29190.0 | 391935.2 | 0.07 | two-dimension rollup, 64 groups, 3-way join |
| o5_store_extremes | sqlite | 200 | 692.5 | 163317.2 | 0.00 | Min+Max per store, 200 groups |
| o6_brand_drill | sqlite | 0 | 2.0 | 489.8 | 0.00 | selective brand point + day range, one Sum |

## points (geomean ratio 0.11 over 5 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| p1_by_id | sqlite | 0 | 0.3 | 1.1 | 0.27 | fresh-id point: key probe vs B-tree descent |
| p2_by_key | sqlite | 0 | 0.9 | 1.3 | 0.69 | keyed string point: dictionary + determinant index |
| p3_bucket_fetch | sqlite | 1744 | 11.0 | 211.5 | 0.05 | small fan-out through a dimension + id ceiling |
| p4_size_band | sqlite | 0 | 0.2 | 111.9 | 0.00 | secondary range folded to Count |
| p5_keyed_get | sqlite | 0 | 1.1 | 1.4 | 0.76 | keyed get (0.5.0): the point read through Doc(key) -> Doc — determinant probe, no query machinery |

## rings (geomean ratio 0.15 over 5 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| r1_wash_ring | sqlite | 0 | 10723.4 | 109872.8 | 0.10 | the equality 3-ring (wash-trade) over power-law hubs — the binary-join exponent, capped |
| r2_temporal_ring | sqlite | 0 | 31293.0 | 156308.2 | 0.20 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r2_temporal_ring | sqlite-tuned | 0 | 31293.0 | 103970.7 | 0.30 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r3_bomb_t1 | sqlite | 1 | 3720.7 | 31441.7 | 0.12 | bipartite-bomb tier 1 (m=48): K_{m,m} + one planted triangle — answer 3 by construction; sized to finish within the cap |
| r4_bomb_t2 | sqlite | 1 | 1833040.5 | DNF>1000ms | — | bipartite-bomb tier 2 (m=384): m^3≈5.7e7 closing probes — the exponent evidence; SQLite predictably exceeds the cap, reported exceeded-cap, excluded and counted |
| r5_reciprocal | sqlite | 1368 | 515.2 | 3288.6 | 0.16 | the reciprocal-pair 2-cycle, kind-filtered |
| r6_two_path_count | sqlite | 1 | 152893.1 | 668027.4 | 0.23 | the denominator story: the distinct 2-path count binary joins must materialize |

## temporal (geomean ratio 0.03 over 4 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| t1_stab | sqlite | 796 | 6.5 | 59.8 | 0.11 | interval stabbing: point-in-span membership probe |
| t2_overlap_join | sqlite | 1 | 63993.2 | DNF>1000ms | — | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t2_overlap_join | sqlite-tuned | 1 | 63993.2 | 500222.1 | 0.13 | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t3_mixed_mask | sqlite | 26543 | 51.7 | 1182.8 | 0.04 | mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data |
| t4_ray_stab | sqlite | 2247 | 43.4 | 4346.5 | 0.01 | open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter |
| t5_pack_key | sqlite-hand | 7 | 2.2 | 99.2 | 0.02 | Pack/coalesce: Snodgrass coalescing per key — SQLite's lane is the hand-written islands SQL (the free_busy precedent) |

Overall geomean ratio across 34 queries: **0.05**; 2 lane(s) DNF > cap (excluded, counted).

## DELTA vs night-2026-07-20

Campaign rerun (RUN 2, wall power) vs the night-2026-07-20 pins. The night baselines predate the fixed-RNG corpus regeneration (R20), so absolute p50 deltas confound engine changes with corpus changes; ratio-vs-night (campaign ratio / night ratio, <1 = our standing improved) is the cross-run comparand. DNF lanes carry no ratio. Campaign-targeted queries (p1/p2/p5, r1/r2/r4/r6, t2/t3, o3/o4/o5, g4/g5) are marked ●.

Provenance: campaign RUN 2, 2026-07-24, code rev `f474202a` (the rings json stamps `1cd8e978` — the five-family pin commit landed mid-run; bench-out only, no code delta), **wall power** (`pmset -g ps`: "Now drawing from 'AC Power'", checked before and after the run); shared-machine boosted per the standing greenlight (`BUMBLEDB_BENCH_BOOST=1`, measurement mutex held for every family; load averages 2.70/3.45/3.54 at start, 7.00/5.51/4.51 late-run). Driven family-by-family (six invocations, binary prebuilt before any timing); per-family artifacts under `family-<name>/`, merged here. Corpus: the R20 fixed-RNG regeneration, loaded fresh per family (seed 1); post-load store digests (sha256):

| scenario | oracle.sqlite | db/data.mdb |
|---|---|---|
| joins | 6c3849450283bc45 | 9c3d1cc19d4cc84a |
| graph | 6cd4181d52a9d40a | 4d7a0d3fb87c9f93 |
| olap | 2fce8fba91e876b3 | 76e154cb5b84e1e0 |
| points | c72ac93ea29d0dff | d51434eebe6629cb |
| rings | ab207af87d76dde5 | e7e08089d2ab6e50 |
| temporal | 82d593ec1a348dad | 71bc540678f8d1a3 |

(full digests in `digests.txt`)

| query | lane | ours p50 (us) | night ours | sqlite p50 (us) | night sqlite | ratio | night ratio | ratio-vs-night |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| j1_filmography | sqlite | 0.2 | 2.3 | 6.8 | 9.5 | 0.04 | 0.24 | 0.15 |
| j2_costars | sqlite | 1.2 | 0.9 | 12.2 | 14.6 | 0.10 | 0.06 | 1.71 |
| j3_keyword_kind | sqlite | 3.5 | 3.8 | 17.5 | 13.0 | 0.20 | 0.29 | 0.70 |
| j4_five_way | sqlite | 1992.5 | 1278.2 | 6131.2 | 5245.2 | 0.32 | 0.24 | 1.33 |
| j5_country_rollup | sqlite | 5056.8 | 5271.8 | 30320.6 | 29481.3 | 0.17 | 0.18 | 0.93 |
| j6_keyword_neighborhood | sqlite | 39.9 | 32.0 | 1449.2 | 1088.7 | 0.03 | 0.03 | 0.94 |
| g1_neighbors | sqlite | 0.3 | 0.2 | 2.9 | 3.0 | 0.10 | 0.07 | 1.44 |
| g2_two_hop | sqlite | 0.3 | 1.3 | 13.4 | 9.6 | 0.02 | 0.14 | 0.16 |
| g3_three_hop_count | sqlite | 1.5 | 1.3 | 42.5 | 33.0 | 0.04 | 0.04 | 0.92 |
| g4_mutual ● | sqlite | 4511.3 | 3609.5 | 31654.2 | 27772.3 | 0.14 | 0.13 | 1.10 |
| g5_triangles_from ● | sqlite | 0.8 | 1.0 | 21.5 | 21.7 | 0.03 | 0.04 | 0.79 |
| g6_weighted_hop | sqlite | 0.3 | 0.6 | 6.9 | 7.7 | 0.05 | 0.08 | 0.64 |
| o1_revenue_by_region | sqlite | 457.8 | 445.9 | 233691.4 | 232071.8 | 0.00 | 0.00 | 1.02 |
| o2_category_window | sqlite | 441.2 | 427.0 | 23156.3 | 21175.1 | 0.02 | 0.02 | 0.95 |
| o3_promo_split ● | sqlite | 318.2 | 7132.0 | 92742.9 | 93156.0 | 0.00 | 0.08 | 0.04 |
| o4_segment_category ● | sqlite | 29190.0 | 29701.8 | 391935.2 | 379868.0 | 0.07 | 0.08 | 0.95 |
| o5_store_extremes ● | sqlite | 692.5 | 8151.6 | 163317.2 | 163659.6 | 0.00 | 0.05 | 0.09 |
| o6_brand_drill | sqlite | 2.0 | 1.5 | 489.8 | 721.0 | 0.00 | 0.00 | 1.87 |
| p1_by_id ● | sqlite | 0.3 | 0.5 | 1.1 | 1.1 | 0.27 | 0.50 | 0.54 |
| p2_by_key ● | sqlite | 0.9 | 0.9 | 1.3 | 1.4 | 0.69 | 0.67 | 1.03 |
| p3_bucket_fetch | sqlite | 11.0 | 10.8 | 211.5 | 209.0 | 0.05 | 0.05 | 1.01 |
| p4_size_band | sqlite | 0.2 | 0.2 | 111.9 | 111.9 | 0.00 | 0.00 | 1.20 |
| p5_keyed_get ● | sqlite | 1.1 | 1.4 | 1.4 | 1.4 | 0.76 | 1.00 | 0.77 |
| r1_wash_ring ● | sqlite | 10723.4 | 62523.5 | 109872.8 | 111221.9 | 0.10 | 0.56 | 0.17 |
| r2_temporal_ring ● | sqlite | 31293.0 | 62857.3 | 156308.2 | 149166.5 | 0.20 | 0.42 | 0.48 |
| r2_temporal_ring ● | sqlite-tuned | 31293.0 | 62857.3 | 103970.7 | 104417.5 | 0.30 | 0.60 | 0.50 |
| r3_bomb_t1 | sqlite | 3720.7 | 3000.8 | 31441.7 | 32407.1 | 0.12 | 0.09 | 1.28 |
| r4_bomb_t2 ● | sqlite | 1833040.5 | 1576952.3 | DNF>1000ms | DNF>1000ms | — | — | — |
| r5_reciprocal | sqlite | 515.2 | 715.2 | 3288.6 | 3240.7 | 0.16 | 0.22 | 0.71 |
| r6_two_path_count ● | sqlite | 152893.1 | 135264.0 | 668027.4 | 674157.7 | 0.23 | 0.20 | 1.14 |
| t1_stab | sqlite | 6.5 | 1.1 | 59.8 | 11.3 | 0.11 | 0.10 | 1.09 |
| t2_overlap_join ● | sqlite | 63993.2 | 162794.7 | DNF>1000ms | DNF>1000ms | — | — | — |
| t2_overlap_join ● | sqlite-tuned | 63993.2 | 162794.7 | 500222.1 | 509756.0 | 0.13 | 0.32 | 0.40 |
| t3_mixed_mask ● | sqlite | 51.7 | 67.9 | 1182.8 | 1309.7 | 0.04 | 0.05 | 0.84 |
| t4_ray_stab | sqlite | 43.4 | 40.2 | 4346.5 | 4182.0 | 0.01 | 0.01 | 1.04 |
| t5_pack_key | sqlite-hand | 2.2 | 3.3 | 99.2 | 102.0 | 0.02 | 0.03 | 0.68 |
