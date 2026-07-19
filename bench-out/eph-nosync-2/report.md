# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: 738ed5e46e2a0ed2309914d31b86b2358c6d1f0d
- timestamp: 2026-07-19T21:04:53Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `e597c9f7288b432630ea7c580dab308ef7b4059336ab1b5604aff07e0894c38b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 6 block(s) still contaminated after retry — treat their percentiles as dirty: range, commit_single, commit_batch, cold_containment_walk, commit_window_baseline, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.7 / 0.8 | 1.4 / 1.8 / 1.9 | 0.35 | WIN |
| containment_walk | 4.5 / 595.0 / 624.8 | 51.7 / 30075.0 / 30341.1 | 0.09 | WIN |
| chain | 57.6 / 93.5 / 104.2 | 560.3 / 1012.8 / 1041.6 | 0.10 | WIN |
| range | 19.3 / 23.3 / 24.5 | 141.0 / 548.7 / 559.7 | 0.14 | WIN |
| balance | 1.0 / 33.3 / 38.0 | 283.1 / 32832.3 / 33661.1 | 0.00 | WIN |
| stats | 1358.5 / 1428.7 / 1579.3 | 74825.1 / 78657.8 / 81404.2 | 0.02 | WIN |
| string | 2.2 / 2.4 / 2.5 | 56.0 / 61.0 / 66.9 | 0.04 | WIN |
| skew | 1543.0 / 2070.2 / 2200.2 | 7352.2 / 9756.8 / 10235.5 | 0.21 | WIN |
| spread | 10657.7 / 12568.4 / 13205.3 | 125729.8 / 128425.3 / 129566.9 | 0.08 | WIN |
| triangle | 9911.6 / 11539.0 / 12462.6 | 37710.0 / 57165.6 / 57820.1 | 0.26 | WIN |
| entries_for_account_set | 1.3 / 506.2 / 543.1 | 11.1 / 4087.8 / 4177.6 | 0.12 | WIN |
| postings_without_tag | 4.7 / 1057.8 / 1252.1 | 46.0 / 12762.1 / 12939.2 | 0.10 | WIN |
| latest_posting_per_account | 2064.7 / 2116.5 / 2277.5 | 40876.8 / 42278.0 / 42982.9 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.0 / 8.1 / 8.3 | 0.03 | WIN |
| mandate_overlap | 8.2 / 12.6 / 13.1 | 203.2 / 310.5 / 325.0 | 0.04 | WIN |
| busy_scan | 7.4 / 9.1 / 10.5 | 3388.0 / 3426.1 / 3452.8 | 0.00 | WIN |
| meets_chain | 3.4 / 1364.4 / 1398.2 | 17.3 / 130.1 / 138.5 | 0.20 | WIN |
| rsvp_union | 863.3 / 927.2 / 981.1 | 17940.3 / 18352.9 / 18692.3 | 0.05 | WIN |
| conflict_pairs | 25.5 / 123.5 / 127.3 | 2692.8 / 366573.5 / 367515.3 | 0.01 | WIN |
| conflict_free | 0.5 / 0.6 / 0.6 | 15.0 / 46.8 / 47.2 | 0.04 | WIN |
| free_busy | 2.7 / 40.2 / 40.3 | 282.1 / 2265.3 / 2332.8 | 0.01 | WIN |
| claim_hours | 508.5 / 520.7 / 533.2 | 6279.9 / 6520.1 / 6686.3 | 0.08 | WIN |
| slot_scan | 28.8 / 29.6 / 35.3 | 2768.8 / 2823.4 / 2901.7 | 0.01 | report |
| slot_booking_overlap | 19.6 / 571.0 / 594.0 | 648.1 / 14565.9 / 14743.1 | 0.03 | report |
| closure_depth | 6.7 / 978.3 / 1021.7 | 16.5 / 1782.5 / 1839.5 | 0.40 | report |
| closure_fanout | 1.0 / 137.9 / 138.1 | 11.2 / 1930.0 / 1986.0 | 0.09 | report |
| disp_probe | 115735.8 / 135289.2 / 135289.2 | 643359.0 / 662755.2 / 662755.2 | 0.18 | report |
| disp_probe_d24 | 119985.6 / 141876.2 / 141876.2 | 640090.6 / 677378.9 / 677378.9 | 0.19 | report |
| disp_probe_d96 | 119438.8 / 123589.9 / 123589.9 | 634971.9 / 670583.1 / 670583.1 | 0.19 | report |
| disp_stream | 132.0 / 138.5 / 138.5 | 39323.4 / 41879.0 / 41879.0 | 0.00 | report |
| disp_stream_d24 | 142.9 / 148.3 / 148.3 | 39326.0 / 39655.3 / 39655.3 | 0.00 | report |
| disp_stream_d96 | 157.7 / 168.4 / 168.4 | 39748.6 / 39936.1 / 39936.1 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 51.2 | 4588.1 | - |
| commit_batch | 5556.3 | 28330.8 | - |
| cold_containment_walk | 3827.6 | 74.8 | - |
| commit_witnessed | 56.8 | - | - |
| commit_window_baseline | 28.1 | - | - |
| commit_window_admission | 36.8 | - | - |
| commit_window_exclusion | 35.0 | - | - |
| bulk | 789667.8 | 925375.2 | 253354 |

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
| point | 3.24 | 3.24 | retried | - |
| containment_walk | 3.24 | 3.26 | retried | - |
| chain | 3.24 | 3.26 | clean | - |
| range | 3.27 | 3.11 | CONTAMINATED | - |
| balance | 3.36 | 3.36 | retried | - |
| stats | 3.24 | 3.27 | clean | - |
| string | 3.41 | 3.41 | clean | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.41 | 3.41 | clean | - |
| triangle | 3.41 | 3.41 | retried | - |
| entries_for_account_set | 3.38 | 3.41 | clean | - |
| postings_without_tag | 3.41 | 3.38 | clean | - |
| latest_posting_per_account | 3.41 | 3.41 | clean | - |
| mandate_at_instant | 3.31 | 3.41 | retried | - |
| mandate_overlap | 3.41 | 3.39 | clean | - |
| busy_scan | 3.41 | 3.41 | clean | - |
| meets_chain | 3.41 | 3.41 | clean | - |
| rsvp_union | 3.40 | 3.41 | clean | - |
| conflict_pairs | 3.35 | 3.41 | clean | - |
| conflict_free | 3.41 | 3.41 | clean | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 3.38 | 3.41 | clean | - |
| slot_scan | 3.41 | 3.41 | clean | - |
| slot_booking_overlap | 3.41 | 3.41 | clean | - |
| closure_depth | 3.38 | 3.36 | retried | - |
| closure_fanout | 3.41 | 3.41 | clean | - |
| disp_probe | 3.41 | 3.41 | clean | - |
| disp_probe_d24 | 3.41 | 3.41 | clean | - |
| disp_probe_d96 | 3.36 | 3.41 | clean | - |
| disp_stream | 3.41 | 3.36 | clean | - |
| disp_stream_d24 | 3.41 | 3.35 | clean | - |
| disp_stream_d96 | 3.41 | 3.41 | clean | - |
| commit_single | 3.26 | 1.74 | CONTAMINATED | - |
| commit_batch | 1.75 | 3.00 | CONTAMINATED | - |
| cold_containment_walk | 2.91 | 3.26 | CONTAMINATED | - |
| commit_witnessed | 3.26 | 3.26 | clean | - |
| commit_window_baseline | 3.15 | 3.41 | CONTAMINATED | - |
| commit_window_admission | 3.31 | 3.41 | clean | - |
| commit_window_exclusion | 3.31 | 3.41 | clean | - |
| bulk | 3.17 | 2.38 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
