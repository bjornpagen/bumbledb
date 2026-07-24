# bumbledb bench report

## Provenance

- crate version: 0.7.0
- engine rev: 7cef4e131049f0f45dfc37fe9e2ddeab5e6edca6
- timestamp: 2026-07-24T17:12:26Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 5.22 4.57 4.27 (start) → 5.65 5.07 4.57 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `6518394f080c2273299b55e00a8b022f88505650932a4d57fba40bd0bdf9a86b`
- verify stamp: `f1af7aff7b1dac94d67278ef6f82469fc7b7daf04d6c87aa5539fb8ab8a2511b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 6 block(s) still contaminated after retry — treat their percentiles as dirty: stats, latest_posting_per_account, deep_chain, disp_stream, commit_witnessed, commit_window_exclusion.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.3 | 1.4 / 1.7 / 2.0 | 0.18 | WIN |
| containment_walk | 6.9 / 647.0 / 701.5 | 53.5 / 30974.7 / 32093.8 | 0.13 | WIN |
| chain | 217.1 / 360.0 / 495.4 | 1833.9 / 3595.4 / 3761.9 | 0.12 | WIN |
| range | 19.9 / 20.1 / 20.2 | 140.5 / 536.5 / 550.0 | 0.14 | WIN |
| balance | 1.1 / 34.3 / 34.8 | 290.0 / 33229.7 / 34222.6 | 0.00 | WIN |
| stats | 1364.2 / 1766.0 / 2065.5 | 77865.9 / 82497.3 / 85592.1 | 0.02 | WIN |
| string | 2.5 / 2.7 / 2.7 | 58.4 / 62.6 / 92.8 | 0.04 | WIN |
| skew | 1576.4 / 2100.5 / 2325.0 | 7525.6 / 10106.0 / 10665.7 | 0.21 | WIN |
| spread | 11116.4 / 13644.7 / 24424.5 | 128717.3 / 137017.8 / 146499.5 | 0.09 | WIN |
| triangle | 2642.8 / 2859.7 / 2995.2 | 37139.8 / 40790.4 / 42218.6 | 0.07 | WIN |
| entries_for_account_set | 4.6 / 617.2 / 693.6 | 10.5 / 4113.7 / 4167.7 | 0.44 | WIN |
| postings_without_tag | 5.9 / 1182.0 / 1357.8 | 46.8 / 13342.1 / 13806.5 | 0.13 | WIN |
| latest_posting_per_account | 2300.5 / 2713.6 / 3735.2 | 42431.3 / 44338.2 / 46211.8 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.1 / 9.0 / 9.8 | 0.04 | WIN |
| mandate_overlap | 16.1 / 18.8 / 20.3 | 414.0 / 465.1 / 501.2 | 0.04 | WIN |
| deep_chain | 469.3 / 717.5 / 844.9 | 3661.6 / 6376.3 / 6528.8 | 0.13 | report |
| busy_scan | 7.8 / 8.8 / 15.8 | 3414.2 / 3631.0 / 3715.5 | 0.00 | WIN |
| meets_chain | 3.2 / 853.3 / 905.4 | 17.5 / 138.6 / 153.2 | 0.18 | WIN |
| rsvp_union | 942.5 / 1159.0 / 1273.1 | 18717.2 / 19600.1 / 20468.5 | 0.05 | WIN |
| conflict_pairs | 34.2 / 94.0 / 102.7 | 7514.8 / 386485.9 / 396379.2 | 0.00 | WIN |
| conflict_free | 0.6 / 0.6 / 0.7 | 21.9 / 50.5 / 55.9 | 0.03 | WIN |
| free_busy | 4.2 / 45.7 / 53.1 | 282.2 / 2331.8 / 2360.5 | 0.01 | WIN |
| claim_hours | 435.3 / 474.5 / 491.5 | 6340.3 / 6685.3 / 6838.3 | 0.07 | WIN |
| slot_scan | 30.3 / 36.4 / 44.0 | 2808.2 / 2913.3 / 2963.3 | 0.01 | report |
| slot_booking_overlap | 11.5 / 61.7 / 64.7 | 690.1 / 15998.5 / 16335.9 | 0.02 | report |
| closure_depth | 5.4 / 1091.6 / 1115.7 | 20.2 / 1865.3 / 1915.3 | 0.27 | report |
| closure_fanout | 1.1 / 146.7 / 154.9 | 15.2 / 2004.9 / 2031.6 | 0.07 | report |
| disp_probe | 94318.8 / 117939.9 / 117939.9 | 725478.3 / 791778.5 / 791778.5 | 0.13 | report |
| disp_probe_d24 | 90325.9 / 138150.4 / 138150.4 | 665885.7 / 767876.6 / 767876.6 | 0.14 | report |
| disp_probe_d96 | 103310.9 / 124052.6 / 124052.6 | 683369.5 / 720237.2 / 720237.2 | 0.15 | report |
| disp_stream | 135.3 / 144.7 / 144.7 | 40627.3 / 41495.8 / 41495.8 | 0.00 | report |
| disp_stream_d24 | 159.0 / 181.0 / 181.0 | 40588.9 / 41796.5 / 41796.5 | 0.00 | report |
| disp_stream_d96 | 157.2 / 181.8 / 181.8 | 40094.1 / 42733.4 / 42733.4 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 45.8 | 34.0 | - |
| commit_batch | 5689.8 | 6363.0 | - |
| cold_containment_walk | 1088.5 | 105.2 | - |
| cold_containment_walk_delete | 4141.0 | 153.0 | - |
| commit_witnessed | 63.2 | - | - |
| commit_window_baseline | 33.2 | - | - |
| commit_window_admission | 41.7 | - | - |
| commit_window_exclusion | 39.5 | - | - |
| bulk | 796464.6 | 491293.5 | 247435 |

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
| point | 3.41 | 3.21 | retried | - |
| containment_walk | 3.36 | 3.27 | retried | - |
| chain | 3.41 | 3.41 | clean | - |
| range | 3.41 | 3.41 | clean | - |
| balance | 3.36 | 3.41 | retried | - |
| stats | 3.16 | 3.25 | CONTAMINATED | - |
| string | 3.27 | 3.21 | retried | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.41 | 3.41 | clean | - |
| triangle | 3.34 | 3.34 | retried | - |
| entries_for_account_set | 3.41 | 3.23 | retried | - |
| postings_without_tag | 3.34 | 3.36 | retried | - |
| latest_posting_per_account | 3.15 | 3.41 | CONTAMINATED | - |
| mandate_at_instant | 3.36 | 3.36 | clean | - |
| mandate_overlap | 3.26 | 3.28 | clean | - |
| deep_chain | 3.32 | 3.01 | CONTAMINATED | - |
| busy_scan | 3.24 | 3.28 | retried | - |
| meets_chain | 3.36 | 3.36 | retried | - |
| rsvp_union | 3.36 | 3.36 | clean | - |
| conflict_pairs | 3.36 | 3.26 | retried | - |
| conflict_free | 3.41 | 3.41 | clean | - |
| free_busy | 3.41 | 3.35 | clean | - |
| claim_hours | 3.36 | 3.41 | retried | - |
| slot_scan | 3.41 | 3.26 | retried | - |
| slot_booking_overlap | 3.26 | 3.36 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.41 | 3.41 | clean | - |
| disp_probe | 3.36 | 3.35 | clean | - |
| disp_probe_d24 | 3.23 | 3.26 | retried | - |
| disp_probe_d96 | 3.35 | 3.34 | clean | - |
| disp_stream | 3.13 | 3.23 | CONTAMINATED | - |
| disp_stream_d24 | 3.26 | 3.36 | clean | - |
| disp_stream_d96 | 3.41 | 3.41 | clean | - |
| commit_single | 3.36 | 3.36 | clean | - |
| commit_batch | 3.36 | 3.41 | clean | - |
| cold_containment_walk | 3.34 | 3.50 | clean | - |
| cold_containment_walk_delete | 3.50 | 3.41 | clean | - |
| commit_witnessed | 3.04 | 3.15 | CONTAMINATED | - |
| commit_window_baseline | 3.41 | 3.31 | clean | - |
| commit_window_admission | 3.41 | 3.21 | clean | - |
| commit_window_exclusion | 3.18 | 3.50 | CONTAMINATED | - |
| bulk | 3.24 | 3.26 | clean | - |

## Flame summaries

(none captured — run with --trace)
