# Campaign vs night: scenarios delta

Campaign run 2026-07-24 (rev `e9abf9ec`) vs night-2026-07-20 (rev `de39f54e`), identical protocol (8 warmups, 64 samples, seed 1, shared-machine boost, oracle-gated, capped adversarial lanes). Corpora regenerated under the fixed RNG (R20) — answer cardinalities move where the stream moved; every lane re-gated value-identical before timing. ratio = ours/theirs; ratio-vs-night = campaign ratio / night ratio (<1 = we improved relative to SQLite).

| query | lane | ours p50 (us) | sqlite p50 (us) | ratio | night ours p50 | night sqlite p50 | night ratio | ratio-vs-night | |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 0.2 | 7.0 | 0.04 | 2.3 | 9.5 | 0.24 | 0.15 | |
| j2_costars | sqlite | 0.9 | 12.1 | 0.07 | 0.9 | 14.6 | 0.06 | 1.20 | |
| j3_keyword_kind | sqlite | 3.6 | 16.5 | 0.22 | 3.8 | 13.0 | 0.29 | 0.76 | |
| j4_five_way | sqlite | 1481.6 | 4520.6 | 0.33 | 1278.2 | 5245.2 | 0.24 | 1.34 | |
| j5_country_rollup | sqlite | 5074.0 | 29343.9 | 0.17 | 5271.8 | 29481.3 | 0.18 | 0.97 | |
| j6_keyword_neighborhood | sqlite | 30.5 | 1314.2 | 0.02 | 32.0 | 1088.7 | 0.03 | 0.79 | |
| g1_neighbors | sqlite | 0.3 | 2.9 | 0.11 | 0.2 | 3.0 | 0.07 | 1.64 | |
| g2_two_hop | sqlite | 0.2 | 10.7 | 0.02 | 1.3 | 9.6 | 0.14 | 0.17 | |
| g3_three_hop_count | sqlite | 1.3 | 30.0 | 0.04 | 1.3 | 33.0 | 0.04 | 1.13 | |
| g4_mutual | sqlite | 3176.8 | 26806.5 | 0.12 | 3609.5 | 27772.3 | 0.13 | 0.91 | ◀ targeted |
| g5_triangles_from | sqlite | 0.7 | 14.2 | 0.05 | 1.0 | 21.7 | 0.04 | 1.13 | ◀ targeted |
| g6_weighted_hop | sqlite | 0.4 | 8.5 | 0.04 | 0.6 | 7.7 | 0.08 | 0.58 | |
| o1_revenue_by_region | sqlite | 458.2 | 229139.1 | 0.00 | 445.9 | 232071.8 | 0.00 | 1.05 | |
| o2_category_window | sqlite | 457.8 | 22007.8 | 0.02 | 427.0 | 21175.1 | 0.02 | 1.03 | |
| o3_promo_split | sqlite | 336.7 | 95263.1 | 0.00 | 7132.0 | 93156.0 | 0.08 | 0.05 | ◀ targeted |
| o4_segment_category | sqlite | 27849.0 | 371225.1 | 0.07 | 29701.8 | 379868.0 | 0.08 | 0.96 | ◀ targeted |
| o5_store_extremes | sqlite | 704.8 | 149807.1 | 0.00 | 8151.6 | 163659.6 | 0.05 | 0.09 | ◀ targeted |
| o6_brand_drill | sqlite | 1.8 | 562.9 | 0.00 | 1.5 | 721.0 | 0.00 | 1.48 | |
| p1_by_id | sqlite | 0.3 | 1.1 | 0.31 | 0.5 | 1.1 | 0.50 | 0.61 | ◀ targeted |
| p2_by_key | sqlite | 1.0 | 1.3 | 0.75 | 0.9 | 1.4 | 0.67 | 1.13 | ◀ targeted |
| p3_bucket_fetch | sqlite | 11.7 | 213.6 | 0.05 | 10.8 | 209.0 | 0.05 | 1.06 | |
| p4_size_band | sqlite | 0.2 | 112.2 | 0.00 | 0.2 | 111.9 | 0.00 | 1.00 | |
| p5_keyed_get | sqlite | 1.1 | 1.4 | 0.79 | 1.4 | 1.4 | 1.00 | 0.80 | ◀ targeted |
| r1_wash_ring | sqlite | 10395.8 | 111719.9 | 0.09 | 62523.5 | 111221.9 | 0.56 | 0.17 | ◀ targeted |
| r2_temporal_ring | sqlite | 32687.7 | 160205.4 | 0.20 | 62857.3 | 149166.5 | 0.42 | 0.48 | ◀ targeted |
| r2_temporal_ring | sqlite-tuned | 32687.7 | 103719.2 | 0.32 | 62857.3 | 104417.5 | 0.60 | 0.52 | ◀ targeted |
| r3_bomb_t1 | sqlite | 3722.9 | 31530.9 | 0.12 | 3000.8 | 32407.1 | 0.09 | 1.28 | |
| r4_bomb_t2 | sqlite | 1737880.8 | DNF>1000ms | — | 1576952.3 | DNF | — | — (DNF both) | ◀ targeted |
| r5_reciprocal | sqlite | 513.5 | 3302.2 | 0.16 | 715.2 | 3240.7 | 0.22 | 0.70 | |
| r6_two_path_count | sqlite | 135227.6 | 664714.5 | 0.20 | 135264.0 | 674157.7 | 0.20 | 1.01 | ◀ targeted |
| t1_stab | sqlite | 1.1 | 5.4 | 0.21 | 1.1 | 11.3 | 0.10 | 2.09 | |
| t2_overlap_join | sqlite | 56831.6 | DNF>1000ms | — | 162794.7 | DNF | — | — (DNF both) | ◀ targeted |
| t2_overlap_join | sqlite-tuned | 56831.6 | 487174.6 | 0.12 | 162794.7 | 509756.0 | 0.32 | 0.37 | ◀ targeted |
| t3_mixed_mask | sqlite | 42.0 | 1108.8 | 0.04 | 67.9 | 1309.7 | 0.05 | 0.73 | ◀ targeted |
| t4_ray_stab | sqlite | 42.2 | 4221.2 | 0.01 | 40.2 | 4182.0 | 0.01 | 1.04 | |
| t5_pack_key | sqlite-hand | 2.2 | 94.7 | 0.02 | 3.3 | 102.0 | 0.03 | 0.71 | |

## The targeted queries

The campaign targeted p1/p2/p5, r1/r2/r4/r6, t2/t3, o3/o4/o5, g4/g5.

**Landed hard:**

- **o3_promo_split** — ours 7132.0 → 336.7us (21.2x); ratio 0.08 → 0.00, ratio-vs-night 0.05. The dense group table for closed/bool group keys (049) removes the hash path entirely on the bool key.
- **o5_store_extremes** — ours 8151.6 → 704.8us (11.6x); ratio 0.05 → 0.00, ratio-vs-night 0.09. Dense group table + fold pushdown (049, 013/014 sink halves) over the 200-store rollup.
- **r1_wash_ring** — ours 62523.5 → 10395.8us (6.0x); ratio 0.56 → 0.09, ratio-vs-night 0.17. The gj_split GJ-end lowering for cyclic rules (009): the equality 3-ring stops paying the binary-join exponent.
- **t2_overlap_join** — ours 162794.7 → 56831.6us (2.9x); tuned-lane ratio 0.32 → 0.12, ratio-vs-night 0.37. The order-based overlap join (012) + const-operand Allen kernel (048); the canonical SQLite OR-chain lane still DNFs the 1000ms cap, both runs.
- **r2_temporal_ring** — ours 62857.3 → 32687.7us (1.9x); canonical ratio 0.42 → 0.20 (rvn 0.48), tuned 0.60 → 0.32 (rvn 0.52). GJ lowering + the overlap-join order path together.
- **p5_keyed_get** — ours 1.4 → 1.1us; ratio 1.00 → 0.79 (rvn 0.80). The allocation-free composed-key point read (R15 pool): the keyed get drops below SQLite parity for the first time.
- **p1_by_id** — ours 0.5 → 0.3us; ratio 0.50 → 0.31 (rvn 0.61). Same pool, fresh-id probe side.
- **t3_mixed_mask** — ours 67.9 → 42.0us (1.6x); rvn 0.73.

**Moved modestly:**

- **g4_mutual** — ours 3609.5 → 3176.8us; rvn 0.91.
- **o4_segment_category** — ours 29701.8 → 27849.0us; rvn 0.96. The 3-way join dominates; the dense-group win is a small share of this lane.

**Honestly flat or adverse:**

- **r6_two_path_count** — ours 135264.0 → 135227.6us; rvn 1.01. Unmoved: the distinct 2-path count is the denominator story and stays materialization-bound on both engines.
- **p2_by_key** — ratio 0.67 → 0.75 (rvn 1.13). Sub-microsecond lane (ours 0.9 → 1.0us, SQLite 1.4 → 1.3us); inside shared-machine jitter at this resolution, but reported as measured.
- **g5_triangles_from** — ours improved 1.0 → 0.7us but SQLite improved more (21.7 → 14.2us); rvn 1.13 on a sub-2us numerator.
- **r4_bomb_t2** — ours 1576952.3 → 1737880.8us (+10.2%); SQLite DNF>1000ms both runs, so no ratio either run. The exponent evidence stands (m^3≈5.7e7 closing probes finish; SQLite's cap trips). The +10% on our side is reported exactly as measured; candidate cause is the graded chunk geometry (094) trading the 384-wide fanout at the tie point, on a shared machine.

Overall geomean across the 34 queries: **0.05** (night: 0.08); 2 DNF lanes, identical set to the night run (r4_bomb_t2 sqlite, t2_overlap_join canonical sqlite).
