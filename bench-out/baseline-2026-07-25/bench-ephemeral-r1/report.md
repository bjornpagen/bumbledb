# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: e511b5404438908f0be4b01cafcd5621e3275c77
- timestamp: 2026-08-02T00:45:38Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.68 2.74 2.73 (start) → 3.03 3.01 2.85 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `5c3b2c1d056c180caa921ba7be8f8eb2a1f3da0d8133532cf045f2f3a429c243 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 3 block(s) still contaminated after retry — treat their percentiles as dirty: entries_for_account_set, free_busy, slot_booking_overlap.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.3 | 1.4 / 1.7 / 2.2 | 0.18 | WIN |
| containment_walk | 2.4 / 606.8 / 618.6 | 51.6 / 29847.0 / 30290.8 | 0.05 | WIN |
| chain | 259.5 / 387.9 / 1120.9 | 2599.2 / 3768.0 / 10607.9 | 0.10 | WIN |
| range | 20.7 / 20.9 / 21.0 | 144.4 / 560.8 / 571.5 | 0.14 | WIN |
| balance | 1.1 / 34.7 / 34.9 | 281.7 / 33136.8 / 34040.5 | 0.00 | WIN |
| stats | 1319.6 / 1410.0 / 1547.1 | 75658.9 / 79612.1 / 82338.4 | 0.02 | WIN |
| string | 2.5 / 2.6 / 2.7 | 59.5 / 64.2 / 72.3 | 0.04 | WIN |
| skew | 1524.0 / 2040.5 / 2117.8 | 7316.5 / 9751.8 / 9917.0 | 0.21 | WIN |
| spread | 10458.8 / 10958.2 / 12618.3 | 126208.0 / 128478.0 / 129013.2 | 0.08 | WIN |
| triangle | 2637.6 / 2714.0 / 2785.7 | 36778.9 / 39935.9 / 40650.2 | 0.07 | WIN |
| entries_for_account_set | 5.7 / 572.1 / 609.8 | 7.5 / 4046.0 / 4169.1 | 0.76 | WIN |
| postings_without_tag | 3.3 / 1014.5 / 1065.2 | 44.0 / 13057.4 / 13339.4 | 0.07 | WIN |
| latest_posting_per_account | 2254.9 / 2374.5 / 2422.5 | 41418.7 / 42958.8 / 44323.2 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.6 | 8.1 / 8.7 / 9.0 | 0.04 | WIN |
| mandate_overlap | 15.8 / 17.1 / 20.8 | 412.8 / 454.0 / 471.5 | 0.04 | WIN |
| deep_chain | 401.1 / 623.2 / 685.6 | 3235.8 / 6143.8 / 6265.7 | 0.12 | report |
| busy_scan | 7.7 / 8.7 / 8.8 | 3370.0 / 3479.3 / 3662.5 | 0.00 | WIN |
| meets_chain | 3.1 / 820.6 / 829.1 | 17.4 / 132.7 / 137.3 | 0.18 | WIN |
| rsvp_union | 929.9 / 971.7 / 1027.9 | 17973.5 / 18363.0 / 19196.7 | 0.05 | WIN |
| conflict_pairs | 31.9 / 94.0 / 99.2 | 2772.4 / 368721.8 / 371762.9 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.6 | 22.4 / 47.2 / 52.0 | 0.03 | WIN |
| free_busy | 4.2 / 41.4 / 44.7 | 282.1 / 2311.5 / 2371.9 | 0.01 | WIN |
| claim_hours | 435.1 / 454.5 / 477.5 | 6267.0 / 6362.9 / 6406.2 | 0.07 | WIN |
| slot_scan | 30.4 / 31.4 / 35.6 | 2787.4 / 2883.4 / 2925.6 | 0.01 | report |
| slot_booking_overlap | 10.7 / 59.6 / 66.7 | 666.2 / 14752.8 / 14866.9 | 0.02 | report |
| closure_depth | 2.8 / 1070.5 / 1147.4 | 14.0 / 1828.8 / 1861.2 | 0.20 | report |
| closure_fanout | 4.6 / 154.3 / 163.1 | 10.3 / 1970.3 / 1998.5 | 0.44 | report |
| disp_probe | 82808.3 / 96528.9 / 96528.9 | 662537.0 / 688048.5 / 688048.5 | 0.12 | report |
| disp_probe_d24 | 84498.4 / 96629.0 / 96629.0 | 635316.2 / 647867.9 / 647867.9 | 0.13 | report |
| disp_probe_d96 | 89330.6 / 95610.9 / 95610.9 | 635648.9 / 646987.8 / 646987.8 | 0.14 | report |
| disp_stream | 131.6 / 138.0 / 138.0 | 39077.8 / 39238.3 / 39238.3 | 0.00 | report |
| disp_stream_d24 | 142.9 / 163.0 / 163.0 | 39534.9 / 41732.9 / 41732.9 | 0.00 | report |
| disp_stream_d96 | 156.7 / 167.7 / 167.7 | 40022.5 / 40278.6 / 40278.6 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 44.9 | 30.1 | - |
| commit_batch | 5417.6 | 6033.4 | - |
| cold_containment_walk | 1104.9 | 87.7 | - |
| cold_containment_walk_delete | 3474.6 | 83.4 | - |
| commit_witnessed | 50.5 | - | - |
| commit_window_baseline | 26.5 | - | - |
| commit_window_admission | 35.0 | - | - |
| commit_window_exclusion | 34.0 | - | - |
| commit_capacity_baseline | 19.8 | - | - |
| commit_capacity_sum | 33.4 | - | - |
| commit_capacity_duration | 32.8 | - | - |
| bulk | 759799.9 | 445119.4 | 263173 |

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
| point | 3.50 | 3.36 | clean | - |
| containment_walk | 3.41 | 3.39 | clean | - |
| chain | 3.26 | 3.26 | clean | - |
| range | 3.26 | 3.26 | clean | - |
| balance | 3.26 | 3.26 | clean | - |
| stats | 3.33 | 3.23 | clean | - |
| string | 3.40 | 3.33 | retried | - |
| skew | 3.34 | 3.41 | clean | - |
| spread | 3.41 | 3.36 | clean | - |
| triangle | 3.36 | 3.34 | clean | - |
| entries_for_account_set | 3.14 | 3.40 | CONTAMINATED | - |
| postings_without_tag | 3.23 | 3.41 | clean | - |
| latest_posting_per_account | 3.36 | 3.21 | clean | - |
| mandate_at_instant | 3.34 | 3.23 | retried | - |
| mandate_overlap | 3.22 | 3.34 | clean | - |
| deep_chain | 3.36 | 3.36 | retried | - |
| busy_scan | 3.22 | 3.41 | clean | - |
| meets_chain | 3.36 | 3.41 | clean | - |
| rsvp_union | 3.28 | 3.41 | clean | - |
| conflict_pairs | 3.41 | 3.41 | clean | - |
| conflict_free | 3.22 | 3.35 | clean | - |
| free_busy | 3.00 | 3.32 | CONTAMINATED | - |
| claim_hours | 3.28 | 3.28 | retried | - |
| slot_scan | 3.36 | 3.41 | clean | - |
| slot_booking_overlap | 3.35 | 3.10 | CONTAMINATED | - |
| closure_depth | 3.36 | 3.41 | retried | - |
| closure_fanout | 3.41 | 3.40 | clean | - |
| disp_probe | 3.25 | 3.33 | clean | - |
| disp_probe_d24 | 3.41 | 3.36 | clean | - |
| disp_probe_d96 | 3.22 | 3.41 | retried | - |
| disp_stream | 3.41 | 3.38 | clean | - |
| disp_stream_d24 | 3.34 | 3.41 | clean | - |
| disp_stream_d96 | 3.41 | 3.35 | retried | - |
| commit_single | 3.50 | 3.41 | clean | - |
| commit_batch | 3.41 | 3.28 | clean | - |
| cold_containment_walk | 3.34 | 3.36 | clean | - |
| cold_containment_walk_delete | 3.29 | 3.41 | clean | - |
| commit_witnessed | 3.41 | 3.41 | clean | - |
| commit_window_baseline | 3.41 | 3.41 | clean | - |
| commit_window_admission | 3.41 | 3.25 | clean | - |
| commit_window_exclusion | 3.41 | 3.35 | clean | - |
| commit_capacity_baseline | 3.41 | 3.41 | clean | - |
| commit_capacity_sum | 3.41 | 3.32 | clean | - |
| commit_capacity_duration | 3.26 | 3.41 | clean | - |
| bulk | 3.36 | 3.34 | clean | - |

## Flame summaries

(none captured — run with --trace)
