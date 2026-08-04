# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: 3b31cd84ca6093b9feb2c0c72488891fd1cbe4f9
- timestamp: 2026-08-03T21:48:42Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.44 2.94 3.05 (start) → 2.50 2.72 2.92 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `d832a1dc42cda1d5873d538439bc0a55ecfdccb4a1830f92f79cada67c47974a (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 13 block(s) still contaminated after retry — treat their percentiles as dirty: claim_hours, disp_probe_d24, disp_stream_d24, commit_single, commit_batch, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, commit_capacity_baseline, commit_capacity_sum, commit_capacity_duration, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.7 | 1.4 / 1.5 / 2.1 | 0.18 | WIN |
| containment_walk | 1.9 / 665.0 / 671.9 | 57.5 / 31206.9 / 31860.7 | 0.03 | WIN |
| chain | 206.8 / 362.3 / 385.4 | 1829.3 / 3580.2 / 3616.2 | 0.11 | WIN |
| range | 20.6 / 23.1 / 30.5 | 142.0 / 557.3 / 574.5 | 0.15 | WIN |
| balance | 1.1 / 33.8 / 41.2 | 279.6 / 32492.0 / 33215.2 | 0.00 | WIN |
| stats | 1428.3 / 1615.2 / 1785.1 | 75531.2 / 80230.3 / 97903.1 | 0.02 | WIN |
| string | 2.7 / 3.3 / 8.3 | 59.2 / 68.9 / 95.0 | 0.05 | WIN |
| skew | 1638.5 / 2272.5 / 2427.4 | 7395.1 / 9884.3 / 10178.9 | 0.22 | WIN |
| spread | 10632.5 / 12318.4 / 13460.9 | 126243.2 / 130305.5 / 138837.2 | 0.08 | WIN |
| triangle | 2563.2 / 2662.5 / 2995.0 | 37739.6 / 41138.9 / 42965.5 | 0.07 | WIN |
| entries_for_account_set | 7.8 / 724.7 / 756.9 | 11.5 / 4258.1 / 4387.1 | 0.67 | WIN |
| postings_without_tag | 2.5 / 1075.1 / 1137.6 | 46.0 / 13329.0 / 13934.6 | 0.06 | WIN |
| latest_posting_per_account | 2517.3 / 2646.9 / 2923.5 | 42306.1 / 44692.6 / 45352.8 | 0.06 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.7 | 8.0 / 8.8 / 9.4 | 0.04 | WIN |
| mandate_overlap | 14.1 / 15.0 / 15.2 | 412.4 / 443.8 / 451.8 | 0.03 | WIN |
| deep_chain | 469.0 / 617.1 / 646.8 | 3692.8 / 6285.1 / 6577.1 | 0.13 | report |
| busy_scan | 8.2 / 9.6 / 9.7 | 3397.3 / 3513.9 / 3698.0 | 0.00 | WIN |
| meets_chain | 3.0 / 119.4 / 122.9 | 17.4 / 132.0 / 134.9 | 0.17 | WIN |
| rsvp_union | 976.3 / 1025.5 / 1064.3 | 18142.5 / 19296.9 / 20103.0 | 0.05 | WIN |
| conflict_pairs | 24.4 / 86.4 / 96.5 | 5884.2 / 370756.5 / 377490.0 | 0.00 | WIN |
| conflict_free | 0.6 / 0.7 / 0.8 | 15.2 / 48.6 / 49.5 | 0.04 | WIN |
| free_busy | 3.1 / 40.1 / 40.3 | 268.5 / 2283.0 / 2319.5 | 0.01 | WIN |
| claim_hours | 436.1 / 444.5 / 463.5 | 6283.1 / 6620.4 / 6787.1 | 0.07 | WIN |
| slot_scan | 32.5 / 34.8 / 39.2 | 2796.2 / 2888.9 / 3014.0 | 0.01 | report |
| slot_booking_overlap | 12.0 / 63.4 / 73.1 | 610.9 / 14653.9 / 14777.6 | 0.02 | report |
| closure_depth | 5.2 / 1095.7 / 1132.8 | 17.1 / 1815.5 / 1850.1 | 0.30 | report |
| closure_fanout | 43.2 / 44.0 / 44.9 | 551.3 / 581.4 / 591.6 | 0.08 | report |
| disp_probe | 91318.3 / 118957.2 / 118957.2 | 672140.7 / 890799.4 / 890799.4 | 0.14 | report |
| disp_probe_d24 | 90212.9 / 117726.5 / 117726.5 | 707850.8 / 842988.8 / 842988.8 | 0.13 | report |
| disp_probe_d96 | 89711.9 / 100682.2 / 100682.2 | 645992.4 / 803581.8 / 803581.8 | 0.14 | report |
| disp_stream | 131.7 / 139.4 / 139.4 | 41791.7 / 43960.8 / 43960.8 | 0.00 | report |
| disp_stream_d24 | 144.8 / 155.9 / 155.9 | 40452.0 / 42301.6 / 42301.6 | 0.00 | report |
| disp_stream_d96 | 167.7 / 206.8 / 206.8 | 40794.2 / 43675.0 / 43675.0 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 5193.5 | 5128.0 | - |
| commit_batch | 25964.0 | 12810.4 | - |
| cold_containment_walk | 1220.7 | 86.0 | - |
| cold_containment_walk_delete | 11369.6 | 95.2 | - |
| commit_witnessed | 4943.5 | - | - |
| commit_window_baseline | 4212.3 | - | - |
| commit_window_admission | 4935.8 | - | - |
| commit_window_exclusion | 4515.7 | - | - |
| commit_capacity_baseline | 4228.2 | - | - |
| commit_capacity_sum | 4920.3 | - | - |
| commit_capacity_duration | 5078.0 | - | - |
| bulk | 1094217.1 | 701888.7 | 182366 |

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
| point | 3.21 | 3.40 | retried | - |
| containment_walk | 3.41 | 3.33 | clean | - |
| chain | 3.35 | 3.38 | clean | - |
| range | 3.36 | 3.24 | clean | - |
| balance | 3.24 | 3.36 | clean | - |
| stats | 3.32 | 3.30 | clean | - |
| string | 3.28 | 3.22 | clean | - |
| skew | 3.35 | 3.41 | clean | - |
| spread | 3.24 | 3.22 | clean | - |
| triangle | 3.36 | 3.36 | clean | - |
| entries_for_account_set | 3.27 | 3.33 | retried | - |
| postings_without_tag | 3.21 | 3.41 | clean | - |
| latest_posting_per_account | 3.41 | 3.27 | clean | - |
| mandate_at_instant | 3.31 | 3.36 | clean | - |
| mandate_overlap | 3.36 | 3.36 | clean | - |
| deep_chain | 3.41 | 3.41 | clean | - |
| busy_scan | 3.41 | 3.41 | clean | - |
| meets_chain | 3.36 | 3.41 | clean | - |
| rsvp_union | 3.41 | 3.36 | clean | - |
| conflict_pairs | 3.41 | 3.27 | clean | - |
| conflict_free | 3.41 | 3.35 | clean | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 2.96 | 3.41 | CONTAMINATED | - |
| slot_scan | 3.36 | 3.36 | clean | - |
| slot_booking_overlap | 3.41 | 3.41 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.41 | 3.28 | clean | - |
| disp_probe | 3.35 | 3.41 | clean | - |
| disp_probe_d24 | 3.09 | 3.35 | CONTAMINATED | - |
| disp_probe_d96 | 3.35 | 3.34 | retried | - |
| disp_stream | 3.41 | 3.22 | clean | - |
| disp_stream_d24 | 3.20 | 3.41 | CONTAMINATED | - |
| disp_stream_d96 | 3.29 | 3.32 | retried | - |
| commit_single | 3.41 | 1.28 | CONTAMINATED | - |
| commit_batch | 1.28 | 3.50 | CONTAMINATED | - |
| cold_containment_walk | 3.50 | 3.35 | clean | - |
| cold_containment_walk_delete | 3.22 | 3.36 | clean | - |
| commit_witnessed | 3.36 | 0.89 | CONTAMINATED | - |
| commit_window_baseline | 3.48 | 1.26 | CONTAMINATED | - |
| commit_window_admission | 1.06 | 2.42 | CONTAMINATED | - |
| commit_window_exclusion | 2.42 | 1.28 | CONTAMINATED | - |
| commit_capacity_baseline | 3.30 | 0.89 | CONTAMINATED | - |
| commit_capacity_sum | 0.91 | 2.29 | CONTAMINATED | - |
| commit_capacity_duration | 2.32 | 1.25 | CONTAMINATED | - |
| bulk | 1.70 | 3.50 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
