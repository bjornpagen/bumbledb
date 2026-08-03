# writes lane — scale S, seed 1, samples 4

## lane `nosync` — sqlite `wal+synchronous=OFF`

| family | batch | ours p50 ns | sqlite p50 ns | ours commits/s | sqlite commits/s | ours rows/s | sqlite rows/s |
|---|---:|---:|---:|---:|---:|---:|---:|
| commit_b1 | 1 | 54666 | 31041 | 18397.9 | 23250.4 | 18397.9 | 23250.4 |
| commit_b10 | 10 | 209750 | 270250 | 4778.7 | 3503.3 | 47787.4 | 35032.8 |
| commit_b100 | 100 | 1320292 | 1818125 | 732.2 | 504.1 | 73215.4 | 50407.5 |
| commit_b1000 | 1000 | 8255041 | 8137167 | 120.0 | 122.3 | 120000.3 | 122329.9 |
| delete_b1 | 1 | 42375 | 28750 | 23088.3 | 34421.0 | 23088.3 | 34421.0 |
| delete_b10 | 10 | 150417 | 229000 | 6548.0 | 4164.0 | 65479.7 | 41639.6 |
| delete_b100 | 100 | 1317250 | 1807625 | 763.4 | 505.4 | 76340.8 | 50535.6 |
| delete_b1000 | 1000 | 7943083 | 9463875 | 125.5 | 106.7 | 125487.3 | 106691.6 |
| bulk_append | 4096 | 731188917 | 445260792 | 66.5 | 109.2 | 272400.6 | 447322.8 |

## lane `durable` — sqlite `wal+synchronous=FULL+fullfsync=ON`

| family | batch | ours p50 ns | sqlite p50 ns | ours commits/s | sqlite commits/s | ours rows/s | sqlite rows/s |
|---|---:|---:|---:|---:|---:|---:|---:|
| commit_b1 | 1 | 5094416 | 4996834 | 172.9 | 199.9 | 172.9 | 199.9 |
| commit_b10 | 10 | 5096959 | 5283500 | 200.4 | 182.0 | 2003.9 | 1820.4 |
| commit_b100 | 100 | 17440750 | 6694459 | 56.0 | 147.0 | 5599.7 | 14702.8 |
| commit_b1000 | 1000 | 32686334 | 14863958 | 30.1 | 65.5 | 30140.8 | 65491.9 |
| delete_b1 | 1 | 5234291 | 4137833 | 188.9 | 230.7 | 188.9 | 230.7 |
| delete_b10 | 10 | 5394500 | 5092083 | 189.3 | 189.8 | 1892.9 | 1897.7 |
| delete_b100 | 100 | 13574125 | 7896334 | 74.6 | 120.5 | 7463.6 | 12052.9 |
| delete_b1000 | 1000 | 38642417 | 15541917 | 25.6 | 63.9 | 25648.9 | 63895.6 |
| bulk_append | 4096 | 1290429583 | 744940083 | 37.8 | 65.5 | 154681.5 | 268175.9 |

## Flame summaries (per cell, --trace)

### nosync / commit_b1

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1       34.000       34.000       34.000       34.000
apply_inserts                  1       11.500       11.500       11.500       11.500
apply_deletes                  1        9.583        9.583        9.583        9.583
write_txn                      1       68.958        4.833       68.958       68.958
judgment_source                1        3.458        3.458        3.458        3.458
commit                         1       64.125        3.084       64.125       64.125
counters_flush                 1        2.500        2.500        2.500        2.500
judgment_target                1        0.000        0.000        0.000        0.000
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 68.958 us
```

### nosync / commit_b10

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1       98.000       98.000       98.000       98.000
apply_inserts                  1       72.000       72.000       72.000       72.000
write_txn                      1      209.958       16.833      209.958      209.958
judgment_source                1       15.291       15.291       15.291       15.291
commit                         1      193.125        5.169      193.125      193.125
counters_flush                 1        2.583        2.583        2.583        2.583
judgment_capacities            1        0.041        0.041        0.041        0.041
apply_deletes                  1        0.041        0.041        0.041        0.041
judgment_target                1        0.000        0.000        0.000        0.000
total wall 209.958 us
```

### nosync / commit_b100

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1      607.750      607.750      607.750      607.750
lmdb_commit                    1      594.750      594.750      594.750      594.750
write_txn                      1     1487.500      134.542     1487.500     1487.500
judgment_source                1      119.708      119.708      119.708      119.708
commit                         1     1352.958       27.169     1352.958     1352.958
counters_flush                 1        3.416        3.416        3.416        3.416
apply_deletes                  1        0.083        0.083        0.083        0.083
judgment_target                1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.041        0.041        0.041        0.041
total wall 1487.500 us
```

### nosync / commit_b1000

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1     4401.708     4401.708     4401.708     4401.708
lmdb_commit                    1     2216.916     2216.916     2216.916     2216.916
write_txn                      1     9274.166     1302.291     9274.166     9274.166
judgment_source                1     1056.958     1056.958     1056.958     1056.958
commit                         1     7971.875      289.378     7971.875     7971.875
counters_flush                 1        6.625        6.625        6.625        6.625
judgment_capacities            1        0.208        0.208        0.208        0.208
judgment_target                1        0.041        0.041        0.041        0.041
apply_deletes                  1        0.041        0.041        0.041        0.041
total wall 9274.166 us
```

### nosync / delete_b1

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1       29.708       29.708       29.708       29.708
apply_deletes                  1        8.541        8.541        8.541        8.541
commit                         1       45.000        4.669       45.000       45.000
write_txn                      1       47.416        2.416       47.416       47.416
counters_flush                 1        1.083        1.083        1.083        1.083
judgment_target                1        0.875        0.875        0.875        0.875
judgment_source                1        0.083        0.083        0.083        0.083
judgment_capacities            1        0.041        0.041        0.041        0.041
apply_inserts                  1        0.000        0.000        0.000        0.000
total wall 47.416 us
```

### nosync / delete_b10

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1       79.083       79.083       79.083       79.083
apply_deletes                  1       56.208       56.208       56.208       56.208
write_txn                      1      157.750       10.542      157.750      157.750
counters_flush                 1        4.458        4.458        4.458        4.458
commit                         1      147.208        4.420      147.208      147.208
judgment_target                1        2.916        2.916        2.916        2.916
judgment_source                1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.041        0.041        0.041        0.041
apply_inserts                  1        0.041        0.041        0.041        0.041
total wall 157.750 us
```

### nosync / delete_b100

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_deletes                  1      612.916      612.916      612.916      612.916
lmdb_commit                    1      539.083      539.083      539.083      539.083
write_txn                      1     1340.583      132.708     1340.583     1340.583
commit                         1     1207.875       27.378     1207.875     1207.875
judgment_target                1       26.291       26.291       26.291       26.291
counters_flush                 1        2.083        2.083        2.083        2.083
judgment_source                1        0.083        0.083        0.083        0.083
apply_inserts                  1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 1340.583 us
```

### nosync / delete_b1000

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_deletes                  1     4741.333     4741.333     4741.333     4741.333
lmdb_commit                    1     2542.666     2542.666     2542.666     2542.666
write_txn                      1     9023.041     1215.541     9023.041     9023.041
commit                         1     7807.500      261.128     7807.500     7807.500
judgment_target                1      258.791      258.791      258.791      258.791
counters_flush                 1        3.375        3.375        3.375        3.375
judgment_source                1        0.166        0.166        0.166        0.166
judgment_capacities            1        0.041        0.041        0.041        0.041
apply_inserts                  1        0.000        0.000        0.000        0.000
total wall 9023.041 us
```

### durable / commit_b1

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     4736.375     4736.375     4736.375     4736.375
apply_inserts                  1       30.333       30.333       30.333       30.333
write_txn                      1     4798.000       13.625     4798.000     4798.000
counters_flush                 1        7.458        7.458        7.458        7.458
commit                         1     4784.375        5.377     4784.375     4784.375
judgment_source                1        4.750        4.750        4.750        4.750
judgment_target                1        0.041        0.041        0.041        0.041
apply_deletes                  1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 4798.000 us
```

### durable / commit_b10

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     5516.000     5516.000     5516.000     5516.000
apply_inserts                  1      190.333      190.333      190.333      190.333
write_txn                      1     5808.583       52.875     5808.583     5808.583
judgment_source                1       27.750       27.750       27.750       27.750
commit                         1     5755.708       12.376     5755.708     5755.708
counters_flush                 1        9.125        9.125        9.125        9.125
judgment_target                1        0.083        0.083        0.083        0.083
apply_deletes                  1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 5808.583 us
```

### durable / commit_b100

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1    13926.791    13926.791    13926.791    13926.791
apply_inserts                  1      804.208      804.208      804.208      804.208
write_txn                      1    15097.875      202.792    15097.875    15097.875
judgment_source                1      129.125      129.125      129.125      129.125
commit                         1    14895.083       29.295    14895.083    14895.083
counters_flush                 1        5.541        5.541        5.541        5.541
judgment_target                1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.041        0.041        0.041        0.041
apply_deletes                  1        0.041        0.041        0.041        0.041
total wall 15097.875 us
```

### durable / commit_b1000

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1    27023.000    27023.000    27023.000    27023.000
apply_inserts                  1     4944.208     4944.208     4944.208     4944.208
write_txn                      1    34975.125     1493.084    34975.125    34975.125
judgment_source                1     1076.666     1076.666     1076.666     1076.666
commit                         1    33482.041      430.544    33482.041    33482.041
counters_flush                 1        7.416        7.416        7.416        7.416
judgment_capacities            1        0.166        0.166        0.166        0.166
apply_deletes                  1        0.041        0.041        0.041        0.041
judgment_target                1        0.000        0.000        0.000        0.000
total wall 34975.125 us
```

### durable / delete_b1

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     5303.625     5303.625     5303.625     5303.625
apply_deletes                  1       27.250       27.250       27.250       27.250
write_txn                      1     5350.541        8.541     5350.541     5350.541
commit                         1     5342.000        5.460     5342.000     5342.000
counters_flush                 1        3.333        3.333        3.333        3.333
judgment_target                1        2.125        2.125        2.125        2.125
judgment_source                1        0.166        0.166        0.166        0.166
judgment_capacities            1        0.041        0.041        0.041        0.041
apply_inserts                  1        0.000        0.000        0.000        0.000
total wall 5350.541 us
```

### durable / delete_b10

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     4775.541     4775.541     4775.541     4775.541
apply_deletes                  1      171.750      171.750      171.750      171.750
write_txn                      1     5014.166       43.625     5014.166     5014.166
commit                         1     4970.541       11.169     4970.541     4970.541
judgment_target                1        7.083        7.083        7.083        7.083
counters_flush                 1        4.833        4.833        4.833        4.833
judgment_source                1        0.083        0.083        0.083        0.083
judgment_capacities            1        0.041        0.041        0.041        0.041
apply_inserts                  1        0.041        0.041        0.041        0.041
total wall 5014.166 us
```

### durable / delete_b100

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1    11749.250    11749.250    11749.250    11749.250
apply_deletes                  1      904.625      904.625      904.625      904.625
write_txn                      1    12957.833      235.000    12957.833    12957.833
commit                         1    12722.833       34.042    12722.833    12722.833
judgment_target                1       31.000       31.000       31.000       31.000
counters_flush                 1        3.750        3.750        3.750        3.750
judgment_source                1        0.125        0.125        0.125        0.125
apply_inserts                  1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 12957.833 us
```

### durable / delete_b1000

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1    32749.333    32749.333    32749.333    32749.333
apply_deletes                  1     5346.083     5346.083     5346.083     5346.083
write_txn                      1    40405.416     1758.458    40405.416    40405.416
judgment_target                1      273.250      273.250      273.250      273.250
commit                         1    38646.958      272.877    38646.958    38646.958
counters_flush                 1        4.583        4.583        4.583        4.583
judgment_capacities            1        0.416        0.416        0.416        0.416
judgment_source                1        0.375        0.375        0.375        0.375
apply_inserts                  1        0.041        0.041        0.041        0.041
total wall 40405.416 us
```

