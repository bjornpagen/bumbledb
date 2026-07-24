# bumbledb bench report

## Provenance

- crate version: 0.7.0
- engine rev: 7cef4e131049f0f45dfc37fe9e2ddeab5e6edca6
- timestamp: 2026-07-24T17:07:44Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 3.53 3.79 3.96 (start) → 5.22 4.57 4.27 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `6518394f080c2273299b55e00a8b022f88505650932a4d57fba40bd0bdf9a86b`
- verify stamp: `f1af7aff7b1dac94d67278ef6f82469fc7b7daf04d6c87aa5539fb8ab8a2511b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 7 block(s) still contaminated after retry — treat their percentiles as dirty: point, postings_without_tag, latest_posting_per_account, busy_scan, slot_scan, disp_stream_d24, commit_single.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.5 | 1.4 / 1.6 / 1.8 | 0.19 | WIN |
| containment_walk | 12.1 / 653.0 / 721.0 | 52.6 / 29615.1 / 30072.8 | 0.23 | WIN |
| chain | 200.1 / 345.0 / 362.4 | 1857.1 / 3497.1 / 3773.5 | 0.11 | WIN |
| range | 19.9 / 24.5 / 33.3 | 143.6 / 551.7 / 591.2 | 0.14 | WIN |
| balance | 1.1 / 33.2 / 33.5 | 278.1 / 32235.6 / 33282.9 | 0.00 | WIN |
| stats | 1343.1 / 1415.7 / 1632.3 | 74908.6 / 77638.7 / 78806.6 | 0.02 | WIN |
| string | 2.5 / 2.6 / 2.7 | 57.8 / 59.6 / 65.0 | 0.04 | WIN |
| skew | 1493.6 / 2011.0 / 2229.5 | 7402.1 / 9817.7 / 10081.9 | 0.20 | WIN |
| spread | 10470.0 / 11786.8 / 12252.4 | 127194.2 / 133960.6 / 137492.3 | 0.08 | WIN |
| triangle | 2623.4 / 2741.0 / 3017.5 | 36951.7 / 40451.6 / 41728.2 | 0.07 | WIN |
| entries_for_account_set | 1.3 / 581.4 / 601.9 | 9.2 / 4098.3 / 4226.4 | 0.15 | WIN |
| postings_without_tag | 6.7 / 1060.1 / 1093.3 | 46.2 / 13573.5 / 14459.0 | 0.14 | WIN |
| latest_posting_per_account | 2301.4 / 2965.9 / 5022.7 | 42378.6 / 43801.2 / 44855.2 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 7.9 / 8.8 / 9.3 | 0.04 | WIN |
| mandate_overlap | 15.6 / 17.1 / 20.0 | 416.2 / 462.5 / 493.2 | 0.04 | WIN |
| deep_chain | 460.4 / 646.1 / 682.0 | 3380.0 / 6258.2 / 6347.2 | 0.14 | report |
| busy_scan | 7.6 / 10.6 / 17.1 | 3428.2 / 3660.0 / 5425.1 | 0.00 | WIN |
| meets_chain | 3.2 / 861.9 / 922.4 | 17.4 / 138.7 / 147.4 | 0.18 | WIN |
| rsvp_union | 960.5 / 1324.8 / 1489.7 | 18655.3 / 19406.0 / 19669.8 | 0.05 | WIN |
| conflict_pairs | 24.4 / 92.0 / 92.7 | 3752.9 / 391161.7 / 399182.1 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.7 | 19.3 / 47.5 / 51.2 | 0.03 | WIN |
| free_busy | 3.5 / 41.6 / 43.4 | 302.1 / 2340.2 / 2389.9 | 0.01 | WIN |
| claim_hours | 440.6 / 446.7 / 452.2 | 6353.1 / 6633.8 / 6774.5 | 0.07 | WIN |
| slot_scan | 30.6 / 38.5 / 41.9 | 2768.7 / 2870.0 / 2984.0 | 0.01 | report |
| slot_booking_overlap | 8.3 / 62.8 / 69.7 | 640.6 / 15860.4 / 16628.4 | 0.01 | report |
| closure_depth | 8.6 / 1113.3 / 1235.0 | 70.0 / 1859.8 / 1944.5 | 0.12 | report |
| closure_fanout | 1.4 / 155.8 / 162.2 | 30.8 / 2043.6 / 2135.8 | 0.04 | report |
| disp_probe | 119113.1 / 136664.9 / 136664.9 | 720975.0 / 838085.7 / 838085.7 | 0.17 | report |
| disp_probe_d24 | 100486.6 / 132200.0 / 132200.0 | 676943.2 / 870145.4 / 870145.4 | 0.15 | report |
| disp_probe_d96 | 140699.0 / 155598.9 / 155598.9 | 749248.7 / 1163236.7 / 1163236.7 | 0.19 | report |
| disp_stream | 180.0 / 296.2 / 296.2 | 41331.8 / 47352.5 / 47352.5 | 0.00 | report |
| disp_stream_d24 | 158.1 / 166.4 / 166.4 | 51811.7 / 60671.1 / 60671.1 | 0.00 | report |
| disp_stream_d96 | 161.8 / 169.3 / 169.3 | 40672.5 / 53589.5 / 53589.5 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 45.8 | 34.0 | - |
| commit_batch | 5620.3 | 6448.1 | - |
| cold_containment_walk | 1141.1 | 122.8 | - |
| cold_containment_walk_delete | 4090.3 | 108.0 | - |
| commit_witnessed | 51.3 | - | - |
| commit_window_baseline | 33.8 | - | - |
| commit_window_admission | 38.3 | - | - |
| commit_window_exclusion | 33.8 | - | - |
| bulk | 833791.2 | 470911.9 | 237784 |

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
| point | 3.18 | 3.25 | CONTAMINATED | - |
| containment_walk | 3.36 | 3.26 | clean | - |
| chain | 3.39 | 3.34 | clean | - |
| range | 3.27 | 3.34 | clean | - |
| balance | 3.36 | 3.27 | retried | - |
| stats | 3.41 | 3.29 | clean | - |
| string | 3.41 | 3.41 | clean | - |
| skew | 3.41 | 3.27 | clean | - |
| spread | 3.33 | 3.41 | clean | - |
| triangle | 3.41 | 3.35 | clean | - |
| entries_for_account_set | 3.41 | 3.28 | clean | - |
| postings_without_tag | 3.25 | 3.02 | CONTAMINATED | - |
| latest_posting_per_account | 3.04 | 3.17 | CONTAMINATED | - |
| mandate_at_instant | 3.41 | 3.41 | clean | - |
| mandate_overlap | 3.41 | 3.41 | clean | - |
| deep_chain | 3.22 | 3.35 | clean | - |
| busy_scan | 3.27 | 3.14 | CONTAMINATED | - |
| meets_chain | 3.41 | 3.24 | clean | - |
| rsvp_union | 3.27 | 3.41 | retried | - |
| conflict_pairs | 3.36 | 3.26 | clean | - |
| conflict_free | 3.21 | 3.41 | clean | - |
| free_busy | 3.40 | 3.36 | clean | - |
| claim_hours | 3.31 | 3.41 | clean | - |
| slot_scan | 2.82 | 3.10 | CONTAMINATED | - |
| slot_booking_overlap | 3.34 | 3.34 | retried | - |
| closure_depth | 3.36 | 3.29 | clean | - |
| closure_fanout | 3.41 | 3.41 | clean | - |
| disp_probe | 3.40 | 3.41 | clean | - |
| disp_probe_d24 | 3.41 | 3.41 | clean | - |
| disp_probe_d96 | 3.29 | 3.27 | clean | - |
| disp_stream | 3.36 | 3.31 | clean | - |
| disp_stream_d24 | 2.42 | 3.12 | CONTAMINATED | - |
| disp_stream_d96 | 3.27 | 3.41 | retried | - |
| commit_single | 3.12 | 3.41 | CONTAMINATED | - |
| commit_batch | 3.41 | 3.41 | clean | - |
| cold_containment_walk | 3.41 | 3.36 | clean | - |
| cold_containment_walk_delete | 3.25 | 3.51 | clean | - |
| commit_witnessed | 3.50 | 3.50 | clean | - |
| commit_window_baseline | 3.41 | 3.41 | clean | - |
| commit_window_admission | 3.41 | 3.41 | clean | - |
| commit_window_exclusion | 3.41 | 3.41 | clean | - |
| bulk | 3.36 | 3.50 | clean | - |

## Flame summaries

(none captured — run with --trace)
