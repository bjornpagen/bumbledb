# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: 738ed5e46e2a0ed2309914d31b86b2358c6d1f0d
- timestamp: 2026-07-19T21:09:15Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `e597c9f7288b432630ea7c580dab308ef7b4059336ab1b5604aff07e0894c38b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 7 block(s) still contaminated after retry — treat their percentiles as dirty: chain, string, skew, commit_single, commit_batch, cold_containment_walk, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.4 / 0.4 / 0.4 | 1.4 / 1.5 / 1.6 | 0.30 | WIN |
| containment_walk | 2.0 / 556.5 / 574.7 | 46.4 / 28694.4 / 29357.7 | 0.04 | WIN |
| chain | 49.7 / 89.4 / 89.6 | 547.8 / 947.0 / 962.9 | 0.09 | WIN |
| range | 18.9 / 19.1 / 19.1 | 139.5 / 529.2 / 544.5 | 0.14 | WIN |
| balance | 1.0 / 33.1 / 33.1 | 277.7 / 31370.0 / 32051.0 | 0.00 | WIN |
| stats | 1248.3 / 1284.7 / 1351.8 | 74362.9 / 76973.8 / 79326.1 | 0.02 | WIN |
| string | 2.3 / 3.0 / 3.2 | 57.3 / 68.1 / 74.6 | 0.04 | WIN |
| skew | 1653.8 / 2186.1 / 2359.9 | 7816.2 / 10431.1 / 10617.0 | 0.21 | WIN |
| spread | 11477.2 / 12740.6 / 13448.0 | 125926.6 / 128514.1 / 130304.2 | 0.09 | WIN |
| triangle | 9878.5 / 11089.0 / 11739.8 | 37641.0 / 56913.6 / 57462.5 | 0.26 | WIN |
| entries_for_account_set | 1.3 / 498.3 / 536.3 | 6.8 / 3955.9 / 3983.8 | 0.19 | WIN |
| postings_without_tag | 2.7 / 1042.9 / 1061.0 | 47.1 / 12889.2 / 13251.3 | 0.06 | WIN |
| latest_posting_per_account | 2070.9 / 2107.9 / 2156.0 | 40921.5 / 41983.6 / 43336.7 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.2 / 8.7 / 9.0 | 0.03 | WIN |
| mandate_overlap | 8.1 / 12.6 / 12.7 | 206.4 / 312.2 / 328.0 | 0.04 | WIN |
| busy_scan | 7.2 / 8.5 / 8.5 | 3383.7 / 3438.8 / 3473.6 | 0.00 | WIN |
| meets_chain | 3.4 / 1353.0 / 1373.0 | 17.8 / 129.8 / 133.7 | 0.19 | WIN |
| rsvp_union | 861.9 / 930.3 / 1097.7 | 17882.4 / 18372.8 / 18690.0 | 0.05 | WIN |
| conflict_pairs | 21.1 / 122.4 / 122.7 | 6644.5 / 370078.9 / 423401.8 | 0.00 | WIN |
| conflict_free | 0.6 / 0.6 / 0.7 | 15.7 / 47.2 / 47.5 | 0.04 | WIN |
| free_busy | 2.7 / 40.0 / 40.2 | 267.8 / 2265.9 / 2308.7 | 0.01 | WIN |
| claim_hours | 508.2 / 518.1 / 538.0 | 6289.8 / 6529.5 / 6710.9 | 0.08 | WIN |
| slot_scan | 28.6 / 29.3 / 29.4 | 2777.5 / 2843.3 / 2867.7 | 0.01 | report |
| slot_booking_overlap | 24.1 / 565.3 / 582.6 | 609.2 / 14545.7 / 14616.6 | 0.04 | report |
| closure_depth | 5.7 / 982.2 / 1049.5 | 16.2 / 1793.9 / 1844.4 | 0.35 | report |
| closure_fanout | 1.1 / 137.6 / 139.4 | 10.6 / 1937.0 / 1968.5 | 0.10 | report |
| disp_probe | 112588.4 / 125323.7 / 125323.7 | 632894.8 / 658612.2 / 658612.2 | 0.18 | report |
| disp_probe_d24 | 117746.4 / 131875.8 / 131875.8 | 639030.2 / 662896.3 / 662896.3 | 0.18 | report |
| disp_probe_d96 | 115883.1 / 120397.3 / 120397.3 | 628151.5 / 648993.9 / 648993.9 | 0.18 | report |
| disp_stream | 134.8 / 140.8 / 140.8 | 39868.5 / 42285.5 / 42285.5 | 0.00 | report |
| disp_stream_d24 | 150.2 / 177.2 / 177.2 | 40245.8 / 40535.5 / 40535.5 | 0.00 | report |
| disp_stream_d96 | 157.8 / 163.2 / 163.2 | 40351.3 / 40678.1 / 40678.1 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 51.8 | 4691.8 | - |
| commit_batch | 5423.9 | 32584.6 | - |
| cold_containment_walk | 3940.2 | 75.4 | - |
| commit_witnessed | 57.7 | - | - |
| commit_window_baseline | 29.0 | - | - |
| commit_window_admission | 39.5 | - | - |
| commit_window_exclusion | 36.8 | - | - |
| bulk | 777972.5 | 980068.0 | 256603 |

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

- bumbledb file (compacted): 77955072 bytes
- sqlite file: 18464768 bytes
- image cache: 0 images, 0 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| point | 3.36 | 3.41 | clean | - |
| containment_walk | 3.40 | 3.40 | clean | - |
| chain | 3.12 | 3.41 | CONTAMINATED | - |
| range | 3.41 | 3.41 | clean | - |
| balance | 3.41 | 3.41 | clean | - |
| stats | 3.41 | 3.35 | clean | - |
| string | 3.18 | 3.12 | CONTAMINATED | - |
| skew | 2.94 | 3.27 | CONTAMINATED | - |
| spread | 3.30 | 3.23 | retried | - |
| triangle | 3.35 | 3.41 | clean | - |
| entries_for_account_set | 3.41 | 3.40 | clean | - |
| postings_without_tag | 3.41 | 3.41 | clean | - |
| latest_posting_per_account | 3.38 | 3.38 | clean | - |
| mandate_at_instant | 3.31 | 3.41 | clean | - |
| mandate_overlap | 3.36 | 3.41 | retried | - |
| busy_scan | 3.41 | 3.36 | clean | - |
| meets_chain | 3.41 | 3.41 | clean | - |
| rsvp_union | 3.41 | 3.41 | clean | - |
| conflict_pairs | 3.41 | 3.41 | clean | - |
| conflict_free | 3.41 | 3.41 | clean | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 3.41 | 3.41 | clean | - |
| slot_scan | 3.41 | 3.38 | clean | - |
| slot_booking_overlap | 3.41 | 3.40 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.41 | 3.41 | clean | - |
| disp_probe | 3.41 | 3.41 | clean | - |
| disp_probe_d24 | 3.41 | 3.41 | clean | - |
| disp_probe_d96 | 3.36 | 3.41 | clean | - |
| disp_stream | 3.30 | 3.21 | clean | - |
| disp_stream_d24 | 3.26 | 3.29 | clean | - |
| disp_stream_d96 | 3.27 | 3.27 | clean | - |
| commit_single | 3.26 | 1.19 | CONTAMINATED | - |
| commit_batch | 1.19 | 3.10 | CONTAMINATED | - |
| cold_containment_walk | 3.09 | 3.26 | CONTAMINATED | - |
| commit_witnessed | 3.26 | 3.26 | clean | - |
| commit_window_baseline | 3.29 | 3.41 | clean | - |
| commit_window_admission | 3.31 | 3.41 | clean | - |
| commit_window_exclusion | 3.31 | 3.41 | clean | - |
| bulk | 2.77 | 2.57 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
