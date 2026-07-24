# bumbledb bench report

## Provenance

- crate version: 0.7.0
- engine rev: 7cef4e131049f0f45dfc37fe9e2ddeab5e6edca6
- timestamp: 2026-07-24T17:03:56Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 4.95 4.19 4.14 (start) → 3.53 3.79 3.96 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `6518394f080c2273299b55e00a8b022f88505650932a4d57fba40bd0bdf9a86b`
- verify stamp: `f1af7aff7b1dac94d67278ef6f82469fc7b7daf04d6c87aa5539fb8ab8a2511b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 9 block(s) still contaminated after retry — treat their percentiles as dirty: triangle, busy_scan, commit_single, commit_batch, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.2 / 0.3 / 0.5 | 1.4 / 1.9 / 2.6 | 0.18 | WIN |
| containment_walk | 6.6 / 636.5 / 674.6 | 50.5 / 29530.3 / 30126.4 | 0.13 | WIN |
| chain | 192.9 / 331.2 / 351.2 | 1807.3 / 3573.9 / 3773.3 | 0.11 | WIN |
| range | 19.9 / 25.1 / 29.6 | 142.1 / 543.8 / 571.5 | 0.14 | WIN |
| balance | 1.0 / 33.6 / 37.5 | 261.5 / 32038.1 / 32437.8 | 0.00 | WIN |
| stats | 1335.1 / 1451.8 / 1580.9 | 75140.8 / 79223.2 / 85099.9 | 0.02 | WIN |
| string | 2.6 / 2.8 / 2.8 | 58.0 / 62.8 / 67.3 | 0.04 | WIN |
| skew | 1525.7 / 2090.4 / 2293.0 | 7404.2 / 9868.8 / 10194.8 | 0.21 | WIN |
| spread | 10665.2 / 12226.1 / 13021.9 | 126554.2 / 130634.1 / 133113.0 | 0.08 | WIN |
| triangle | 2633.2 / 2766.6 / 2912.9 | 36867.1 / 40258.0 / 40836.2 | 0.07 | WIN |
| entries_for_account_set | 1.5 / 570.1 / 585.1 | 10.4 / 4041.0 / 4148.4 | 0.14 | WIN |
| postings_without_tag | 3.2 / 1006.0 / 1047.6 | 44.4 / 12990.3 / 13425.2 | 0.07 | WIN |
| latest_posting_per_account | 2246.9 / 2304.2 / 2341.1 | 41376.4 / 42811.3 / 43215.5 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.0 / 8.8 / 9.8 | 0.04 | WIN |
| mandate_overlap | 15.5 / 17.0 / 17.2 | 414.0 / 459.3 / 479.0 | 0.04 | WIN |
| deep_chain | 370.2 / 621.1 / 640.1 | 3460.1 / 6181.6 / 6261.0 | 0.11 | report |
| busy_scan | 7.8 / 8.9 / 12.9 | 3382.6 / 3465.8 / 3515.8 | 0.00 | WIN |
| meets_chain | 3.1 / 810.6 / 826.0 | 17.5 / 129.0 / 130.7 | 0.18 | WIN |
| rsvp_union | 929.8 / 965.6 / 1135.0 | 18113.5 / 18363.4 / 18546.7 | 0.05 | WIN |
| conflict_pairs | 34.5 / 91.4 / 99.8 | 2990.4 / 386102.5 / 394666.9 | 0.01 | WIN |
| conflict_free | 0.6 / 0.7 / 0.7 | 23.0 / 49.5 / 59.5 | 0.03 | WIN |
| free_busy | 3.0 / 41.4 / 41.7 | 261.9 / 2257.8 / 2273.6 | 0.01 | WIN |
| claim_hours | 431.6 / 444.2 / 456.8 | 6282.9 / 6518.3 / 6847.0 | 0.07 | WIN |
| slot_scan | 30.4 / 34.8 / 43.1 | 2765.8 / 2863.1 / 3063.3 | 0.01 | report |
| slot_booking_overlap | 6.8 / 60.2 / 60.3 | 737.7 / 15853.3 / 15947.0 | 0.01 | report |
| closure_depth | 9.5 / 1073.2 / 1123.5 | 28.5 / 1788.0 / 1815.2 | 0.33 | report |
| closure_fanout | 1.0 / 147.0 / 150.5 | 14.5 / 1962.8 / 2049.7 | 0.07 | report |
| disp_probe | 92505.9 / 122080.9 / 122080.9 | 646479.4 / 708650.0 / 708650.0 | 0.14 | report |
| disp_probe_d24 | 86515.0 / 99996.4 / 99996.4 | 636755.5 / 680178.2 / 680178.2 | 0.14 | report |
| disp_probe_d96 | 95893.2 / 109062.0 / 109062.0 | 631225.2 / 668073.7 / 668073.7 | 0.15 | report |
| disp_stream | 131.8 / 145.2 / 145.2 | 39725.5 / 41919.6 / 41919.6 | 0.00 | report |
| disp_stream_d24 | 141.6 / 167.5 / 167.5 | 39264.2 / 40075.8 / 40075.8 | 0.00 | report |
| disp_stream_d96 | 159.0 / 201.6 / 201.6 | 39988.0 / 40406.5 / 40406.5 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 5015.2 | 5018.9 | - |
| commit_batch | 27095.3 | 12060.5 | - |
| cold_containment_walk | 1259.2 | 84.7 | - |
| cold_containment_walk_delete | 3672.6 | 93.0 | - |
| commit_witnessed | 5125.2 | - | - |
| commit_window_baseline | 4683.2 | - | - |
| commit_window_admission | 5128.0 | - | - |
| commit_window_exclusion | 5136.3 | - | - |
| bulk | 1216362.7 | 690014.6 | 163458 |

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
| point | 3.30 | 3.26 | clean | - |
| containment_walk | 3.41 | 3.41 | clean | - |
| chain | 3.41 | 3.41 | clean | - |
| range | 3.29 | 3.23 | retried | - |
| balance | 3.35 | 3.41 | clean | - |
| stats | 3.41 | 3.41 | clean | - |
| string | 3.23 | 3.21 | clean | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.21 | 3.41 | clean | - |
| triangle | 3.05 | 3.26 | CONTAMINATED | - |
| entries_for_account_set | 3.31 | 3.36 | retried | - |
| postings_without_tag | 3.41 | 3.36 | clean | - |
| latest_posting_per_account | 3.41 | 3.41 | clean | - |
| mandate_at_instant | 3.40 | 3.41 | retried | - |
| mandate_overlap | 3.29 | 3.41 | clean | - |
| deep_chain | 3.41 | 3.36 | clean | - |
| busy_scan | 3.19 | 3.41 | CONTAMINATED | - |
| meets_chain | 3.35 | 3.41 | retried | - |
| rsvp_union | 3.34 | 3.36 | clean | - |
| conflict_pairs | 3.41 | 3.36 | clean | - |
| conflict_free | 3.36 | 3.36 | clean | - |
| free_busy | 3.24 | 3.41 | clean | - |
| claim_hours | 3.41 | 3.39 | clean | - |
| slot_scan | 3.24 | 3.33 | retried | - |
| slot_booking_overlap | 3.41 | 3.28 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.27 | 3.34 | clean | - |
| disp_probe | 3.41 | 3.22 | retried | - |
| disp_probe_d24 | 3.31 | 3.41 | clean | - |
| disp_probe_d96 | 3.41 | 3.41 | clean | - |
| disp_stream | 3.34 | 3.41 | clean | - |
| disp_stream_d24 | 3.40 | 3.41 | retried | - |
| disp_stream_d96 | 3.32 | 3.32 | clean | - |
| commit_single | 3.41 | 1.27 | CONTAMINATED | - |
| commit_batch | 1.28 | 3.50 | CONTAMINATED | - |
| cold_containment_walk | 3.50 | 3.40 | clean | - |
| cold_containment_walk_delete | 3.31 | 3.42 | clean | - |
| commit_witnessed | 3.41 | 1.72 | CONTAMINATED | - |
| commit_window_baseline | 3.30 | 0.91 | CONTAMINATED | - |
| commit_window_admission | 0.90 | 1.75 | CONTAMINATED | - |
| commit_window_exclusion | 1.75 | 0.91 | CONTAMINATED | - |
| bulk | 1.28 | 3.51 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
