# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: ac2b538846beb6136cf5880dc9bc93bd35fda51a
- timestamp: 2026-08-03T17:31:51Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.16 2.25 2.27 (start) → 2.69 2.37 2.31 (end)
- config: scale S, seed 1, 8 samples, durable stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `7a1a4951e1d408b0515a1c37f7807a7152707e425d8862522750f576c0f71f38 (families + 500 randomized cases)`

## Gate verdict

PARTIAL — filtered run; the ALL-WIN claim needs every family.
p99 budget (<= 10 ms warm): FAIL (informational below scale L).
clock proxy: 1 block(s) still contaminated after retry — treat their percentiles as dirty: balance.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 0.3 / 0.3 / 0.3 | 1.4 / 1.4 / 1.4 | 0.19 | WIN |
| containment_walk | 2.2 / 622.6 / 622.6 | 45.8 / 28391.7 / 28391.7 | 0.05 | WIN |
| chain | 181.3 / 337.0 / 337.0 | 1683.5 / 3414.3 / 3414.3 | 0.11 | WIN |
| range | 19.6 / 23.7 / 23.7 | 142.9 / 556.1 / 556.1 | 0.14 | WIN |
| balance | 1.0 / 36.8 / 36.8 | 207.6 / 30580.0 / 30580.0 | 0.00 | WIN |
| stats | 1293.1 / 1296.2 / 1296.2 | 71802.9 / 73629.2 / 73629.2 | 0.02 | WIN |
| string | 2.5 / 2.7 / 2.7 | 55.5 / 58.3 / 58.3 | 0.04 | WIN |
| skew | 1539.2 / 2019.5 / 2019.5 | 6989.2 / 9348.7 / 9348.7 | 0.22 | WIN |
| spread | 10341.5 / 10379.2 / 10379.2 | 124987.5 / 126237.1 / 126237.1 | 0.08 | WIN |
| triangle | 2582.8 / 2616.1 / 2616.1 | 35022.6 / 37829.5 / 37829.5 | 0.07 | WIN |
| entries_for_account_set | 1.2 / 537.4 / 537.4 | 6.5 / 3940.2 / 3940.2 | 0.19 | WIN |
| postings_without_tag | 2.5 / 980.6 / 980.6 | 42.4 / 12689.9 / 12689.9 | 0.06 | WIN |
| latest_posting_per_account | 2252.8 / 2275.4 / 2275.4 | 40469.0 / 40770.9 / 40770.9 | 0.06 | WIN |
| mandate_at_instant | 0.3 / 0.3 / 0.3 | 8.2 / 8.5 / 8.5 | 0.04 | WIN |
| mandate_overlap | 15.7 / 19.2 / 19.2 | 407.2 / 475.6 / 475.6 | 0.04 | WIN |
| deep_chain | 354.3 / 643.2 / 643.2 | 2917.9 / 5843.5 / 5843.5 | 0.12 | report |
| busy_scan | 7.5 / 8.7 / 8.7 | 3385.0 / 3406.7 / 3406.7 | 0.00 | WIN |
| meets_chain | 3.1 / 811.7 / 811.7 | 17.8 / 132.0 / 132.0 | 0.18 | WIN |
| rsvp_union | 952.9 / 1032.9 / 1032.9 | 18140.4 / 18230.5 / 18230.5 | 0.05 | WIN |
| conflict_pairs | 23.6 / 95.1 / 95.1 | 2534.0 / 366626.5 / 366626.5 | 0.01 | WIN |
| conflict_free | 0.6 / 0.7 / 0.7 | 14.6 / 46.0 / 46.0 | 0.04 | WIN |
| free_busy | 3.0 / 41.8 / 41.8 | 232.2 / 2235.0 / 2235.0 | 0.01 | WIN |
| claim_hours | 440.8 / 448.7 / 448.7 | 6191.5 / 6290.1 / 6290.1 | 0.07 | WIN |
| slot_scan | 29.8 / 30.3 / 30.3 | 2785.2 / 2792.6 / 2792.6 | 0.01 | report |
| slot_booking_overlap | 6.6 / 58.8 / 58.8 | 557.5 / 14404.0 / 14404.0 | 0.01 | report |
| closure_depth | 3.0 / 1106.0 / 1106.0 | 8.8 / 1765.8 / 1765.8 | 0.34 | report |
| closure_fanout | 1.1 / 143.5 / 143.5 | 8.0 / 1959.1 / 1959.1 | 0.13 | report |
| disp_probe | 79013.4 / 81993.3 / 81993.3 | 615374.4 / 621245.0 / 621245.0 | 0.13 | report |
| disp_probe_d24 | 81095.6 / 82520.9 / 82520.9 | 615534.0 / 620808.0 / 620808.0 | 0.13 | report |
| disp_probe_d96 | 84315.6 / 90592.1 / 90592.1 | 615460.3 / 617050.4 / 617050.4 | 0.14 | report |
| disp_stream | 131.6 / 137.2 / 137.2 | 39817.1 / 41490.3 / 41490.3 | 0.00 | report |
| disp_stream_d24 | 140.2 / 140.9 / 140.9 | 38987.5 / 39161.6 / 39161.6 | 0.00 | report |
| disp_stream_d96 | 154.8 / 158.7 / 158.7 | 39259.9 / 39342.4 / 39342.4 | 0.00 | report |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|

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
| point | 3.38 | 3.43 | clean | - |
| containment_walk | 3.40 | 3.44 | clean | - |
| chain | 3.36 | 3.43 | retried | - |
| range | 3.44 | 3.31 | clean | - |
| balance | 3.37 | 3.10 | CONTAMINATED | - |
| stats | 3.38 | 3.46 | clean | - |
| string | 3.26 | 3.50 | retried | - |
| skew | 3.26 | 3.41 | clean | - |
| spread | 3.41 | 3.22 | clean | - |
| triangle | 3.31 | 3.29 | clean | - |
| entries_for_account_set | 3.51 | 3.50 | clean | - |
| postings_without_tag | 3.41 | 3.34 | clean | - |
| latest_posting_per_account | 3.36 | 3.36 | clean | - |
| mandate_at_instant | 3.44 | 3.30 | clean | - |
| mandate_overlap | 3.34 | 3.31 | retried | - |
| deep_chain | 3.28 | 3.28 | clean | - |
| busy_scan | 3.44 | 3.41 | clean | - |
| meets_chain | 3.41 | 3.38 | clean | - |
| rsvp_union | 3.50 | 3.34 | clean | - |
| conflict_pairs | 3.41 | 3.41 | clean | - |
| conflict_free | 3.41 | 3.42 | clean | - |
| free_busy | 3.48 | 3.36 | clean | - |
| claim_hours | 3.35 | 3.35 | clean | - |
| slot_scan | 3.41 | 3.41 | clean | - |
| slot_booking_overlap | 3.41 | 3.20 | clean | - |
| closure_depth | 3.50 | 3.41 | retried | - |
| closure_fanout | 3.50 | 3.41 | clean | - |
| disp_probe | 3.43 | 3.38 | clean | - |
| disp_probe_d24 | 3.51 | 3.41 | clean | - |
| disp_probe_d96 | 3.50 | 3.41 | clean | - |
| disp_stream | 3.41 | 3.41 | clean | - |
| disp_stream_d24 | 3.50 | 3.41 | clean | - |
| disp_stream_d96 | 3.50 | 3.41 | clean | - |

## Flame summaries

### point

```text
span                       calls     total_us      self_us       p50_us       max_us
bind_params                    1        8.083        8.083        8.083        8.083
execute                        1        9.250        0.834        9.250        9.250
key_probe                      1        0.333        0.333        0.333        0.333
total wall 9.250 us
```

### containment_walk

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        6.916        6.916        6.916        6.916
finalize                       1        0.791        0.791        0.791        0.791
execute                        1        9.291        0.668        9.291        9.291
rule_0                         1        7.791        0.584        7.791        7.791
views                          1        0.250        0.250        0.250        0.250
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
dict_resolve                   1        0.000        0.000        0.000        0.000
total wall 9.291 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.625        312        0.625
jp_probe_n0               2        0.416        208        0.416
jp_residual_n0            1        0.333        333        0.333
jp_descend_n0             1        0.208        208        0.000
jp_force_n0               2        0.041         20        0.041
jp_residual_n1            1        0.000          0        0.000
jp_descend_n1             1        2.291       2291        1.083
jp_descend_n2             1        1.208       1208        1.208
```

### chain

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       86.375       86.375       86.375       86.375
finalize                       1        6.000        6.000        6.000        6.000
execute                        1       97.333        3.667       97.333       97.333
views                          1        0.625        0.625        0.625        0.625
rule_0                         1       87.625        0.542       87.625       87.625
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass                 13        0.000        0.000        0.000        0.000
total wall 97.333 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.500        250        0.500
jp_probe_n0               2        2.750       1375        2.750
jp_residual_n0            2        0.291        145        0.291
jp_descend_n0             2        1.708        854        0.000
jp_force_n0               2        0.000          0        0.000
jp_hash_n1               11        2.000        181        2.000
jp_probe_n1              11        8.708        791        8.708
jp_residual_n1           11        0.041          3        0.041
jp_descend_n1            11       54.458       4950       19.083
jp_force_n1              11        0.000          0        0.000
jp_descend_n2          1328       35.375         26       35.375
```

### range

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       12.916       12.916       12.916       12.916
finalize                       1        5.666        5.666        5.666        5.666
execute                        1       21.416        2.167       21.416       21.416
rule_0                         1       13.500        0.293       13.500       13.500
views                          1        0.250        0.250        0.250        0.250
bind_params                    1        0.083        0.083        0.083        0.083
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 21.416 us

phase                 calls     total_us     avg_ns      excl_us
jp_descend_n0             1       12.708      12708       12.708
```

### balance

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       48.458       48.458       48.458       48.458
execute                        1       50.916        1.001       50.916       50.916
rule_0                         1       49.666        0.709       49.666       49.666
views                          1        0.458        0.458        0.458        0.458
finalize                       1        0.208        0.208        0.208        0.208
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  1        0.000        0.000        0.000        0.000
total wall 50.916 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.375        375        0.375
jp_probe_n0               1        0.333        333        0.333
jp_residual_n0            1        0.500        500        0.500
jp_descend_n0             1       45.416      45416        1.625
jp_force_n0               1        0.000          0        0.000
jp_descend_n1             7       43.791       6255       43.791
```

### stats

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     1238.583     1238.583     1238.583     1238.583
execute                        1     1306.250       63.834     1306.250     1306.250
views                          1        3.125        3.125        3.125        3.125
rule_0                         1     1242.250        0.542     1242.250     1242.250
finalize                       1        0.166        0.166        0.166        0.166
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  4        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 1306.250 us

phase                 calls     total_us     avg_ns      excl_us
jp_residual_n0            1        0.500        500        0.500
jp_descend_n0             1        0.291        291        0.000
jp_hash_n1                4        1.041        260        1.041
jp_probe_n1               4        3.416        854        3.416
jp_residual_n1            4        0.041         10        0.041
jp_descend_n1             4     1227.791     306947       47.959
jp_force_n1               4        0.000          0        0.000
jp_iter_n2             1394      150.208        107      150.208
jp_residual_n2          894        1.541          1        1.541
jp_descend_n2           894     1028.083       1149     1028.083
```

### string

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        5.166        5.166        5.166        5.166
execute                        1        7.833        1.208        7.833        7.833
rule_0                         1        5.875        0.502        5.875        5.875
finalize                       1        0.500        0.500        0.500        0.500
bind_params                    1        0.250        0.250        0.250        0.250
views                          1        0.166        0.166        0.166        0.166
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 7.833 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.333        333        0.333
jp_probe_n0               1        0.291        291        0.291
jp_residual_n0            1        0.458        458        0.458
jp_descend_n0             1        2.375       2375        0.667
jp_force_n0               1        0.000          0        0.000
jp_descend_n1             1        1.708       1708        1.708
```

### skew

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     1483.916     1483.916     1483.916     1483.916
finalize                       1       91.291       91.291       91.291       91.291
execute                        1     1603.750       27.668     1603.750     1603.750
rule_0                         1     1484.791        0.501     1484.791     1484.791
views                          1        0.333        0.333        0.333        0.333
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                235        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 1603.750 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              235       44.666        190       44.666
jp_probe_n0             235      200.166        851      200.166
jp_residual_n0          235        0.291          1        0.291
jp_descend_n0           235     1074.666       4573      356.250
jp_force_n0             235        0.208          0        0.208
jp_descend_n1         30066      718.416         23      718.416
```

### spread

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     9878.208     9878.208     9878.208     9878.208
finalize                       1      445.125      445.125      445.125      445.125
execute                        1    10397.750       73.168    10397.750    10397.750
rule_0                         1     9879.416        0.625     9879.416     9879.416
views                          1        0.583        0.583        0.583        0.583
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                782        0.000        0.000        0.000        0.000
total wall 10397.750 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              782      144.791        185      144.791
jp_probe_n0             782      846.208       1082      846.208
jp_residual_n0          782        2.375          3        2.375
jp_descend_n0           782     8272.625      10578     2740.167
jp_force_n0             782        1.375          1        1.375
jp_descend_n1        100000     5532.458         55     5532.458
```

### triangle

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     2671.958     2671.958     2671.958     2671.958
execute                        1     2682.291        9.417     2682.291     2682.291
rule_0                         1     2672.750        0.418     2672.750     2672.750
views                          1        0.291        0.291        0.291        0.291
resolve_filters                1        0.083        0.083        0.083        0.083
finalize                       1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass               1580        0.000        0.000        0.000        0.000
total wall 2682.291 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0             1564      304.541        194      304.541
jp_probe_n0            1564     1442.333        922     1442.333
jp_residual_n0          782        1.750          2        1.750
jp_descend_n0           782       17.458         22        0.000
jp_force_n0            1564        3.208          2        3.208
jp_hash_n1               16        2.375        148        2.375
jp_probe_n1              16       24.083       1505       24.083
jp_residual_n1           16        0.166         10        0.166
jp_descend_n1            16       23.250       1453       16.167
jp_iter_n2               79        0.500          6        0.500
jp_residual_n2           79        0.708          8        0.708
jp_descend_n2           529        5.875         11        5.875
```

### entries_for_account_set

```text
span                       calls     total_us      self_us       p50_us       max_us
execute                        1        5.875        3.376        5.875        5.875
join                           1        1.541        1.541        1.541        1.541
rule_0                         1        2.333        0.418        2.333        2.333
views                          1        0.333        0.333        0.333        0.333
finalize                       1        0.125        0.125        0.125        0.125
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 5.875 us

phase                 calls     total_us     avg_ns      excl_us
jp_descend_n0             1        1.000       1000        1.000
```

### postings_without_tag

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        4.625        4.625        4.625        4.625
execute                        1        5.791        0.376        5.791        5.791
rule_0                         1        5.166        0.334        5.166        5.166
views                          1        0.166        0.166        0.166        0.166
finalize                       1        0.166        0.166        0.166        0.166
bind_params                    1        0.083        0.083        0.083        0.083
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  1        0.000        0.000        0.000        0.000
total wall 5.791 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                2        0.291        145        0.291
jp_hash_n0                1        0.250        250        0.250
jp_probe_n0               1        1.625       1625        1.625
jp_residual_n0            1        0.333        333        0.333
jp_descend_n0             1        0.500        500        0.500
jp_force_n0               1        0.000          0        0.000
```

### latest_posting_per_account

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     2269.833     2269.833     2269.833     2269.833
finalize                       1        6.750        6.750        6.750        6.750
execute                        1     2278.666        1.542     2278.666     2278.666
rule_0                         1     2270.333        0.334     2270.333     2270.333
views                          1        0.166        0.166        0.166        0.166
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 2278.666 us

phase                 calls     total_us     avg_ns      excl_us
jp_residual_n0            4        0.458        114        0.458
jp_descend_n0             4     2265.833     566458       56.001
jp_iter_n1             1394      138.250         99      138.250
jp_residual_n1          894        1.416          1        1.416
jp_descend_n1           894     2070.166       2315     2070.166
```

### mandate_at_instant

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        3.208        3.208        3.208        3.208
execute                        1        4.375        0.459        4.375        4.375
rule_0                         1        3.875        0.418        3.875        3.875
views                          1        0.208        0.208        0.208        0.208
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
finalize                       1        0.000        0.000        0.000        0.000
total wall 4.375 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.291        291        0.291
jp_probe_n0               1        0.208        208        0.208
jp_residual_n0            1        0.458        458        0.458
jp_descend_n0             1        0.541        541        0.500
jp_force_n0               1        0.000          0        0.000
jp_descend_n1             1        0.041         41        0.041
```

### mandate_overlap

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       12.375       12.375       12.375       12.375
execute                        1       14.166        0.834       14.166       14.166
finalize                       1        0.500        0.500        0.500        0.500
views                          1        0.250        0.250        0.250        0.250
rule_0                         1       12.791        0.125       12.791       12.791
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  1        0.000        0.000        0.000        0.000
total wall 14.166 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.041         41        0.041
jp_probe_n0               1        0.291        291        0.291
jp_residual_n0            1        0.000          0        0.000
jp_descend_n0             1       11.041      11041        6.000
jp_force_n0               1        0.041         41        0.041
jp_iter_n1               52        1.375         26        1.375
jp_residual_n1           26        1.875         72        1.875
jp_descend_n1            26        1.791         68        1.791
```

### deep_chain

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      163.208      163.208      163.208      163.208
finalize                       1       34.166       34.166       34.166       34.166
execute                        1      204.583        6.584      204.583      204.583
views                          1        0.291        0.291        0.291        0.291
rule_0                         1      163.833        0.251      163.833      163.833
resolve_filters                1        0.083        0.083        0.083        0.083
view_memo_hit                  4        0.000        0.000        0.000        0.000
select_probe                   4        0.000        0.000        0.000        0.000
prefetch_pass                 22        0.000        0.000        0.000        0.000
dict_resolve                 121        0.000        0.000        0.000        0.000
total wall 204.583 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.375        375        0.375
jp_probe_n0               1        1.541       1541        1.541
jp_residual_n0            1        0.000          0        0.000
jp_descend_n0             1        2.166       2166        0.000
jp_force_n0               1        0.000          0        0.000
jp_hash_n1                4        0.875        218        0.875
jp_probe_n1               4        7.083       1770        7.083
jp_residual_n1            4        0.000          0        0.000
jp_descend_n1             4        5.541       1385        0.000
jp_force_n1               4        0.000          0        0.000
jp_hash_n2               17        3.125        183        3.125
jp_probe_n2              17       13.500        794       13.500
jp_residual_n2           17        0.083          4        0.083
jp_descend_n2            17       90.916       5348       30.041
jp_force_n2              17        0.000          0        0.000
jp_descend_n3          2000       60.875         30       60.875
```

### busy_scan

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        7.750        7.750        7.750        7.750
finalize                       1        1.666        1.666        1.666        1.666
execute                        1       10.833        1.001       10.833       10.833
rule_0                         1        8.166        0.209        8.166        8.166
views                          1        0.166        0.166        0.166        0.166
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 10.833 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                6        1.666        277        1.666
jp_residual_n0            5        0.083         16        0.083
jp_descend_n0             5        5.208       1041        5.208
```

### meets_chain

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      805.541      805.541      805.541      805.541
execute                        1      808.000        1.585      808.000      808.000
finalize                       1        0.541        0.541        0.541        0.541
rule_0                         1      805.833        0.126      805.833      805.833
views                          1        0.125        0.125        0.125        0.125
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  4        0.000        0.000        0.000        0.000
total wall 808.000 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                4        0.541        135        0.541
jp_probe_n0               4        3.791        947        3.791
jp_residual_n0            4        0.041         10        0.041
jp_descend_n0             4      798.041     199510       58.626
jp_force_n0               4        0.041         10        0.041
jp_iter_n1             2555      284.416        111      284.416
jp_residual_n1         2044      451.416        220      451.416
jp_descend_n1           170        3.583         21        3.583
```

### rsvp_union

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           3      642.000      642.000      212.875      218.000
finalize                       1      254.000      254.000      254.000      254.000
execute                        1      945.875       48.668      945.875      945.875
views                          3        0.624        0.624        0.083        0.458
rule_0                         1      218.791        0.333      218.791      218.791
rule_1                         1      213.125        0.167      213.125      213.125
rule_2                         1      211.291        0.083      211.291      211.291
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 945.875 us

phase                 calls     total_us     avg_ns      excl_us
jp_descend_n0             3      641.208     213736      641.208
```

### conflict_pairs

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      101.916      101.916      101.916      101.916
execute                        1      105.250        2.126      105.250      105.250
rule_0                         1      102.958        0.709      102.958      102.958
views                          1        0.250        0.250        0.250        0.250
finalize                       1        0.166        0.166        0.166        0.166
resolve_filters                1        0.083        0.083        0.083        0.083
view_memo_hit                  4        0.000        0.000        0.000        0.000
select_probe                   4        0.000        0.000        0.000        0.000
prefetch_pass                  3        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 105.250 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.291        145        0.291
jp_probe_n0               2        0.583        291        0.583
jp_residual_n0            1        0.291        291        0.291
jp_descend_n0             1        0.416        416        0.000
jp_force_n0               2        0.000          0        0.000
jp_hash_n1                1        0.125        125        0.125
jp_probe_n1               1        0.458        458        0.458
jp_residual_n1            1        0.000          0        0.000
jp_descend_n1             1        0.958        958        0.000
jp_force_n1               1        0.000          0        0.000
jp_residual_n2          100        0.250          2        0.250
jp_descend_n2           100       60.916        609       56.542
jp_iter_n3               82        0.958         11        0.958
jp_residual_n3           64        2.583         40        2.583
jp_descend_n3            64        0.833         13        0.833
```

### conflict_free

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        4.541        4.541        4.541        4.541
execute                        1        7.625        2.210        7.625        7.625
rule_0                         1        5.291        0.376        5.291        5.291
views                          1        0.333        0.333        0.333        0.333
bind_params                    1        0.083        0.083        0.083        0.083
resolve_filters                1        0.041        0.041        0.041        0.041
finalize                       1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass                  2        0.000        0.000        0.000        0.000
total wall 7.625 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.458        229        0.458
jp_probe_n0               2        0.583        291        0.583
jp_residual_n0            1        0.625        625        0.625
jp_descend_n0             1        0.750        750        0.542
jp_force_n0               2        0.083         41        0.083
jp_descend_n1             6        0.208         34        0.208
```

### free_busy

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       24.708       24.708       24.708       24.708
finalize                       1       21.208       21.208       21.208       21.208
execute                        1       48.000        1.376       48.000       48.000
rule_0                         1       25.375        0.418       25.375       25.375
views                          1        0.166        0.166        0.166        0.166
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  1        0.000        0.000        0.000        0.000
total wall 48.000 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.333        333        0.333
jp_probe_n0               1        0.375        375        0.375
jp_residual_n0            1        0.500        500        0.500
jp_descend_n0             1       21.583      21583        2.001
jp_force_n0               1        0.041         41        0.041
jp_iter_n1               21        2.041         97        2.041
jp_residual_n1           13        0.375         28        0.375
jp_descend_n1            13       17.166       1320       17.166
```

### claim_hours

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      444.166      444.166      444.166      444.166
execute                        1      445.708        1.043      445.708      445.708
views                          1        0.208        0.208        0.208        0.208
rule_0                         1      444.541        0.167      444.541      444.541
finalize                       1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 445.708 us

phase                 calls     total_us     avg_ns      excl_us
jp_residual_n0            1        0.083         83        0.083
jp_descend_n0             1      443.500     443500        3.668
jp_iter_n1              266       57.958        217       57.958
jp_residual_n1          264        0.291          1        0.291
jp_descend_n1           264      381.583       1445      381.583
```

### slot_scan

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       25.041       25.041       25.041       25.041
finalize                       1        6.916        6.916        6.916        6.916
execute                        1       33.541        1.042       33.541       33.541
rule_0                         1       25.583        0.251       25.583       25.583
views                          1        0.250        0.250        0.250        0.250
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 33.541 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0               18        4.916        273        4.916
jp_residual_n0           17        0.416         24        0.416
jp_descend_n0            17       18.083       1063       18.083
```

### slot_booking_overlap

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       63.625       63.625       63.625       63.625
execute                        1       65.791        0.959       65.791       65.791
finalize                       1        0.708        0.708        0.708        0.708
rule_0                         1       64.083        0.251       64.083       64.083
views                          1        0.166        0.166        0.166        0.166
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  4        0.000        0.000        0.000        0.000
total wall 65.791 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                4        0.500        125        0.500
jp_probe_n0               4        3.250        812        3.250
jp_residual_n0            4        0.166         41        0.166
jp_descend_n0             4       55.875      13968       45.543
jp_force_n0               4        0.000          0        0.000
jp_iter_n1              501        3.666          7        3.666
jp_residual_n1           91        3.166         34        3.166
jp_descend_n1            91        3.500         38        3.500
```

