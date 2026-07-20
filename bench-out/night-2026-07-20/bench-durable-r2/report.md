# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: ec0b9c75f013ce85c3aa4fce0c055ae7c46e0d49
- timestamp: 2026-07-20T12:12:07Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 4.90 3.09 2.63 (start) → 3.22 3.74 3.06 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `11727f470c9a5464631fe3b1c7ba5448a6060cf3637b34ba7bb0f171cc7df1fa (families + 500 randomized cases)`

## Gate verdict

FAIL — losing families: entries_for_account_set.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 9 block(s) still contaminated after retry — treat their percentiles as dirty: commit_single, commit_batch, cold_containment_walk, cold_containment_walk_delete, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.5 / 0.5 | 1.4 / 1.4 / 1.4 | 0.36 | WIN |
| containment_walk | 1.9 / 546.8 / 571.0 | 44.7 / 27707.0 / 29244.7 | 0.04 | WIN |
| chain | 48.3 / 87.2 / 87.5 | 480.4 / 931.8 / 946.2 | 0.10 | WIN |
| range | 18.4 / 18.6 / 18.7 | 134.7 / 519.9 / 522.8 | 0.14 | WIN |
| balance | 1.0 / 32.0 / 32.1 | 257.1 / 30417.2 / 31318.2 | 0.00 | WIN |
| stats | 1208.0 / 1220.2 / 1234.6 | 72171.4 / 75157.5 / 76562.4 | 0.02 | WIN |
| string | 2.1 / 2.4 / 2.4 | 55.1 / 60.3 / 69.0 | 0.04 | WIN |
| skew | 1542.4 / 2068.8 / 2242.2 | 7163.6 / 9570.6 / 10018.4 | 0.22 | WIN |
| spread | 10440.8 / 12092.1 / 12745.9 | 123014.2 / 126527.2 / 129652.2 | 0.08 | WIN |
| triangle | 9556.8 / 10638.0 / 11648.5 | 36597.5 / 55555.5 / 55993.9 | 0.26 | WIN |
| entries_for_account_set | 7.3 / 482.0 / 485.2 | 6.6 / 3875.2 / 3926.4 | 1.11 | LOSS |
| postings_without_tag | 2.6 / 1016.6 / 1034.8 | 45.0 / 12507.3 / 12676.7 | 0.06 | WIN |
| latest_posting_per_account | 1994.9 / 2032.0 / 2135.0 | 39783.6 / 42186.5 / 43904.5 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.2 / 8.7 / 9.2 | 0.03 | WIN |
| mandate_overlap | 8.1 / 13.3 / 16.7 | 206.5 / 329.9 / 336.9 | 0.04 | WIN |
| busy_scan | 7.5 / 8.7 / 8.8 | 3410.4 / 3623.0 / 3699.5 | 0.00 | WIN |
| meets_chain | 3.3 / 1308.1 / 1313.6 | 17.1 / 132.0 / 136.8 | 0.19 | WIN |
| rsvp_union | 840.6 / 855.1 / 884.0 | 17518.4 / 18476.8 / 18762.1 | 0.05 | WIN |
| conflict_pairs | 20.7 / 119.1 / 123.0 | 2742.2 / 379550.9 / 383581.3 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.7 | 18.3 / 48.0 / 53.6 | 0.03 | WIN |
| free_busy | 3.2 / 39.6 / 47.3 | 294.7 / 2203.2 / 2279.0 | 0.01 | WIN |
| claim_hours | 493.9 / 505.4 / 530.1 | 6098.3 / 6279.3 / 6443.9 | 0.08 | WIN |
| slot_scan | 27.9 / 29.4 / 46.7 | 2803.0 / 2997.1 / 3040.2 | 0.01 | report |
| slot_booking_overlap | 24.8 / 545.1 / 565.3 | 607.8 / 14783.4 / 14872.9 | 0.04 | report |
| closure_depth | 3.6 / 957.8 / 1033.8 | 11.5 / 1837.8 / 1866.6 | 0.31 | report |
| closure_fanout | 1.0 / 134.8 / 136.1 | 12.4 / 1882.4 / 1897.6 | 0.08 | report |
| disp_probe | 111267.7 / 126306.2 / 126306.2 | 614945.8 / 631365.6 / 631365.6 | 0.18 | report |
| disp_probe_d24 | 115335.7 / 130799.2 / 130799.2 | 612350.7 / 639049.5 / 639049.5 | 0.19 | report |
| disp_probe_d96 | 113453.6 / 121270.2 / 121270.2 | 609890.3 / 612930.8 / 612930.8 | 0.19 | report |
| disp_stream | 127.9 / 135.8 / 135.8 | 37695.0 / 38559.0 / 38559.0 | 0.00 | report |
| disp_stream_d24 | 139.9 / 140.7 / 140.7 | 38167.6 / 38425.7 / 38425.7 | 0.00 | report |
| disp_stream_d96 | 153.5 / 158.4 / 158.4 | 38414.1 / 38647.5 / 38647.5 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 5134.0 | 4562.9 | - |
| commit_batch | 24587.6 | 29414.5 | - |
| cold_containment_walk | 1571.1 | 76.5 | - |
| cold_containment_walk_delete | 4158.2 | 76.7 | - |
| commit_witnessed | 5172.6 | - | - |
| commit_window_baseline | 5115.7 | - | - |
| commit_window_admission | 5142.1 | - | - |
| commit_window_exclusion | 5197.1 | - | - |
| bulk | 1260647.9 | 926716.5 | 155547 |

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
| containment_walk | 3.50 | 3.48 | clean | - |
| chain | 3.51 | 3.50 | clean | - |
| range | 3.50 | 3.50 | clean | - |
| balance | 3.46 | 3.45 | clean | - |
| stats | 3.50 | 3.50 | clean | - |
| string | 3.50 | 3.45 | clean | - |
| skew | 3.23 | 3.23 | clean | - |
| spread | 3.51 | 3.45 | clean | - |
| triangle | 3.50 | 3.45 | clean | - |
| entries_for_account_set | 3.50 | 3.50 | clean | - |
| postings_without_tag | 3.44 | 3.50 | clean | - |
| latest_posting_per_account | 3.50 | 3.35 | clean | - |
| mandate_at_instant | 3.35 | 3.25 | clean | - |
| mandate_overlap | 3.22 | 3.34 | retried | - |
| busy_scan | 3.35 | 3.36 | clean | - |
| meets_chain | 3.50 | 3.50 | clean | - |
| rsvp_union | 3.50 | 3.50 | clean | - |
| conflict_pairs | 3.50 | 3.50 | clean | - |
| conflict_free | 3.36 | 3.36 | clean | - |
| free_busy | 3.50 | 3.37 | clean | - |
| claim_hours | 3.50 | 3.50 | clean | - |
| slot_scan | 3.50 | 3.50 | clean | - |
| slot_booking_overlap | 3.49 | 3.41 | clean | - |
| closure_depth | 3.50 | 3.50 | retried | - |
| closure_fanout | 3.47 | 3.50 | clean | - |
| disp_probe | 3.32 | 3.50 | clean | - |
| disp_probe_d24 | 3.50 | 3.50 | clean | - |
| disp_probe_d96 | 3.50 | 3.43 | clean | - |
| disp_stream | 3.50 | 3.50 | clean | - |
| disp_stream_d24 | 3.50 | 3.50 | clean | - |
| disp_stream_d96 | 3.50 | 3.50 | clean | - |
| commit_single | 3.14 | 1.44 | CONTAMINATED | - |
| commit_batch | 1.43 | 3.13 | CONTAMINATED | - |
| cold_containment_walk | 3.10 | 3.26 | CONTAMINATED | - |
| cold_containment_walk_delete | 3.23 | 3.13 | CONTAMINATED | - |
| commit_witnessed | 3.07 | 0.91 | CONTAMINATED | - |
| commit_window_baseline | 3.22 | 2.00 | CONTAMINATED | - |
| commit_window_admission | 2.00 | 0.91 | CONTAMINATED | - |
| commit_window_exclusion | 0.91 | 1.19 | CONTAMINATED | - |
| bulk | 1.18 | 2.21 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
