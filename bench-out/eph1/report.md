# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: adac4010e85c9c82cc30d866f3918b7d0ec742d3
- timestamp: 2026-07-16T19:28:01Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `f4a9d0941bd4f5a18de60fd6c9f103147e34ddf7c92f9edfcc6c55ffa0849d29 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 8 block(s) still contaminated after retry — treat their percentiles as dirty: range, entries_for_account_set, busy_scan, meets_chain, disp_stream, commit_single, commit_batch, cold_containment_walk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.6 / 0.6 | 1.5 / 1.5 / 1.5 | 0.34 | WIN |
| containment_walk | 10.2 / 782.7 / 854.9 | 52.3 / 29846.4 / 30702.1 | 0.20 | WIN |
| chain | 59.7 / 93.6 / 97.5 | 593.0 / 989.2 / 1030.7 | 0.10 | WIN |
| range | 23.3 / 30.3 / 56.0 | 140.4 / 538.7 / 555.3 | 0.17 | WIN |
| balance | 1.0 / 33.3 / 33.4 | 285.3 / 32317.3 / 32820.4 | 0.00 | WIN |
| stats | 1296.4 / 1776.5 / 2117.5 | 75871.1 / 82321.9 / 85642.9 | 0.02 | WIN |
| string | 2.6 / 3.0 / 3.0 | 56.4 / 66.5 / 96.3 | 0.05 | WIN |
| skew | 1597.7 / 2133.2 / 2198.1 | 7409.9 / 9927.9 / 10765.6 | 0.22 | WIN |
| spread | 11715.3 / 15168.3 / 16557.7 | 127847.5 / 132277.5 / 137700.1 | 0.09 | WIN |
| triangle | 11425.7 / 15642.2 / 18721.9 | 38609.0 / 58134.5 / 59427.2 | 0.30 | WIN |
| entries_for_account_set | 7.4 / 606.4 / 1186.7 | 111.5 / 4634.1 / 7120.1 | 0.07 | WIN |
| postings_without_tag | 12.0 / 1408.4 / 5502.0 | 67.9 / 13696.4 / 13938.4 | 0.18 | WIN |
| latest_posting_per_account | 2134.2 / 2284.4 / 2596.6 | 43184.6 / 67719.1 / 88189.5 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.2 / 9.4 / 10.8 | 0.03 | WIN |
| mandate_overlap | 8.5 / 13.8 / 13.8 | 207.3 / 333.8 / 355.8 | 0.04 | WIN |
| busy_scan | 8.5 / 10.0 / 10.7 | 3482.7 / 3700.0 / 9232.3 | 0.00 | WIN |
| meets_chain | 3.6 / 1401.8 / 1444.3 | 18.1 / 134.9 / 142.8 | 0.20 | WIN |
| rsvp_union | 991.0 / 1221.8 / 1347.4 | 18784.9 / 21347.8 / 26612.0 | 0.05 | WIN |
| conflict_pairs | 30.8 / 136.4 / 145.5 | 2807.2 / 389834.8 / 509647.8 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.6 | 22.2 / 50.0 / 56.4 | 0.03 | WIN |
| free_busy | 2.6 / 38.2 / 38.3 | 273.5 / 2309.1 / 2361.6 | 0.01 | WIN |
| claim_hours | 512.5 / 555.3 / 588.7 | 6366.9 / 6635.0 / 6776.1 | 0.08 | WIN |
| slot_scan | 31.2 / 35.6 / 43.7 | 2813.6 / 2923.5 / 2973.3 | 0.01 | report |
| slot_booking_overlap | 27.0 / 568.4 / 585.9 | 674.3 / 14762.1 / 14869.8 | 0.04 | report |
| closure_depth | 15.9 / 14525.5 / 19881.2 | 26.1 / 1846.9 / 1902.0 | 0.61 | report |
| closure_fanout | 1.2 / 153.6 / 170.7 | 14.8 / 2045.0 / 2086.1 | 0.08 | report |
| disp_probe | 179095.7 / 187128.1 / 187128.1 | 814410.7 / 914912.5 / 914912.5 | 0.22 | report |
| disp_probe_d24 | 157573.9 / 165437.3 / 165437.3 | 721930.9 / 862010.2 / 862010.2 | 0.22 | report |
| disp_probe_d96 | 132248.4 / 142713.8 / 142713.8 | 691458.1 / 738501.7 / 738501.7 | 0.19 | report |
| disp_stream | 134.7 / 149.8 / 149.8 | 40285.2 / 43031.5 / 43031.5 | 0.00 | report |
| disp_stream_d24 | 145.1 / 169.4 / 169.4 | 40023.7 / 41720.4 / 41720.4 | 0.00 | report |
| disp_stream_d96 | 161.5 / 187.1 / 187.1 | 40386.3 / 40590.5 / 40590.5 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 25.3 | 5050.5 | - |
| commit_batch | 4505.0 | 28967.9 | - |
| cold_containment_walk | 4169.0 | 134.0 | - |
| commit_witnessed | 27.0 | - | - |
| commit_window_baseline | 11.6 | - | - |
| commit_window_admission | 20.5 | - | - |
| commit_window_exclusion | 18.0 | - | - |
| bulk | 824734.0 | 907926.8 | 241759 |

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

- bumbledb file (compacted): 4294967296 bytes
- sqlite file: 18464768 bytes
- image cache: 0 images, 0 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| point | 3.26 | 3.21 | retried | - |
| containment_walk | 3.21 | 3.36 | clean | - |
| chain | 3.36 | 3.36 | clean | - |
| range | 3.24 | 2.98 | CONTAMINATED | - |
| balance | 3.31 | 3.28 | clean | - |
| stats | 3.26 | 3.36 | retried | - |
| string | 3.36 | 3.35 | clean | - |
| skew | 3.21 | 3.36 | clean | - |
| spread | 3.36 | 3.29 | clean | - |
| triangle | 3.30 | 3.24 | retried | - |
| entries_for_account_set | 1.94 | 3.24 | CONTAMINATED | - |
| postings_without_tag | 3.26 | 3.26 | clean | - |
| latest_posting_per_account | 3.36 | 3.31 | clean | - |
| mandate_at_instant | 3.27 | 3.26 | retried | - |
| mandate_overlap | 3.34 | 3.23 | retried | - |
| busy_scan | 2.92 | 3.26 | CONTAMINATED | - |
| meets_chain | 3.10 | 3.26 | CONTAMINATED | - |
| rsvp_union | 3.26 | 3.26 | retried | - |
| conflict_pairs | 3.26 | 3.21 | retried | - |
| conflict_free | 3.28 | 3.35 | retried | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 3.29 | 3.41 | clean | - |
| slot_scan | 3.25 | 3.41 | clean | - |
| slot_booking_overlap | 3.41 | 3.41 | retried | - |
| closure_depth | 3.28 | 3.36 | retried | - |
| closure_fanout | 3.35 | 3.41 | clean | - |
| disp_probe | 3.41 | 3.32 | clean | - |
| disp_probe_d24 | 3.35 | 3.35 | clean | - |
| disp_probe_d96 | 3.41 | 3.41 | clean | - |
| disp_stream | 3.12 | 3.27 | CONTAMINATED | - |
| disp_stream_d24 | 3.36 | 3.41 | clean | - |
| disp_stream_d96 | 3.30 | 3.29 | clean | - |
| commit_single | 3.26 | 1.06 | CONTAMINATED | - |
| commit_batch | 1.19 | 3.13 | CONTAMINATED | - |
| cold_containment_walk | 3.11 | 3.26 | CONTAMINATED | - |
| commit_witnessed | 3.26 | 3.26 | clean | - |
| commit_window_baseline | 3.26 | 3.41 | clean | - |
| commit_window_admission | 3.28 | 3.40 | clean | - |
| commit_window_exclusion | 3.30 | 3.38 | clean | - |
| bulk | 3.22 | 3.50 | clean | - |

## Flame summaries

(none captured — run with --trace)
