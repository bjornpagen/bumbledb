# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: 3b31cd84ca6093b9feb2c0c72488891fd1cbe4f9
- timestamp: 2026-08-03T22:11:46Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.84 3.16 3.18 (start) → 3.23 3.46 3.32 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `d832a1dc42cda1d5873d538439bc0a55ecfdccb4a1830f92f79cada67c47974a (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 4 block(s) still contaminated after retry — treat their percentiles as dirty: skew, claim_hours, disp_stream, disp_stream_d24.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.3 | 1.4 / 1.4 / 1.6 | 0.19 | WIN |
| containment_walk | 2.0 / 686.4 / 728.5 | 62.0 / 31059.2 / 31954.9 | 0.03 | WIN |
| chain | 211.5 / 351.5 / 365.0 | 1915.6 / 3646.7 / 3849.0 | 0.11 | WIN |
| range | 20.6 / 20.8 / 30.0 | 139.4 / 545.8 / 565.2 | 0.15 | WIN |
| balance | 1.0 / 33.1 / 36.6 | 300.4 / 33624.1 / 34392.6 | 0.00 | WIN |
| stats | 1407.7 / 1547.1 / 1732.9 | 78883.9 / 81969.2 / 100309.0 | 0.02 | WIN |
| string | 2.7 / 3.4 / 3.6 | 59.3 / 69.5 / 78.6 | 0.04 | WIN |
| skew | 1797.9 / 2643.7 / 2875.8 | 7805.0 / 10446.5 / 10746.3 | 0.23 | WIN |
| spread | 11474.2 / 14660.6 / 18860.4 | 132716.2 / 159843.2 / 166838.2 | 0.09 | WIN |
| triangle | 2603.7 / 2757.1 / 3127.8 | 38504.1 / 42906.1 / 61614.9 | 0.07 | WIN |
| entries_for_account_set | 2.8 / 708.4 / 720.9 | 18.2 / 4263.2 / 8598.5 | 0.15 | WIN |
| postings_without_tag | 2.7 / 1224.3 / 1354.7 | 52.3 / 13669.2 / 14477.2 | 0.05 | WIN |
| latest_posting_per_account | 2562.0 / 2717.5 / 3064.3 | 41741.2 / 44299.6 / 45220.6 | 0.06 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.1 / 8.5 / 8.8 | 0.04 | WIN |
| mandate_overlap | 13.8 / 14.8 / 14.8 | 412.5 / 447.3 / 454.6 | 0.03 | WIN |
| deep_chain | 382.6 / 622.7 / 657.8 | 3376.8 / 6187.5 / 6424.0 | 0.11 | report |
| busy_scan | 8.4 / 9.8 / 9.9 | 3442.1 / 3709.7 / 3849.0 | 0.00 | WIN |
| meets_chain | 2.9 / 120.0 / 129.0 | 17.5 / 133.7 / 138.5 | 0.17 | WIN |
| rsvp_union | 971.3 / 1018.0 / 1037.5 | 18385.2 / 19303.7 / 19503.0 | 0.05 | WIN |
| conflict_pairs | 30.4 / 87.5 / 94.6 | 2839.0 / 371363.3 / 376066.8 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.7 | 24.5 / 49.2 / 54.0 | 0.02 | WIN |
| free_busy | 3.9 / 40.3 / 45.2 | 301.9 / 2341.7 / 2461.1 | 0.01 | WIN |
| claim_hours | 444.9 / 482.0 / 507.2 | 6242.8 / 6568.3 / 6777.5 | 0.07 | WIN |
| slot_scan | 32.9 / 40.4 / 59.0 | 2794.5 / 2869.6 / 2922.3 | 0.01 | report |
| slot_booking_overlap | 7.5 / 60.9 / 61.2 | 707.5 / 15036.0 / 15346.6 | 0.01 | report |
| closure_depth | 1.0 / 1132.2 / 1151.7 | 21.9 / 1903.6 / 1951.5 | 0.05 | report |
| closure_fanout | 4.2 / 154.1 / 167.2 | 17.1 / 1968.2 / 2059.8 | 0.25 | report |
| disp_probe | 83584.0 / 105851.7 / 105851.7 | 649215.6 / 828883.7 / 828883.7 | 0.13 | report |
| disp_probe_d24 | 100899.1 / 149637.4 / 149637.4 | 875032.8 / 963856.0 / 963856.0 | 0.12 | report |
| disp_probe_d96 | 84795.6 / 95482.6 / 95482.6 | 655161.4 / 717633.6 / 717633.6 | 0.13 | report |
| disp_stream | 134.4 / 147.0 / 147.0 | 39783.1 / 42190.2 / 42190.2 | 0.00 | report |
| disp_stream_d24 | 143.7 / 152.2 / 152.2 | 40063.1 / 40677.5 / 40677.5 | 0.00 | report |
| disp_stream_d96 | 159.1 / 183.4 / 183.4 | 40255.5 / 40599.0 / 40599.0 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 44.5 | 32.0 | - |
| commit_batch | 5070.6 | 6126.6 | - |
| cold_containment_walk | 1096.1 | 81.8 | - |
| cold_containment_walk_delete | 11752.9 | 98.2 | - |
| commit_witnessed | 54.7 | - | - |
| commit_window_baseline | 27.7 | - | - |
| commit_window_admission | 37.5 | - | - |
| commit_window_exclusion | 34.5 | - | - |
| commit_capacity_baseline | 19.1 | - | - |
| commit_capacity_sum | 34.0 | - | - |
| commit_capacity_duration | 31.2 | - | - |
| bulk | 653945.3 | 455436.5 | 306385 |

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
| point | 3.44 | 3.35 | clean | - |
| containment_walk | 3.41 | 3.31 | clean | - |
| chain | 3.27 | 3.41 | retried | - |
| range | 3.41 | 3.41 | retried | - |
| balance | 3.41 | 3.29 | clean | - |
| stats | 3.42 | 3.34 | clean | - |
| string | 3.33 | 3.40 | retried | - |
| skew | 3.35 | 3.18 | CONTAMINATED | - |
| spread | 3.31 | 3.30 | retried | - |
| triangle | 3.36 | 3.31 | clean | - |
| entries_for_account_set | 3.29 | 3.29 | clean | - |
| postings_without_tag | 3.36 | 3.36 | clean | - |
| latest_posting_per_account | 3.23 | 3.41 | clean | - |
| mandate_at_instant | 3.41 | 3.41 | clean | - |
| mandate_overlap | 3.36 | 3.36 | clean | - |
| deep_chain | 3.26 | 3.36 | clean | - |
| busy_scan | 3.41 | 3.41 | clean | - |
| meets_chain | 3.36 | 3.41 | clean | - |
| rsvp_union | 3.35 | 3.41 | clean | - |
| conflict_pairs | 3.41 | 3.36 | clean | - |
| conflict_free | 3.26 | 3.35 | clean | - |
| free_busy | 3.22 | 3.23 | clean | - |
| claim_hours | 3.40 | 2.89 | CONTAMINATED | - |
| slot_scan | 3.41 | 3.41 | retried | - |
| slot_booking_overlap | 3.41 | 3.41 | clean | - |
| closure_depth | 3.36 | 3.28 | clean | - |
| closure_fanout | 3.41 | 3.35 | clean | - |
| disp_probe | 3.36 | 3.22 | clean | - |
| disp_probe_d24 | 3.41 | 3.41 | clean | - |
| disp_probe_d96 | 3.36 | 3.28 | retried | - |
| disp_stream | 2.84 | 3.27 | CONTAMINATED | - |
| disp_stream_d24 | 3.02 | 3.36 | CONTAMINATED | - |
| disp_stream_d96 | 3.31 | 3.41 | clean | - |
| commit_single | 3.41 | 3.36 | clean | - |
| commit_batch | 3.33 | 3.35 | clean | - |
| cold_containment_walk | 3.41 | 3.50 | clean | - |
| cold_containment_walk_delete | 3.45 | 3.50 | clean | - |
| commit_witnessed | 3.35 | 3.50 | clean | - |
| commit_window_baseline | 3.24 | 3.26 | clean | - |
| commit_window_admission | 3.41 | 3.31 | clean | - |
| commit_window_exclusion | 3.29 | 3.41 | clean | - |
| commit_capacity_baseline | 3.41 | 3.41 | clean | - |
| commit_capacity_sum | 3.41 | 3.41 | clean | - |
| commit_capacity_duration | 3.27 | 3.41 | clean | - |
| bulk | 3.41 | 3.28 | clean | - |

## Flame summaries

(none captured — run with --trace)
