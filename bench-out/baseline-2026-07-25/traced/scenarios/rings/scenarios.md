# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 1 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## rings (geomean ratio 0.15 over 5 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| r1_wash_ring | sqlite | 1 | 10506.8 | 109504.5 | 0.10 | the equality 3-ring (wash-trade) over power-law hubs — the binary-join exponent, capped |
| r2_temporal_ring | sqlite | 1 | 32767.9 | 156321.8 | 0.21 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r2_temporal_ring | sqlite-tuned | 1 | 32767.9 | 108369.2 | 0.30 | the ring + pairwise Allen INTERSECTS — the temporal-ring shape |
| r3_bomb_t1 | sqlite | 1 | 3771.2 | 30964.5 | 0.12 | bipartite-bomb tier 1 (m=48): K_{m,m} + one planted triangle — answer 3 by construction; sized to finish within the cap |
| r4_bomb_t2 | sqlite | 1 | 1715351.8 | DNF>1000ms | — | bipartite-bomb tier 2 (m=384): m^3≈5.7e7 closing probes — the exponent evidence; SQLite predictably exceeds the cap, reported exceeded-cap, excluded and counted |
| r5_reciprocal | sqlite | 1808 | 499.7 | 3249.9 | 0.15 | the reciprocal-pair 2-cycle, kind-filtered |
| r6_two_path_count | sqlite | 1 | 127490.5 | 649037.1 | 0.20 | the denominator story: the distinct 2-path count binary joins must materialize |

Overall geomean ratio across 6 queries: **0.15**; 1 lane(s) DNF > cap (excluded, counted).

## Flame summaries (per query, --trace)

### rings / r1_wash_ring

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1    10522.458    10522.458    10522.458    10522.458
execute                        1    10526.583        3.334    10526.583    10526.583
rule_0                         1    10523.125        0.334    10523.125    10523.125
views                          1        0.250        0.250        0.250        0.250
resolve_filters                1        0.083        0.083        0.083        0.083
finalize                       1        0.083        0.083        0.083        0.083
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass               9927        0.000        0.000        0.000        0.000
total wall 10526.583 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              532       96.291        180       96.291
jp_probe_n0             532      678.833       1276      678.833
jp_residual_n0          266        2.375          8        2.375
jp_descend_n0           266      414.666       1558        0.000
jp_force_n0             532        1.041          1        1.041
jp_hash_n1            16099      623.500         38      623.500
jp_probe_n1           16099     2798.750        173     2798.750
jp_residual_n1        16099       29.875          1       29.875
jp_descend_n1         16099     1481.541         92      887.459
jp_iter_n2             8504       82.333          9       82.333
jp_residual_n2         8504       11.041          1       11.041
jp_descend_n2         18101      500.708         27      500.708
```

### rings / r2_temporal_ring

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1    34618.875    34618.875    34618.875    34618.875
bind_params                    1        3.541        3.541        3.541        3.541
execute                        1    34624.166        0.792    34624.166    34624.166
rule_0                         1    34619.708        0.459    34619.708    34619.708
views                          1        0.208        0.208        0.208        0.208
resolve_filters                1        0.166        0.166        0.166        0.166
finalize                       1        0.125        0.125        0.125        0.125
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass               1478        0.000        0.000        0.000        0.000
total wall 34624.166 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              532       97.041        182       97.041
jp_probe_n0             532      804.750       1512      804.750
jp_residual_n0          266        5.333         20        5.333
jp_descend_n0           266      483.666       1818        0.000
jp_force_n0             532        1.000          1        1.000
jp_hash_n1            19681      229.916         11      229.916
jp_probe_n1           19681      433.708         22      433.708
jp_residual_n1        33373    14884.416        446    14884.416
jp_descend_n1         33373      143.458          4       95.876
jp_iter_n2              886       18.541         20       18.541
jp_residual_n2          443       28.166         63       28.166
jp_descend_n2            10        0.875         87        0.875
```

### rings / r3_bomb_t1

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     3771.041     3771.041     3771.041     3771.041
execute                        1     3771.875        0.335     3771.875     3771.875
rule_0                         1     3771.416        0.209     3771.416     3771.416
views                          1        0.166        0.166        0.166        0.166
bind_params                    1        0.083        0.083        0.083        0.083
finalize                       1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass               1802        0.000        0.000        0.000        0.000
total wall 3771.875 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0               74       14.958        202       14.958
jp_probe_n0              74       60.541        818       60.541
jp_residual_n0           37        0.083          2        0.083
jp_descend_n0            37       64.666       1747        0.000
jp_force_n0              74        0.291          3        0.291
jp_hash_n1             1730      329.833        190      329.833
jp_probe_n1            1730     1620.041        936     1620.041
jp_residual_n1         1730        2.500          1        2.500
jp_descend_n1          1730        3.291          1        3.125
jp_descend_n2             3        0.166         55        0.166
```

### rings / r4_bomb_t2

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1  1704551.083  1704551.083  1704551.083  1704551.083
views                          1        3.000        3.000        3.000        3.000
execute                        1  1704557.750        1.835  1704557.750  1704557.750
rule_0                         1  1704555.166        1.083  1704555.166  1704555.166
finalize                       1        0.708        0.708        0.708        0.708
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass             889344        0.000        0.000        0.000        0.000
total wall 1704557.750 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0             4610      918.875        199      918.875
jp_probe_n0            4610     4150.208        900     4150.208
jp_residual_n0         2305       26.750         11       26.750
jp_descend_n0          2305     4135.125       1793        0.000
jp_force_n0            4610       45.333          9       45.333
jp_hash_n1           884737   165377.791        186   165377.791
jp_probe_n1          884737   734002.500        829   734002.500
jp_residual_n1       884737     1554.416          1     1554.416
jp_descend_n1        884737     1426.625          1     1424.750
jp_descend_n2             3        1.875        625        1.875
```

### rings / r5_reciprocal

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      523.291      523.291      523.291      523.291
finalize                       1        5.875        5.875        5.875        5.875
execute                        1      530.208        0.375      530.208      530.208
rule_0                         1      523.875        0.293      523.875      523.875
views                          1        0.250        0.250        0.250        0.250
bind_params                    1        0.083        0.083        0.083        0.083
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
prefetch_pass                408        0.000        0.000        0.000        0.000
total wall 530.208 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0               78       12.416        159       12.416
jp_probe_n0              78      101.916       1306      101.916
jp_residual_n0           39        0.333          8        0.333
jp_descend_n0            39       37.875        971        0.000
jp_force_n0              78        0.041          0        0.041
jp_hash_n1              776       15.250         19       15.250
jp_probe_n1             776       89.208        114       89.208
jp_residual_n1          776        2.000          2        2.000
jp_descend_n1           776       60.416         77       31.541
jp_iter_n2               52        0.375          7        0.375
jp_residual_n2           52        0.125          2        0.125
jp_descend_n2          1808       28.375         15       28.375
```

### rings / r6_two_path_count

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1   124636.625   124636.625   124636.625   124636.625
execute                        1   128039.583     3399.959   128039.583   128039.583
views                          1        1.458        1.458        1.458        1.458
rule_0                         1   124639.083        1.000   124639.083   124639.083
finalize                       1        0.500        0.500        0.500        0.500
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass                528        0.000        0.000        0.000        0.000
total wall 128039.584 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              528       98.875        187       98.875
jp_probe_n0             528      577.791       1094      577.791
jp_residual_n0          528        1.458          2        1.458
jp_descend_n0           528   123246.583     233421     8955.709
jp_force_n0             528        0.541          1        0.541
jp_iter_n1           148943     9328.500         62     9328.500
jp_residual_n1        91958      148.583          1      148.583
jp_descend_n1         99950   104813.791       1048   104813.791
```

