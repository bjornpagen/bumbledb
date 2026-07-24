# bumbledb bench report

## Provenance

- crate version: 0.7.0
- engine rev: 7cef4e131049f0f45dfc37fe9e2ddeab5e6edca6
- timestamp: 2026-07-24T16:54:10Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 3.17 3.54 4.06 (start) → 4.22 4.02 4.11 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `6518394f080c2273299b55e00a8b022f88505650932a4d57fba40bd0bdf9a86b`
- verify stamp: `f1af7aff7b1dac94d67278ef6f82469fc7b7daf04d6c87aa5539fb8ab8a2511b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 15 block(s) still contaminated after retry — treat their percentiles as dirty: point, postings_without_tag, mandate_at_instant, mandate_overlap, meets_chain, rsvp_union, conflict_pairs, disp_stream_d24, commit_single, commit_batch, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.7 | 1.3 / 1.6 / 2.1 | 0.19 | WIN |
| containment_walk | 15.2 / 625.1 / 646.2 | 49.4 / 28983.9 / 29402.0 | 0.31 | WIN |
| chain | 194.1 / 338.8 / 351.3 | 1802.0 / 3499.0 / 3608.8 | 0.11 | WIN |
| range | 19.8 / 20.4 / 23.7 | 140.6 / 536.8 / 550.9 | 0.14 | WIN |
| balance | 1.1 / 36.4 / 44.0 | 278.1 / 31730.2 / 31959.9 | 0.00 | WIN |
| stats | 1336.2 / 1469.3 / 1593.8 | 75364.0 / 79287.6 / 86997.0 | 0.02 | WIN |
| string | 2.5 / 2.6 / 2.6 | 58.8 / 61.2 / 67.6 | 0.04 | WIN |
| skew | 1556.2 / 2079.8 / 2203.8 | 7482.3 / 10019.8 / 10300.6 | 0.21 | WIN |
| spread | 11533.0 / 13352.3 / 16242.2 | 136968.9 / 157020.8 / 200182.7 | 0.08 | WIN |
| triangle | 2667.5 / 3084.6 / 3250.2 | 38075.5 / 41558.3 / 42854.7 | 0.07 | WIN |
| entries_for_account_set | 5.0 / 601.8 / 661.1 | 17.4 / 4261.4 / 4359.8 | 0.29 | WIN |
| postings_without_tag | 6.0 / 1132.7 / 1399.5 | 56.2 / 13803.7 / 14018.7 | 0.11 | WIN |
| latest_posting_per_account | 2336.2 / 2676.1 / 2798.3 | 43752.6 / 44492.7 / 44702.5 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.0 / 9.4 / 12.8 | 0.04 | WIN |
| mandate_overlap | 15.3 / 16.8 / 17.1 | 414.1 / 476.7 / 511.3 | 0.04 | WIN |
| deep_chain | 468.9 / 711.7 / 842.9 | 3380.7 / 6525.2 / 6702.4 | 0.14 | report |
| busy_scan | 7.9 / 9.8 / 22.7 | 3528.5 / 3703.0 / 3763.9 | 0.00 | WIN |
| meets_chain | 3.2 / 863.5 / 920.3 | 17.6 / 133.0 / 154.5 | 0.18 | WIN |
| rsvp_union | 982.8 / 1217.9 / 1338.0 | 19123.9 / 19609.8 / 23692.8 | 0.05 | WIN |
| conflict_pairs | 39.2 / 100.0 / 113.3 | 2703.8 / 373444.0 / 385722.6 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.8 | 23.6 / 47.1 / 52.4 | 0.02 | WIN |
| free_busy | 3.6 / 41.7 / 54.1 | 295.1 / 2324.3 / 2418.6 | 0.01 | WIN |
| claim_hours | 445.2 / 489.8 / 535.5 | 6442.3 / 6776.2 / 6907.1 | 0.07 | WIN |
| slot_scan | 30.9 / 33.6 / 45.2 | 2832.6 / 2932.1 / 2967.0 | 0.01 | report |
| slot_booking_overlap | 7.1 / 58.6 / 66.0 | 622.9 / 14735.9 / 14967.4 | 0.01 | report |
| closure_depth | 3.3 / 1062.2 / 1124.2 | 26.8 / 1838.2 / 1887.2 | 0.12 | report |
| closure_fanout | 1.3 / 150.5 / 164.2 | 25.6 / 1977.9 / 2030.2 | 0.05 | report |
| disp_probe | 135771.1 / 171873.6 / 171873.6 | 664105.9 / 786017.3 / 786017.3 | 0.20 | report |
| disp_probe_d24 | 93349.3 / 120048.9 / 120048.9 | 742124.0 / 803410.0 / 803410.0 | 0.13 | report |
| disp_probe_d96 | 89443.8 / 93900.3 / 93900.3 | 690656.0 / 773410.2 / 773410.2 | 0.13 | report |
| disp_stream | 132.9 / 154.2 / 154.2 | 39855.8 / 41012.7 / 41012.7 | 0.00 | report |
| disp_stream_d24 | 149.1 / 173.3 / 173.3 | 41167.9 / 42102.2 / 42102.2 | 0.00 | report |
| disp_stream_d96 | 157.8 / 196.2 / 196.2 | 40973.0 / 41791.0 / 41791.0 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 4191.9 | 4653.5 | - |
| commit_batch | 27711.0 | 12088.3 | - |
| cold_containment_walk | 1229.8 | 92.0 | - |
| cold_containment_walk_delete | 3911.2 | 100.2 | - |
| commit_witnessed | 5146.7 | - | - |
| commit_window_baseline | 4938.5 | - | - |
| commit_window_admission | 5134.9 | - | - |
| commit_window_exclusion | 5219.2 | - | - |
| bulk | 1320073.0 | 720438.7 | 149901 |

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
| point | 2.94 | 3.33 | CONTAMINATED | - |
| containment_walk | 3.44 | 3.41 | retried | - |
| chain | 3.41 | 3.41 | clean | - |
| range | 3.41 | 3.38 | clean | - |
| balance | 3.41 | 3.34 | clean | - |
| stats | 3.41 | 3.41 | clean | - |
| string | 3.41 | 3.41 | clean | - |
| skew | 3.41 | 3.34 | retried | - |
| spread | 3.36 | 3.35 | retried | - |
| triangle | 3.27 | 3.22 | retried | - |
| entries_for_account_set | 3.28 | 3.41 | retried | - |
| postings_without_tag | 2.96 | 3.41 | CONTAMINATED | - |
| latest_posting_per_account | 3.32 | 3.36 | clean | - |
| mandate_at_instant | 3.02 | 3.36 | CONTAMINATED | - |
| mandate_overlap | 3.16 | 3.36 | CONTAMINATED | - |
| deep_chain | 3.36 | 3.35 | clean | - |
| busy_scan | 3.35 | 3.34 | clean | - |
| meets_chain | 3.41 | 2.76 | CONTAMINATED | - |
| rsvp_union | 2.88 | 3.19 | CONTAMINATED | - |
| conflict_pairs | 3.02 | 3.41 | CONTAMINATED | - |
| conflict_free | 3.24 | 3.27 | retried | - |
| free_busy | 3.34 | 3.34 | clean | - |
| claim_hours | 3.23 | 3.27 | retried | - |
| slot_scan | 3.36 | 3.29 | retried | - |
| slot_booking_overlap | 3.36 | 3.35 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.28 | 3.28 | clean | - |
| disp_probe | 3.36 | 3.26 | clean | - |
| disp_probe_d24 | 3.24 | 3.25 | clean | - |
| disp_probe_d96 | 3.40 | 3.41 | clean | - |
| disp_stream | 3.30 | 3.36 | clean | - |
| disp_stream_d24 | 3.13 | 3.11 | CONTAMINATED | - |
| disp_stream_d96 | 3.36 | 3.27 | clean | - |
| commit_single | 3.28 | 1.13 | CONTAMINATED | - |
| commit_batch | 1.16 | 3.41 | CONTAMINATED | - |
| cold_containment_walk | 3.41 | 3.36 | clean | - |
| cold_containment_walk_delete | 3.36 | 3.36 | clean | - |
| commit_witnessed | 3.36 | 1.75 | CONTAMINATED | - |
| commit_window_baseline | 3.24 | 2.36 | CONTAMINATED | - |
| commit_window_admission | 2.36 | 2.00 | CONTAMINATED | - |
| commit_window_exclusion | 2.00 | 2.42 | CONTAMINATED | - |
| bulk | 1.75 | 2.89 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
