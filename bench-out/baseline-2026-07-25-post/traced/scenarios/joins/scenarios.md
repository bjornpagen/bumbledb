# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## joins (geomean ratio 0.08 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 32 | 0.2 | 5.9 | 0.04 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | sqlite | 317 | 0.9 | 11.8 | 0.07 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | sqlite | 53 | 1.7 | 14.3 | 0.12 | 3-way pinched by string point + year range |
| j4_five_way | sqlite | 10171 | 775.4 | 4779.5 | 0.16 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | sqlite | 8 | 4794.6 | 29060.4 | 0.16 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | sqlite | 6807 | 29.5 | 1242.6 | 0.02 | fan-out explosion through shared keywords |

Overall geomean ratio across 6 queries: **0.08**.

## Flame summaries (per query, --trace)

### joins / j1_filmography

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        1.500        1.500        1.500        1.500
execute                        1        2.291        0.292        2.291        2.291
views                          1        0.166        0.166        0.166        0.166
rule_0                         1        1.875        0.127        1.875        1.875
finalize                       1        0.083        0.083        0.083        0.083
selections                     1        0.041        0.041        0.041        0.041
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 2.291 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.083         83        0.083
jp_probe_n0               1        0.166        166        0.166
jp_residual_n0            1        0.041         41        0.041
jp_descend_n0             1        0.250        250        0.167
jp_force_n0               1        0.000          0        0.000
jp_gather_n0              3        0.291         97        0.291
jp_descend_n1             2        0.083         41        0.083
```

### joins / j2_costars

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        1.500        1.500        1.500        1.500
execute                        1        2.083        0.210        2.083        2.083
views                          1        0.125        0.125        0.125        0.125
rule_0                         1        1.791        0.125        1.791        1.791
selections                     1        0.041        0.041        0.041        0.041
finalize                       1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
resolve_filters                1        0.000        0.000        0.000        0.000
total wall 2.083 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.041         41        0.041
jp_probe_n0               1        0.125        125        0.125
jp_residual_n0            1        0.000          0        0.000
jp_descend_n0             1        0.750        750        0.167
jp_force_n0               1        0.041         41        0.041
jp_gather_n0              3        0.208         69        0.208
jp_descend_n1             4        0.583        145        0.583
```

### joins / j3_keyword_kind

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        1.791        1.791        1.791        1.791
bind_params                    1        0.333        0.333        0.333        0.333
execute                        1        2.958        0.293        2.958        2.958
views                          1        0.166        0.166        0.166        0.166
finalize                       1        0.166        0.166        0.166        0.166
selections                     1        0.083        0.083        0.083        0.083
resolve_filters                1        0.083        0.083        0.083        0.083
rule_0                         1        2.166        0.043        2.166        2.166
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
total wall 2.958 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.041         41        0.041
jp_probe_n0               1        0.041         41        0.041
jp_residual_n0            1        0.041         41        0.041
jp_descend_n0             1        0.083         83        0.000
jp_force_n0               1        0.000          0        0.000
jp_gather_n0              3        0.291         97        0.291
jp_hash_n1                1        0.041         41        0.041
jp_probe_n1               1        0.250        250        0.250
jp_residual_n1            1        0.000          0        0.000
jp_descend_n1             1        0.250        250        0.084
jp_force_n1               1        0.000          0        0.000
jp_gather_n1              3        0.208         69        0.208
jp_descend_n2             7        0.166         23        0.166
```

### joins / j4_five_way

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     1967.375     1967.375     1967.375     1967.375
finalize                       1      345.583      345.583      345.583      345.583
execute                        1     2314.875        0.834     2314.875     2314.875
selections                     1        0.333        0.333        0.333        0.333
rule_0                         1     1968.375        0.293     1968.375     1968.375
views                          1        0.291        0.291        0.291        0.291
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.083        0.083        0.083        0.083
view_memo_hit                  5        0.000        0.000        0.000        0.000
select_probe                   5        0.000        0.000        0.000        0.000
total wall 2314.875 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0               56       10.958        195       10.958
jp_probe_n0              56       48.875        872       48.875
jp_residual_n0           56        0.333          5        0.333
jp_descend_n0            56       81.583       1456        0.000
jp_force_n0              56        0.125          2        0.125
jp_gather_n0            168       15.791         93       15.791
jp_hash_n1              336       40.250        119       40.250
jp_probe_n1             336      189.875        565      189.875
jp_residual_n1          168        0.416          2        0.416
jp_descend_n1           168       38.500        229        0.000
jp_force_n1             336        0.625          1        0.625
jp_gather_n1            556      250.625        450      250.625
jp_residual_n2           39        0.083          2        0.083
jp_descend_n2            39       30.125        772        0.000
jp_gather_n2            119       39.375        330       39.375
jp_hash_n3              223       41.041        184       41.041
jp_probe_n3             223      248.708       1115      248.708
jp_residual_n3          223        2.083          9        2.083
jp_descend_n3           223      430.041       1928      137.208
jp_force_n3             223        0.416          1        0.416
jp_gather_n3            669      288.208        430      288.208
jp_descend_n4          9430      292.833         31      292.833
```

### joins / j5_country_rollup

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     5069.125     5069.125     5069.125     5069.125
execute                        1     5070.541        0.542     5070.541     5070.541
views                          1        0.333        0.333        0.333        0.333
finalize                       1        0.208        0.208        0.208        0.208
selections                     1        0.166        0.166        0.166        0.166
rule_0                         1     5069.750        0.126     5069.750     5069.750
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass                782        0.000        0.000        0.000        0.000
total wall 5070.541 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              196       38.125        194       38.125
jp_probe_n0             196      174.375        889      174.375
jp_residual_n0          196        0.291          1        0.291
jp_descend_n0           196      294.125       1500        0.000
jp_force_n0             196        0.333          1        0.333
jp_gather_n0            588       48.916         83       48.916
jp_hash_n1              586      113.416        193      113.416
jp_probe_n1             586      505.958        863      505.958
jp_residual_n1          586        2.041          3        2.041
jp_descend_n1           586     2723.875       4648      963.167
jp_force_n1             586        0.833          1        0.833
jp_gather_n1           1936      831.875        429      831.875
jp_descend_n2         74983     1760.708         23     1760.708
```

### joins / j6_keyword_neighborhood

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       34.250       34.250       34.250       34.250
finalize                       1        4.125        4.125        4.125        4.125
execute                        1       39.416        0.459       39.416       39.416
views                          1        0.208        0.208        0.208        0.208
selections                     1        0.166        0.166        0.166        0.166
rule_0                         1       34.791        0.126       34.791       34.791
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
total wall 39.416 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.208        208        0.208
jp_probe_n0               1        0.166        166        0.166
jp_residual_n0            1        0.083         83        0.083
jp_descend_n0             1        0.208        208        0.000
jp_force_n0               1        0.000          0        0.000
jp_gather_n0              3        0.333        111        0.333
jp_hash_n1                1        0.083         83        0.083
jp_probe_n1               1        0.208        208        0.208
jp_residual_n1            1        0.000          0        0.000
jp_descend_n1             1       31.750      31750        0.959
jp_force_n1               1        0.000          0        0.000
jp_gather_n1              3        0.291         97        0.291
jp_descend_n2            30       30.791       1026       30.791
```

