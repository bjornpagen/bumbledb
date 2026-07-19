# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: adac4010e85c9c82cc30d866f3918b7d0ec742d3
- timestamp: 2026-07-16T19:23:02Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `f4a9d0941bd4f5a18de60fd6c9f103147e34ddf7c92f9edfcc6c55ffa0849d29 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 9 block(s) still contaminated after retry — treat their percentiles as dirty: balance, stats, spread, mandate_at_instant, mandate_overlap, free_busy, slot_scan, disp_probe, commit_window_baseline.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.4 / 0.4 / 0.7 | 1.4 / 1.6 / 1.9 | 0.29 | WIN |
| containment_walk | 6.9 / 774.6 / 793.9 | 53.8 / 29929.0 / 30773.0 | 0.13 | WIN |
| chain | 71.4 / 94.2 / 107.5 | 581.1 / 984.4 / 1055.1 | 0.12 | WIN |
| range | 22.3 / 23.0 / 27.0 | 139.0 / 541.1 / 597.2 | 0.16 | WIN |
| balance | 1.0 / 32.9 / 36.4 | 284.9 / 32468.1 / 32991.7 | 0.00 | WIN |
| stats | 1266.7 / 1396.8 / 1533.1 | 76182.0 / 79460.8 / 80957.0 | 0.02 | WIN |
| string | 2.7 / 3.0 / 3.0 | 56.6 / 60.6 / 64.5 | 0.05 | WIN |
| skew | 1677.6 / 2291.3 / 2328.7 | 7293.2 / 9689.0 / 10413.0 | 0.23 | WIN |
| spread | 11741.2 / 14687.1 / 15787.0 | 128967.7 / 135653.5 / 143038.9 | 0.09 | WIN |
| triangle | 10102.6 / 11406.8 / 12827.7 | 38020.3 / 57366.3 / 57942.5 | 0.27 | WIN |
| entries_for_account_set | 5.7 / 570.9 / 610.8 | 9.3 / 4115.5 / 4227.7 | 0.61 | WIN |
| postings_without_tag | 3.1 / 1124.3 / 1152.8 | 46.4 / 12915.5 / 13348.2 | 0.07 | WIN |
| latest_posting_per_account | 2063.7 / 2178.8 / 2264.6 | 41524.3 / 42633.7 / 43615.8 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.6 | 8.0 / 8.9 / 9.4 | 0.03 | WIN |
| mandate_overlap | 8.6 / 13.8 / 14.0 | 206.6 / 324.3 / 344.0 | 0.04 | WIN |
| busy_scan | 8.2 / 9.7 / 9.8 | 3408.8 / 3543.8 / 3575.1 | 0.00 | WIN |
| meets_chain | 3.5 / 1381.8 / 1434.8 | 17.3 / 136.0 / 146.3 | 0.20 | WIN |
| rsvp_union | 953.6 / 1018.2 / 1050.8 | 18148.3 / 18562.7 / 18720.3 | 0.05 | WIN |
| conflict_pairs | 26.3 / 128.3 / 136.0 | 2772.9 / 371072.4 / 374688.1 | 0.01 | WIN |
| conflict_free | 0.6 / 0.6 / 0.8 | 20.0 / 47.0 / 51.4 | 0.03 | WIN |
| free_busy | 3.8 / 43.8 / 55.2 | 285.2 / 2306.4 / 2345.8 | 0.01 | WIN |
| claim_hours | 512.3 / 547.5 / 567.3 | 6366.1 / 6622.6 / 6760.2 | 0.08 | WIN |
| slot_scan | 31.5 / 35.6 / 40.4 | 2851.9 / 2961.0 / 3017.5 | 0.01 | report |
| slot_booking_overlap | 29.4 / 577.6 / 593.3 | 727.9 / 14889.4 / 15125.8 | 0.04 | report |
| closure_depth | 10.5 / 14217.0 / 14657.0 | 27.7 / 1829.7 / 1895.8 | 0.38 | report |
| closure_fanout | 1.3 / 154.1 / 166.4 | 13.8 / 2002.4 / 2069.5 | 0.09 | report |
| disp_probe | 137476.0 / 151002.6 / 151002.6 | 711559.5 / 730126.5 / 730126.5 | 0.19 | report |
| disp_probe_d24 | 138497.6 / 151982.3 / 151982.3 | 700788.4 / 752599.8 / 752599.8 | 0.20 | report |
| disp_probe_d96 | 139120.8 / 154764.2 / 154764.2 | 674728.5 / 753066.0 / 753066.0 | 0.21 | report |
| disp_stream | 140.7 / 161.3 / 161.3 | 40763.1 / 42551.2 / 42551.2 | 0.00 | report |
| disp_stream_d24 | 144.7 / 155.3 / 155.3 | 40852.7 / 43083.8 / 43083.8 | 0.00 | report |
| disp_stream_d96 | 157.5 / 166.2 / 166.2 | 40052.7 / 40273.7 / 40273.7 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 4195.7 | 4170.6 | - |
| commit_batch | 28349.1 | 29472.3 | - |
| cold_containment_walk | 4358.9 | 111.2 | - |
| commit_witnessed | 4192.8 | - | - |
| commit_window_baseline | 4186.0 | - | - |
| commit_window_admission | 4196.2 | - | - |
| commit_window_exclusion | 4653.2 | - | - |
| bulk | 1320201.6 | 905594.3 | 148320 |

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
| point | 3.38 | 3.41 | retried | - |
| containment_walk | 3.25 | 3.41 | clean | - |
| chain | 3.41 | 3.35 | retried | - |
| range | 3.22 | 3.34 | clean | - |
| balance | 3.30 | 3.19 | CONTAMINATED | - |
| stats | 3.05 | 3.36 | CONTAMINATED | - |
| string | 3.38 | 3.25 | retried | - |
| skew | 3.41 | 3.41 | clean | - |
| spread | 3.05 | 3.41 | CONTAMINATED | - |
| triangle | 3.21 | 3.26 | clean | - |
| entries_for_account_set | 3.41 | 3.36 | retried | - |
| postings_without_tag | 3.41 | 3.36 | clean | - |
| latest_posting_per_account | 3.41 | 3.33 | clean | - |
| mandate_at_instant | 3.08 | 3.36 | CONTAMINATED | - |
| mandate_overlap | 3.33 | 3.20 | CONTAMINATED | - |
| busy_scan | 3.22 | 3.41 | retried | - |
| meets_chain | 3.29 | 3.26 | retried | - |
| rsvp_union | 3.25 | 3.36 | clean | - |
| conflict_pairs | 3.32 | 3.24 | retried | - |
| conflict_free | 3.41 | 3.35 | clean | - |
| free_busy | 3.36 | 3.19 | CONTAMINATED | - |
| claim_hours | 3.41 | 3.39 | clean | - |
| slot_scan | 3.09 | 3.26 | CONTAMINATED | - |
| slot_booking_overlap | 3.27 | 3.41 | clean | - |
| closure_depth | 3.41 | 3.41 | retried | - |
| closure_fanout | 3.36 | 3.36 | clean | - |
| disp_probe | 3.10 | 3.30 | CONTAMINATED | - |
| disp_probe_d24 | 3.36 | 3.24 | clean | - |
| disp_probe_d96 | 3.35 | 3.23 | retried | - |
| disp_stream | 3.27 | 3.21 | clean | - |
| disp_stream_d24 | 3.24 | 3.26 | clean | - |
| disp_stream_d96 | 3.21 | 3.22 | clean | - |
| commit_single | 3.26 | 3.26 | clean | - |
| commit_batch | 3.26 | 3.36 | clean | - |
| cold_containment_walk | 3.21 | 3.26 | clean | - |
| commit_witnessed | 3.26 | 3.26 | clean | - |
| commit_window_baseline | 3.05 | 3.25 | CONTAMINATED | - |
| commit_window_admission | 3.26 | 3.26 | clean | - |
| commit_window_exclusion | 3.26 | 3.26 | clean | - |
| bulk | 3.26 | 3.27 | clean | - |

## Flame summaries

(none captured — run with --trace)
