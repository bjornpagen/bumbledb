# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: e511b5404438908f0be4b01cafcd5621e3275c77
- timestamp: 2026-08-02T00:53:22Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.19 2.57 2.68 (start) → 3.71 2.99 2.82 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `5c3b2c1d056c180caa921ba7be8f8eb2a1f3da0d8133532cf045f2f3a429c243 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 2 block(s) still contaminated after retry — treat their percentiles as dirty: commit_capacity_baseline, commit_capacity_duration.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.2 / 0.3 / 0.3 | 1.4 / 1.4 / 1.6 | 0.17 | WIN |
| containment_walk | 6.0 / 621.2 / 671.5 | 49.0 / 28934.4 / 29470.5 | 0.12 | WIN |
| chain | 180.1 / 332.9 / 339.2 | 1801.9 / 3502.5 / 3559.2 | 0.10 | WIN |
| range | 19.9 / 20.1 / 24.0 | 138.7 / 534.8 / 550.5 | 0.14 | WIN |
| balance | 1.0 / 33.5 / 36.8 | 276.7 / 31733.6 / 33095.5 | 0.00 | WIN |
| stats | 1334.8 / 1394.1 / 1441.9 | 74588.2 / 76896.0 / 82442.8 | 0.02 | WIN |
| string | 2.5 / 2.6 / 2.7 | 58.8 / 61.0 / 63.0 | 0.04 | WIN |
| skew | 1511.4 / 2021.2 / 2052.2 | 7304.5 / 9735.0 / 10289.8 | 0.21 | WIN |
| spread | 10287.1 / 11459.5 / 12430.7 | 126316.8 / 139690.7 / 165930.7 | 0.08 | WIN |
| triangle | 2639.7 / 2701.2 / 2762.4 | 37044.9 / 41133.6 / 43105.8 | 0.07 | WIN |
| entries_for_account_set | 1.2 / 553.7 / 557.7 | 9.5 / 3999.0 / 4101.3 | 0.13 | WIN |
| postings_without_tag | 6.7 / 1014.2 / 1023.5 | 45.7 / 13698.2 / 14216.2 | 0.15 | WIN |
| latest_posting_per_account | 2316.6 / 2700.0 / 2837.7 | 42830.0 / 44936.0 / 65037.0 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.6 | 8.0 / 8.8 / 9.8 | 0.03 | WIN |
| mandate_overlap | 15.7 / 17.1 / 25.7 | 413.1 / 462.8 / 507.8 | 0.04 | WIN |
| deep_chain | 469.3 / 633.5 / 715.6 | 3230.2 / 6266.7 / 6344.9 | 0.15 | report |
| busy_scan | 7.8 / 8.8 / 8.9 | 3417.6 / 3563.9 / 3633.4 | 0.00 | WIN |
| meets_chain | 3.1 / 838.0 / 908.5 | 17.7 / 133.6 / 154.5 | 0.17 | WIN |
| rsvp_union | 941.5 / 1002.2 / 1026.4 | 18598.2 / 19654.4 / 26152.5 | 0.05 | WIN |
| conflict_pairs | 33.7 / 91.3 / 99.2 | 4332.4 / 377071.8 / 393093.2 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.8 | 19.1 / 47.8 / 52.2 | 0.03 | WIN |
| free_busy | 4.2 / 41.8 / 47.5 | 249.2 / 2247.9 / 2293.8 | 0.02 | WIN |
| claim_hours | 435.1 / 440.9 / 451.6 | 6267.9 / 6437.9 / 6762.3 | 0.07 | WIN |
| slot_scan | 30.6 / 31.8 / 36.2 | 2768.2 / 2831.4 / 2896.6 | 0.01 | report |
| slot_booking_overlap | 6.7 / 59.8 / 62.8 | 655.2 / 14616.3 / 14782.4 | 0.01 | report |
| closure_depth | 4.5 / 1051.0 / 1078.5 | 20.8 / 1801.3 / 1818.0 | 0.22 | report |
| closure_fanout | 1.0 / 149.9 / 156.4 | 34.1 / 1935.8 / 1961.8 | 0.03 | report |
| disp_probe | 81635.5 / 90647.6 / 90647.6 | 637512.1 / 674329.1 / 674329.1 | 0.13 | report |
| disp_probe_d24 | 83118.4 / 113111.7 / 113111.7 | 639028.9 / 646989.0 / 646989.0 | 0.13 | report |
| disp_probe_d96 | 89233.4 / 118295.5 / 118295.5 | 636010.4 / 784513.7 / 784513.7 | 0.14 | report |
| disp_stream | 131.5 / 135.0 / 135.0 | 39876.7 / 40801.0 / 40801.0 | 0.00 | report |
| disp_stream_d24 | 156.2 / 172.9 / 172.9 | 40602.4 / 41398.0 / 41398.0 | 0.00 | report |
| disp_stream_d96 | 163.2 / 210.0 / 210.0 | 40699.3 / 41951.1 / 41951.1 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 49.5 | 33.0 | - |
| commit_batch | 5627.8 | 6216.1 | - |
| cold_containment_walk | 1142.7 | 94.7 | - |
| cold_containment_walk_delete | 3645.4 | 95.2 | - |
| commit_witnessed | 53.8 | - | - |
| commit_window_baseline | 29.8 | - | - |
| commit_window_admission | 38.3 | - | - |
| commit_window_exclusion | 34.8 | - | - |
| commit_capacity_baseline | 20.3 | - | - |
| commit_capacity_sum | 35.8 | - | - |
| commit_capacity_duration | 32.4 | - | - |
| bulk | 764709.7 | 442387.5 | 259620 |

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
| point | 3.50 | 3.41 | clean | - |
| containment_walk | 3.40 | 3.41 | clean | - |
| chain | 3.36 | 3.23 | clean | - |
| range | 3.33 | 3.41 | clean | - |
| balance | 3.36 | 3.35 | clean | - |
| stats | 3.35 | 3.41 | clean | - |
| string | 3.41 | 3.41 | clean | - |
| skew | 3.32 | 3.41 | clean | - |
| spread | 3.41 | 3.20 | clean | - |
| triangle | 3.30 | 3.41 | clean | - |
| entries_for_account_set | 3.41 | 3.41 | clean | - |
| postings_without_tag | 3.41 | 3.24 | retried | - |
| latest_posting_per_account | 3.41 | 3.35 | clean | - |
| mandate_at_instant | 3.35 | 3.21 | retried | - |
| mandate_overlap | 3.41 | 3.28 | retried | - |
| deep_chain | 3.41 | 3.41 | clean | - |
| busy_scan | 3.41 | 3.41 | clean | - |
| meets_chain | 3.41 | 3.41 | clean | - |
| rsvp_union | 3.29 | 3.31 | clean | - |
| conflict_pairs | 3.21 | 3.41 | clean | - |
| conflict_free | 3.41 | 3.41 | clean | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 3.41 | 3.34 | clean | - |
| slot_scan | 3.36 | 3.32 | clean | - |
| slot_booking_overlap | 3.35 | 3.41 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.41 | 3.41 | clean | - |
| disp_probe | 3.41 | 3.31 | retried | - |
| disp_probe_d24 | 3.36 | 3.41 | clean | - |
| disp_probe_d96 | 3.30 | 3.41 | clean | - |
| disp_stream | 3.31 | 3.35 | clean | - |
| disp_stream_d24 | 3.36 | 3.27 | clean | - |
| disp_stream_d96 | 3.23 | 3.36 | clean | - |
| commit_single | 3.26 | 3.26 | clean | - |
| commit_batch | 3.26 | 3.26 | clean | - |
| cold_containment_walk | 3.26 | 3.26 | clean | - |
| cold_containment_walk_delete | 3.22 | 3.22 | clean | - |
| commit_witnessed | 3.26 | 3.26 | clean | - |
| commit_window_baseline | 3.26 | 3.26 | clean | - |
| commit_window_admission | 3.26 | 3.36 | clean | - |
| commit_window_exclusion | 3.33 | 3.36 | clean | - |
| commit_capacity_baseline | 3.10 | 3.36 | CONTAMINATED | - |
| commit_capacity_sum | 3.30 | 3.23 | clean | - |
| commit_capacity_duration | 3.20 | 3.36 | CONTAMINATED | - |
| bulk | 3.26 | 3.32 | clean | - |

## Flame summaries

(none captured — run with --trace)
