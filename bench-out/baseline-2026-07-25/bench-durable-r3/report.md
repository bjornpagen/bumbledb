# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: e511b5404438908f0be4b01cafcd5621e3275c77
- timestamp: 2026-08-02T00:41:47Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.41 2.83 2.75 (start) → 2.53 2.71 2.72 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `5c3b2c1d056c180caa921ba7be8f8eb2a1f3da0d8133532cf045f2f3a429c243 (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 10 block(s) still contaminated after retry — treat their percentiles as dirty: commit_single, commit_batch, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, commit_capacity_baseline, commit_capacity_sum, commit_capacity_duration, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.6 | 1.4 / 1.8 / 2.0 | 0.19 | WIN |
| containment_walk | 2.2 / 621.3 / 647.0 | 48.8 / 29104.5 / 29460.1 | 0.04 | WIN |
| chain | 211.0 / 343.2 / 351.7 | 1847.0 / 3556.5 / 3619.8 | 0.11 | WIN |
| range | 19.9 / 25.5 / 31.8 | 141.7 / 557.2 / 607.4 | 0.14 | WIN |
| balance | 1.1 / 38.9 / 47.0 | 279.9 / 31926.1 / 32924.5 | 0.00 | WIN |
| stats | 1348.4 / 1411.8 / 1476.9 | 75302.8 / 77017.5 / 77840.0 | 0.02 | WIN |
| string | 2.5 / 2.6 / 2.7 | 59.9 / 65.1 / 71.4 | 0.04 | WIN |
| skew | 1537.4 / 2080.9 / 2145.3 | 7447.4 / 9925.2 / 10179.0 | 0.21 | WIN |
| spread | 10486.3 / 10826.9 / 12151.1 | 127672.1 / 129439.8 / 130191.5 | 0.08 | WIN |
| triangle | 2584.3 / 2707.5 / 2756.9 | 37067.0 / 40367.2 / 40690.1 | 0.07 | WIN |
| entries_for_account_set | 1.3 / 559.5 / 568.8 | 10.4 / 4101.5 / 4236.5 | 0.12 | WIN |
| postings_without_tag | 7.2 / 1008.5 / 1063.4 | 43.7 / 13111.7 / 13271.8 | 0.17 | WIN |
| latest_posting_per_account | 2254.9 / 2346.8 / 2400.2 | 41409.8 / 42590.4 / 43499.8 | 0.05 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.1 / 8.6 / 9.0 | 0.03 | WIN |
| mandate_overlap | 15.8 / 17.2 / 17.7 | 408.1 / 454.2 / 460.4 | 0.04 | WIN |
| deep_chain | 373.8 / 618.5 / 634.1 | 3210.8 / 6226.1 / 6329.4 | 0.12 | report |
| busy_scan | 7.7 / 8.8 / 8.9 | 3403.9 / 3552.5 / 3731.2 | 0.00 | WIN |
| meets_chain | 3.1 / 836.9 / 884.1 | 17.6 / 132.3 / 136.8 | 0.18 | WIN |
| rsvp_union | 934.8 / 980.5 / 1057.0 | 18174.1 / 18476.3 / 18802.1 | 0.05 | WIN |
| conflict_pairs | 23.6 / 91.8 / 95.7 | 2843.9 / 370424.5 / 371443.1 | 0.01 | WIN |
| conflict_free | 0.6 / 0.7 / 0.8 | 23.8 / 47.8 / 54.5 | 0.03 | WIN |
| free_busy | 3.1 / 41.5 / 48.2 | 284.8 / 2286.4 / 2383.5 | 0.01 | WIN |
| claim_hours | 439.7 / 480.4 / 520.8 | 6294.1 / 6452.9 / 6627.0 | 0.07 | WIN |
| slot_scan | 30.3 / 36.7 / 45.8 | 2799.2 / 2898.9 / 2938.7 | 0.01 | report |
| slot_booking_overlap | 11.5 / 59.2 / 68.3 | 724.4 / 14793.4 / 14929.2 | 0.02 | report |
| closure_depth | 2.8 / 1074.1 / 1151.2 | 13.2 / 1801.0 / 1921.4 | 0.22 | report |
| closure_fanout | 1.0 / 154.5 / 166.2 | 8.6 / 1957.8 / 2056.9 | 0.12 | report |
| disp_probe | 79186.8 / 87988.2 / 87988.2 | 636288.6 / 760670.4 / 760670.4 | 0.12 | report |
| disp_probe_d24 | 83261.4 / 88364.8 / 88364.8 | 633925.6 / 645437.4 / 645437.4 | 0.13 | report |
| disp_probe_d96 | 87127.4 / 92463.6 / 92463.6 | 630517.6 / 645741.5 / 645741.5 | 0.14 | report |
| disp_stream | 131.6 / 138.9 / 138.9 | 39147.0 / 43151.0 / 43151.0 | 0.00 | report |
| disp_stream_d24 | 147.8 / 166.9 / 166.9 | 39840.3 / 40779.2 / 40779.2 | 0.00 | report |
| disp_stream_d96 | 158.2 / 174.1 / 174.1 | 39791.0 / 40149.2 / 40149.2 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 5022.4 | 5025.4 | - |
| commit_batch | 24970.4 | 12877.7 | - |
| cold_containment_walk | 1233.4 | 85.7 | - |
| cold_containment_walk_delete | 3456.0 | 83.9 | - |
| commit_witnessed | 5172.2 | - | - |
| commit_window_baseline | 5095.0 | - | - |
| commit_window_admission | 5117.5 | - | - |
| commit_window_exclusion | 5154.6 | - | - |
| commit_capacity_baseline | 4587.3 | - | - |
| commit_capacity_sum | 5130.2 | - | - |
| commit_capacity_duration | 5141.2 | - | - |
| bulk | 1187128.0 | 687463.9 | 168043 |

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
| point | 3.28 | 3.28 | clean | - |
| containment_walk | 3.24 | 3.21 | retried | - |
| chain | 3.26 | 3.29 | clean | - |
| range | 3.28 | 3.35 | retried | - |
| balance | 3.41 | 3.41 | clean | - |
| stats | 3.32 | 3.34 | clean | - |
| string | 3.28 | 3.40 | clean | - |
| skew | 3.34 | 3.41 | clean | - |
| spread | 3.35 | 3.34 | clean | - |
| triangle | 3.41 | 3.35 | retried | - |
| entries_for_account_set | 3.36 | 3.41 | clean | - |
| postings_without_tag | 3.41 | 3.34 | clean | - |
| latest_posting_per_account | 3.36 | 3.34 | retried | - |
| mandate_at_instant | 3.40 | 3.40 | clean | - |
| mandate_overlap | 3.32 | 3.38 | clean | - |
| deep_chain | 3.40 | 3.35 | clean | - |
| busy_scan | 3.32 | 3.35 | clean | - |
| meets_chain | 3.41 | 3.27 | clean | - |
| rsvp_union | 3.30 | 3.35 | retried | - |
| conflict_pairs | 3.34 | 3.41 | clean | - |
| conflict_free | 3.41 | 3.26 | clean | - |
| free_busy | 3.28 | 3.28 | retried | - |
| claim_hours | 3.41 | 3.33 | clean | - |
| slot_scan | 3.41 | 3.35 | clean | - |
| slot_booking_overlap | 3.34 | 3.34 | retried | - |
| closure_depth | 3.28 | 3.41 | retried | - |
| closure_fanout | 3.35 | 3.23 | clean | - |
| disp_probe | 3.41 | 3.41 | clean | - |
| disp_probe_d24 | 3.21 | 3.28 | retried | - |
| disp_probe_d96 | 3.41 | 3.36 | clean | - |
| disp_stream | 3.33 | 3.38 | clean | - |
| disp_stream_d24 | 3.36 | 3.36 | clean | - |
| disp_stream_d96 | 3.36 | 3.34 | clean | - |
| commit_single | 3.41 | 0.91 | CONTAMINATED | - |
| commit_batch | 0.91 | 3.39 | CONTAMINATED | - |
| cold_containment_walk | 3.50 | 3.36 | clean | - |
| cold_containment_walk_delete | 3.33 | 3.41 | clean | - |
| commit_witnessed | 3.32 | 0.91 | CONTAMINATED | - |
| commit_window_baseline | 3.34 | 1.27 | CONTAMINATED | - |
| commit_window_admission | 1.26 | 0.91 | CONTAMINATED | - |
| commit_window_exclusion | 0.89 | 2.00 | CONTAMINATED | - |
| commit_capacity_baseline | 3.50 | 0.91 | CONTAMINATED | - |
| commit_capacity_sum | 0.91 | 1.25 | CONTAMINATED | - |
| commit_capacity_duration | 1.26 | 0.85 | CONTAMINATED | - |
| bulk | 0.75 | 3.36 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
