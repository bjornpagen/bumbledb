# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: adac4010e85c9c82cc30d866f3918b7d0ec742d3
- timestamp: 2026-07-16T19:17:36Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `f4a9d0941bd4f5a18de60fd6c9f103147e34ddf7c92f9edfcc6c55ffa0849d29 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 14 block(s) still contaminated after retry — treat their percentiles as dirty: chain, string, mandate_overlap, claim_hours, closure_depth, disp_probe, commit_single, commit_batch, cold_containment_walk, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.5 / 0.7 / 0.8 | 1.5 / 1.5 / 1.6 | 0.34 | WIN |
| containment_walk | 7.8 / 969.6 / 1231.1 | 74.9 / 35530.1 / 79751.6 | 0.10 | WIN |
| chain | 73.3 / 103.0 / 115.3 | 551.5 / 1015.2 / 1054.1 | 0.13 | WIN |
| range | 22.6 / 24.5 / 27.0 | 144.6 / 564.3 / 609.6 | 0.16 | WIN |
| balance | 1.0 / 34.4 / 35.3 | 301.7 / 33427.3 / 36013.3 | 0.00 | WIN |
| stats | 1310.8 / 1516.6 / 1703.9 | 75709.6 / 79849.7 / 89910.0 | 0.02 | WIN |
| string | 2.8 / 3.5 / 4.0 | 57.1 / 70.8 / 78.8 | 0.05 | WIN |
| skew | 1704.3 / 2425.9 / 2670.2 | 7946.9 / 10774.2 / 11450.5 | 0.21 | WIN |
| spread | 12283.9 / 15853.6 / 17664.2 | 130676.5 / 141092.3 / 148503.3 | 0.09 | WIN |
| triangle | 10799.8 / 12960.3 / 13812.5 | 38672.2 / 58295.8 / 60703.8 | 0.28 | WIN |
| entries_for_account_set | 1.8 / 565.6 / 583.8 | 12.1 / 4087.2 / 4192.6 | 0.15 | WIN |
| postings_without_tag | 8.0 / 1109.1 / 1149.1 | 48.7 / 13508.2 / 14366.0 | 0.16 | WIN |
| latest_posting_per_account | 2094.8 / 2187.3 / 2362.5 | 42291.9 / 43870.5 / 45285.0 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.4 | 8.0 / 9.2 / 9.8 | 0.03 | WIN |
| mandate_overlap | 8.4 / 13.7 / 18.5 | 206.1 / 321.4 / 337.2 | 0.04 | WIN |
| busy_scan | 8.4 / 9.7 / 11.4 | 3444.2 / 3642.6 / 3934.9 | 0.00 | WIN |
| meets_chain | 3.5 / 1381.0 / 1442.5 | 17.3 / 136.4 / 147.5 | 0.20 | WIN |
| rsvp_union | 969.7 / 1115.8 / 1443.6 | 18350.8 / 18918.0 / 19187.8 | 0.05 | WIN |
| conflict_pairs | 33.0 / 135.8 / 142.4 | 2875.1 / 373926.8 / 380668.2 | 0.01 | WIN |
| conflict_free | 0.6 / 0.7 / 0.8 | 19.2 / 46.9 / 56.4 | 0.03 | WIN |
| free_busy | 3.8 / 39.1 / 46.6 | 282.5 / 2340.8 / 2430.7 | 0.01 | WIN |
| claim_hours | 512.3 / 561.5 / 578.3 | 6503.0 / 7052.1 / 10066.0 | 0.08 | WIN |
| slot_scan | 31.7 / 37.4 / 79.7 | 2826.6 / 3035.7 / 4171.7 | 0.01 | report |
| slot_booking_overlap | 28.1 / 570.8 / 591.5 | 784.2 / 15134.5 / 15379.2 | 0.04 | report |
| closure_depth | 15.2 / 15776.5 / 18382.9 | 21.0 / 1839.0 / 1904.9 | 0.72 | report |
| closure_fanout | 1.2 / 150.7 / 160.0 | 20.6 / 2017.0 / 2082.8 | 0.06 | report |
| disp_probe | 144081.7 / 170462.4 / 170462.4 | 699008.3 / 892209.1 / 892209.1 | 0.21 | report |
| disp_probe_d24 | 162702.0 / 177417.8 / 177417.8 | 673305.2 / 807724.5 / 807724.5 | 0.24 | report |
| disp_probe_d96 | 133238.0 / 146636.8 / 146636.8 | 702276.0 / 819496.5 / 819496.5 | 0.19 | report |
| disp_stream | 132.0 / 138.9 / 138.9 | 39841.8 / 41656.9 / 41656.9 | 0.00 | report |
| disp_stream_d24 | 144.2 / 146.1 / 146.1 | 40346.1 / 41539.8 / 41539.8 | 0.00 | report |
| disp_stream_d96 | 158.9 / 195.7 / 195.7 | 39682.2 / 40441.2 / 40441.2 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 5066.9 | 4598.2 | - |
| commit_batch | 26734.7 | 69142.4 | - |
| cold_containment_walk | 4330.8 | 125.8 | - |
| commit_witnessed | 5631.5 | - | - |
| commit_window_baseline | 6082.1 | - | - |
| commit_window_admission | 6674.0 | - | - |
| commit_window_exclusion | 6233.2 | - | - |
| bulk | 1281528.8 | 910865.8 | 153240 |

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
| point | 3.26 | 3.26 | clean | - |
| containment_walk | 3.26 | 3.36 | clean | - |
| chain | 3.24 | 2.79 | CONTAMINATED | - |
| range | 3.24 | 3.26 | retried | - |
| balance | 3.26 | 3.26 | clean | - |
| stats | 3.26 | 3.36 | clean | - |
| string | 3.19 | 3.36 | CONTAMINATED | - |
| skew | 3.21 | 3.23 | retried | - |
| spread | 3.24 | 3.41 | clean | - |
| triangle | 3.41 | 3.41 | clean | - |
| entries_for_account_set | 3.41 | 3.41 | clean | - |
| postings_without_tag | 3.41 | 3.33 | clean | - |
| latest_posting_per_account | 3.41 | 3.28 | clean | - |
| mandate_at_instant | 3.36 | 3.37 | retried | - |
| mandate_overlap | 3.41 | 3.05 | CONTAMINATED | - |
| busy_scan | 3.24 | 3.41 | clean | - |
| meets_chain | 3.24 | 3.33 | clean | - |
| rsvp_union | 3.27 | 3.35 | clean | - |
| conflict_pairs | 3.36 | 3.30 | retried | - |
| conflict_free | 3.30 | 3.34 | clean | - |
| free_busy | 3.41 | 3.41 | clean | - |
| claim_hours | 2.51 | 3.32 | CONTAMINATED | - |
| slot_scan | 3.28 | 3.41 | clean | - |
| slot_booking_overlap | 3.41 | 3.27 | clean | - |
| closure_depth | 3.01 | 3.20 | CONTAMINATED | - |
| closure_fanout | 3.36 | 3.41 | clean | - |
| disp_probe | 2.77 | 3.25 | CONTAMINATED | - |
| disp_probe_d24 | 3.40 | 3.41 | retried | - |
| disp_probe_d96 | 3.41 | 3.29 | clean | - |
| disp_stream | 3.41 | 3.41 | clean | - |
| disp_stream_d24 | 3.30 | 3.41 | retried | - |
| disp_stream_d96 | 3.41 | 3.41 | retried | - |
| commit_single | 3.11 | 1.45 | CONTAMINATED | - |
| commit_batch | 1.45 | 3.21 | CONTAMINATED | - |
| cold_containment_walk | 3.26 | 3.13 | CONTAMINATED | - |
| commit_witnessed | 3.10 | 1.45 | CONTAMINATED | - |
| commit_window_baseline | 3.25 | 2.14 | CONTAMINATED | - |
| commit_window_admission | 2.21 | 1.45 | CONTAMINATED | - |
| commit_window_exclusion | 1.45 | 1.45 | CONTAMINATED | - |
| bulk | 1.70 | 3.26 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
