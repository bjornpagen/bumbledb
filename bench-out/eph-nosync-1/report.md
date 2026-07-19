# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: 738ed5e46e2a0ed2309914d31b86b2358c6d1f0d
- timestamp: 2026-07-19T21:00:59Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `e597c9f7288b432630ea7c580dab308ef7b4059336ab1b5604aff07e0894c38b (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 7 block(s) still contaminated after retry — treat their percentiles as dirty: disp_stream_d24, commit_single, commit_batch, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.4 / 0.4 / 0.5 | 1.3 / 1.4 / 1.5 | 0.30 | WIN |
| containment_walk | 2.9 / 572.2 / 593.5 | 47.1 / 28518.5 / 29021.9 | 0.06 | WIN |
| chain | 55.0 / 89.8 / 90.7 | 514.2 / 935.4 / 971.8 | 0.11 | WIN |
| range | 19.0 / 19.1 / 19.2 | 139.4 / 536.7 / 577.3 | 0.14 | WIN |
| balance | 1.0 / 34.3 / 39.3 | 283.2 / 31780.8 / 32408.3 | 0.00 | WIN |
| stats | 1250.3 / 1340.9 / 1524.5 | 74249.0 / 76301.9 / 77527.8 | 0.02 | WIN |
| string | 2.1 / 2.4 / 2.4 | 56.5 / 61.1 / 61.5 | 0.04 | WIN |
| skew | 1527.9 / 2046.7 / 2149.5 | 7320.4 / 9804.6 / 10313.1 | 0.21 | WIN |
| spread | 10323.3 / 11545.7 / 11998.6 | 126321.7 / 129104.4 / 133600.4 | 0.08 | WIN |
| triangle | 9838.0 / 11136.9 / 12097.4 | 37788.0 / 56893.3 / 57516.4 | 0.26 | WIN |
| entries_for_account_set | 2.1 / 497.6 / 541.3 | 7.2 / 3949.9 / 3989.3 | 0.29 | WIN |
| postings_without_tag | 3.4 / 1050.4 / 1076.5 | 46.3 / 12895.5 / 13049.8 | 0.07 | WIN |
| latest_posting_per_account | 2066.2 / 2143.2 / 2196.0 | 41020.6 / 42147.3 / 42558.2 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 7.9 / 8.3 / 8.9 | 0.03 | WIN |
| mandate_overlap | 7.9 / 12.7 / 12.8 | 204.3 / 313.3 / 326.0 | 0.04 | WIN |
| busy_scan | 7.3 / 8.5 / 8.6 | 3383.5 / 3513.9 / 3592.5 | 0.00 | WIN |
| meets_chain | 3.4 / 1344.5 / 1382.9 | 17.3 / 129.0 / 133.0 | 0.20 | WIN |
| rsvp_union | 865.0 / 882.9 / 903.7 | 17939.7 / 18337.0 / 18613.1 | 0.05 | WIN |
| conflict_pairs | 29.0 / 122.7 / 137.8 | 2637.5 / 367681.1 / 370270.0 | 0.01 | WIN |
| conflict_free | 0.5 / 0.6 / 0.6 | 15.6 / 47.0 / 47.3 | 0.03 | WIN |
| free_busy | 2.7 / 39.9 / 40.1 | 296.4 / 2298.1 / 2381.8 | 0.01 | WIN |
| claim_hours | 508.7 / 526.0 / 543.3 | 6275.0 / 6435.8 / 6702.1 | 0.08 | WIN |
| slot_scan | 28.5 / 29.2 / 29.4 | 2786.8 / 2830.5 / 2872.5 | 0.01 | report |
| slot_booking_overlap | 22.9 / 563.2 / 565.7 | 614.0 / 14631.7 / 14737.2 | 0.04 | report |
| closure_depth | 6.0 / 980.3 / 1033.7 | 11.6 / 1785.3 / 1816.2 | 0.51 | report |
| closure_fanout | 1.3 / 137.7 / 144.5 | 24.6 / 1935.6 / 1962.6 | 0.05 | report |
| disp_probe | 121798.9 / 132074.6 / 132074.6 | 641288.0 / 662937.3 / 662937.3 | 0.19 | report |
| disp_probe_d24 | 113598.8 / 126908.2 / 126908.2 | 674129.5 / 721917.1 / 721917.1 | 0.17 | report |
| disp_probe_d96 | 128438.5 / 138453.0 / 138453.0 | 652644.3 / 693306.4 / 693306.4 | 0.20 | report |
| disp_stream | 131.8 / 138.6 / 138.6 | 41058.6 / 42034.7 / 42034.7 | 0.00 | report |
| disp_stream_d24 | 154.3 / 168.6 / 168.6 | 40379.4 / 41263.0 / 41263.0 | 0.00 | report |
| disp_stream_d96 | 155.3 / 159.3 / 159.3 | 39491.2 / 39633.4 / 39633.4 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 51.2 | 4514.8 | - |
| commit_batch | 5965.2 | 25783.7 | - |
| cold_containment_walk | 4146.9 | 104.0 | - |
| commit_witnessed | 56.2 | - | - |
| commit_window_baseline | 32.1 | - | - |
| commit_window_admission | 41.5 | - | - |
| commit_window_exclusion | 39.2 | - | - |
| bulk | 799499.8 | 909158.0 | 244736 |

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
| point | 3.30 | 3.50 | clean | - |
| containment_walk | 3.41 | 3.29 | clean | - |
| chain | 3.41 | 3.36 | clean | - |
| range | 3.36 | 3.41 | clean | - |
| balance | 3.25 | 3.41 | clean | - |
| stats | 3.34 | 3.41 | retried | - |
| string | 3.40 | 3.41 | retried | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.41 | 3.41 | clean | - |
| triangle | 3.41 | 3.41 | clean | - |
| entries_for_account_set | 3.25 | 3.41 | clean | - |
| postings_without_tag | 3.41 | 3.41 | clean | - |
| latest_posting_per_account | 3.41 | 3.35 | clean | - |
| mandate_at_instant | 3.27 | 3.36 | clean | - |
| mandate_overlap | 3.34 | 3.41 | clean | - |
| busy_scan | 3.41 | 3.36 | clean | - |
| meets_chain | 3.41 | 3.36 | clean | - |
| rsvp_union | 3.41 | 3.41 | clean | - |
| conflict_pairs | 3.40 | 3.36 | clean | - |
| conflict_free | 3.41 | 3.41 | clean | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 3.41 | 3.40 | clean | - |
| slot_scan | 3.41 | 3.41 | clean | - |
| slot_booking_overlap | 3.41 | 3.40 | clean | - |
| closure_depth | 3.41 | 3.40 | retried | - |
| closure_fanout | 3.41 | 3.41 | clean | - |
| disp_probe | 3.41 | 3.41 | clean | - |
| disp_probe_d24 | 3.30 | 3.27 | clean | - |
| disp_probe_d96 | 3.32 | 3.25 | retried | - |
| disp_stream | 3.41 | 3.41 | clean | - |
| disp_stream_d24 | 3.26 | 3.14 | CONTAMINATED | - |
| disp_stream_d96 | 3.40 | 3.41 | clean | - |
| commit_single | 3.26 | 0.60 | CONTAMINATED | - |
| commit_batch | 1.86 | 3.23 | CONTAMINATED | - |
| cold_containment_walk | 3.26 | 3.26 | clean | - |
| commit_witnessed | 3.26 | 3.26 | clean | - |
| commit_window_baseline | 3.21 | 3.16 | CONTAMINATED | - |
| commit_window_admission | 3.18 | 3.12 | CONTAMINATED | - |
| commit_window_exclusion | 3.26 | 3.05 | CONTAMINATED | - |
| bulk | 2.92 | 3.21 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
