# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: 3b31cd84ca6093b9feb2c0c72488891fd1cbe4f9
- timestamp: 2026-08-03T21:54:13Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.28 2.66 2.89 (start) → 3.69 3.22 3.06 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `d832a1dc42cda1d5873d538439bc0a55ecfdccb4a1830f92f79cada67c47974a (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 12 block(s) still contaminated after retry — treat their percentiles as dirty: point, balance, spread, meets_chain, conflict_pairs, commit_single, commit_batch, commit_witnessed, commit_capacity_baseline, commit_capacity_sum, commit_capacity_duration, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.5 | 1.4 / 1.7 / 2.3 | 0.18 | WIN |
| containment_walk | 2.4 / 799.0 / 1071.5 | 69.6 / 31193.5 / 32015.2 | 0.03 | WIN |
| chain | 216.0 / 375.5 / 413.8 | 1911.5 / 3697.7 / 3801.7 | 0.11 | WIN |
| range | 21.0 / 22.5 / 25.0 | 142.5 / 586.3 / 606.2 | 0.15 | WIN |
| balance | 1.1 / 33.3 / 36.8 | 285.4 / 33794.7 / 34419.3 | 0.00 | WIN |
| stats | 1554.4 / 2368.8 / 6226.5 | 78244.2 / 84213.9 / 110790.2 | 0.02 | WIN |
| string | 2.6 / 2.7 / 7.0 | 60.5 / 73.8 / 126.8 | 0.04 | WIN |
| skew | 1917.5 / 2604.2 / 2780.2 | 7647.2 / 10345.7 / 10880.0 | 0.25 | WIN |
| spread | 10808.1 / 12780.5 / 14159.6 | 132286.0 / 146023.8 / 179853.0 | 0.08 | WIN |
| triangle | 2542.6 / 2797.1 / 3788.8 | 37800.6 / 42396.5 / 48320.5 | 0.07 | WIN |
| entries_for_account_set | 3.5 / 731.6 / 843.4 | 13.1 / 4360.7 / 4512.7 | 0.27 | WIN |
| postings_without_tag | 2.5 / 1079.0 / 1125.5 | 46.4 / 13743.9 / 14265.3 | 0.05 | WIN |
| latest_posting_per_account | 2620.3 / 3022.1 / 3182.1 | 41774.8 / 44717.2 / 45710.5 | 0.06 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.1 / 8.2 / 8.3 | 0.04 | WIN |
| mandate_overlap | 13.8 / 14.8 / 15.0 | 411.9 / 445.3 / 460.9 | 0.03 | WIN |
| deep_chain | 375.6 / 616.9 / 629.5 | 4539.7 / 6346.6 / 11361.4 | 0.08 | report |
| busy_scan | 8.3 / 9.7 / 11.5 | 3392.5 / 3498.0 / 3576.8 | 0.00 | WIN |
| meets_chain | 3.0 / 117.2 / 120.6 | 17.4 / 134.3 / 140.5 | 0.17 | WIN |
| rsvp_union | 973.3 / 1081.2 / 1238.7 | 18717.3 / 19577.4 / 20936.1 | 0.05 | WIN |
| conflict_pairs | 34.4 / 88.1 / 118.6 | 2871.5 / 377302.6 / 381724.6 | 0.01 | WIN |
| conflict_free | 0.6 / 0.7 / 0.7 | 22.5 / 48.8 / 49.2 | 0.03 | WIN |
| free_busy | 3.1 / 40.8 / 45.7 | 275.0 / 2295.8 / 2356.3 | 0.01 | WIN |
| claim_hours | 439.8 / 464.0 / 471.5 | 6224.2 / 6384.2 / 6631.3 | 0.07 | WIN |
| slot_scan | 32.6 / 35.0 / 43.1 | 2782.2 / 2863.8 / 2943.6 | 0.01 | report |
| slot_booking_overlap | 7.7 / 63.9 / 70.7 | 692.4 / 15023.1 / 15198.9 | 0.01 | report |
| closure_depth | 1.5 / 1154.2 / 1176.5 | 35.7 / 1878.7 / 1894.5 | 0.04 | report |
| closure_fanout | 43.9 / 45.2 / 45.6 | 551.1 / 567.8 / 583.1 | 0.08 | report |
| disp_probe | 86397.5 / 125331.2 / 125331.2 | 646433.2 / 765724.2 / 765724.2 | 0.13 | report |
| disp_probe_d24 | 86604.9 / 112797.9 / 112797.9 | 652293.5 / 733259.7 / 733259.7 | 0.13 | report |
| disp_probe_d96 | 87091.5 / 92508.8 / 92508.8 | 638938.8 / 712593.3 / 712593.3 | 0.14 | report |
| disp_stream | 131.8 / 141.8 / 141.8 | 39103.5 / 40428.9 / 40428.9 | 0.00 | report |
| disp_stream_d24 | 142.9 / 146.0 / 146.0 | 39701.5 / 40122.8 / 40122.8 | 0.00 | report |
| disp_stream_d96 | 162.0 / 182.4 / 182.4 | 40277.3 / 41404.0 / 41404.0 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 4573.5 | 4486.5 | - |
| commit_batch | 25612.6 | 12931.8 | - |
| cold_containment_walk | 1195.6 | 86.6 | - |
| cold_containment_walk_delete | 11559.3 | 86.1 | - |
| commit_witnessed | 5129.0 | - | - |
| commit_window_baseline | 4408.8 | - | - |
| commit_window_admission | 4566.4 | - | - |
| commit_window_exclusion | 4557.2 | - | - |
| commit_capacity_baseline | 4644.7 | - | - |
| commit_capacity_sum | 5090.1 | - | - |
| commit_capacity_duration | 5129.6 | - | - |
| bulk | 1127731.0 | 682632.0 | 178718 |

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
| point | 3.36 | 3.19 | CONTAMINATED | - |
| containment_walk | 3.26 | 3.36 | retried | - |
| chain | 3.21 | 3.26 | clean | - |
| range | 3.36 | 3.31 | retried | - |
| balance | 3.36 | 3.16 | CONTAMINATED | - |
| stats | 3.26 | 3.26 | retried | - |
| string | 3.38 | 3.41 | clean | - |
| skew | 3.30 | 3.38 | clean | - |
| spread | 2.78 | 3.33 | CONTAMINATED | - |
| triangle | 3.36 | 3.24 | clean | - |
| entries_for_account_set | 3.41 | 3.30 | retried | - |
| postings_without_tag | 3.27 | 3.41 | clean | - |
| latest_posting_per_account | 3.36 | 3.36 | clean | - |
| mandate_at_instant | 3.41 | 3.41 | clean | - |
| mandate_overlap | 3.41 | 3.41 | clean | - |
| deep_chain | 3.41 | 3.41 | clean | - |
| busy_scan | 3.41 | 3.41 | retried | - |
| meets_chain | 3.11 | 3.24 | CONTAMINATED | - |
| rsvp_union | 3.36 | 3.36 | clean | - |
| conflict_pairs | 3.25 | 2.67 | CONTAMINATED | - |
| conflict_free | 3.36 | 3.41 | clean | - |
| free_busy | 3.36 | 3.41 | clean | - |
| claim_hours | 3.32 | 3.24 | clean | - |
| slot_scan | 3.29 | 3.20 | clean | - |
| slot_booking_overlap | 3.41 | 3.26 | clean | - |
| closure_depth | 3.21 | 3.26 | clean | - |
| closure_fanout | 3.28 | 3.41 | clean | - |
| disp_probe | 3.41 | 3.41 | clean | - |
| disp_probe_d24 | 3.22 | 3.41 | retried | - |
| disp_probe_d96 | 3.40 | 3.40 | clean | - |
| disp_stream | 3.41 | 3.41 | clean | - |
| disp_stream_d24 | 3.35 | 3.40 | clean | - |
| disp_stream_d96 | 3.23 | 3.35 | clean | - |
| commit_single | 3.41 | 2.35 | CONTAMINATED | - |
| commit_batch | 2.42 | 3.12 | CONTAMINATED | - |
| cold_containment_walk | 3.30 | 3.36 | clean | - |
| cold_containment_walk_delete | 3.33 | 3.36 | clean | - |
| commit_witnessed | 3.36 | 1.75 | CONTAMINATED | - |
| commit_window_baseline | 3.34 | 3.30 | clean | - |
| commit_window_admission | 3.36 | 3.28 | clean | - |
| commit_window_exclusion | 3.36 | 3.26 | clean | - |
| commit_capacity_baseline | 3.36 | 0.89 | CONTAMINATED | - |
| commit_capacity_sum | 0.91 | 1.28 | CONTAMINATED | - |
| commit_capacity_duration | 1.28 | 2.24 | CONTAMINATED | - |
| bulk | 2.42 | 3.51 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
