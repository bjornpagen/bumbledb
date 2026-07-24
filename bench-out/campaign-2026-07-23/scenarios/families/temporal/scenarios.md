# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## temporal (geomean ratio 0.04 over 4 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| t1_stab | sqlite | 796 | 1.1 | 5.4 | 0.21 | interval stabbing: point-in-span membership probe |
| t2_overlap_join | sqlite | 1 | 56831.6 | DNF>1000ms | — | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t2_overlap_join | sqlite-tuned | 1 | 56831.6 | 487174.6 | 0.12 | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t3_mixed_mask | sqlite | 26543 | 42.0 | 1108.8 | 0.04 | mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data |
| t4_ray_stab | sqlite | 2247 | 42.2 | 4221.2 | 0.01 | open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter |
| t5_pack_key | sqlite-hand | 7 | 2.2 | 94.7 | 0.02 | Pack/coalesce: Snodgrass coalescing per key — SQLite's lane is the hand-written islands SQL (the free_busy precedent) |

Overall geomean ratio across 5 queries: **0.04**; 1 lane(s) DNF > cap (excluded, counted).
