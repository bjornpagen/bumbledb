# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: ec0b9c75f013ce85c3aa4fce0c055ae7c46e0d49
- timestamp: 2026-07-20T12:00:16Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 4.26 3.09 2.58 (start) → 4.90 3.09 2.63 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `11727f470c9a5464631fe3b1c7ba5448a6060cf3637b34ba7bb0f171cc7df1fa (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 8 block(s) still contaminated after retry — treat their percentiles as dirty: commit_single, commit_batch, cold_containment_walk, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.5 / 0.6 | 1.4 / 1.4 / 1.5 | 0.36 | WIN |
| containment_walk | 5.4 / 557.1 / 577.0 | 46.1 / 28685.8 / 29247.2 | 0.12 | WIN |
| chain | 57.6 / 91.5 / 94.9 | 510.2 / 955.9 / 982.0 | 0.11 | WIN |
| range | 18.9 / 19.1 / 19.2 | 138.2 / 536.9 / 544.9 | 0.14 | WIN |
| balance | 1.0 / 33.0 / 33.1 | 266.8 / 31424.6 / 31878.5 | 0.00 | WIN |
| stats | 1241.5 / 1386.3 / 1536.1 | 74737.8 / 76539.1 / 77959.8 | 0.02 | WIN |
| string | 2.2 / 2.4 / 2.5 | 56.9 / 62.3 / 69.3 | 0.04 | WIN |
| skew | 1533.7 / 2056.9 / 2266.8 | 7334.7 / 9770.0 / 9897.3 | 0.21 | WIN |
| spread | 10546.1 / 11622.2 / 12602.0 | 126108.2 / 127920.5 / 129256.9 | 0.08 | WIN |
| triangle | 9814.5 / 10450.2 / 11535.2 | 37473.6 / 56808.2 / 57256.0 | 0.26 | WIN |
| entries_for_account_set | 1.1 / 500.1 / 517.4 | 6.5 / 3989.7 / 4015.5 | 0.17 | WIN |
| postings_without_tag | 5.6 / 1065.5 / 1201.5 | 45.7 / 12639.0 / 12739.1 | 0.12 | WIN |
| latest_posting_per_account | 2063.1 / 2098.2 / 2132.9 | 40662.6 / 41497.0 / 42349.5 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 7.9 / 8.1 / 8.7 | 0.03 | WIN |
| mandate_overlap | 7.8 / 12.6 / 12.7 | 200.6 / 311.1 / 312.7 | 0.04 | WIN |
| busy_scan | 7.2 / 8.3 / 8.3 | 3504.3 / 3555.9 / 3646.2 | 0.00 | WIN |
| meets_chain | 3.3 / 1346.7 / 1361.1 | 17.3 / 130.8 / 133.9 | 0.19 | WIN |
| rsvp_union | 865.7 / 899.2 / 1041.8 | 17894.0 / 18345.2 / 26022.0 | 0.05 | WIN |
| conflict_pairs | 23.0 / 123.0 / 123.2 | 2778.7 / 379213.1 / 380227.5 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.6 | 15.2 / 47.0 / 47.2 | 0.04 | WIN |
| free_busy | 2.6 / 40.0 / 42.9 | 255.8 / 2259.2 / 2275.3 | 0.01 | WIN |
| claim_hours | 507.8 / 512.9 / 522.3 | 6257.9 / 6389.4 / 6451.8 | 0.08 | WIN |
| slot_scan | 28.5 / 29.2 / 29.4 | 2861.9 / 2916.8 / 2955.1 | 0.01 | report |
| slot_booking_overlap | 23.3 / 558.1 / 582.5 | 691.0 / 15150.1 / 15263.5 | 0.03 | report |
| closure_depth | 2.8 / 950.0 / 954.0 | 10.0 / 1774.2 / 1790.2 | 0.28 | report |
| closure_fanout | 1.2 / 138.6 / 145.7 | 14.0 / 1923.6 / 1967.9 | 0.09 | report |
| disp_probe | 126193.5 / 136796.0 / 136796.0 | 643537.0 / 660816.0 / 660816.0 | 0.20 | report |
| disp_probe_d24 | 126596.3 / 142094.5 / 142094.5 | 619717.0 / 849738.0 / 849738.0 | 0.20 | report |
| disp_probe_d96 | 114703.0 / 120442.4 / 120442.4 | 614202.3 / 644038.6 / 644038.6 | 0.19 | report |
| disp_stream | 128.2 / 135.8 / 135.8 | 37785.4 / 38887.4 / 38887.4 | 0.00 | report |
| disp_stream_d24 | 139.9 / 141.2 / 141.2 | 38311.4 / 39635.3 / 39635.3 | 0.00 | report |
| disp_stream_d96 | 155.2 / 192.6 / 192.6 | 38424.3 / 38993.5 / 38993.5 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 5165.7 | 5093.2 | - |
| commit_batch | 27987.5 | 29861.0 | - |
| cold_containment_walk | 1656.0 | 98.5 | - |
| cold_containment_walk_delete | 4113.0 | 78.3 | - |
| commit_witnessed | 5112.8 | - | - |
| commit_window_baseline | 4831.5 | - | - |
| commit_window_admission | 5124.2 | - | - |
| commit_window_exclusion | 5146.1 | - | - |
| bulk | 1258970.0 | 927869.0 | 158695 |

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

- bumbledb file (compacted): 78004224 bytes
- sqlite file: 18464768 bytes
- image cache: 0 images, 0 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| point | 3.50 | 3.43 | clean | - |
| containment_walk | 3.41 | 3.41 | clean | - |
| chain | 3.41 | 3.40 | clean | - |
| range | 3.41 | 3.41 | clean | - |
| balance | 3.39 | 3.36 | clean | - |
| stats | 3.36 | 3.36 | clean | - |
| string | 3.41 | 3.41 | clean | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.41 | 3.41 | clean | - |
| triangle | 3.36 | 3.36 | clean | - |
| entries_for_account_set | 3.41 | 3.35 | clean | - |
| postings_without_tag | 3.41 | 3.41 | clean | - |
| latest_posting_per_account | 3.41 | 3.41 | clean | - |
| mandate_at_instant | 3.25 | 3.32 | clean | - |
| mandate_overlap | 3.36 | 3.41 | clean | - |
| busy_scan | 3.41 | 3.41 | clean | - |
| meets_chain | 3.41 | 3.41 | clean | - |
| rsvp_union | 3.41 | 3.30 | retried | - |
| conflict_pairs | 3.41 | 3.41 | clean | - |
| conflict_free | 3.41 | 3.41 | clean | - |
| free_busy | 3.35 | 3.41 | clean | - |
| claim_hours | 3.41 | 3.40 | clean | - |
| slot_scan | 3.34 | 3.36 | clean | - |
| slot_booking_overlap | 3.41 | 3.41 | clean | - |
| closure_depth | 3.33 | 3.41 | retried | - |
| closure_fanout | 3.41 | 3.35 | clean | - |
| disp_probe | 3.50 | 3.49 | clean | - |
| disp_probe_d24 | 3.32 | 3.34 | clean | - |
| disp_probe_d96 | 3.36 | 3.50 | clean | - |
| disp_stream | 3.50 | 3.50 | clean | - |
| disp_stream_d24 | 3.41 | 3.50 | clean | - |
| disp_stream_d96 | 3.50 | 3.51 | clean | - |
| commit_single | 3.21 | 1.75 | CONTAMINATED | - |
| commit_batch | 1.75 | 3.00 | CONTAMINATED | - |
| cold_containment_walk | 2.97 | 3.34 | CONTAMINATED | - |
| cold_containment_walk_delete | 3.22 | 3.23 | clean | - |
| commit_witnessed | 3.27 | 0.89 | CONTAMINATED | - |
| commit_window_baseline | 3.26 | 0.91 | CONTAMINATED | - |
| commit_window_admission | 0.91 | 0.91 | CONTAMINATED | - |
| commit_window_exclusion | 0.86 | 0.91 | CONTAMINATED | - |
| bulk | 2.00 | 1.97 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
