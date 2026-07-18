# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: 5f531e17fbc7402cdc5f5fee85b95759fb963190
- timestamp: 2026-07-18T15:15:45Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `df9b224c12d113d080e42ea806a6bec26edcaff4e01d6ad8427e2af42bdb28c7 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 8 block(s) still contaminated after retry — treat their percentiles as dirty: commit_single, commit_batch, cold_containment_walk, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.4 / 0.4 / 0.4 | 1.3 / 1.4 / 1.6 | 0.30 | WIN |
| containment_walk | 1.9 / 541.5 / 547.7 | 44.1 / 27503.7 / 28488.3 | 0.04 | WIN |
| chain | 53.5 / 86.6 / 93.6 | 479.2 / 912.9 / 916.8 | 0.11 | WIN |
| range | 18.4 / 18.8 / 18.9 | 134.4 / 514.5 / 516.6 | 0.14 | WIN |
| balance | 0.9 / 31.8 / 31.9 | 233.2 / 30300.4 / 30503.8 | 0.00 | WIN |
| stats | 1213.0 / 1236.0 / 1283.1 | 71859.8 / 73476.5 / 74983.0 | 0.02 | WIN |
| string | 2.1 / 2.3 / 2.3 | 54.8 / 59.7 / 60.2 | 0.04 | WIN |
| skew | 1485.8 / 1964.2 / 1980.1 | 7007.3 / 9317.1 / 9489.9 | 0.21 | WIN |
| spread | 10077.2 / 10275.1 / 10405.0 | 121876.7 / 123462.8 / 123998.6 | 0.08 | WIN |
| triangle | 10338.9 / 10766.1 / 11239.1 | 36470.9 / 54971.9 / 55219.6 | 0.28 | WIN |
| entries_for_account_set | 1.0 / 484.2 / 485.5 | 5.6 / 3875.3 / 3923.1 | 0.18 | WIN |
| postings_without_tag | 11.1 / 1018.2 / 1022.8 | 44.9 / 12584.8 / 12623.0 | 0.25 | WIN |
| latest_posting_per_account | 1986.6 / 2038.5 / 2135.9 | 39614.3 / 40247.6 / 41497.5 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 7.8 / 8.0 / 8.2 | 0.03 | WIN |
| mandate_overlap | 7.8 / 12.5 / 12.6 | 198.0 / 303.9 / 305.4 | 0.04 | WIN |
| busy_scan | 7.0 / 8.1 / 8.1 | 3284.1 / 3348.6 / 3383.5 | 0.00 | WIN |
| meets_chain | 3.3 / 1339.0 / 1349.8 | 16.9 / 128.9 / 134.7 | 0.20 | WIN |
| rsvp_union | 841.5 / 857.2 / 884.2 | 17402.8 / 17536.9 / 17805.2 | 0.05 | WIN |
| conflict_pairs | 20.8 / 119.8 / 122.9 | 2678.2 / 363083.0 / 365139.3 | 0.01 | WIN |
| conflict_free | 0.5 / 0.6 / 0.6 | 14.6 / 45.5 / 45.8 | 0.04 | WIN |
| free_busy | 2.5 / 38.8 / 39.0 | 288.9 / 2201.1 / 2322.4 | 0.01 | WIN |
| claim_hours | 507.6 / 518.9 / 533.4 | 6204.6 / 6257.0 / 6318.2 | 0.08 | WIN |
| slot_scan | 27.8 / 28.4 / 34.7 | 2754.1 / 2818.9 / 2846.8 | 0.01 | report |
| slot_booking_overlap | 19.1 / 553.9 / 557.5 | 603.4 / 14656.1 / 14688.4 | 0.03 | report |
| closure_depth | 3.0 / 1112.1 / 1148.9 | 11.3 / 1723.5 / 1742.1 | 0.26 | report |
| closure_fanout | 1.0 / 137.3 / 140.4 | 17.5 / 1879.6 / 1918.2 | 0.06 | report |
| disp_probe | 108600.9 / 119048.8 / 119048.8 | 613331.4 / 630925.2 / 630925.2 | 0.18 | report |
| disp_probe_d24 | 110594.5 / 122012.4 / 122012.4 | 613350.5 / 624773.1 / 624773.1 | 0.18 | report |
| disp_probe_d96 | 111480.5 / 115211.7 / 115211.7 | 610115.4 / 614587.2 / 614587.2 | 0.18 | report |
| disp_stream | 128.0 / 137.0 / 137.0 | 38938.3 / 39363.8 / 39363.8 | 0.00 | report |
| disp_stream_d24 | 141.0 / 154.1 / 154.1 | 39223.2 / 39639.2 / 39639.2 | 0.00 | report |
| disp_stream_d96 | 153.3 / 170.8 / 170.8 | 39421.8 / 39685.1 / 39685.1 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 4979.0 | 5019.5 | - |
| commit_batch | 24029.8 | 28478.1 | - |
| cold_containment_walk | 4497.1 | 76.4 | - |
| commit_witnessed | 4994.2 | - | - |
| commit_window_baseline | 4864.7 | - | - |
| commit_window_admission | 4978.6 | - | - |
| commit_window_exclusion | 5102.5 | - | - |
| bulk | 1208049.6 | 918013.4 | 165192 |

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
| point | 3.38 | 3.50 | clean | - |
| containment_walk | 3.30 | 3.50 | clean | - |
| chain | 3.40 | 3.45 | clean | - |
| range | 3.50 | 3.45 | clean | - |
| balance | 3.48 | 3.50 | clean | - |
| stats | 3.50 | 3.50 | clean | - |
| string | 3.50 | 3.39 | clean | - |
| skew | 3.50 | 3.45 | clean | - |
| spread | 3.51 | 3.50 | clean | - |
| triangle | 3.45 | 3.50 | clean | - |
| entries_for_account_set | 3.50 | 3.45 | clean | - |
| postings_without_tag | 3.45 | 3.37 | clean | - |
| latest_posting_per_account | 3.45 | 3.45 | clean | - |
| mandate_at_instant | 3.38 | 3.50 | clean | - |
| mandate_overlap | 3.50 | 3.50 | clean | - |
| busy_scan | 3.50 | 3.50 | clean | - |
| meets_chain | 3.50 | 3.51 | clean | - |
| rsvp_union | 3.50 | 3.50 | clean | - |
| conflict_pairs | 3.50 | 3.41 | clean | - |
| conflict_free | 3.45 | 3.45 | clean | - |
| free_busy | 3.50 | 3.50 | clean | - |
| claim_hours | 3.41 | 3.41 | clean | - |
| slot_scan | 3.50 | 3.33 | clean | - |
| slot_booking_overlap | 3.41 | 3.41 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.50 | 3.41 | retried | - |
| disp_probe | 3.39 | 3.50 | clean | - |
| disp_probe_d24 | 3.44 | 3.45 | clean | - |
| disp_probe_d96 | 3.50 | 3.50 | clean | - |
| disp_stream | 3.41 | 3.50 | clean | - |
| disp_stream_d24 | 3.50 | 3.43 | clean | - |
| disp_stream_d96 | 3.50 | 3.36 | clean | - |
| commit_single | 3.26 | 0.91 | CONTAMINATED | - |
| commit_batch | 0.91 | 3.00 | CONTAMINATED | - |
| cold_containment_walk | 2.98 | 3.26 | CONTAMINATED | - |
| commit_witnessed | 3.26 | 1.28 | CONTAMINATED | - |
| commit_window_baseline | 3.36 | 0.91 | CONTAMINATED | - |
| commit_window_admission | 0.91 | 0.91 | CONTAMINATED | - |
| commit_window_exclusion | 0.91 | 1.28 | CONTAMINATED | - |
| bulk | 2.00 | 2.21 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
