# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: adac4010e85c9c82cc30d866f3918b7d0ec742d3
- timestamp: 2026-07-16T19:38:12Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `f4a9d0941bd4f5a18de60fd6c9f103147e34ddf7c92f9edfcc6c55ffa0849d29 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 13 block(s) still contaminated after retry — treat their percentiles as dirty: latest_posting_per_account, rsvp_union, conflict_free, claim_hours, closure_depth, disp_probe_d24, disp_probe_d96, disp_stream, disp_stream_d24, commit_single, commit_batch, commit_window_baseline, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.6 / 0.6 | 1.5 / 1.5 / 1.5 | 0.34 | WIN |
| containment_walk | 12.9 / 833.7 / 1323.3 | 64.5 / 30368.2 / 30781.8 | 0.20 | WIN |
| chain | 61.4 / 96.0 / 127.7 | 679.5 / 1044.8 / 1125.0 | 0.09 | WIN |
| range | 22.8 / 24.3 / 34.1 | 145.9 / 570.7 / 627.5 | 0.16 | WIN |
| balance | 1.1 / 34.6 / 34.8 | 367.3 / 33518.5 / 35161.8 | 0.00 | WIN |
| stats | 1386.9 / 1783.5 / 1972.2 | 80811.7 / 92237.5 / 103447.0 | 0.02 | WIN |
| string | 2.7 / 3.0 / 3.1 | 59.1 / 90.4 / 143.1 | 0.05 | WIN |
| skew | 2086.5 / 3625.0 / 5084.8 | 8020.2 / 11049.7 / 14332.2 | 0.26 | WIN |
| spread | 13915.4 / 16764.9 / 18153.8 | 135264.1 / 154162.2 / 171879.0 | 0.10 | WIN |
| triangle | 10297.6 / 11659.6 / 12313.5 | 41319.7 / 64555.4 / 68575.3 | 0.25 | WIN |
| entries_for_account_set | 15.7 / 571.2 / 617.1 | 17.0 / 4343.0 / 4796.6 | 0.92 | WIN |
| postings_without_tag | 10.2 / 1406.3 / 1669.6 | 60.9 / 14195.9 / 15179.5 | 0.17 | WIN |
| latest_posting_per_account | 2217.2 / 2762.7 / 3417.8 | 44703.3 / 47940.5 / 57551.6 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.6 | 8.1 / 11.6 / 27.0 | 0.03 | WIN |
| mandate_overlap | 8.6 / 14.0 / 14.1 | 213.9 / 356.1 / 550.2 | 0.04 | WIN |
| busy_scan | 8.5 / 12.5 / 19.8 | 3546.6 / 4859.3 / 5718.5 | 0.00 | WIN |
| meets_chain | 3.8 / 1483.7 / 2027.4 | 17.8 / 138.8 / 163.3 | 0.21 | WIN |
| rsvp_union | 1023.7 / 2029.4 / 3538.2 | 19945.8 / 33657.4 / 45444.2 | 0.05 | WIN |
| conflict_pairs | 51.2 / 138.8 / 155.2 | 4105.5 / 390726.2 / 396494.4 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.7 | 22.8 / 50.0 / 68.4 | 0.03 | WIN |
| free_busy | 3.8 / 40.2 / 48.6 | 309.7 / 2516.5 / 2962.0 | 0.01 | WIN |
| claim_hours | 527.9 / 606.8 / 985.9 | 6820.1 / 7747.8 / 8814.9 | 0.08 | WIN |
| slot_scan | 31.8 / 43.9 / 57.7 | 2917.3 / 3459.1 / 4071.5 | 0.01 | report |
| slot_booking_overlap | 40.5 / 595.8 / 683.4 | 745.5 / 15810.6 / 16347.0 | 0.05 | report |
| closure_depth | 46.7 / 18944.4 / 21198.7 | 595.0 / 2344.0 / 2863.0 | 0.08 | report |
| closure_fanout | 1.3 / 154.6 / 164.9 | 150.4 / 2461.5 / 3398.7 | 0.01 | report |
| disp_probe | 201988.0 / 209224.9 / 209224.9 | 986798.2 / 1016624.6 / 1016624.6 | 0.20 | report |
| disp_probe_d24 | 201051.3 / 207248.2 / 207248.2 | 995840.6 / 1024718.6 / 1024718.6 | 0.20 | report |
| disp_probe_d96 | 207075.0 / 219848.2 / 219848.2 | 1004686.8 / 1091105.6 / 1091105.6 | 0.21 | report |
| disp_stream | 142.5 / 314.6 / 314.6 | 42381.2 / 45047.9 / 45047.9 | 0.00 | report |
| disp_stream_d24 | 175.4 / 213.6 / 213.6 | 43880.7 / 47108.5 / 47108.5 | 0.00 | report |
| disp_stream_d96 | 193.5 / 218.9 / 218.9 | 45321.0 / 46616.9 / 46616.9 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 28.8 | 4223.0 | - |
| commit_batch | 5237.2 | 29853.9 | - |
| cold_containment_walk | 5002.9 | 176.8 | - |
| commit_witnessed | 28.2 | - | - |
| commit_window_baseline | 12.3 | - | - |
| commit_window_admission | 17.8 | - | - |
| commit_window_exclusion | 17.2 | - | - |
| bulk | 954993.8 | 976001.4 | 209290 |

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
| point | 3.26 | 3.26 | clean | - |
| containment_walk | 3.26 | 3.25 | retried | - |
| chain | 3.26 | 3.24 | clean | - |
| range | 3.26 | 3.26 | clean | - |
| balance | 3.26 | 3.26 | clean | - |
| stats | 3.26 | 3.31 | clean | - |
| string | 3.26 | 3.27 | clean | - |
| skew | 3.26 | 3.36 | retried | - |
| spread | 3.27 | 3.23 | retried | - |
| triangle | 3.41 | 3.33 | clean | - |
| entries_for_account_set | 3.27 | 3.35 | clean | - |
| postings_without_tag | 3.41 | 3.23 | clean | - |
| latest_posting_per_account | 3.10 | 3.40 | CONTAMINATED | - |
| mandate_at_instant | 3.27 | 3.23 | retried | - |
| mandate_overlap | 3.36 | 3.24 | clean | - |
| busy_scan | 3.29 | 3.35 | retried | - |
| meets_chain | 3.26 | 3.36 | retried | - |
| rsvp_union | 1.83 | 3.24 | CONTAMINATED | - |
| conflict_pairs | 3.41 | 3.41 | retried | - |
| conflict_free | 1.73 | 3.18 | CONTAMINATED | - |
| free_busy | 3.36 | 3.20 | clean | - |
| claim_hours | 3.11 | 2.97 | CONTAMINATED | - |
| slot_scan | 3.28 | 3.41 | clean | - |
| slot_booking_overlap | 3.32 | 3.26 | retried | - |
| closure_depth | 0.55 | 3.07 | CONTAMINATED | - |
| closure_fanout | 3.23 | 3.26 | clean | - |
| disp_probe | 3.34 | 3.21 | clean | - |
| disp_probe_d24 | 2.78 | 2.41 | CONTAMINATED | - |
| disp_probe_d96 | 2.98 | 2.14 | CONTAMINATED | - |
| disp_stream | 3.31 | 3.07 | CONTAMINATED | - |
| disp_stream_d24 | 3.07 | 3.25 | CONTAMINATED | - |
| disp_stream_d96 | 3.29 | 3.31 | clean | - |
| commit_single | 2.27 | 1.05 | CONTAMINATED | - |
| commit_batch | 1.28 | 3.28 | CONTAMINATED | - |
| cold_containment_walk | 3.24 | 3.36 | clean | - |
| commit_witnessed | 3.22 | 3.36 | clean | - |
| commit_window_baseline | 3.16 | 3.35 | CONTAMINATED | - |
| commit_window_admission | 3.41 | 3.41 | clean | - |
| commit_window_exclusion | 3.41 | 3.33 | clean | - |
| bulk | 3.26 | 1.97 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
