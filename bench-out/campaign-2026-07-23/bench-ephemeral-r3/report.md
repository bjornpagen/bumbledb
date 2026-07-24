# bumbledb bench report

## Provenance

- crate version: 0.7.0
- engine rev: 7cef4e131049f0f45dfc37fe9e2ddeab5e6edca6
- timestamp: 2026-07-24T17:16:26Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 5.65 5.07 4.57 (start) → 3.30 4.45 4.43 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `6518394f080c2273299b55e00a8b022f88505650932a4d57fba40bd0bdf9a86b`
- verify stamp: `f1af7aff7b1dac94d67278ef6f82469fc7b7daf04d6c87aa5539fb8ab8a2511b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 7 block(s) still contaminated after retry — treat their percentiles as dirty: stats, free_busy, disp_probe_d96, disp_stream_d96, cold_containment_walk_delete, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.3 | 1.4 / 1.6 / 2.8 | 0.18 | WIN |
| containment_walk | 10.1 / 632.5 / 908.8 | 57.6 / 31740.4 / 32413.1 | 0.18 | WIN |
| chain | 249.9 / 348.3 / 406.0 | 1922.8 / 3705.2 / 3881.2 | 0.13 | WIN |
| range | 19.9 / 26.9 / 32.9 | 146.8 / 587.9 / 668.1 | 0.14 | WIN |
| balance | 1.1 / 33.2 / 37.3 | 298.7 / 34077.6 / 34751.5 | 0.00 | WIN |
| stats | 1379.4 / 1811.8 / 2003.4 | 79593.1 / 82024.2 / 90463.7 | 0.02 | WIN |
| string | 2.5 / 2.6 / 2.6 | 58.0 / 62.7 / 75.2 | 0.04 | WIN |
| skew | 1711.8 / 2322.6 / 2440.7 | 7631.6 / 10114.3 / 10304.4 | 0.22 | WIN |
| spread | 11960.2 / 13164.9 / 16132.5 | 127368.0 / 134213.0 / 138573.2 | 0.09 | WIN |
| triangle | 2631.6 / 2735.1 / 2780.3 | 37292.5 / 41783.5 / 45213.7 | 0.07 | WIN |
| entries_for_account_set | 8.2 / 574.9 / 601.2 | 9.8 / 4085.0 / 4128.0 | 0.83 | WIN |
| postings_without_tag | 6.2 / 1105.1 / 1271.4 | 51.2 / 13688.4 / 14118.1 | 0.12 | WIN |
| latest_posting_per_account | 2325.8 / 2596.8 / 2786.0 | 41598.8 / 43542.9 / 44128.0 | 0.06 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 7.9 / 8.1 / 8.2 | 0.04 | WIN |
| mandate_overlap | 15.4 / 16.8 / 17.0 | 411.5 / 449.0 / 478.7 | 0.04 | WIN |
| deep_chain | 465.9 / 641.5 / 767.4 | 3522.3 / 6353.5 / 6613.9 | 0.13 | report |
| busy_scan | 7.8 / 9.3 / 10.9 | 3432.7 / 3698.2 / 7985.9 | 0.00 | WIN |
| meets_chain | 3.1 / 815.0 / 831.9 | 17.6 / 133.2 / 138.5 | 0.18 | WIN |
| rsvp_union | 954.8 / 1098.7 / 1518.9 | 18446.8 / 19233.7 / 19948.4 | 0.05 | WIN |
| conflict_pairs | 31.6 / 93.2 / 97.3 | 2834.2 / 379753.2 / 405966.0 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.7 | 23.6 / 51.0 / 58.5 | 0.02 | WIN |
| free_busy | 4.2 / 46.5 / 59.1 | 301.4 / 2451.5 / 2527.5 | 0.01 | WIN |
| claim_hours | 457.6 / 724.3 / 1190.5 | 6534.3 / 6843.1 / 7016.8 | 0.07 | WIN |
| slot_scan | 30.5 / 36.4 / 58.2 | 2833.0 / 3001.9 / 3054.0 | 0.01 | report |
| slot_booking_overlap | 31.9 / 60.1 / 66.4 | 727.5 / 14790.3 / 14928.5 | 0.04 | report |
| closure_depth | 8.0 / 1162.2 / 1269.5 | 89.2 / 1905.6 / 1968.9 | 0.09 | report |
| closure_fanout | 1.7 / 158.8 / 180.5 | 46.2 / 2069.6 / 2114.9 | 0.04 | report |
| disp_probe | 159513.5 / 186462.4 / 186462.4 | 885629.2 / 952675.3 / 952675.3 | 0.18 | report |
| disp_probe_d24 | 147516.9 / 160944.4 / 160944.4 | 698400.9 / 852449.8 / 852449.8 | 0.21 | report |
| disp_probe_d96 | 93244.0 / 122249.6 / 122249.6 | 639652.5 / 661799.7 / 661799.7 | 0.15 | report |
| disp_stream | 131.6 / 147.0 / 147.0 | 39183.2 / 39562.4 / 39562.4 | 0.00 | report |
| disp_stream_d24 | 142.8 / 146.7 / 146.7 | 40498.8 / 42164.7 / 42164.7 | 0.00 | report |
| disp_stream_d96 | 158.4 / 212.0 / 212.0 | 40365.4 / 40835.2 / 40835.2 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 45.8 | 34.2 | - |
| commit_batch | 5613.6 | 6308.0 | - |
| cold_containment_walk | 1144.7 | 114.0 | - |
| cold_containment_walk_delete | 3847.9 | 117.3 | - |
| commit_witnessed | 53.2 | - | - |
| commit_window_baseline | 28.4 | - | - |
| commit_window_admission | 38.3 | - | - |
| commit_window_exclusion | 35.9 | - | - |
| bulk | 795971.1 | 463914.0 | 249315 |

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
| point | 3.21 | 3.36 | clean | - |
| containment_walk | 3.41 | 3.41 | retried | - |
| chain | 3.41 | 3.41 | clean | - |
| range | 3.41 | 3.35 | clean | - |
| balance | 3.27 | 3.30 | clean | - |
| stats | 3.19 | 3.24 | CONTAMINATED | - |
| string | 3.38 | 3.41 | clean | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.25 | 3.29 | clean | - |
| triangle | 3.36 | 3.41 | clean | - |
| entries_for_account_set | 3.35 | 3.41 | clean | - |
| postings_without_tag | 3.36 | 3.34 | clean | - |
| latest_posting_per_account | 3.29 | 3.26 | clean | - |
| mandate_at_instant | 3.41 | 3.34 | clean | - |
| mandate_overlap | 3.41 | 3.41 | clean | - |
| deep_chain | 3.39 | 3.41 | clean | - |
| busy_scan | 3.41 | 3.34 | clean | - |
| meets_chain | 3.36 | 3.36 | clean | - |
| rsvp_union | 3.28 | 3.41 | clean | - |
| conflict_pairs | 3.36 | 3.35 | clean | - |
| conflict_free | 3.21 | 3.41 | retried | - |
| free_busy | 2.85 | 3.28 | CONTAMINATED | - |
| claim_hours | 3.36 | 3.26 | retried | - |
| slot_scan | 3.25 | 3.28 | clean | - |
| slot_booking_overlap | 3.21 | 3.41 | clean | - |
| closure_depth | 3.29 | 3.35 | retried | - |
| closure_fanout | 3.40 | 3.41 | clean | - |
| disp_probe | 3.33 | 3.25 | clean | - |
| disp_probe_d24 | 3.24 | 3.41 | clean | - |
| disp_probe_d96 | 2.77 | 3.21 | CONTAMINATED | - |
| disp_stream | 3.36 | 3.36 | clean | - |
| disp_stream_d24 | 3.23 | 3.41 | clean | - |
| disp_stream_d96 | 2.82 | 3.24 | CONTAMINATED | - |
| commit_single | 3.28 | 3.34 | clean | - |
| commit_batch | 3.25 | 3.35 | clean | - |
| cold_containment_walk | 3.35 | 3.41 | clean | - |
| cold_containment_walk_delete | 3.07 | 3.41 | CONTAMINATED | - |
| commit_witnessed | 3.41 | 3.36 | clean | - |
| commit_window_baseline | 3.41 | 3.28 | clean | - |
| commit_window_admission | 3.41 | 3.36 | clean | - |
| commit_window_exclusion | 3.19 | 3.38 | CONTAMINATED | - |
| bulk | 3.26 | 3.16 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
