# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: 3b31cd84ca6093b9feb2c0c72488891fd1cbe4f9
- timestamp: 2026-08-03T22:03:15Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 4.41 3.65 3.28 (start) → 3.99 3.50 3.28 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `d832a1dc42cda1d5873d538439bc0a55ecfdccb4a1830f92f79cada67c47974a (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 1 block(s) still contaminated after retry — treat their percentiles as dirty: latest_posting_per_account.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.3 | 1.4 / 1.4 / 1.4 | 0.19 | WIN |
| containment_walk | 2.0 / 683.6 / 696.6 | 50.0 / 29467.0 / 30060.2 | 0.04 | WIN |
| chain | 197.4 / 355.3 / 372.0 | 1827.6 / 3536.4 / 3594.3 | 0.11 | WIN |
| range | 20.6 / 20.7 / 20.8 | 139.4 / 547.5 / 560.5 | 0.15 | WIN |
| balance | 1.1 / 37.5 / 44.7 | 253.6 / 31910.5 / 32926.2 | 0.00 | WIN |
| stats | 1399.9 / 1438.6 / 1464.3 | 75039.5 / 77897.2 / 80311.8 | 0.02 | WIN |
| string | 2.6 / 2.8 / 2.8 | 57.8 / 61.6 / 94.2 | 0.05 | WIN |
| skew | 1635.8 / 2228.2 / 2559.7 | 7440.8 / 9942.2 / 10325.8 | 0.22 | WIN |
| spread | 10728.6 / 12304.4 / 13044.5 | 126944.8 / 135828.0 / 154897.5 | 0.08 | WIN |
| triangle | 2561.9 / 2802.1 / 3061.3 | 37253.8 / 41000.0 / 41855.6 | 0.07 | WIN |
| entries_for_account_set | 2.9 / 702.0 / 732.1 | 10.4 / 4191.6 / 4360.0 | 0.28 | WIN |
| postings_without_tag | 2.6 / 1229.9 / 1415.0 | 44.5 / 13332.0 / 13630.5 | 0.06 | WIN |
| latest_posting_per_account | 2569.7 / 2795.8 / 2952.3 | 42621.1 / 44672.4 / 50984.5 | 0.06 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.1 / 8.8 / 9.7 | 0.04 | WIN |
| mandate_overlap | 13.9 / 18.9 / 24.9 | 414.4 / 449.0 / 465.6 | 0.03 | WIN |
| deep_chain | 368.2 / 626.5 / 658.8 | 3422.2 / 6309.7 / 6828.0 | 0.11 | report |
| busy_scan | 8.4 / 9.8 / 9.9 | 3425.7 / 3644.8 / 3753.8 | 0.00 | WIN |
| meets_chain | 3.0 / 128.3 / 142.7 | 17.7 / 141.2 / 153.8 | 0.17 | WIN |
| rsvp_union | 979.1 / 1060.4 / 1108.3 | 18301.7 / 18959.1 / 19471.2 | 0.05 | WIN |
| conflict_pairs | 29.1 / 87.2 / 97.8 | 2838.8 / 372214.6 / 374161.7 | 0.01 | WIN |
| conflict_free | 0.6 / 0.7 / 0.8 | 24.3 / 49.1 / 57.7 | 0.03 | WIN |
| free_busy | 3.0 / 40.3 / 40.5 | 261.4 / 2312.3 / 2332.8 | 0.01 | WIN |
| claim_hours | 436.0 / 451.5 / 469.1 | 6279.7 / 6498.4 / 6880.6 | 0.07 | WIN |
| slot_scan | 32.3 / 34.9 / 39.6 | 2782.7 / 2863.5 / 2902.7 | 0.01 | report |
| slot_booking_overlap | 7.5 / 60.1 / 63.2 | 682.8 / 14709.8 / 14808.8 | 0.01 | report |
| closure_depth | 0.9 / 1096.5 / 1116.3 | 23.6 / 1816.1 / 1847.8 | 0.04 | report |
| closure_fanout | 0.5 / 151.4 / 154.7 | 8.0 / 1938.0 / 1947.5 | 0.06 | report |
| disp_probe | 86441.1 / 105776.8 / 105776.8 | 645230.5 / 653975.8 / 653975.8 | 0.13 | report |
| disp_probe_d24 | 84527.7 / 100342.4 / 100342.4 | 638945.9 / 646713.1 / 646713.1 | 0.13 | report |
| disp_probe_d96 | 84301.0 / 97041.1 / 97041.1 | 632305.9 / 639706.4 / 639706.4 | 0.13 | report |
| disp_stream | 131.9 / 145.1 / 145.1 | 39211.6 / 40300.3 / 40300.3 | 0.00 | report |
| disp_stream_d24 | 143.5 / 156.7 / 156.7 | 39513.1 / 40403.9 / 40403.9 | 0.00 | report |
| disp_stream_d96 | 158.1 / 182.5 / 182.5 | 39932.2 / 40216.0 / 40216.0 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 45.0 | 29.8 | - |
| commit_batch | 4954.2 | 6054.9 | - |
| cold_containment_walk | 1070.0 | 80.8 | - |
| cold_containment_walk_delete | 11601.4 | 82.8 | - |
| commit_witnessed | 50.3 | - | - |
| commit_window_baseline | 26.7 | - | - |
| commit_window_admission | 35.2 | - | - |
| commit_window_exclusion | 33.5 | - | - |
| commit_capacity_baseline | 19.0 | - | - |
| commit_capacity_sum | 32.9 | - | - |
| commit_capacity_duration | 30.4 | - | - |
| bulk | 622524.0 | 444785.0 | 319993 |

## Allocations

(not captured — run with the alloc window)

## Execution digests

| family | worst est/actual | covers | emitted | absorbed |
|---|---|---|---|---|
| point | 1.00 |  | 1 | 0 |
| containment_walk | 2.08 | n0:s0x1/s1x0/s2x0 n1:s0x1 n2:s0x1 | 96 | 0 |
| chain | 6.25 | n0:s0x1/s1x0 n1:s0x141/s1x0 n2:s0x1328 | 1328 | 0 |
| range | 3.12 | n0:s0x1 | 2000 | 0 |
| balance | 63.80 | n0:s0x1/s1x0 n1:s0x7 | 51042 | 0 |
| stats | 166.67 | n0:s0x1 n1:s0x3/s1x0 n2:s0x500 | 100000 | 0 |
| string | 8.00 | n0:s0x1/s1x0 n1:s0x1 | 202 | 0 |
| skew | 1.20 | n0:s0x1/s1x0 n1:s0x40014 | 40014 | 0 |
| spread | 2.00 | n0:s0x1/s1x0 n1:s0x100000 | 99944 | 0 |
| triangle | 4536.86 | n0:s0x1/s1x0/s2x0 n1:s0x529/s1x0 n2:s0x529 | 529 | 524 |
| postings_without_tag | 4.00 | n0:s0x1 | 50 | 0 |
| latest_posting_per_account | 200.00 | n0:s0x1 n1:s0x500 | 100000 | 0 |
| mandate_at_instant | 1.00 | n0:s0x1/s1x0 n1:s0x1 | 1 | 0 |
| mandate_overlap | 2.97 | n0:s0x1/s1x0 n1:s0x26 | 224 | 1 |
| deep_chain | 12.50 | n0:s0x1/s1x0 n1:s0x123/s1x0 n2:s0x426/s1x0 n3:s0x2000 | 2000 | 0 |
| busy_scan | 19.49 | n0:s0x1 | 605 | 0 |
| meets_chain | 511.00 | n0:s0x1/s1x0 n1:s0x511 | 170 | 0 |
| rsvp_union | 1.00 | n0:s0x1 n1:s0x1 n2:s0x1 | 82983 | 0 |
| conflict_pairs | 200.06 | n0:s0x1/s1x0/s2x0 n1:s0x8/s1x0 n2:s0x64 n3:s0x82 | 64 | 0 |
| conflict_free | 576.00 | n0:s0x1/s1x0 n1:s0x6 | 6 | 0 |
| free_busy | 18.18 | n0:s0x1/s1x0 n1:s0x8 | 1600 | 0 |
| claim_hours | 5240.50 | n0:s0x1 n1:s0x2 | 33564 | 0 |
| slot_scan | 10.42 | n0:s0x1 | 2125 | 0 |
| slot_booking_overlap | 21.53 | n0:s0x1/s1x0 n1:s0x410 | 214 | 0 |

## Store

- bumbledb file (compacted): 64274432 bytes
- sqlite file: 18432000 bytes
- image cache: 0 images, 0 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| point | 3.50 | 3.50 | clean | - |
| containment_walk | 3.35 | 3.41 | retried | - |
| chain | 3.23 | 3.36 | clean | - |
| range | 3.36 | 3.33 | clean | - |
| balance | 3.30 | 3.28 | clean | - |
| stats | 3.26 | 3.36 | clean | - |
| string | 3.41 | 3.38 | clean | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.27 | 3.36 | clean | - |
| triangle | 3.33 | 3.29 | clean | - |
| entries_for_account_set | 3.41 | 3.38 | clean | - |
| postings_without_tag | 3.22 | 3.34 | clean | - |
| latest_posting_per_account | 3.16 | 3.16 | CONTAMINATED | - |
| mandate_at_instant | 3.41 | 3.27 | clean | - |
| mandate_overlap | 3.35 | 3.41 | clean | - |
| deep_chain | 3.41 | 3.29 | clean | - |
| busy_scan | 3.41 | 3.41 | clean | - |
| meets_chain | 3.34 | 3.41 | clean | - |
| rsvp_union | 3.33 | 3.41 | clean | - |
| conflict_pairs | 3.41 | 3.36 | clean | - |
| conflict_free | 3.41 | 3.41 | retried | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 3.41 | 3.35 | clean | - |
| slot_scan | 3.36 | 3.36 | clean | - |
| slot_booking_overlap | 3.36 | 3.41 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.36 | 3.41 | clean | - |
| disp_probe | 3.36 | 3.36 | clean | - |
| disp_probe_d24 | 3.50 | 3.25 | retried | - |
| disp_probe_d96 | 3.31 | 3.41 | clean | - |
| disp_stream | 3.41 | 3.23 | clean | - |
| disp_stream_d24 | 3.41 | 3.24 | clean | - |
| disp_stream_d96 | 3.41 | 3.41 | retried | - |
| commit_single | 3.41 | 3.45 | clean | - |
| commit_batch | 3.26 | 3.41 | clean | - |
| cold_containment_walk | 3.41 | 3.50 | clean | - |
| cold_containment_walk_delete | 3.50 | 3.50 | clean | - |
| commit_witnessed | 3.50 | 3.50 | clean | - |
| commit_window_baseline | 3.41 | 3.41 | clean | - |
| commit_window_admission | 3.41 | 3.41 | clean | - |
| commit_window_exclusion | 3.41 | 3.27 | clean | - |
| commit_capacity_baseline | 3.41 | 3.41 | clean | - |
| commit_capacity_sum | 3.41 | 3.36 | clean | - |
| commit_capacity_duration | 3.29 | 3.41 | clean | - |
| bulk | 3.36 | 3.32 | clean | - |

## Flame summaries

(none captured — run with --trace)
