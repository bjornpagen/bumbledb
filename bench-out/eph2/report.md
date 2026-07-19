# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: adac4010e85c9c82cc30d866f3918b7d0ec742d3
- timestamp: 2026-07-16T19:32:04Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `f4a9d0941bd4f5a18de60fd6c9f103147e34ddf7c92f9edfcc6c55ffa0849d29 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 6 block(s) still contaminated after retry — treat their percentiles as dirty: conflict_free, disp_stream, disp_stream_d96, commit_single, commit_batch, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.5 / 0.6 | 1.4 / 1.5 / 1.5 | 0.36 | WIN |
| containment_walk | 10.1 / 767.3 / 831.0 | 58.5 / 30049.7 / 38955.9 | 0.17 | WIN |
| chain | 70.5 / 94.6 / 99.7 | 530.5 / 967.9 / 1026.2 | 0.13 | WIN |
| range | 22.3 / 22.5 / 25.7 | 140.8 / 551.5 / 616.2 | 0.16 | WIN |
| balance | 1.0 / 32.9 / 33.0 | 276.2 / 31829.7 / 32688.9 | 0.00 | WIN |
| stats | 1270.6 / 1398.0 / 1601.1 | 77038.8 / 81305.0 / 94591.4 | 0.02 | WIN |
| string | 2.6 / 2.9 / 3.0 | 56.2 / 64.8 / 73.4 | 0.05 | WIN |
| skew | 1662.8 / 2253.7 / 2574.6 | 7656.8 / 10398.1 / 10764.5 | 0.22 | WIN |
| spread | 11099.7 / 13159.0 / 14362.0 | 128904.7 / 173072.8 / 186489.6 | 0.09 | WIN |
| triangle | 10199.2 / 12091.2 / 13179.1 | 38707.2 / 59527.5 / 67528.9 | 0.26 | WIN |
| entries_for_account_set | 1.2 / 566.8 / 583.8 | 11.4 / 4155.9 / 4320.0 | 0.11 | WIN |
| postings_without_tag | 11.5 / 1183.0 / 1438.8 | 49.9 / 13659.1 / 14281.0 | 0.23 | WIN |
| latest_posting_per_account | 2092.8 / 2160.9 / 2192.3 | 42885.3 / 44804.1 / 46929.0 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.1 / 9.6 / 12.5 | 0.03 | WIN |
| mandate_overlap | 8.5 / 15.8 / 20.5 | 206.5 / 339.5 / 362.0 | 0.04 | WIN |
| busy_scan | 8.4 / 9.9 / 12.0 | 3460.9 / 3808.0 / 4235.7 | 0.00 | WIN |
| meets_chain | 3.8 / 1410.3 / 1462.6 | 17.8 / 136.7 / 150.4 | 0.21 | WIN |
| rsvp_union | 987.3 / 1095.1 / 1435.7 | 18374.7 / 21371.4 / 23190.3 | 0.05 | WIN |
| conflict_pairs | 32.2 / 135.5 / 141.6 | 3024.4 / 374817.9 / 380114.7 | 0.01 | WIN |
| conflict_free | 0.6 / 0.8 / 0.9 | 15.3 / 47.0 / 49.8 | 0.04 | WIN |
| free_busy | 3.8 / 38.5 / 42.5 | 296.8 / 2357.6 / 2534.7 | 0.01 | WIN |
| claim_hours | 526.6 / 575.8 / 615.2 | 6397.7 / 6813.6 / 7080.0 | 0.08 | WIN |
| slot_scan | 31.2 / 35.6 / 39.2 | 2790.9 / 2925.5 / 2978.2 | 0.01 | report |
| slot_booking_overlap | 33.8 / 569.4 / 606.8 | 665.8 / 14753.2 / 14871.8 | 0.05 | report |
| closure_depth | 10.5 / 13952.8 / 14588.7 | 29.3 / 1817.5 / 1861.8 | 0.36 | report |
| closure_fanout | 1.3 / 151.0 / 159.2 | 13.7 / 1986.1 / 2037.7 | 0.10 | report |
| disp_probe | 158163.6 / 176681.6 / 176681.6 | 775351.2 / 865459.5 / 865459.5 | 0.20 | report |
| disp_probe_d24 | 142108.1 / 168881.7 / 168881.7 | 675539.8 / 712445.3 / 712445.3 | 0.21 | report |
| disp_probe_d96 | 132813.2 / 148025.8 / 148025.8 | 696000.7 / 876204.3 / 876204.3 | 0.19 | report |
| disp_stream | 132.0 / 140.5 / 140.5 | 39958.6 / 41693.7 / 41693.7 | 0.00 | report |
| disp_stream_d24 | 149.1 / 171.3 / 171.3 | 41536.4 / 42271.7 / 42271.7 | 0.00 | report |
| disp_stream_d96 | 184.7 / 238.2 / 238.2 | 39997.0 / 40246.5 / 40246.5 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 24.5 | 5012.2 | - |
| commit_batch | 4103.8 | 26456.2 | - |
| cold_containment_walk | 4257.3 | 120.8 | - |
| commit_witnessed | 30.1 | - | - |
| commit_window_baseline | 12.7 | - | - |
| commit_window_admission | 17.0 | - | - |
| commit_window_exclusion | 15.8 | - | - |
| bulk | 821321.3 | 921641.0 | 242107 |

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
| point | 3.26 | 3.50 | clean | - |
| containment_walk | 3.41 | 3.41 | clean | - |
| chain | 3.36 | 3.35 | retried | - |
| range | 3.27 | 3.41 | clean | - |
| balance | 3.41 | 3.36 | clean | - |
| stats | 3.41 | 3.24 | clean | - |
| string | 3.41 | 3.41 | clean | - |
| skew | 3.41 | 3.25 | clean | - |
| spread | 3.26 | 3.41 | retried | - |
| triangle | 3.36 | 3.29 | clean | - |
| entries_for_account_set | 3.41 | 3.41 | clean | - |
| postings_without_tag | 3.27 | 3.35 | clean | - |
| latest_posting_per_account | 3.23 | 3.34 | clean | - |
| mandate_at_instant | 3.41 | 3.36 | retried | - |
| mandate_overlap | 3.34 | 3.41 | retried | - |
| busy_scan | 3.25 | 3.26 | clean | - |
| meets_chain | 3.26 | 3.26 | retried | - |
| rsvp_union | 3.36 | 3.27 | retried | - |
| conflict_pairs | 3.27 | 3.35 | clean | - |
| conflict_free | 3.03 | 3.37 | CONTAMINATED | - |
| free_busy | 3.27 | 3.41 | clean | - |
| claim_hours | 3.21 | 3.41 | clean | - |
| slot_scan | 3.35 | 3.34 | retried | - |
| slot_booking_overlap | 3.27 | 3.41 | clean | - |
| closure_depth | 3.29 | 3.28 | clean | - |
| closure_fanout | 3.35 | 3.41 | clean | - |
| disp_probe | 3.41 | 3.21 | retried | - |
| disp_probe_d24 | 3.41 | 3.41 | clean | - |
| disp_probe_d96 | 3.35 | 3.31 | retried | - |
| disp_stream | 2.75 | 3.41 | CONTAMINATED | - |
| disp_stream_d24 | 3.41 | 3.21 | retried | - |
| disp_stream_d96 | 2.97 | 3.30 | CONTAMINATED | - |
| commit_single | 3.35 | 1.55 | CONTAMINATED | - |
| commit_batch | 1.76 | 3.50 | CONTAMINATED | - |
| cold_containment_walk | 3.33 | 3.26 | clean | - |
| commit_witnessed | 3.35 | 3.44 | clean | - |
| commit_window_baseline | 3.41 | 3.41 | clean | - |
| commit_window_admission | 3.41 | 3.40 | clean | - |
| commit_window_exclusion | 3.41 | 3.41 | clean | - |
| bulk | 3.22 | 1.77 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
