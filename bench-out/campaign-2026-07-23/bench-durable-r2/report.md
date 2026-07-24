# bumbledb bench report

## Provenance

- crate version: 0.7.0
- engine rev: 7cef4e131049f0f45dfc37fe9e2ddeab5e6edca6
- timestamp: 2026-07-24T16:59:34Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 4.22 4.02 4.11 (start) → 4.95 4.19 4.14 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `6518394f080c2273299b55e00a8b022f88505650932a4d57fba40bd0bdf9a86b`
- verify stamp: `f1af7aff7b1dac94d67278ef6f82469fc7b7daf04d6c87aa5539fb8ab8a2511b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 16 block(s) still contaminated after retry — treat their percentiles as dirty: containment_walk, range, balance, stats, spread, postings_without_tag, latest_posting_per_account, mandate_at_instant, conflict_free, commit_single, commit_batch, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.2 / 0.3 / 0.3 | 1.4 / 1.7 / 2.3 | 0.18 | WIN |
| containment_walk | 6.8 / 703.9 / 892.5 | 61.8 / 31452.7 / 31913.9 | 0.11 | WIN |
| chain | 243.3 / 355.7 / 399.3 | 2111.5 / 3712.4 / 3922.1 | 0.12 | WIN |
| range | 20.2 / 26.2 / 39.6 | 143.9 / 575.9 / 615.8 | 0.14 | WIN |
| balance | 1.1 / 33.0 / 37.3 | 295.0 / 33929.5 / 34282.2 | 0.00 | WIN |
| stats | 1399.5 / 1824.2 / 1961.3 | 80150.7 / 83020.0 / 94398.0 | 0.02 | WIN |
| string | 2.5 / 2.7 / 6.9 | 59.7 / 72.3 / 86.1 | 0.04 | WIN |
| skew | 1759.2 / 2415.5 / 2842.0 | 7753.2 / 10271.8 / 10758.5 | 0.23 | WIN |
| spread | 12419.0 / 13514.6 / 14164.7 | 129056.2 / 134228.1 / 136993.0 | 0.10 | WIN |
| triangle | 2653.0 / 2868.1 / 3093.3 | 37410.8 / 41054.0 / 42176.9 | 0.07 | WIN |
| entries_for_account_set | 5.2 / 570.3 / 608.9 | 7.3 / 4087.0 / 4148.2 | 0.70 | WIN |
| postings_without_tag | 3.7 / 1025.4 / 1063.5 | 43.9 / 13111.6 / 13619.2 | 0.08 | WIN |
| latest_posting_per_account | 2259.1 / 2417.1 / 2777.5 | 42349.2 / 43864.1 / 45764.9 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.4 / 0.6 | 7.9 / 8.8 / 9.2 | 0.04 | WIN |
| mandate_overlap | 15.5 / 16.9 / 26.7 | 417.4 / 470.1 / 505.1 | 0.04 | WIN |
| deep_chain | 385.8 / 630.0 / 643.5 | 4227.8 / 6241.9 / 6323.0 | 0.09 | report |
| busy_scan | 7.7 / 8.7 / 8.8 | 3401.9 / 3550.6 / 3674.5 | 0.00 | WIN |
| meets_chain | 3.1 / 847.5 / 910.9 | 17.3 / 132.8 / 145.5 | 0.18 | WIN |
| rsvp_union | 948.9 / 1053.5 / 1148.4 | 18385.9 / 19104.4 / 19544.9 | 0.05 | WIN |
| conflict_pairs | 29.0 / 90.7 / 94.0 | 2945.0 / 392908.5 / 399770.4 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.6 | 23.7 / 50.5 / 57.1 | 0.02 | WIN |
| free_busy | 4.2 / 44.6 / 51.2 | 301.8 / 2410.3 / 2547.2 | 0.01 | WIN |
| claim_hours | 436.8 / 468.9 / 522.7 | 6575.9 / 6908.4 / 7098.7 | 0.07 | WIN |
| slot_scan | 30.9 / 40.1 / 48.7 | 2842.8 / 3026.2 / 3158.5 | 0.01 | report |
| slot_booking_overlap | 15.7 / 60.1 / 71.3 | 743.0 / 16245.4 / 16495.0 | 0.02 | report |
| closure_depth | 9.2 / 1152.8 / 1206.2 | 27.2 / 1889.2 / 2046.0 | 0.34 | report |
| closure_fanout | 1.4 / 153.5 / 163.0 | 29.4 / 2053.2 / 2207.9 | 0.05 | report |
| disp_probe | 147983.1 / 155376.5 / 155376.5 | 697104.2 / 906714.8 / 906714.8 | 0.21 | report |
| disp_probe_d24 | 87121.9 / 110982.2 / 110982.2 | 652635.4 / 738514.5 / 738514.5 | 0.13 | report |
| disp_probe_d96 | 90494.3 / 94897.6 / 94897.6 | 629402.4 / 645063.7 / 645063.7 | 0.14 | report |
| disp_stream | 132.0 / 143.9 / 143.9 | 40279.3 / 41926.1 / 41926.1 | 0.00 | report |
| disp_stream_d24 | 150.8 / 156.8 / 156.8 | 39651.7 / 41692.8 / 41692.8 | 0.00 | report |
| disp_stream_d96 | 156.4 / 210.5 / 210.5 | 40173.2 / 40392.2 / 40392.2 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 5110.2 | 5155.5 | - |
| commit_batch | 24935.3 | 12940.6 | - |
| cold_containment_walk | 1196.1 | 86.5 | - |
| cold_containment_walk_delete | 3657.0 | 87.2 | - |
| commit_witnessed | 4536.5 | - | - |
| commit_window_baseline | 4439.3 | - | - |
| commit_window_admission | 5128.4 | - | - |
| commit_window_exclusion | 5159.6 | - | - |
| bulk | 1216505.8 | 683191.8 | 163576 |

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
| point | 3.41 | 3.23 | clean | - |
| containment_walk | 2.93 | 3.24 | CONTAMINATED | - |
| chain | 3.34 | 3.36 | clean | - |
| range | 3.09 | 3.34 | CONTAMINATED | - |
| balance | 3.15 | 3.35 | CONTAMINATED | - |
| stats | 3.36 | 3.07 | CONTAMINATED | - |
| string | 3.41 | 3.30 | clean | - |
| skew | 3.23 | 3.22 | clean | - |
| spread | 0.73 | 3.29 | CONTAMINATED | - |
| triangle | 3.36 | 3.26 | clean | - |
| entries_for_account_set | 3.23 | 3.41 | clean | - |
| postings_without_tag | 3.22 | 0.66 | CONTAMINATED | - |
| latest_posting_per_account | 2.91 | 3.36 | CONTAMINATED | - |
| mandate_at_instant | 2.99 | 3.16 | CONTAMINATED | - |
| mandate_overlap | 3.41 | 3.35 | clean | - |
| deep_chain | 3.41 | 3.35 | clean | - |
| busy_scan | 3.26 | 3.41 | retried | - |
| meets_chain | 3.41 | 3.30 | clean | - |
| rsvp_union | 3.41 | 3.21 | clean | - |
| conflict_pairs | 3.41 | 3.33 | clean | - |
| conflict_free | 3.16 | 2.93 | CONTAMINATED | - |
| free_busy | 3.23 | 3.36 | clean | - |
| claim_hours | 3.34 | 3.21 | retried | - |
| slot_scan | 3.26 | 3.41 | retried | - |
| slot_booking_overlap | 3.26 | 3.32 | retried | - |
| closure_depth | 3.27 | 3.28 | retried | - |
| closure_fanout | 3.36 | 3.26 | clean | - |
| disp_probe | 3.34 | 3.41 | clean | - |
| disp_probe_d24 | 3.36 | 3.36 | clean | - |
| disp_probe_d96 | 3.35 | 3.41 | clean | - |
| disp_stream | 3.41 | 3.41 | clean | - |
| disp_stream_d24 | 3.41 | 3.41 | clean | - |
| disp_stream_d96 | 3.41 | 3.22 | clean | - |
| commit_single | 3.50 | 1.75 | CONTAMINATED | - |
| commit_batch | 1.75 | 3.50 | CONTAMINATED | - |
| cold_containment_walk | 3.50 | 3.36 | clean | - |
| cold_containment_walk_delete | 3.36 | 3.36 | clean | - |
| commit_witnessed | 3.36 | 0.91 | CONTAMINATED | - |
| commit_window_baseline | 3.23 | 1.71 | CONTAMINATED | - |
| commit_window_admission | 1.59 | 1.68 | CONTAMINATED | - |
| commit_window_exclusion | 1.67 | 1.75 | CONTAMINATED | - |
| bulk | 1.85 | 3.51 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
