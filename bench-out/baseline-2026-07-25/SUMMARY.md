# Baseline 2026-07-25 (run 2026-08-01, wall power) vs campaign 2026-07-23 — full-suite geomeans

The bugbash-perf campaign's Phase 3 full-suite baseline (TODO item 2), run at v0.9.0+instrument (`e511b540`; later lanes stamp `3997bb1f`/`0aad29bc`/`b25c9226` — bench-out-only commits landed mid-run, no code delta). Protocol identical to the campaign run: seed 1, scale S, registered lane protocols, oracle-gated everywhere the campaign gated, shared-machine boost (`BUMBLEDB_BENCH_BOOST=1`), measurement mutex per lane, AC verified by pmset before AND after every lane. Corpus: the capacity-spelling regeneration (0.8.0 cutover), identity digest `fa73e680324f9b26…`, verify stamp `5c3b2c1d…` — the campaign rode the pre-capacity corpus (`6518394f…`), so absolute deltas could confound engine and corpus changes; the storage lane pins the confound at zero (fact counts and on-disk bytes byte-identical across the two corpora, see below), so read/write deltas here are engine + machine-noise only. Every geomean is recomputed from both runs' JSON with identical pairing (common cells only; DNF/capped/twinless cells excluded-and-counted identically both sides). ratio = ours/sqlite (lower is better); vs-campaign = baseline geomean / campaign geomean (<1 = our standing improved).

| suite | cells | baseline geomean | campaign geomean | vs-campaign | note |
|---|---:|---:|---:|---:|---|
| bench-durable-r1 | 38 | 0.0845 | 0.0793 | 1.07 | all_win holds all six reps, 33/33 read families WIN; cells = 33 reads + 5 twinned writes common to both runs (the baseline roster adds the 0.8.0 window/capacity write families — 12 write cells now, 7 engine-only or unpaired vs the campaign's 9) |
| bench-durable-r2 | 38 | 0.0845 | 0.0812 | 1.04 | reads-only: 0.0461 vs 0.0441 → 1.04 |
| bench-durable-r3 | 38 | 0.0758 | 0.0768 | 0.99 | reads-only: 0.0407 vs 0.0411 → 0.99 |
| bench-ephemeral-r1 | 38 | 0.0809 | 0.0754 | 1.07 | like-for-like this time (both runs ride the 020 honest pairing) |
| bench-ephemeral-r2 | 38 | 0.0737 | 0.0771 | 0.96 | reads-only: 0.0400 vs 0.0432 → 0.93 |
| bench-ephemeral-r3 | 38 | 0.0740 | 0.0811 | 0.91 | reads-only: 0.0404 vs 0.0455 → 0.89 |
| scenarios | 32 | 0.0527 | 0.0512 | 1.03 | same roster (34 queries), same two SQLite DNFs (r4_bomb_t2, t2_overlap_join) excluded-and-counted; t5_pack_key rides its hand lane both runs; movers below |
| curves | 4 | 0.0403 | 0.0416 | 0.97 | 0 capped points both runs; per-family below |
| crud | 22 | 1.6382 | 1.6831 | 0.97 | SQLite's home turf; nosync narrows again (upsert 2.25→1.60, update_hot 3.50→2.84, mixed 1.27→1.04), durable gives a little back (upsert 0.97→1.20, rmw 1.01→1.21) |
| lawful | 12 | 2.8123 | 3.1890 | 0.88 | the segment-1 re-pin under the capacity spelling (e511b540), not re-run; law_reject_key durable 1282×→312× — the volatile side is again SQLite's instant UNIQUE abort (the campaign flagged the same swing 573→1282) |
| storage (bytes) | 2 | 3.487 / 3.608 | 3.487 / 3.608 | 1.00 | byte-identical to the campaign per world (ledger 64.27 MB / 253264 facts, calendar 64.68 MB / 192369 facts) — the capacity-spelling corpus regeneration moved the digest, not the stores; the cross-corpus confound is zero |
| writes | 18 | 1.1188 | 1.1308 | 0.99 | the campaign's own wall-power writes rerun (0.7.0 @ 8c1250db) is the comparand; vs the night pins 1.0965 → 1.02 |
| churn | 7 lanes | — | — | — | vs night only (no campaign churn exists); table below |

The report-class picture: durable r1/r2 slipped 4-7%, r3 flat; ephemeral r2/r3 improved 4-9%, r1 slipped 7% — rep-to-rep spread straddles 1.0 in both directions, so the whole-suite read tier is flat-to-noise vs the campaign, with no single-family collapse (all_win holds everywhere). Per-query attribution is the traced pass's job (Phase 3 item 3), not this table's.

## Scenario movers (baseline ratio / campaign ratio; <1 = improved)

Improved: t3_mixed_mask 0.41 (0.0437→0.0179), j2_costars 0.71, j6_keyword_neighborhood 0.76, g4_mutual 0.79, g1_neighbors 0.82, p4_size_band 0.86.
Regressed: g6_weighted_hop 2.20 (0.0486→0.1071), g3_three_hop_count 1.88 (0.0363→0.0682), t1_stab 1.86 (0.1086→0.2015), g5_triangles_from 1.32 (0.0349→0.0459) — the graph world carries the regression cluster; flag for the Hunt phase's trace readers (attribution first, no intuition fixes).

## Curves per-family (SQLite p50 / ours p50, S scale)

| family | baseline | campaign | mover |
|---|---:|---:|---|
| busy_scan | 451× | 424× | ours 8166 → 7459 ns |
| triangle | 14.0× | 13.9× | flat (ours 2.75 → 2.55 ms) |
| closure_fanout | 12.3× | 13.3× | ours flat (1083 → 1000 ns) |
| point | 4.9× | 4.3× | ours 333 → 291 ns |

Warmth panel present (0 capped regions, ghz stamped per point).

## Writes (18 twinned cells, both durability lanes × commit/delete ladder)

vs campaign 0.99 (flat); vs night 1.02. The night-relative movers are lane-shaped, not ladder-shaped: nosync improved across the board (commit_b100 1.04→0.66, commit_b10 1.03→0.69, delete_b1 1.77→1.07) while durable large batches regressed (commit_b1000 0.98→1.97, delete_b1000 1.15→2.10, commit_b100 1.12→1.69) — but vs the campaign's own wall-power rerun the same cells are ≤1.09, so the swing is night-vs-wall-power fsync physics, not a post-campaign engine change.

## Churn (final-cycle, 10000 cycles, vs night 2026-07-20 — first wall-power churn)

| run / lane | baseline commits/s | night commits/s | final probe p50 geomean vs night |
|---|---:|---:|---:|
| steady / ours-durable | 38.9 | 43.1 | 0.84 |
| steady / sqlite-bare | 35.2 | 47.5 | 1.08 |
| steady / sqlite-maint | 32.6 | 44.1 | 0.98 |
| nosync / ours-ephemeral | 261.9 | 256.7 | 0.82 |
| nosync / sqlite-nosync | 202.6 | 220.9 | 1.07 |
| delete-heavy / ours-durable | 20.6 | 19.8 | 0.82 |
| delete-heavy / sqlite-bare | 21.1 | 27.3 | 1.04 |

Our probe p50s improved 16-18% on every lane while SQLite's held or degraded; commit throughput moved both directions inside the durable-fsync noise band. Zero oracle mismatches on every gated churn sample.

## Oracle honesty

Zero mismatches baseline-wide: every timed cell in every gated lane re-earned value-identical multiset agreement before its window (fresh verify stamp on the fa73e680 corpus; scenario worlds gated per query; churn gated per sample; crud/lawful post-state folds value-identical). DNFs: the same two scenario SQLite DNFs as the night and campaign, excluded and counted; 0 capped curve points. Contamination discriminators ran everywhere and are reported, never hidden: reads r1 2/33, r2 5/33, r3 0/33, e1 3/33, e2 1/33, e3 0/33 (campaign: 8/9/2/6/4/4); durable writes 10-11/12 (fsync-DVFS physics, campaign 7/9); crud 12/22 (campaign 7/22), lawful 6/12 (campaign 3/12).
