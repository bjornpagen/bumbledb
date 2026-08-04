# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 1 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## joins (geomean ratio 0.15 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 128 | 42.7 | 76.9 | 0.55 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | sqlite | 1207 | 23.5 | 340.6 | 0.07 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | sqlite | 197 | 70.3 | 199.8 | 0.35 | 3-way pinched by string point + year range |
| j4_five_way | sqlite | 2244 | 1170.0 | 4214.3 | 0.28 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | sqlite | 8 | 5046.4 | 30651.8 | 0.16 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | sqlite | 21089 | 724.3 | 36014.0 | 0.02 | fan-out explosion through shared keywords |

Overall geomean ratio across 6 queries: **0.15**.

## Flame summaries (per query, --trace)

### joins / j1_filmography

```text
span                       calls     total_us      self_us       p50_us       max_us
finalize                       1       36.708       36.708       36.708       36.708
join                           1        7.541        7.541        7.541        7.541
execute                        1       44.958        0.293       44.958       44.958
rule_0                         1        7.916        0.209        7.916        7.916
views                          1        0.125        0.125        0.125        0.125
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  1        0.000        0.000        0.000        0.000
total wall 44.958 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.291        291        0.291
jp_probe_n0               1        1.041       1041        1.041
jp_residual_n0            1        0.041         41        0.041
jp_descend_n0             1        4.500       4500        1.709
jp_force_n0               1        0.041         41        0.041
jp_descend_n1           128        2.791         21        2.791
```

### joins / j2_costars

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       22.791       22.791       22.791       22.791
finalize                       1        1.416        1.416        1.416        1.416
execute                        1       25.000        0.293       25.000       25.000
rule_0                         1       23.250        0.210       23.250       23.250
views                          1        0.208        0.208        0.208        0.208
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                  2        0.000        0.000        0.000        0.000
total wall 25.000 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.291        145        0.291
jp_probe_n0               2        1.166        583        1.166
jp_residual_n0            2        0.000          0        0.000
jp_descend_n0             2       19.708       9854        3.875
jp_force_n0               2        0.000          0        0.000
jp_descend_n1           145       15.833        109       15.833
```

### joins / j3_keyword_kind

```text
span                       calls     total_us      self_us       p50_us       max_us
finalize                       1       56.416       56.416       56.416       56.416
join                           1       15.916       15.916       15.916       15.916
bind_params                    1        0.458        0.458        0.458        0.458
execute                        1       73.541        0.292       73.541       73.541
rule_0                         1       16.375        0.210       16.375       16.375
views                          1        0.208        0.208        0.208        0.208
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass                  4        0.000        0.000        0.000        0.000
total wall 73.541 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.041         41        0.041
jp_probe_n0               1        0.125        125        0.125
jp_residual_n0            1        0.041         41        0.041
jp_descend_n0             1        0.083         83        0.000
jp_force_n0               1        0.000          0        0.000
jp_hash_n1                4        0.666        166        0.666
jp_probe_n1               4        4.458       1114        4.458
jp_residual_n1            4        0.041         10        0.041
jp_descend_n1             4        7.500       1875        3.000
jp_force_n1               4        0.000          0        0.000
jp_descend_n2           197        4.500         22        4.500
```

### joins / j4_five_way

```text
span                       calls     total_us      self_us       p50_us       max_us
finalize                       1      626.458      626.458      626.458      626.458
join                           1      563.750      563.750      563.750      563.750
execute                        1     1192.125        0.917     1192.125     1192.125
rule_0                         1      564.625        0.459      564.625      564.625
views                          1        0.250        0.250        0.250        0.250
resolve_filters                1        0.166        0.166        0.166        0.166
bind_params                    1        0.125        0.125        0.125        0.125
view_memo_hit                  5        0.000        0.000        0.000        0.000
select_probe                   5        0.000        0.000        0.000        0.000
prefetch_pass                158        0.000        0.000        0.000        0.000
total wall 1192.125 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0               15        3.291        219        3.291
jp_probe_n0              15       13.791        919       13.791
jp_residual_n0           15        0.250         16        0.250
jp_descend_n0            15       28.791       1919        0.000
jp_force_n0              15        0.041          2        0.041
jp_hash_n1               88       11.125        126       11.125
jp_probe_n1              88       51.458        584       51.458
jp_residual_n1           44        0.250          5        0.250
jp_descend_n1            44       11.083        251        2.833
jp_force_n1              88        0.125          1        0.125
jp_residual_n2           10        0.000          0        0.000
jp_descend_n2            10        8.250        825        0.000
jp_hash_n3               55       10.833        196       10.833
jp_probe_n3              55       64.250       1168       64.250
jp_residual_n3           55        0.625         11        0.625
jp_descend_n3            55       98.750       1795       32.750
jp_force_n3              55        0.125          2        0.125
jp_descend_n4          2246       66.000         29       66.000
```

### joins / j5_country_rollup

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     5150.833     5150.833     5150.833     5150.833
execute                        1     5154.833        3.292     5154.833     5154.833
rule_0                         1     5151.333        0.292     5151.333     5151.333
views                          1        0.208        0.208        0.208        0.208
finalize                       1        0.208        0.208        0.208        0.208
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass                782        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 5154.833 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              196       39.333        200       39.333
jp_probe_n0             196      178.666        911      178.666
jp_residual_n0          196        0.916          4        0.916
jp_descend_n0           196      297.125       1515        0.000
jp_force_n0             196        0.416          2        0.416
jp_hash_n1              586      115.333        196      115.333
jp_probe_n1             586      517.041        882      517.041
jp_residual_n1          586        1.500          2        1.500
jp_descend_n1           586     2687.000       4585      950.792
jp_force_n1             586        1.083          1        1.083
jp_descend_n2         74983     1736.208         23     1736.208
```

### joins / j6_keyword_neighborhood

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      689.041      689.041      689.041      689.041
finalize                       1       28.791       28.791       28.791       28.791
execute                        1      718.750        0.335      718.750      718.750
rule_0                         1      689.583        0.293      689.583      689.583
views                          1        0.166        0.166        0.166        0.166
resolve_filters                1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass                  8        0.000        0.000        0.000        0.000
total wall 718.750 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                2        0.250        125        0.250
jp_probe_n0               2        1.166        583        1.166
jp_residual_n0            2        0.041         20        0.041
jp_descend_n0             2        1.625        812        0.000
jp_force_n0               2        0.000          0        0.000
jp_hash_n1                7        1.291        184        1.291
jp_probe_n1               7        5.333        761        5.333
jp_residual_n1            7        0.000          0        0.000
jp_descend_n1             7      667.416      95345       16.041
jp_force_n1               7        0.041          5        0.041
jp_descend_n2           785      651.375        829      651.375
```

