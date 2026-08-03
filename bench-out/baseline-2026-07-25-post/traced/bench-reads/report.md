# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: 114701f561bbbc336bb2013b297ecbc503d51174
- timestamp: 2026-08-03T23:05:00Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 3.67 3.00 2.86 (start) → 3.28 3.11 2.94 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `dc701dacee6847b4fc3d1a0e33bd62dd2870f742d9df7cec4f59e33289cf78bc (families + 500 randomized cases)`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 11 block(s) still contaminated after retry — treat their percentiles as dirty: disp_probe_d24, commit_single, commit_batch, commit_witnessed, commit_window_baseline, commit_window_admission, commit_window_exclusion, commit_capacity_baseline, commit_capacity_sum, commit_capacity_duration, bulk.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.3 | 1.4 / 1.7 / 1.9 | 0.19 | WIN |
| containment_walk | 2.0 / 658.2 / 679.2 | 49.7 / 29371.7 / 31281.3 | 0.04 | WIN |
| chain | 194.2 / 355.2 / 376.0 | 2188.7 / 3569.0 / 3627.6 | 0.09 | WIN |
| range | 20.6 / 20.8 / 23.2 | 140.2 / 538.0 / 571.6 | 0.15 | WIN |
| balance | 1.1 / 33.5 / 33.6 | 278.3 / 31968.9 / 32548.8 | 0.00 | WIN |
| stats | 1414.5 / 1577.0 / 1686.2 | 74144.8 / 77108.1 / 80259.1 | 0.02 | WIN |
| string | 2.7 / 2.8 / 2.8 | 59.6 / 63.5 / 66.3 | 0.04 | WIN |
| skew | 1793.9 / 2426.9 / 2654.8 | 7617.5 / 10317.3 / 11750.9 | 0.24 | WIN |
| spread | 10607.0 / 11266.1 / 13230.7 | 126170.6 / 129023.9 / 139347.8 | 0.08 | WIN |
| triangle | 2616.2 / 2707.8 / 3036.8 | 36938.5 / 40085.2 / 41096.7 | 0.07 | WIN |
| entries_for_account_set | 2.8 / 694.5 / 726.7 | 15.8 / 4004.0 / 4070.5 | 0.17 | WIN |
| postings_without_tag | 2.5 / 1038.1 / 1060.5 | 43.9 / 12849.5 / 13072.4 | 0.06 | WIN |
| latest_posting_per_account | 2535.7 / 2622.3 / 2718.5 | 41471.8 / 43596.0 / 44287.1 | 0.06 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.0 / 8.3 / 9.5 | 0.04 | WIN |
| mandate_overlap | 13.9 / 14.8 / 14.9 | 413.2 / 447.9 / 465.5 | 0.03 | WIN |
| deep_chain | 368.2 / 628.4 / 671.3 | 3376.6 / 6382.3 / 6613.8 | 0.11 | report |
| busy_scan | 8.4 / 9.8 / 10.2 | 3403.0 / 3576.1 / 3755.6 | 0.00 | WIN |
| meets_chain | 3.0 / 118.6 / 119.8 | 17.5 / 130.2 / 135.5 | 0.17 | WIN |
| rsvp_union | 972.7 / 1006.7 / 1022.3 | 17992.2 / 18322.1 / 18993.2 | 0.05 | WIN |
| conflict_pairs | 34.5 / 86.2 / 96.0 | 2853.7 / 372020.5 / 377323.0 | 0.01 | WIN |
| conflict_free | 0.6 / 0.7 / 0.7 | 15.0 / 47.2 / 50.1 | 0.04 | WIN |
| free_busy | 3.1 / 40.0 / 40.5 | 273.9 / 2289.5 / 2363.5 | 0.01 | WIN |
| claim_hours | 436.4 / 461.8 / 474.6 | 6246.1 / 6528.7 / 9484.5 | 0.07 | WIN |
| slot_scan | 32.2 / 36.2 / 40.8 | 2830.5 / 3020.5 / 3099.9 | 0.01 | report |
| slot_booking_overlap | 19.6 / 64.1 / 73.0 | 680.7 / 14905.1 / 15154.9 | 0.03 | report |
| closure_depth | 1.1 / 1159.9 / 1179.4 | 15.0 / 1825.7 / 1858.4 | 0.07 | report |
| closure_fanout | 0.5 / 153.5 / 156.4 | 22.4 / 1956.0 / 2025.1 | 0.02 | report |
| disp_probe | 98212.9 / 110342.5 / 110342.5 | 666612.6 / 745792.1 / 745792.1 | 0.15 | report |
| disp_probe_d24 | 95405.3 / 110874.0 / 110874.0 | 654914.9 / 832179.8 / 832179.8 | 0.15 | report |
| disp_probe_d96 | 88310.2 / 95083.0 / 95083.0 | 634352.8 / 658109.2 / 658109.2 | 0.14 | report |
| disp_stream | 131.9 / 135.1 / 135.1 | 39482.9 / 40519.9 / 40519.9 | 0.00 | report |
| disp_stream_d24 | 144.6 / 160.1 / 160.1 | 39973.6 / 41255.3 / 41255.3 | 0.00 | report |
| disp_stream_d96 | 155.3 / 163.1 / 163.1 | 40332.8 / 40499.6 / 40499.6 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 4995.8 | 4702.5 | - |
| commit_batch | 24395.0 | 12898.2 | - |
| cold_containment_walk | 1181.1 | 89.8 | - |
| cold_containment_walk_delete | 11141.5 | 96.8 | - |
| commit_witnessed | 5099.9 | - | - |
| commit_window_baseline | 4646.9 | - | - |
| commit_window_admission | 5063.6 | - | - |
| commit_window_exclusion | 5113.6 | - | - |
| commit_capacity_baseline | 4244.6 | - | - |
| commit_capacity_sum | 5061.0 | - | - |
| commit_capacity_duration | 4827.5 | - | - |
| bulk | 1066193.1 | 680383.8 | 187847 |

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
- image cache: 7 images, 8049456 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| point | 3.34 | 3.41 | clean | - |
| containment_walk | 3.40 | 3.41 | clean | - |
| chain | 3.41 | 3.20 | clean | - |
| range | 3.26 | 3.41 | clean | - |
| balance | 3.41 | 3.41 | clean | - |
| stats | 3.36 | 3.41 | clean | - |
| string | 3.25 | 3.41 | clean | - |
| skew | 3.36 | 3.25 | clean | - |
| spread | 3.41 | 3.41 | clean | - |
| triangle | 3.41 | 3.41 | retried | - |
| entries_for_account_set | 3.29 | 3.41 | retried | - |
| postings_without_tag | 3.31 | 3.36 | clean | - |
| latest_posting_per_account | 3.36 | 3.41 | clean | - |
| mandate_at_instant | 3.41 | 3.41 | retried | - |
| mandate_overlap | 3.40 | 3.41 | clean | - |
| deep_chain | 3.41 | 3.35 | clean | - |
| busy_scan | 3.41 | 3.41 | clean | - |
| meets_chain | 3.41 | 3.35 | clean | - |
| rsvp_union | 3.41 | 3.36 | clean | - |
| conflict_pairs | 3.41 | 3.36 | clean | - |
| conflict_free | 3.36 | 3.41 | retried | - |
| free_busy | 3.41 | 3.32 | clean | - |
| claim_hours | 3.36 | 3.41 | clean | - |
| slot_scan | 3.23 | 3.40 | clean | - |
| slot_booking_overlap | 3.34 | 3.41 | clean | - |
| closure_depth | 3.29 | 3.41 | retried | - |
| closure_fanout | 3.41 | 3.40 | clean | - |
| disp_probe | 3.41 | 3.41 | clean | - |
| disp_probe_d24 | 3.15 | 3.34 | CONTAMINATED | - |
| disp_probe_d96 | 3.41 | 3.31 | clean | - |
| disp_stream | 3.41 | 3.35 | clean | - |
| disp_stream_d24 | 3.41 | 3.41 | clean | - |
| disp_stream_d96 | 3.41 | 3.36 | clean | - |
| commit_single | 3.41 | 0.76 | CONTAMINATED | - |
| commit_batch | 0.85 | 3.49 | CONTAMINATED | - |
| cold_containment_walk | 3.50 | 3.41 | clean | - |
| cold_containment_walk_delete | 3.33 | 3.36 | clean | - |
| commit_witnessed | 3.33 | 1.70 | CONTAMINATED | - |
| commit_window_baseline | 3.33 | 1.21 | CONTAMINATED | - |
| commit_window_admission | 1.28 | 1.28 | CONTAMINATED | - |
| commit_window_exclusion | 1.28 | 1.96 | CONTAMINATED | - |
| commit_capacity_baseline | 3.47 | 1.48 | CONTAMINATED | - |
| commit_capacity_sum | 2.42 | 0.82 | CONTAMINATED | - |
| commit_capacity_duration | 1.28 | 1.60 | CONTAMINATED | - |
| bulk | 1.28 | 3.50 | CONTAMINATED | - |

## Flame summaries

### point

```text
span                       calls     total_us      self_us       p50_us       max_us
bind_params                    1        5.666        5.666        5.666        5.666
execute                        1        7.041        1.042        7.041        7.041
key_probe                      1        0.333        0.333        0.333        0.333
total wall 7.041 us
```

### containment_walk

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        8.166        8.166        8.166        8.166
execute                        1       10.500        0.835       10.500       10.500
finalize                       1        0.458        0.458        0.458        0.458
views                          1        0.333        0.333        0.333        0.333
rule_0                         1        9.166        0.293        9.166        9.166
selections                     1        0.291        0.291        0.291        0.291
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
total wall 10.500 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.416        208        0.416
jp_probe_n0               2        0.333        166        0.333
jp_residual_n0            1        0.250        250        0.250
jp_descend_n0             1        0.291        291        0.000
jp_force_n0               2        0.083         41        0.083
jp_gather_n0              3        0.708        236        0.708
jp_residual_n1            1        0.000          0        0.000
jp_descend_n1             1        2.583       2583        1.208
jp_gather_n1              3        0.125         41        0.125
jp_descend_n2             1        1.375       1375        1.375
```

### chain

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       94.166       94.166       94.166       94.166
finalize                       1        6.000        6.000        6.000        6.000
execute                        1      104.166        2.875      104.166      104.166
rule_0                         1       95.250        0.376       95.250       95.250
views                          1        0.375        0.375        0.375        0.375
selections                     1        0.250        0.250        0.250        0.250
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
total wall 104.166 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.583        291        0.583
jp_probe_n0               2        2.791       1395        2.791
jp_residual_n0            2        0.333        166        0.333
jp_descend_n0             2        1.791        895        0.000
jp_force_n0               2        0.083         41        0.083
jp_gather_n0              6        0.958        159        0.958
jp_hash_n1               11        2.083        189        2.083
jp_probe_n1              11        9.041        821        9.041
jp_residual_n1           11        0.041          3        0.041
jp_descend_n1            11       61.583       5598       21.042
jp_force_n1              11        0.000          0        0.000
jp_gather_n1             33        8.250        250        8.250
jp_descend_n2          1328       40.541         30       40.541
```

### range

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       15.791       15.791       15.791       15.791
finalize                       1        5.500        5.500        5.500        5.500
execute                        1       27.458        5.250       27.458       27.458
rule_0                         1       16.625        0.335       16.625       16.625
views                          1        0.208        0.208        0.208        0.208
selections                     1        0.208        0.208        0.208        0.208
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.083        0.083        0.083        0.083
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 27.459 us

phase                 calls     total_us     avg_ns      excl_us
jp_descend_n0             1       15.250      15250       15.250
```

### balance

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       37.875       37.875       37.875       37.875
execute                        1       42.541        3.709       42.541       42.541
rule_0                         1       38.666        0.250       38.666       38.666
views                          1        0.208        0.208        0.208        0.208
selections                     1        0.208        0.208        0.208        0.208
resolve_filters                1        0.125        0.125        0.125        0.125
finalize                       1        0.125        0.125        0.125        0.125
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 42.541 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.375        375        0.375
jp_probe_n0               1        0.375        375        0.375
jp_residual_n0            1        0.291        291        0.291
jp_descend_n0             1       34.916      34916        1.875
jp_force_n0               1        0.083         83        0.083
jp_gather_n0              3        0.666        222        0.666
jp_descend_n1             7       33.041       4720       33.041
```

### stats

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     1412.333     1412.333     1412.333     1412.333
execute                        1     1419.083        5.876     1419.083     1419.083
rule_0                         1     1413.000        0.334     1413.000     1413.000
views                          1        0.250        0.250        0.250        0.250
finalize                       1        0.166        0.166        0.166        0.166
selections                     1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  4        0.000        0.000        0.000        0.000
total wall 1419.083 us

phase                 calls     total_us     avg_ns      excl_us
jp_residual_n0            1        0.250        250        0.250
jp_descend_n0             1        0.333        333        0.000
jp_gather_n0              3        0.708        236        0.708
jp_hash_n1                4        1.166        291        1.166
jp_probe_n1               4        3.625        906        3.625
jp_residual_n1            4        0.041         10        0.041
jp_descend_n1             4     1401.333     350333       49.126
jp_force_n1               4        0.000          0        0.000
jp_gather_n1             12        1.416        118        1.416
jp_iter_n2             1894      162.166         85      162.166
jp_residual_n2          894        3.208          3        3.208
jp_descend_n2           894     1186.833       1327     1186.833
```

### string

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        5.333        5.333        5.333        5.333
execute                        1        8.625        1.835        8.625        8.625
finalize                       1        0.458        0.458        0.458        0.458
rule_0                         1        6.041        0.376        6.041        6.041
bind_params                    1        0.291        0.291        0.291        0.291
views                          1        0.166        0.166        0.166        0.166
selections                     1        0.125        0.125        0.125        0.125
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 8.625 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.291        291        0.291
jp_probe_n0               1        0.333        333        0.333
jp_residual_n0            1        0.250        250        0.250
jp_descend_n0             1        2.625       2625        0.542
jp_force_n0               1        0.125        125        0.125
jp_gather_n0              3        0.583        194        0.583
jp_descend_n1             1        2.083       2083        2.083
```

### skew

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     2312.583     2312.583     2312.583     2312.583
finalize                       1      140.250      140.250      140.250      140.250
execute                        1     2457.333        3.292     2457.333     2457.333
selections                     1        0.375        0.375        0.375        0.375
views                          1        0.333        0.333        0.333        0.333
rule_0                         1     2313.750        0.293     2313.750     2313.750
resolve_filters                1        0.166        0.166        0.166        0.166
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 2457.333 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              313       59.833        191       59.833
jp_probe_n0             313      322.958       1031      322.958
jp_residual_n0          313        2.833          9        2.833
jp_descend_n0           313     1718.000       5488      519.334
jp_force_n0             313        0.375          1        0.375
jp_gather_n0            939       53.541         57       53.541
jp_descend_n1         40014     1198.666         29     1198.666
```

### spread

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1    10462.833    10462.833    10462.833    10462.833
finalize                       1      382.541      382.541      382.541      382.541
execute                        1    10853.291        6.209    10853.291    10853.291
views                          1        0.916        0.916        0.916        0.916
rule_0                         1    10464.416        0.584    10464.416    10464.416
bind_params                    1        0.125        0.125        0.125        0.125
selections                     1        0.083        0.083        0.083        0.083
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                782        0.000        0.000        0.000        0.000
total wall 10853.291 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              782      149.416        191      149.416
jp_probe_n0             782      852.500       1090      852.500
jp_residual_n0          782        5.041          6        5.041
jp_descend_n0           782     8909.708      11393     3093.667
jp_force_n0             782        1.708          2        1.708
jp_gather_n0           2346      183.750         78      183.750
jp_descend_n1        100000     5816.041         58     5816.041
```

### triangle

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     2619.958     2619.958     2619.958     2619.958
execute                        1     2622.500        1.335     2622.500     2622.500
views                          1        0.458        0.458        0.458        0.458
rule_0                         1     2620.916        0.334     2620.916     2620.916
finalize                       1        0.208        0.208        0.208        0.208
selections                     1        0.083        0.083        0.083        0.083
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
total wall 2622.500 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0             1564      311.083        198      311.083
jp_probe_n0            1564     1471.791        941     1471.791
jp_residual_n0          782        2.750          3        2.750
jp_descend_n0           782       12.458         15        0.000
jp_force_n0            1564        3.333          2        3.333
jp_gather_n0           2346      172.333         73      172.333
jp_hash_n1               16        2.416        151        2.416
jp_probe_n1              16       24.333       1520       24.333
jp_residual_n1           16        0.083          5        0.083
jp_descend_n1            16       24.458       1528       16.584
jp_gather_n1             48       24.708        514       24.708
jp_iter_n2              158        0.958          6        0.958
jp_residual_n2           79        0.666          8        0.666
jp_descend_n2           529        6.250         11        6.250
```

### entries_for_account_set

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        1.833        1.833        1.833        1.833
execute                        1        4.958        1.668        4.958        4.958
selections                     1        0.375        0.375        0.375        0.375
rule_0                         1        2.916        0.292        2.916        2.916
finalize                       1        0.291        0.291        0.291        0.291
views                          1        0.250        0.250        0.250        0.250
resolve_filters                1        0.166        0.166        0.166        0.166
bind_params                    1        0.083        0.083        0.083        0.083
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 4.958 us

phase                 calls     total_us     avg_ns      excl_us
jp_descend_n0             1        1.291       1291        1.291
```

### postings_without_tag

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        5.000        5.000        5.000        5.000
execute                        1        6.333        0.584        6.333        6.333
rule_0                         1        5.583        0.209        5.583        5.583
views                          1        0.208        0.208        0.208        0.208
selections                     1        0.125        0.125        0.125        0.125
finalize                       1        0.125        0.125        0.125        0.125
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 6.333 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                3        0.291         97        0.291
jp_hash_n0                1        0.291        291        0.291
jp_probe_n0               1        1.666       1666        1.666
jp_residual_n0            1        0.458        458        0.458
jp_descend_n0             1        0.458        458        0.458
jp_force_n0               1        0.000          0        0.000
```

### latest_posting_per_account

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     2534.708     2534.708     2534.708     2534.708
finalize                       1        6.833        6.833        6.833        6.833
execute                        1     2542.875        0.709     2542.875     2542.875
rule_0                         1     2535.333        0.334     2535.333     2535.333
views                          1        0.208        0.208        0.208        0.208
selections                     1        0.083        0.083        0.083        0.083
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 2542.875 us

phase                 calls     total_us     avg_ns      excl_us
jp_residual_n0            4        0.375         93        0.375
jp_descend_n0             4     2530.833     632708       49.834
jp_gather_n0             12        2.250        187        2.250
jp_iter_n1             1894      137.791         72      137.791
jp_residual_n1          894        3.000          3        3.000
jp_descend_n1           894     2340.208       2617     2340.208
```

### mandate_at_instant

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        3.666        3.666        3.666        3.666
rule_0                         1        5.000        0.585        5.000        5.000
execute                        1        5.625        0.543        5.625        5.625
views                          1        0.541        0.541        0.541        0.541
selections                     1        0.125        0.125        0.125        0.125
resolve_filters                1        0.083        0.083        0.083        0.083
finalize                       1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 5.625 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.541        541        0.541
jp_probe_n0               1        0.291        291        0.291
jp_residual_n0            1        0.583        583        0.583
jp_descend_n0             1        0.416        416        0.208
jp_force_n0               1        0.041         41        0.041
jp_gather_n0              3        0.583        194        0.583
jp_descend_n1             1        0.208        208        0.208
```

### mandate_overlap

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       12.750       12.750       12.750       12.750
finalize                       1        0.500        0.500        0.500        0.500
execute                        1       14.125        0.418       14.125       14.125
views                          1        0.166        0.166        0.166        0.166
rule_0                         1       13.166        0.126       13.166       13.166
selections                     1        0.083        0.083        0.083        0.083
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 14.125 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.041         41        0.041
jp_probe_n0               1        0.375        375        0.375
jp_residual_n0            1        0.000          0        0.000
jp_descend_n0             1       10.958      10958        4.376
jp_force_n0               1        0.041         41        0.041
jp_gather_n0              3        0.625        208        0.625
jp_iter_n1               78        2.416         30        2.416
jp_residual_n1           26        2.125         81        2.125
jp_descend_n1            26        2.041         78        2.041
```

### deep_chain

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      172.791      172.791      172.791      172.791
finalize                       1       21.416       21.416       21.416       21.416
execute                        1      202.125        6.918      202.125      202.125
views                          1        0.583        0.583        0.583        0.583
rule_0                         1      173.791        0.210      173.791      173.791
resolve_filters                1        0.166        0.166        0.166        0.166
selections                     1        0.041        0.041        0.041        0.041
view_memo_hit                  4        0.000        0.000        0.000        0.000
select_probe                   4        0.000        0.000        0.000        0.000
prefetch_pass                 22        0.000        0.000        0.000        0.000
total wall 202.125 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.500        500        0.500
jp_probe_n0               1        1.291       1291        1.291
jp_residual_n0            1        0.041         41        0.041
jp_descend_n0             1        1.666       1666        0.000
jp_force_n0               1        0.000          0        0.000
jp_gather_n0              3        0.833        277        0.833
jp_hash_n1                4        0.833        208        0.833
jp_probe_n1               4        6.625       1656        6.625
jp_residual_n1            4        0.000          0        0.000
jp_descend_n1             4        4.791       1197        0.000
jp_force_n1               4        0.000          0        0.000
jp_gather_n1             12        6.291        524        6.291
jp_hash_n2               17        3.041        178        3.041
jp_probe_n2              17       13.708        806       13.708
jp_residual_n2           17        0.125          7        0.125
jp_descend_n2            17      102.291       6017       32.625
jp_force_n2              17        0.000          0        0.000
jp_gather_n2             51       20.416        400       20.416
jp_descend_n3          2000       69.666         34       69.666
```

### busy_scan

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       10.166       10.166       10.166       10.166
finalize                       1        1.500        1.500        1.500        1.500
execute                        1       13.833        1.376       13.833       13.833
selections                     1        0.291        0.291        0.291        0.291
rule_0                         1       10.916        0.252       10.916       10.916
views                          1        0.166        0.166        0.166        0.166
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 13.834 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                7        1.541        220        1.541
jp_residual_n0            5        0.500        100        0.500
jp_descend_n0             5        6.291       1258        6.291
```

### meets_chain

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      130.416      130.416      130.416      130.416
execute                        1      133.458        1.584      133.458      133.458
finalize                       1        0.625        0.625        0.625        0.625
rule_0                         1      131.166        0.335      131.166      131.166
views                          1        0.166        0.166        0.166        0.166
selections                     1        0.166        0.166        0.166        0.166
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.083        0.083        0.083        0.083
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 133.458 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                4        0.750        187        0.750
jp_probe_n0               4        3.791        947        3.791
jp_residual_n0            4        0.250         62        0.250
jp_descend_n0             4      121.333      30333       38.668
jp_force_n0               4        0.083         20        0.083
jp_gather_n0             12        1.500        125        1.500
jp_iter_n1             1536       71.041         46       71.041
jp_residual_n1          514        7.916         15        7.916
jp_descend_n1           170        3.708         21        3.708
```

### rsvp_union

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           3      723.249      723.249      241.333      241.708
finalize                       1      245.000      245.000      245.000      245.000
execute                        1      971.333        1.959      971.333      971.333
selections                     3        0.374        0.374        0.083        0.208
views                          3        0.332        0.332        0.041        0.250
rule_0                         1      242.333        0.167      242.333      242.333
bind_params                    1        0.125        0.125        0.125        0.125
rule_1                         1      240.416        0.084      240.416      240.416
rule_2                         1      241.500        0.043      241.500      241.500
view_memo_hit                  3        0.000        0.000        0.000        0.000
total wall 971.333 us

phase                 calls     total_us     avg_ns      excl_us
jp_descend_n0             3      722.583     240861      722.583
```

### conflict_pairs

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       87.250       87.250       87.250       87.250
execute                        1       90.333        2.168       90.333       90.333
rule_0                         1       87.958        0.251       87.958       87.958
selections                     1        0.250        0.250        0.250        0.250
views                          1        0.166        0.166        0.166        0.166
finalize                       1        0.166        0.166        0.166        0.166
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  4        0.000        0.000        0.000        0.000
select_probe                   4        0.000        0.000        0.000        0.000
total wall 90.333 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.708        354        0.708
jp_probe_n0               2        0.583        291        0.583
jp_residual_n0            1        0.291        291        0.291
jp_descend_n0             1        0.291        291        0.000
jp_force_n0               2        0.041         20        0.041
jp_gather_n0              3        0.500        166        0.500
jp_hash_n1                1        0.166        166        0.166
jp_probe_n1               1        0.416        416        0.416
jp_residual_n1            1        0.000          0        0.000
jp_descend_n1             1        0.875        875        0.000
jp_force_n1               1        0.000          0        0.000
jp_gather_n1              3        0.583        194        0.583
jp_residual_n2          100        0.333          3        0.333
jp_descend_n2           100       61.041        610       31.708
jp_gather_n2            301       18.083         60       18.083
jp_iter_n3              165       24.250        146       24.250
jp_residual_n3           65        3.958         60        3.958
jp_descend_n3            64        1.125         17        1.125
```

### conflict_free

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       14.875       14.875       14.875       14.875
execute                        1       17.125        1.293       17.125       17.125
rule_0                         1       15.708        0.292       15.708       15.708
views                          1        0.250        0.250        0.250        0.250
selections                     1        0.250        0.250        0.250        0.250
bind_params                    1        0.083        0.083        0.083        0.083
resolve_filters                1        0.041        0.041        0.041        0.041
finalize                       1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
total wall 17.125 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.416        208        0.416
jp_probe_n0               2        0.833        416        0.833
jp_residual_n0            1        0.416        416        0.416
jp_descend_n0             1        0.625        625        0.459
jp_force_n0               2        0.125         62        0.125
jp_gather_n0              3        0.541        180        0.541
jp_descend_n1             6        0.166         27        0.166
```

### free_busy

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       25.416       25.416       25.416       25.416
finalize                       1       19.833       19.833       19.833       19.833
views                          1        0.583        0.583        0.583        0.583
execute                        1       46.708        0.459       46.708       46.708
selections                     1        0.166        0.166        0.166        0.166
rule_0                         1       26.333        0.127       26.333       26.333
bind_params                    1        0.083        0.083        0.083        0.083
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 46.708 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.333        333        0.333
jp_probe_n0               1        0.708        708        0.708
jp_residual_n0            1        0.458        458        0.458
jp_descend_n0             1       21.750      21750        1.877
jp_force_n0               1        0.000          0        0.000
jp_gather_n0              3        0.708        236        0.708
jp_iter_n1               29        2.166         74        2.166
jp_residual_n1           13        0.291         22        0.291
jp_descend_n1            13       17.416       1339       17.416
```

### claim_hours

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      450.000      450.000      450.000      450.000
execute                        1      452.000        1.084      452.000      452.000
rule_0                         1      450.666        0.375      450.666      450.666
finalize                       1        0.250        0.250        0.250        0.250
selections                     1        0.166        0.166        0.166        0.166
views                          1        0.125        0.125        0.125        0.125
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 452.000 us

phase                 calls     total_us     avg_ns      excl_us
jp_residual_n0            1        0.208        208        0.208
jp_descend_n0             1      448.208     448208        4.167
jp_gather_n0              3        0.666        222        0.666
jp_iter_n1              268       56.166        209       56.166
jp_residual_n1          264        0.750          2        0.750
jp_descend_n1           264      387.125       1466      387.125
```

### slot_scan

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       29.625       29.625       29.625       29.625
finalize                       1        6.458        6.458        6.458        6.458
execute                        1       37.375        0.585       37.375       37.375
rule_0                         1       30.291        0.293       30.291       30.291
views                          1        0.166        0.166        0.166        0.166
selections                     1        0.166        0.166        0.166        0.166
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 37.375 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0               19        5.125        269        5.125
jp_residual_n0           17        0.375         22        0.375
jp_descend_n0            17       22.083       1299       22.083
```

### slot_booking_overlap

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       68.291       68.291       68.291       68.291
execute                        1       71.583        1.751       71.583       71.583
finalize                       1        0.750        0.750        0.750        0.750
rule_0                         1       69.041        0.335       69.041       69.041
views                          1        0.166        0.166        0.166        0.166
selections                     1        0.166        0.166        0.166        0.166
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 71.583 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                4        0.666        166        0.666
jp_probe_n0               4        3.083        770        3.083
jp_residual_n0            4        0.375         93        0.375
jp_descend_n0             4       60.083      15020       29.417
jp_force_n0               4        0.083         20        0.083
jp_gather_n0             12        1.333        111        1.333
jp_iter_n1              913       23.583         25       23.583
jp_residual_n1           93        3.708         39        3.708
jp_descend_n1            91        3.375         37        3.375
```

### commit_window_baseline

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     5355.250     5355.250     5355.250     5355.250
commit                         1     5469.125       57.585     5469.125     5469.125
apply_inserts                  1       39.750       39.750       39.750       39.750
write_txn                      1     5500.916       31.791     5500.916     5500.916
counters_flush                 1       10.958       10.958       10.958       10.958
judgment_source                1        5.250        5.250        5.250        5.250
judgment_target                1        0.166        0.166        0.166        0.166
judgment_capacities            1        0.083        0.083        0.083        0.083
apply_deletes                  1        0.083        0.083        0.083        0.083
total wall 5500.916 us
```

### commit_window_admission

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     5424.083     5424.083     5424.083     5424.083
commit                         1     5541.041       46.586     5541.041     5541.041
apply_inserts                  1       45.666       45.666       45.666       45.666
write_txn                      1     5572.500       31.459     5572.500     5572.500
judgment_capacities            1       12.791       12.791       12.791       12.791
counters_flush                 1        6.041        6.041        6.041        6.041
judgment_source                1        5.208        5.208        5.208        5.208
judgment_target                1        0.458        0.458        0.458        0.458
apply_deletes                  1        0.208        0.208        0.208        0.208
total wall 5572.500 us
```

### commit_window_exclusion

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     5064.125     5064.125     5064.125     5064.125
write_txn                      1     5218.208       48.625     5218.208     5218.208
apply_inserts                  1       45.750       45.750       45.750       45.750
commit                         1     5169.583       43.502     5169.583     5169.583
judgment_source                1        5.708        5.708        5.708        5.708
counters_flush                 1        5.416        5.416        5.416        5.416
judgment_capacities            1        4.750        4.750        4.750        4.750
judgment_target                1        0.291        0.291        0.291        0.291
apply_deletes                  1        0.041        0.041        0.041        0.041
total wall 5218.208 us
```

### commit_capacity_baseline

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     5783.416     5783.416     5783.416     5783.416
write_txn                      1     5914.208       49.708     5914.208     5914.208
commit                         1     5864.500       42.670     5864.500     5864.500
apply_inserts                  1       31.041       31.041       31.041       31.041
counters_flush                 1        5.541        5.541        5.541        5.541
judgment_source                1        1.041        1.041        1.041        1.041
judgment_target                1        0.500        0.500        0.500        0.500
apply_deletes                  1        0.250        0.250        0.250        0.250
judgment_capacities            1        0.041        0.041        0.041        0.041
total wall 5914.208 us
```

### commit_capacity_sum

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     5446.083     5446.083     5446.083     5446.083
apply_inserts                  1       49.166       49.166       49.166       49.166
write_txn                      1     5623.291       46.000     5623.291     5623.291
commit                         1     5577.291       45.418     5577.291     5577.291
counters_flush                 1       22.083       22.083       22.083       22.083
judgment_source                1        7.208        7.208        7.208        7.208
judgment_capacities            1        6.625        6.625        6.625        6.625
judgment_target                1        0.500        0.500        0.500        0.500
apply_deletes                  1        0.208        0.208        0.208        0.208
total wall 5623.291 us
```

### commit_capacity_duration

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     4275.083     4275.083     4275.083     4275.083
write_txn                      1     4384.583       35.250     4384.583     4384.583
apply_inserts                  1       27.500       27.500       27.500       27.500
commit                         1     4349.333       22.876     4349.333     4349.333
counters_flush                 1       14.166       14.166       14.166       14.166
judgment_source                1        4.833        4.833        4.833        4.833
judgment_capacities            1        4.375        4.375        4.375        4.375
judgment_target                1        0.250        0.250        0.250        0.250
apply_deletes                  1        0.250        0.250        0.250        0.250
total wall 4384.583 us
```

