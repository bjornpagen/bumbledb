# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: ec0b9c75f013ce85c3aa4fce0c055ae7c46e0d49
- timestamp: 2026-07-20T12:19:53Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 3.22 3.74 3.06 (start) → 1.63 2.86 2.85 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `11727f470c9a5464631fe3b1c7ba5448a6060cf3637b34ba7bb0f171cc7df1fa (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 8 block(s) still contaminated after retry — treat their percentiles as dirty: commit_single, commit_batch, cold_containment_walk, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.6 / 0.6 | 1.4 / 1.5 / 1.5 | 0.36 | WIN |
| containment_walk | 6.4 / 577.9 / 629.1 | 44.7 / 28177.8 / 29208.5 | 0.14 | WIN |
| chain | 55.9 / 88.0 / 94.2 | 521.0 / 928.7 / 945.7 | 0.11 | WIN |
| range | 18.4 / 18.6 / 18.7 | 134.6 / 521.6 / 524.2 | 0.14 | WIN |
| balance | 0.9 / 32.1 / 32.2 | 241.5 / 30542.9 / 31326.5 | 0.00 | WIN |
| stats | 1204.9 / 1238.8 / 1332.0 | 72496.9 / 75181.5 / 76411.4 | 0.02 | WIN |
| string | 2.0 / 2.3 / 2.4 | 54.8 / 60.0 / 62.4 | 0.04 | WIN |
| skew | 1493.1 / 1975.6 / 2170.1 | 7160.2 / 9602.3 / 9928.5 | 0.21 | WIN |
| spread | 10143.8 / 10875.5 / 12764.5 | 122564.2 / 128301.1 / 133243.9 | 0.08 | WIN |
| triangle | 9553.0 / 10325.7 / 11129.9 | 36745.0 / 55615.7 / 56347.8 | 0.26 | WIN |
| entries_for_account_set | 1.0 / 482.8 / 488.8 | 6.0 / 3876.4 / 3959.9 | 0.17 | WIN |
| postings_without_tag | 5.9 / 1045.9 / 1156.1 | 45.4 / 12437.5 / 12958.8 | 0.13 | WIN |
| latest_posting_per_account | 1988.2 / 2071.1 / 2204.8 | 39537.4 / 40667.5 / 41315.8 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 7.8 / 8.3 / 9.2 | 0.04 | WIN |
| mandate_overlap | 7.8 / 12.4 / 12.5 | 194.9 / 306.0 / 318.2 | 0.04 | WIN |
| busy_scan | 7.2 / 8.8 / 10.1 | 3398.5 / 3465.5 / 3528.7 | 0.00 | WIN |
| meets_chain | 3.3 / 1304.5 / 1345.2 | 16.9 / 127.8 / 141.1 | 0.19 | WIN |
| rsvp_union | 843.9 / 863.6 / 921.1 | 17399.8 / 17852.8 / 18150.0 | 0.05 | WIN |
| conflict_pairs | 22.3 / 119.6 / 130.8 | 2682.8 / 370823.2 / 372031.4 | 0.01 | WIN |
| conflict_free | 0.5 / 0.6 / 0.6 | 14.6 / 45.8 / 45.9 | 0.04 | WIN |
| free_busy | 2.6 / 39.1 / 39.3 | 237.9 / 2204.4 / 2219.6 | 0.01 | WIN |
| claim_hours | 493.5 / 500.8 / 517.5 | 6074.9 / 6246.5 / 6760.4 | 0.08 | WIN |
| slot_scan | 27.7 / 28.5 / 35.1 | 2784.5 / 2879.2 / 2965.0 | 0.01 | report |
| slot_booking_overlap | 19.1 / 544.3 / 561.8 | 658.0 / 14785.2 / 14887.5 | 0.03 | report |
| closure_depth | 2.8 / 929.5 / 935.4 | 8.9 / 1738.5 / 1750.4 | 0.31 | report |
| closure_fanout | 1.1 / 134.5 / 136.0 | 11.9 / 1889.5 / 2007.5 | 0.09 | report |
| disp_probe | 128677.9 / 133749.9 / 133749.9 | 640380.7 / 674318.6 / 674318.6 | 0.20 | report |
| disp_probe_d24 | 115970.7 / 126620.6 / 126620.6 | 638046.3 / 649771.0 / 649771.0 | 0.18 | report |
| disp_probe_d96 | 118872.1 / 125942.9 / 125942.9 | 621000.2 / 631772.6 / 631772.6 | 0.19 | report |
| disp_stream | 128.3 / 134.5 / 134.5 | 38365.0 / 40889.5 / 40889.5 | 0.00 | report |
| disp_stream_d24 | 140.9 / 144.3 / 144.3 | 39036.2 / 39750.2 / 39750.2 | 0.00 | report |
| disp_stream_d96 | 152.2 / 170.9 / 170.9 | 38704.6 / 38970.7 / 38970.7 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 4549.7 | 5057.1 | - |
| commit_batch | 24982.3 | 28339.6 | - |
| cold_containment_walk | 1715.5 | 99.7 | - |
| cold_containment_walk_delete | 4055.9 | 86.7 | - |
| commit_witnessed | 4506.4 | - | - |
| commit_window_baseline | 4502.0 | - | - |
| commit_window_admission | 5113.8 | - | - |
| commit_window_exclusion | 5090.1 | - | - |
| bulk | 1249187.2 | 932000.0 | 159938 |

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

- bumbledb file (compacted): 78004224 bytes
- sqlite file: 18464768 bytes
- image cache: 0 images, 0 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| point | 3.50 | 3.50 | clean | - |
| containment_walk | 3.47 | 3.50 | clean | - |
| chain | 3.50 | 3.50 | clean | - |
| range | 3.50 | 3.50 | clean | - |
| balance | 3.50 | 3.50 | clean | - |
| stats | 3.50 | 3.50 | clean | - |
| string | 3.51 | 3.50 | clean | - |
| skew | 3.50 | 3.50 | clean | - |
| spread | 3.51 | 3.50 | clean | - |
| triangle | 3.50 | 3.50 | clean | - |
| entries_for_account_set | 3.50 | 3.45 | clean | - |
| postings_without_tag | 3.50 | 3.26 | clean | - |
| latest_posting_per_account | 3.26 | 3.50 | clean | - |
| mandate_at_instant | 3.36 | 3.50 | clean | - |
| mandate_overlap | 3.50 | 3.41 | clean | - |
| busy_scan | 3.25 | 3.36 | clean | - |
| meets_chain | 3.31 | 3.50 | clean | - |
| rsvp_union | 3.50 | 3.50 | clean | - |
| conflict_pairs | 3.50 | 3.50 | clean | - |
| conflict_free | 3.50 | 3.41 | clean | - |
| free_busy | 3.42 | 3.50 | clean | - |
| claim_hours | 3.50 | 3.51 | clean | - |
| slot_scan | 3.50 | 3.42 | clean | - |
| slot_booking_overlap | 3.50 | 3.50 | clean | - |
| closure_depth | 3.50 | 3.50 | retried | - |
| closure_fanout | 3.45 | 3.45 | clean | - |
| disp_probe | 3.29 | 3.30 | clean | - |
| disp_probe_d24 | 3.50 | 3.50 | clean | - |
| disp_probe_d96 | 3.50 | 3.45 | clean | - |
| disp_stream | 3.50 | 3.50 | clean | - |
| disp_stream_d24 | 3.50 | 3.50 | clean | - |
| disp_stream_d96 | 3.50 | 3.50 | clean | - |
| commit_single | 3.23 | 2.34 | CONTAMINATED | - |
| commit_batch | 2.42 | 3.13 | CONTAMINATED | - |
| cold_containment_walk | 3.05 | 3.28 | CONTAMINATED | - |
| cold_containment_walk_delete | 3.22 | 3.36 | clean | - |
| commit_witnessed | 3.21 | 1.43 | CONTAMINATED | - |
| commit_window_baseline | 3.16 | 2.42 | CONTAMINATED | - |
| commit_window_admission | 1.54 | 1.18 | CONTAMINATED | - |
| commit_window_exclusion | 1.18 | 0.91 | CONTAMINATED | - |
| bulk | 2.00 | 2.21 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
