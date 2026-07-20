# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: ec0b9c75f013ce85c3aa4fce0c055ae7c46e0d49
- timestamp: 2026-07-20T12:27:25Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 1.67 2.13 2.52 (start) → 2.06 2.10 2.41 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `11727f470c9a5464631fe3b1c7ba5448a6060cf3637b34ba7bb0f171cc7df1fa (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 7 block(s) still contaminated after retry — treat their percentiles as dirty: range, commit_single, commit_batch, cold_containment_walk, cold_containment_walk_delete, commit_witnessed, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.4 / 0.4 / 0.4 | 1.3 / 1.3 / 1.6 | 0.30 | WIN |
| containment_walk | 2.2 / 555.9 / 575.4 | 45.6 / 28480.1 / 29100.0 | 0.05 | WIN |
| chain | 54.3 / 90.9 / 95.6 | 493.0 / 940.8 / 953.8 | 0.11 | WIN |
| range | 18.5 / 18.7 / 20.8 | 138.5 / 534.4 / 547.9 | 0.13 | WIN |
| balance | 1.0 / 34.5 / 39.8 | 268.1 / 31038.2 / 31632.5 | 0.00 | WIN |
| stats | 1236.3 / 1255.6 / 1293.5 | 73848.3 / 74986.4 / 75958.3 | 0.02 | WIN |
| string | 2.2 / 2.4 / 2.4 | 55.7 / 61.4 / 62.8 | 0.04 | WIN |
| skew | 1523.8 / 2023.1 / 2034.1 | 7228.4 / 9734.5 / 9867.7 | 0.21 | WIN |
| spread | 10395.9 / 11392.8 / 11924.5 | 124281.9 / 125897.0 / 127485.8 | 0.08 | WIN |
| triangle | 9808.1 / 10561.5 / 11275.8 | 37298.6 / 56862.2 / 57059.2 | 0.26 | WIN |
| entries_for_account_set | 1.0 / 498.9 / 511.1 | 6.6 / 3996.6 / 4037.5 | 0.15 | WIN |
| postings_without_tag | 2.9 / 1053.2 / 1088.6 | 46.2 / 12805.0 / 13069.7 | 0.06 | WIN |
| latest_posting_per_account | 2045.8 / 2175.3 / 2254.4 | 40640.8 / 41823.3 / 42744.0 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.3 / 8.3 / 8.5 | 0.03 | WIN |
| mandate_overlap | 8.2 / 13.5 / 16.0 | 195.2 / 311.5 / 314.6 | 0.04 | WIN |
| busy_scan | 7.2 / 8.4 / 8.5 | 3493.2 / 3567.6 / 3632.2 | 0.00 | WIN |
| meets_chain | 3.4 / 1326.3 / 1329.1 | 17.5 / 130.5 / 131.5 | 0.19 | WIN |
| rsvp_union | 863.9 / 905.8 / 1001.7 | 18027.1 / 18288.2 / 18523.6 | 0.05 | WIN |
| conflict_pairs | 27.8 / 126.3 / 130.5 | 2758.9 / 386044.0 / 388144.5 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.7 | 20.4 / 51.0 / 55.6 | 0.03 | WIN |
| free_busy | 3.8 / 45.3 / 49.5 | 269.5 / 2317.1 / 2380.0 | 0.01 | WIN |
| claim_hours | 515.4 / 534.4 / 582.4 | 6337.7 / 6432.2 / 6492.0 | 0.08 | WIN |
| slot_scan | 28.7 / 29.3 / 29.8 | 2895.0 / 2957.5 / 2970.5 | 0.01 | report |
| slot_booking_overlap | 24.7 / 594.8 / 610.8 | 629.6 / 15433.0 / 15590.9 | 0.04 | report |
| closure_depth | 6.4 / 976.4 / 1017.2 | 23.4 / 1805.2 / 1824.4 | 0.27 | report |
| closure_fanout | 1.1 / 143.2 / 147.9 | 11.9 / 1971.1 / 1985.4 | 0.09 | report |
| disp_probe | 108218.7 / 126549.5 / 126549.5 | 630206.6 / 650230.6 / 650230.6 | 0.17 | report |
| disp_probe_d24 | 109985.2 / 132018.0 / 132018.0 | 629804.8 / 646759.6 / 646759.6 | 0.17 | report |
| disp_probe_d96 | 115433.4 / 120112.9 / 120112.9 | 624635.0 / 635802.8 / 635802.8 | 0.18 | report |
| disp_stream | 131.4 / 139.0 / 139.0 | 38901.5 / 39937.4 / 39937.4 | 0.00 | report |
| disp_stream_d24 | 142.3 / 145.8 / 145.8 | 39432.6 / 39631.5 / 39631.5 | 0.00 | report |
| disp_stream_d96 | 153.9 / 174.9 / 174.9 | 39650.5 / 39724.2 / 39724.2 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 53.2 | 4983.8 | - |
| commit_batch | 5352.1 | 29049.8 | - |
| cold_containment_walk | 1374.4 | 73.0 | - |
| cold_containment_walk_delete | 3733.0 | 78.4 | - |
| commit_witnessed | 58.8 | - | - |
| commit_window_baseline | 26.9 | - | - |
| commit_window_admission | 35.2 | - | - |
| commit_window_exclusion | 34.0 | - | - |
| bulk | 775226.0 | 939194.0 | 257332 |

## Allocations

(not captured — run with the alloc window)

## Execution digests

| family | worst est/actual | covers | emitted | absorbed |
|---|---|---|---|---|
| point | 1.00 |  | 1 | 0 |
| containment_walk | 2.11 | n0:s0x1/s1x0/s2x0 n1:s0x1 n2:s0x1 | 95 | 0 |
| chain | 24.92 | n0:s0x1/s1x0 n1:s0x143/s1x0 n2:s0x333 | 333 | 0 |
| range | 3.12 | n0:s0x1 | 2000 | 0 |
| balance | 63.47 | n0:s0x1/s1x0 n1:s0x7 | 50774 | 0 |
| stats | 1.00 | n0:s0x1/s1x0 n1:s0x500 | 100000 | 0 |
| string | 8.52 | n0:s0x1/s1x0 n1:s0x1 | 183 | 0 |
| skew | 2.50 | n0:s0x1/s1x0 n1:s0x40015 | 40015 | 0 |
| spread | 2.01 | n0:s0x1/s1x0 n1:s0x100000 | 99467 | 0 |
| triangle | 4761.90 | n0:s0x1/s1x0 n1:s0x100000/s1x0 n2:s0x504 | 504 | 499 |
| postings_without_tag | 3.70 | n0:s0x1 | 54 | 0 |
| latest_posting_per_account | 1.00 | n0:s0x1 | 100000 | 0 |
| mandate_at_instant | 1.00 | n0:s0x1/s1x0 n1:s0x1 | 1 | 0 |
| mandate_overlap | 2.22 | n0:s0x1/s1x0 n1:s0x31 | 433 | 1 |
| busy_scan | 38.10 | n0:s0x1 | 619 | 0 |
| meets_chain | 511.00 | n0:s0x1/s1x0 n1:s0x511 | 170 | 0 |
| rsvp_union | 3.00 | n0:s0x1 n1:s0x1 n2:s0x1 | 82983 | 0 |
| conflict_pairs | 289.00 | n0:s0x1/s1x0/s2x0 n1:s0x8/s1x0 n2:s0x64 n3:s0x82 | 64 | 0 |
| conflict_free | 691.20 | n0:s0x1/s1x0 n1:s0x5 | 5 | 0 |
| free_busy | 18.18 | n0:s0x1/s1x0 n1:s0x8 | 1600 | 0 |
| claim_hours | 3.20 | n0:s0x1 | 33564 | 0 |
| slot_scan | 10.42 | n0:s0x1 | 2125 | 0 |
| slot_booking_overlap | 31.10 | n0:s0x1/s1x0 n1:s0x410 | 214 | 0 |

## Store

- bumbledb file (compacted): 77955072 bytes
- sqlite file: 18464768 bytes
- image cache: 0 images, 0 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| point | 3.35 | 3.45 | clean | - |
| containment_walk | 3.50 | 3.22 | retried | - |
| chain | 3.41 | 3.41 | clean | - |
| range | 3.20 | 3.41 | CONTAMINATED | - |
| balance | 3.40 | 3.41 | retried | - |
| stats | 3.35 | 3.39 | clean | - |
| string | 3.36 | 3.36 | clean | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.41 | 3.36 | clean | - |
| triangle | 3.41 | 3.23 | clean | - |
| entries_for_account_set | 3.39 | 3.41 | clean | - |
| postings_without_tag | 3.41 | 3.30 | clean | - |
| latest_posting_per_account | 3.35 | 3.40 | clean | - |
| mandate_at_instant | 3.22 | 3.28 | clean | - |
| mandate_overlap | 3.26 | 3.30 | clean | - |
| busy_scan | 3.41 | 3.40 | clean | - |
| meets_chain | 3.41 | 3.41 | clean | - |
| rsvp_union | 3.41 | 3.35 | clean | - |
| conflict_pairs | 3.29 | 3.28 | clean | - |
| conflict_free | 3.28 | 3.28 | clean | - |
| free_busy | 3.22 | 3.22 | clean | - |
| claim_hours | 3.34 | 3.30 | clean | - |
| slot_scan | 3.41 | 3.41 | clean | - |
| slot_booking_overlap | 3.29 | 3.28 | clean | - |
| closure_depth | 3.41 | 3.37 | retried | - |
| closure_fanout | 3.41 | 3.27 | clean | - |
| disp_probe | 3.50 | 3.45 | clean | - |
| disp_probe_d24 | 3.50 | 3.41 | clean | - |
| disp_probe_d96 | 3.50 | 3.37 | clean | - |
| disp_stream | 3.36 | 3.35 | clean | - |
| disp_stream_d24 | 3.41 | 3.41 | clean | - |
| disp_stream_d96 | 3.45 | 3.41 | clean | - |
| commit_single | 3.23 | 1.99 | CONTAMINATED | - |
| commit_batch | 2.00 | 3.00 | CONTAMINATED | - |
| cold_containment_walk | 2.82 | 3.26 | CONTAMINATED | - |
| cold_containment_walk_delete | 3.23 | 3.13 | CONTAMINATED | - |
| commit_witnessed | 3.08 | 3.13 | CONTAMINATED | - |
| commit_window_baseline | 3.29 | 3.45 | clean | - |
| commit_window_admission | 3.44 | 3.50 | clean | - |
| commit_window_exclusion | 3.39 | 3.34 | clean | - |
| bulk | 3.37 | 1.97 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
