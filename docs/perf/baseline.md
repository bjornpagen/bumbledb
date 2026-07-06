# Baseline capture — the denominator for every measured gate

One traced run (obs build, `bench --scale S --seed 1 --trace`) on the
reference host. Timing tables are untraced measurements from the same run;
phase tables come from one traced warm sample per family (the rotation may
have drawn a hot parameter set — gates in the PRDs say which number they
cite). Cold attribution at the bottom is from a separate
`trace --family fk_walk` capture at the same revision.

## Provenance


- crate version: 0.1.0
- engine rev: 80209ea938a6a314eba6fb98d9c5e7b5b57ccd9e
- timestamp: 2026-07-06T22:35:34Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples
- corpus digest: `12d08c93fe2b654aa74fbe1f1a5e84fa255e805e284e6d216ba1702f2ddc1af0`
- verify stamp: `fc05fee87ee002bbcd030659b93097ef2e86187a59dd50f9917e34060c6ca535 (families + 500 randomized cases)`


## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 1.1 / 1.2 / 1.3 | 1.4 / 1.6 / 1.9 | 0.79 | WIN |
| fk_walk | 12.8 / 1797.7 / 1856.8 | 64.0 / 31305.7 / 32039.2 | 0.20 | WIN |
| chain | 210.0 / 345.8 / 360.6 | 2159.9 / 4169.0 / 4331.9 | 0.10 | WIN |
| range | 59.1 / 68.5 / 74.8 | 557.2 / 618.6 / 676.4 | 0.11 | WIN |
| balance | 12.3 / 1110.2 / 1146.1 | 180.8 / 33122.0 / 34279.5 | 0.07 | WIN |
| stats | 4130.9 / 5012.7 / 5323.8 | 86813.5 / 91567.7 / 94080.0 | 0.05 | WIN |
| string | 1.8 / 2.1 / 6.2 | 7.8 / 10.2 / 18.3 | 0.23 | WIN |
| skew | 59.5 / 1880.3 / 2321.7 | 321.9 / 31154.2 / 31581.1 | 0.18 | WIN |
| spread | 13415.1 / 16698.1 / 17881.0 | 130747.1 / 139002.1 / 144138.5 | 0.10 | WIN |
| triangle | 17480.9 / 20147.5 / 21677.6 | 110262.3 / 117895.7 / 126595.0 | 0.16 | WIN |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 4651.9 | 4750.1 | - |
| commit_batch | 27008.4 | 29107.1 | - |
| cold_fk_walk | 6922.3 | 84.1 | - |
| bulk | 959317.7 | 738906.0 | 104325 |


## Flame + phase tables


### point

```text
span                       calls     total_us      self_us       p50_us       max_us
execute                        1        2.167        1.459        2.167        2.167
guard_probe                    1        0.625        0.625        0.625        0.625
finalize                       1        0.042        0.042        0.042        0.042
bind_params                    1        0.041        0.041        0.041        0.041
total wall 2.167 us
```

### fk_walk

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        3.875        3.875        3.875        3.875
execute                        1        9.917        3.042        9.917        9.917
finalize                       1        2.167        2.167        2.167        2.167
views                          1        0.667        0.667        0.667        0.667
resolve_filters                1        0.125        0.125        0.125        0.125
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
dict_resolve                   1        0.000        0.000        0.000        0.000
total wall 9.917 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                2        0.208        104        0.208
jp_hash_n0                2        0.166         83        0.166
jp_probe_n0               2        0.083         41        0.083
jp_residual_n0            1        0.000          0        0.000
jp_descend_n0             1        2.875       2875        0.250
jp_force_n0               2        0.041         20        0.041
jp_iter_n1                2        0.000          0        0.000
jp_residual_n1            1        0.000          0        0.000
jp_descend_n1             1        2.625       2625        0.167
jp_iter_n2                2        0.750        375        0.750
jp_residual_n2            1        0.000          0        0.000
jp_descend_n2             1        1.708       1708        1.708
```

### chain

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1      112.167      112.167      112.167      112.167
finalize                       1       14.792       14.792       14.792       14.792
execute                        1      131.209        3.709      131.209      131.209
views                          1        0.458        0.458        0.458        0.458
resolve_filters                1        0.083        0.083        0.083        0.083
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 131.209 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                2        0.416        208        0.416
jp_hash_n0                1        0.583        583        0.583
jp_probe_n0               1        2.083       2083        2.083
jp_residual_n0            1        0.000          0        0.000
jp_descend_n0             1      108.166     108166        5.584
jp_force_n0               1        0.041         41        0.041
jp_iter_n1              248        4.500         18        4.500
jp_hash_n1              124        1.625         13        1.625
jp_probe_n1             124       11.541         93       11.541
jp_residual_n1          124        0.000          0        0.000
jp_descend_n1           124       84.666        682       16.001
jp_force_n1             124        0.250          2        0.250
jp_iter_n2              769       29.916         38       29.916
jp_residual_n2          388        0.208          0        0.208
jp_descend_n2           388       38.541         99       38.541
```

### range

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       47.959       47.959       47.959       47.959
finalize                       1       10.625       10.625       10.625       10.625
execute                        1       62.000        2.917       62.000       62.000
views                          1        0.458        0.458        0.458        0.458
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 62.000 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0               17        8.333        490        8.333
jp_residual_n0           16        0.125          7        0.125
jp_descend_n0            16       38.666       2416       38.666
```

### balance

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     1050.708     1050.708     1050.708     1050.708
execute                        1     1054.167        2.792     1054.167     1054.167
views                          1        0.500        0.500        0.500        0.500
finalize                       1        0.125        0.125        0.125        0.125
resolve_filters                1        0.042        0.042        0.042        0.042
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 1054.167 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                2        0.166         83        0.166
jp_hash_n0                1        0.083         83        0.083
jp_probe_n0               1        0.208        208        0.208
jp_residual_n0            1        0.041         41        0.041
jp_descend_n0             1     1049.333    1049333        3.126
jp_force_n0               1        0.041         41        0.041
jp_iter_n1              407      271.791        667      271.791
jp_residual_n1          399        0.083          0        0.083
jp_descend_n1           399      774.333       1940      774.333
```

### stats

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     4356.542     4356.542     4356.542     4356.542
execute                        1     4409.666       48.499     4409.666     4409.666
resolve_filters                1        2.917        2.917        2.917        2.917
views                          1        1.458        1.458        1.458        1.458
finalize                       1        0.250        0.250        0.250        0.250
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 4409.666 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                5        2.125        425        2.125
jp_hash_n0                4        1.666        416        1.666
jp_probe_n0               4       15.416       3854       15.416
jp_residual_n0            4        0.000          0        0.000
jp_descend_n0             4     4336.458    1084114       47.584
jp_force_n0               4        0.000          0        0.000
jp_iter_n1             1536      653.708        425      653.708
jp_residual_n1         1024        0.250          0        0.250
jp_descend_n1          1024     3634.916       3549     3634.916
```

### string

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        1.500        1.500        1.500        1.500
execute                        1        2.958        0.583        2.958        2.958
bind_params                    1        0.417        0.417        0.417        0.417
views                          1        0.250        0.250        0.250        0.250
finalize                       1        0.167        0.167        0.167        0.167
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 2.958 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                2        0.125         62        0.125
jp_residual_n0            1        0.125        125        0.125
jp_descend_n0             1        0.666        666        0.666
```

### skew

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1     1108.583     1108.583     1108.583     1108.583
finalize                       1      617.458      617.458      617.458      617.458
execute                        1     1730.417        3.043     1730.417     1730.417
bind_params                    1        0.708        0.708        0.708        0.708
views                          1        0.542        0.542        0.542        0.542
resolve_filters                1        0.083        0.083        0.083        0.083
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
dict_resolve                   1        0.000        0.000        0.000        0.000
total wall 1730.417 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                2        0.125         62        0.125
jp_hash_n0                1        0.166        166        0.166
jp_probe_n0               1        0.041         41        0.041
jp_residual_n0            1        0.125        125        0.125
jp_descend_n0             1     1104.125    1104125        0.419
jp_force_n0               1        0.000          0        0.000
jp_iter_n1                2        0.166         83        0.166
jp_hash_n1                1        0.041         41        0.041
jp_probe_n1               1        0.083         83        0.083
jp_residual_n1            1        0.000          0        0.000
jp_descend_n1             1     1103.416    1103416        3.292
jp_force_n1               1        0.000          0        0.000
jp_iter_n2              399      213.666        535      213.666
jp_residual_n2          395        0.000          0        0.000
jp_descend_n2           395      886.458       2244      886.458
```

### spread

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1    13252.584    13252.584    13252.584    13252.584
finalize                       1      579.917      579.917      579.917      579.917
execute                        1    13881.084       47.875    13881.084    13881.084
views                          1        0.666        0.666        0.666        0.666
resolve_filters                1        0.042        0.042        0.042        0.042
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 13881.084 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0              783      336.916        430      336.916
jp_hash_n0              782      321.875        411      321.875
jp_probe_n0             782     2090.541       2673     2090.541
jp_residual_n0          782        0.583          0        0.583
jp_descend_n0           782    10484.250      13406     3626.751
jp_force_n0             782        6.958          8        6.958
jp_iter_n1           200000     3342.291         16     3342.291
jp_residual_n1       100000     1061.458         10     1061.458
jp_descend_n1        100000     2453.750         24     2453.750
```

### triangle

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1    18306.167    18306.167    18306.167    18306.167
execute                        1    18308.500        1.541    18308.500    18308.500
views                          1        0.625        0.625        0.625        0.625
finalize                       1        0.083        0.083        0.083        0.083
resolve_filters                1        0.042        0.042        0.042        0.042
bind_params                    1        0.042        0.042        0.042        0.042
view_memo_hit                  3        0.000        0.000        0.000        0.000
select_probe                   3        0.000        0.000        0.000        0.000
total wall 18308.500 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0              783      342.250        437      342.250
jp_hash_n0              782      333.125        425      333.125
jp_probe_n0             782     2125.958       2718     2125.958
jp_residual_n0          782        0.500          0        0.500
jp_descend_n0           782    15458.375      19767     4219.793
jp_force_n0             782       21.916         28       21.916
jp_iter_n1           199536     3078.250         15     3078.250
jp_hash_n1           100000     1529.875         15     1529.875
jp_probe_n1          100000     6005.083         60     6005.083
jp_residual_n1       100000       39.791          0       39.791
jp_descend_n1        100000      324.000          3      305.251
jp_force_n1          100000      261.583          2      261.583
jp_iter_n2              464        7.416         15        7.416
jp_residual_n2          464        0.083          0        0.083
jp_descend_n2           464       11.250         24       11.250
```


## Cold attribution (trace --family fk_walk, same revision)

```text
execute                   1       5869.5us
views                     1       4702.4us
view_build                3       4699.5us
image_build               3       4619.5us   <- 98% of the cold read
join                      1         14.2us
finalize                  1          8.0us
```

## The reading

- stats: jp_descend_n1 excl 3,634.9 us of 4,356.5 us join (83%) — per-row
  recursion bookkeeping + aggregate emit. iter_n1 653.7 us (~6.5 ns/pos).
- balance: descend_n1 774.3 us + iter_n1 271.8 us of 1,050.7 us join.
- spread: batch-of-1 leaf — iter_n1 3,342.3 us across 200,000 calls,
  residual_n1 1,061.5 us, descend_n1 2,453.8 us, n0 descend excl 3,626.8 us:
  ~10 ms of the 13.3 ms join is per-row framework, not data work.
  jp_probe_n0 2,090.5 us for 100k probes (21 ns/probe, batch 128 — MLP
  already engaged at the root).
- triangle: jp_probe_n1 6,005.1 us across 100,000 single-probe passes
  (60 ns each, serialized — no batch to overlap); n0 descend excl
  4,219.8 us; iter_n1 3,078.3 us at batch size 1. The WCOJ work is ~6 ms;
  the rest is framework at fanout-sized batches.
- skew: finalize 617.5 us of 1,730.4 us execute (36%); descend_n2 886.5 us
  for 50,412 folded rows (17.6 ns/row).
- chain: descend_n2 38.5 us + iter_n2 29.9 us for 1,914 emits; finalize
  14.8 us of 131.2 us.
- point: execute self 1.459 us vs guard_probe 0.625 us — fixed prologue
  costs dominate the actual probe.
