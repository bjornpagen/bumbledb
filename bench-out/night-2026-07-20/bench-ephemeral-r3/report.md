# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: ec0b9c75f013ce85c3aa4fce0c055ae7c46e0d49
- timestamp: 2026-07-20T12:31:07Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.06 2.10 2.41 (start) → 1.71 2.06 2.33 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `11727f470c9a5464631fe3b1c7ba5448a6060cf3637b34ba7bb0f171cc7df1fa (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 6 block(s) still contaminated after retry — treat their percentiles as dirty: commit_single, commit_batch, cold_containment_walk, cold_containment_walk_delete, commit_witnessed, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.6 / 0.7 | 1.4 / 1.6 / 1.9 | 0.36 | WIN |
| containment_walk | 2.0 / 556.4 / 558.6 | 46.3 / 28561.3 / 29106.8 | 0.04 | WIN |
| chain | 52.7 / 89.3 / 89.6 | 507.5 / 937.6 / 949.0 | 0.10 | WIN |
| range | 19.0 / 19.2 / 19.3 | 137.7 / 529.0 / 531.5 | 0.14 | WIN |
| balance | 1.0 / 32.8 / 35.9 | 276.5 / 31278.6 / 31954.8 | 0.00 | WIN |
| stats | 1234.3 / 1319.8 / 1619.2 | 74129.8 / 75726.0 / 76637.0 | 0.02 | WIN |
| string | 2.2 / 2.4 / 2.5 | 56.7 / 61.5 / 61.8 | 0.04 | WIN |
| skew | 1520.5 / 2007.3 / 2044.6 | 7269.5 / 9697.6 / 9796.3 | 0.21 | WIN |
| spread | 10306.7 / 10854.5 / 12281.9 | 124850.6 / 125903.5 / 128194.6 | 0.08 | WIN |
| triangle | 9801.7 / 10285.0 / 11700.3 | 37517.7 / 56733.3 / 56956.0 | 0.26 | WIN |
| entries_for_account_set | 1.1 / 497.5 / 502.2 | 9.2 / 4002.4 / 4123.5 | 0.12 | WIN |
| postings_without_tag | 3.2 / 1048.9 / 1070.8 | 46.2 / 12771.4 / 13020.5 | 0.07 | WIN |
| latest_posting_per_account | 2049.6 / 2079.2 / 2147.6 | 40607.7 / 41581.1 / 42070.1 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.0 / 8.2 / 8.8 | 0.03 | WIN |
| mandate_overlap | 8.0 / 12.8 / 12.8 | 200.8 / 311.2 / 313.9 | 0.04 | WIN |
| busy_scan | 7.3 / 8.5 / 8.5 | 3494.2 / 3531.8 / 3605.6 | 0.00 | WIN |
| meets_chain | 3.5 / 1393.7 / 1403.8 | 17.4 / 130.8 / 133.8 | 0.20 | WIN |
| rsvp_union | 864.5 / 876.5 / 891.8 | 17896.3 / 18122.5 / 18253.6 | 0.05 | WIN |
| conflict_pairs | 27.1 / 124.0 / 133.8 | 2786.6 / 378787.8 / 379248.3 | 0.01 | WIN |
| conflict_free | 0.5 / 0.6 / 0.7 | 15.0 / 47.2 / 50.0 | 0.04 | WIN |
| free_busy | 2.6 / 40.0 / 42.3 | 271.0 / 2272.1 / 2323.3 | 0.01 | WIN |
| claim_hours | 509.1 / 521.6 / 530.8 | 6244.5 / 6347.1 / 6506.9 | 0.08 | WIN |
| slot_scan | 28.5 / 29.1 / 32.3 | 2862.6 / 2918.0 / 2953.0 | 0.01 | report |
| slot_booking_overlap | 19.2 / 560.8 / 563.0 | 620.4 / 15173.9 / 15214.9 | 0.03 | report |
| closure_depth | 3.5 / 960.0 / 999.7 | 10.5 / 1773.2 / 1800.0 | 0.33 | report |
| closure_fanout | 1.3 / 138.7 / 151.5 | 19.4 / 1929.4 / 1965.4 | 0.07 | report |
| disp_probe | 112564.1 / 122818.2 / 122818.2 | 628773.3 / 654873.2 / 654873.2 | 0.18 | report |
| disp_probe_d24 | 113338.4 / 127586.2 / 127586.2 | 628103.2 / 647960.4 / 647960.4 | 0.18 | report |
| disp_probe_d96 | 114038.6 / 120139.4 / 120139.4 | 625690.5 / 636172.1 / 636172.1 | 0.18 | report |
| disp_stream | 131.4 / 136.0 / 136.0 | 38681.7 / 38992.2 / 38992.2 | 0.00 | report |
| disp_stream_d24 | 142.5 / 146.3 / 146.3 | 39582.5 / 40318.6 / 40318.6 | 0.00 | report |
| disp_stream_d96 | 153.6 / 160.2 / 160.2 | 39481.4 / 39584.9 / 39584.9 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 51.5 | 4766.9 | - |
| commit_batch | 5376.0 | 29287.8 | - |
| cold_containment_walk | 1418.3 | 74.4 | - |
| cold_containment_walk_delete | 3835.8 | 73.3 | - |
| commit_witnessed | 57.2 | - | - |
| commit_window_baseline | 27.4 | - | - |
| commit_window_admission | 38.3 | - | - |
| commit_window_exclusion | 34.4 | - | - |
| bulk | 787967.8 | 928999.7 | 253855 |

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
| point | 3.50 | 3.44 | clean | - |
| containment_walk | 3.41 | 3.41 | clean | - |
| chain | 3.41 | 3.41 | clean | - |
| range | 3.40 | 3.41 | retried | - |
| balance | 3.41 | 3.27 | clean | - |
| stats | 3.41 | 3.41 | clean | - |
| string | 3.41 | 3.41 | clean | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.41 | 3.41 | clean | - |
| triangle | 3.41 | 3.41 | clean | - |
| entries_for_account_set | 3.38 | 3.41 | retried | - |
| postings_without_tag | 3.41 | 3.41 | clean | - |
| latest_posting_per_account | 3.29 | 3.40 | clean | - |
| mandate_at_instant | 3.30 | 3.41 | clean | - |
| mandate_overlap | 3.36 | 3.40 | clean | - |
| busy_scan | 3.36 | 3.41 | retried | - |
| meets_chain | 3.41 | 3.41 | clean | - |
| rsvp_union | 3.24 | 3.24 | clean | - |
| conflict_pairs | 3.40 | 3.41 | clean | - |
| conflict_free | 3.41 | 3.40 | clean | - |
| free_busy | 3.39 | 3.41 | clean | - |
| claim_hours | 3.41 | 3.41 | clean | - |
| slot_scan | 3.29 | 3.41 | clean | - |
| slot_booking_overlap | 3.41 | 3.41 | clean | - |
| closure_depth | 3.41 | 3.23 | retried | - |
| closure_fanout | 3.36 | 3.40 | clean | - |
| disp_probe | 3.50 | 3.50 | clean | - |
| disp_probe_d24 | 3.45 | 3.41 | clean | - |
| disp_probe_d96 | 3.50 | 3.41 | clean | - |
| disp_stream | 3.22 | 3.41 | clean | - |
| disp_stream_d24 | 3.36 | 3.41 | clean | - |
| disp_stream_d96 | 3.50 | 3.41 | clean | - |
| commit_single | 3.23 | 0.91 | CONTAMINATED | - |
| commit_batch | 0.91 | 3.00 | CONTAMINATED | - |
| cold_containment_walk | 2.97 | 3.13 | CONTAMINATED | - |
| cold_containment_walk_delete | 3.10 | 3.26 | CONTAMINATED | - |
| commit_witnessed | 3.17 | 3.26 | CONTAMINATED | - |
| commit_window_baseline | 3.23 | 3.50 | clean | - |
| commit_window_admission | 3.37 | 3.50 | clean | - |
| commit_window_exclusion | 3.44 | 3.50 | clean | - |
| bulk | 2.98 | 2.20 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
