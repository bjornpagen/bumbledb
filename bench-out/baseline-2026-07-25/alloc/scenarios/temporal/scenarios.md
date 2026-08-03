# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 1 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## temporal (geomean ratio 0.02 over 4 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| t1_stab | sqlite | 1986 | 28.5 | 2499.9 | 0.01 | interval stabbing: point-in-span membership probe |
| t2_overlap_join | sqlite | 1 | 55898.1 | DNF>1000ms | — | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t2_overlap_join | sqlite-tuned | 1 | 55898.1 | 515864.9 | 0.11 | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t3_mixed_mask | sqlite | 105992 | 40599.0 | 1845033.5 | 0.02 | mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data |
| t4_ray_stab | sqlite | 2997 | 41.3 | 4259.5 | 0.01 | open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter |
| t5_pack_key | sqlite-hand | 2 | 76.5 | 3065.8 | 0.02 | Pack/coalesce: Snodgrass coalescing per key — SQLite's lane is the hand-written islands SQL (the free_busy precedent) |

Overall geomean ratio across 5 queries: **0.02**; 1 lane(s) DNF > cap (excluded, counted).

## Allocations (per query, --alloc)

| query | allocs | alloc bytes | deallocs | dealloc bytes |
|---|---:|---:|---:|---:|
| t1_stab | 2 | 32 | 1 | 24 |
| t2_overlap_join | 1 | 8 | 0 | 0 |
| t3_mixed_mask | 2 | 32 | 1 | 24 |
| t4_ray_stab | 2 | 32 | 1 | 24 |
| t5_pack_key | 2 | 32 | 1 | 24 |
