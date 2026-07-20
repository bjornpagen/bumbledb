# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: ec0b9c75f013ce85c3aa4fce0c055ae7c46e0d49
- timestamp: 2026-07-20T12:23:41Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 1.63 2.86 2.85 (start) → 1.67 2.13 2.52 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `11727f470c9a5464631fe3b1c7ba5448a6060cf3637b34ba7bb0f171cc7df1fa (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 5 block(s) still contaminated after retry — treat their percentiles as dirty: commit_single, commit_batch, cold_containment_walk, commit_window_baseline, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.5 / 0.5 | 1.4 / 1.4 / 1.4 | 0.36 | WIN |
| containment_walk | 2.7 / 560.2 / 616.7 | 46.7 / 28338.8 / 29209.3 | 0.06 | WIN |
| chain | 48.1 / 86.5 / 86.9 | 511.2 / 923.4 / 1006.9 | 0.09 | WIN |
| range | 18.5 / 18.7 / 18.8 | 134.5 / 519.0 / 548.0 | 0.14 | WIN |
| balance | 1.0 / 32.0 / 36.8 | 267.2 / 30966.8 / 31665.0 | 0.00 | WIN |
| stats | 1223.0 / 1400.5 / 1623.3 | 72955.2 / 74848.1 / 75614.9 | 0.02 | WIN |
| string | 2.1 / 2.3 / 2.4 | 55.3 / 61.0 / 71.6 | 0.04 | WIN |
| skew | 1580.6 / 2171.0 / 2377.3 | 7337.6 / 9809.8 / 10030.0 | 0.22 | WIN |
| spread | 10657.6 / 12076.5 / 12775.4 | 122023.7 / 124307.0 / 125545.7 | 0.09 | WIN |
| triangle | 9940.7 / 10459.0 / 11326.6 | 36885.5 / 56221.4 / 56678.6 | 0.27 | WIN |
| entries_for_account_set | 1.2 / 496.0 / 517.7 | 6.7 / 4004.2 / 4040.8 | 0.17 | WIN |
| postings_without_tag | 6.4 / 1082.7 / 1118.1 | 45.8 / 12704.3 / 12745.3 | 0.14 | WIN |
| latest_posting_per_account | 2043.9 / 2080.8 / 2166.2 | 40624.2 / 41267.0 / 41841.3 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 7.8 / 8.0 / 8.1 | 0.04 | WIN |
| mandate_overlap | 8.0 / 12.8 / 12.8 | 202.2 / 311.0 / 312.7 | 0.04 | WIN |
| busy_scan | 7.1 / 8.2 / 8.3 | 3501.7 / 3538.8 / 3577.2 | 0.00 | WIN |
| meets_chain | 3.3 / 1329.9 / 1350.7 | 17.4 / 130.8 / 134.0 | 0.19 | WIN |
| rsvp_union | 863.8 / 909.4 / 991.5 | 17858.0 / 18117.5 / 18520.3 | 0.05 | WIN |
| conflict_pairs | 22.5 / 123.0 / 126.2 | 2743.1 / 378971.3 / 379780.5 | 0.01 | WIN |
| conflict_free | 0.5 / 0.6 / 0.6 | 14.9 / 47.1 / 47.5 | 0.04 | WIN |
| free_busy | 2.6 / 40.2 / 40.3 | 281.3 / 2255.6 / 2318.6 | 0.01 | WIN |
| claim_hours | 508.0 / 531.7 / 574.8 | 6245.7 / 6348.5 / 6400.1 | 0.08 | WIN |
| slot_scan | 28.1 / 31.8 / 36.5 | 2853.3 / 2907.3 / 2942.7 | 0.01 | report |
| slot_booking_overlap | 20.8 / 560.7 / 571.5 | 727.5 / 15157.3 / 15228.1 | 0.03 | report |
| closure_depth | 3.8 / 955.5 / 1017.2 | 11.0 / 1787.3 / 1797.9 | 0.34 | report |
| closure_fanout | 1.0 / 138.4 / 141.7 | 8.6 / 1927.3 / 1935.8 | 0.12 | report |
| disp_probe | 111814.3 / 124352.0 / 124352.0 | 623951.2 / 650730.9 / 650730.9 | 0.18 | report |
| disp_probe_d24 | 112197.3 / 124420.7 / 124420.7 | 627770.5 / 657541.0 / 657541.0 | 0.18 | report |
| disp_probe_d96 | 112677.8 / 118627.5 / 118627.5 | 624572.3 / 628203.0 / 628203.0 | 0.18 | report |
| disp_stream | 131.8 / 145.5 / 145.5 | 38969.8 / 39300.7 / 39300.7 | 0.00 | report |
| disp_stream_d24 | 139.9 / 154.9 / 154.9 | 39666.4 / 40580.5 / 40580.5 | 0.00 | report |
| disp_stream_d96 | 153.7 / 184.9 / 184.9 | 39913.9 / 40050.0 / 40050.0 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 53.2 | 4943.8 | - |
| commit_batch | 5332.8 | 29121.3 | - |
| cold_containment_walk | 1381.7 | 75.8 | - |
| cold_containment_walk_delete | 3682.5 | 73.8 | - |
| commit_witnessed | 56.9 | - | - |
| commit_window_baseline | 28.6 | - | - |
| commit_window_admission | 36.5 | - | - |
| commit_window_exclusion | 34.2 | - | - |
| bulk | 770118.5 | 946048.5 | 259516 |

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
| point | 3.50 | 3.50 | clean | - |
| containment_walk | 3.51 | 3.50 | clean | - |
| chain | 3.50 | 3.26 | clean | - |
| range | 3.36 | 3.50 | clean | - |
| balance | 3.49 | 3.50 | clean | - |
| stats | 3.50 | 3.50 | clean | - |
| string | 3.50 | 3.50 | clean | - |
| skew | 3.45 | 3.51 | clean | - |
| spread | 3.41 | 3.41 | clean | - |
| triangle | 3.41 | 3.41 | clean | - |
| entries_for_account_set | 3.41 | 3.41 | clean | - |
| postings_without_tag | 3.41 | 3.36 | clean | - |
| latest_posting_per_account | 3.51 | 3.26 | clean | - |
| mandate_at_instant | 3.37 | 3.50 | clean | - |
| mandate_overlap | 3.41 | 3.41 | clean | - |
| busy_scan | 3.41 | 3.41 | clean | - |
| meets_chain | 3.41 | 3.32 | clean | - |
| rsvp_union | 3.41 | 3.32 | clean | - |
| conflict_pairs | 3.41 | 3.41 | clean | - |
| conflict_free | 3.41 | 3.41 | clean | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 3.36 | 3.36 | clean | - |
| slot_scan | 3.33 | 3.41 | clean | - |
| slot_booking_overlap | 3.36 | 3.41 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.36 | 3.41 | clean | - |
| disp_probe | 3.42 | 3.41 | retried | - |
| disp_probe_d24 | 3.50 | 3.36 | clean | - |
| disp_probe_d96 | 3.51 | 3.36 | clean | - |
| disp_stream | 3.41 | 3.41 | clean | - |
| disp_stream_d24 | 3.50 | 3.41 | clean | - |
| disp_stream_d96 | 3.51 | 3.41 | clean | - |
| commit_single | 3.23 | 0.91 | CONTAMINATED | - |
| commit_batch | 0.91 | 3.00 | CONTAMINATED | - |
| cold_containment_walk | 2.97 | 3.26 | CONTAMINATED | - |
| cold_containment_walk_delete | 3.23 | 3.26 | clean | - |
| commit_witnessed | 3.23 | 3.26 | clean | - |
| commit_window_baseline | 3.29 | 3.16 | CONTAMINATED | - |
| commit_window_admission | 3.44 | 3.50 | clean | - |
| commit_window_exclusion | 3.40 | 3.50 | clean | - |
| bulk | 2.99 | 2.39 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
