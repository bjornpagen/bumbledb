# writes lane — scale S, seed 1, samples 32

## lane `nosync` — sqlite `wal+synchronous=OFF`

| family | batch | ours p50 ns | sqlite p50 ns | ours commits/s | sqlite commits/s | ours rows/s | sqlite rows/s |
|---|---:|---:|---:|---:|---:|---:|---:|
| commit_b1 | 1 | 48458 | 33209 | 20011.6 | 26855.7 | 20011.6 | 26855.7 |
| commit_b10 | 10 | 189167 | 267875 | 5150.6 | 3597.1 | 51505.8 | 35971.5 |
| commit_b100 | 100 | 1530167 | 1918208 | 651.6 | 510.9 | 65161.4 | 51090.6 |
| commit_b1000 | 1000 | 8314959 | 10653208 | 120.1 | 92.2 | 120120.6 | 92163.3 |
| delete_b1 | 1 | 45292 | 29375 | 21378.0 | 33279.0 | 21378.0 | 33279.0 |
| delete_b10 | 10 | 187583 | 324000 | 5245.4 | 3298.0 | 52454.1 | 32980.1 |
| delete_b100 | 100 | 1566167 | 2924583 | 621.3 | 353.7 | 62128.5 | 35366.6 |
| delete_b1000 | 1000 | 11661042 | 14016375 | 86.0 | 70.4 | 85971.7 | 70395.9 |
| bulk_append | 4096 | 626237625 | 443233542 | 77.2 | 109.1 | 316143.6 | 446753.1 |

## lane `durable` — sqlite `wal+synchronous=FULL+fullfsync=ON`

| family | batch | ours p50 ns | sqlite p50 ns | ours commits/s | sqlite commits/s | ours rows/s | sqlite rows/s |
|---|---:|---:|---:|---:|---:|---:|---:|
| commit_b1 | 1 | 5150292 | 5149666 | 203.8 | 201.4 | 203.8 | 201.4 |
| commit_b10 | 10 | 7141000 | 5156125 | 142.3 | 192.7 | 1423.2 | 1926.7 |
| commit_b100 | 100 | 12982875 | 8425708 | 77.0 | 121.6 | 7703.3 | 12164.2 |
| commit_b1000 | 1000 | 32731167 | 15929083 | 31.4 | 62.3 | 31449.4 | 62343.7 |
| delete_b1 | 1 | 4462542 | 4702166 | 217.6 | 201.3 | 217.6 | 201.3 |
| delete_b10 | 10 | 5255583 | 5439625 | 171.6 | 179.8 | 1716.2 | 1798.3 |
| delete_b100 | 100 | 13241083 | 8338542 | 75.1 | 119.4 | 7512.2 | 11941.0 |
| delete_b1000 | 1000 | 42829041 | 19488125 | 23.4 | 53.0 | 23391.6 | 52979.0 |
| bulk_append | 4096 | 1069041542 | 679700583 | 45.2 | 71.5 | 185228.8 | 292673.6 |

## Flame summaries (per cell, --trace)

### nosync / commit_b1

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1       28.708       28.708       28.708       28.708
apply_inserts                  1       10.375       10.375       10.375       10.375
apply_deletes                  1        5.458        5.458        5.458        5.458
commit                         1       53.791        4.418       53.791       53.791
write_txn                      1       57.791        4.000       57.791       57.791
counters_flush                 1        2.416        2.416        2.416        2.416
judgment_source                1        2.333        2.333        2.333        2.333
judgment_target                1        0.083        0.083        0.083        0.083
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 57.791 us
```

### nosync / commit_b10

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1      112.291      112.291      112.291      112.291
apply_inserts                  1       91.958       91.958       91.958       91.958
write_txn                      1      273.250       29.667      273.250      273.250
judgment_source                1       23.750       23.750       23.750       23.750
commit                         1      243.583       10.793      243.583      243.583
counters_flush                 1        4.500        4.500        4.500        4.500
judgment_target                1        0.166        0.166        0.166        0.166
apply_deletes                  1        0.125        0.125        0.125        0.125
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 273.250 us
```

### nosync / commit_b100

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1      820.458      820.458      820.458      820.458
apply_inserts                  1      764.250      764.250      764.250      764.250
judgment_source                1      203.750      203.750      203.750      203.750
write_txn                      1     2023.333      186.250     2023.333     2023.333
commit                         1     1837.083       43.544     1837.083     1837.083
counters_flush                 1        4.791        4.791        4.791        4.791
apply_deletes                  1        0.166        0.166        0.166        0.166
judgment_target                1        0.083        0.083        0.083        0.083
judgment_capacities            1        0.041        0.041        0.041        0.041
total wall 2023.333 us
```

### nosync / commit_b1000

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1     4897.500     4897.500     4897.500     4897.500
lmdb_commit                    1     3402.750     3402.750     3402.750     3402.750
write_txn                      1    10875.500     1341.584    10875.500    10875.500
judgment_source                1      896.333      896.333      896.333      896.333
commit                         1     9533.916      326.459     9533.916     9533.916
counters_flush                 1       10.458       10.458       10.458       10.458
judgment_target                1        0.291        0.291        0.291        0.291
apply_deletes                  1        0.125        0.125        0.125        0.125
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 10875.500 us
```

### nosync / delete_b1

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1       31.791       31.791       31.791       31.791
apply_deletes                  1       11.541       11.541       11.541       11.541
write_txn                      1       51.958        3.417       51.958       51.958
commit                         1       48.541        2.794       48.541       48.541
counters_flush                 1        1.583        1.583        1.583        1.583
judgment_target                1        0.708        0.708        0.708        0.708
judgment_source                1        0.083        0.083        0.083        0.083
apply_inserts                  1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 51.958 us
```

### nosync / delete_b10

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1      107.500      107.500      107.500      107.500
apply_deletes                  1       86.458       86.458       86.458       86.458
write_txn                      1      223.208       16.792      223.208      223.208
commit                         1      206.416        6.584      206.416      206.416
judgment_target                1        3.500        3.500        3.500        3.500
counters_flush                 1        2.208        2.208        2.208        2.208
judgment_source                1        0.125        0.125        0.125        0.125
judgment_capacities            1        0.041        0.041        0.041        0.041
apply_inserts                  1        0.000        0.000        0.000        0.000
total wall 223.208 us
```

### nosync / delete_b100

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     1086.000     1086.000     1086.000     1086.000
apply_deletes                  1      952.666      952.666      952.666      952.666
write_txn                      1     2285.708      174.375     2285.708     2285.708
commit                         1     2111.333       40.960     2111.333     2111.333
judgment_target                1       27.583       27.583       27.583       27.583
counters_flush                 1        3.833        3.833        3.833        3.833
judgment_source                1        0.250        0.250        0.250        0.250
apply_inserts                  1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.000        0.000        0.000        0.000
total wall 2285.708 us
```

### nosync / delete_b1000

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_deletes                  1     6142.916     6142.916     6142.916     6142.916
lmdb_commit                    1     4531.583     4531.583     4531.583     4531.583
write_txn                      1    12585.208     1316.542    12585.208    12585.208
commit                         1    11268.666      326.794    11268.666    11268.666
judgment_target                1      262.666      262.666      262.666      262.666
counters_flush                 1        4.125        4.125        4.125        4.125
judgment_source                1        0.375        0.375        0.375        0.375
apply_inserts                  1        0.166        0.166        0.166        0.166
judgment_capacities            1        0.041        0.041        0.041        0.041
total wall 12585.208 us
```

### durable / commit_b1

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     4485.541     4485.541     4485.541     4485.541
apply_inserts                  1       49.916       49.916       49.916       49.916
commit                         1     4610.250       44.129     4610.250     4610.250
write_txn                      1     4653.208       42.958     4653.208     4653.208
counters_flush                 1       18.166       18.166       18.166       18.166
judgment_source                1       11.916       11.916       11.916       11.916
judgment_target                1        0.291        0.291        0.291        0.291
apply_deletes                  1        0.208        0.208        0.208        0.208
judgment_capacities            1        0.083        0.083        0.083        0.083
total wall 4653.208 us
```

### durable / commit_b10

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     7320.125     7320.125     7320.125     7320.125
apply_inserts                  1      282.958      282.958      282.958      282.958
write_txn                      1     7791.916       89.208     7791.916     7791.916
judgment_source                1       61.750       61.750       61.750       61.750
commit                         1     7702.708       23.169     7702.708     7702.708
counters_flush                 1       14.416       14.416       14.416       14.416
apply_deletes                  1        0.166        0.166        0.166        0.166
judgment_target                1        0.083        0.083        0.083        0.083
judgment_capacities            1        0.041        0.041        0.041        0.041
total wall 7791.916 us
```

### durable / commit_b100

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1    15209.041    15209.041    15209.041    15209.041
apply_inserts                  1     1540.333     1540.333     1540.333     1540.333
write_txn                      1    17546.500      389.417    17546.500    17546.500
judgment_source                1      316.458      316.458      316.458      316.458
commit                         1    17157.083       79.169    17157.083    17157.083
counters_flush                 1       11.708       11.708       11.708       11.708
apply_deletes                  1        0.250        0.250        0.250        0.250
judgment_target                1        0.083        0.083        0.083        0.083
judgment_capacities            1        0.041        0.041        0.041        0.041
total wall 17546.500 us
```

### durable / commit_b1000

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1    33207.208    33207.208    33207.208    33207.208
apply_inserts                  1     5326.000     5326.000     5326.000     5326.000
write_txn                      1    41342.000     1599.917    41342.000    41342.000
judgment_source                1      883.416      883.416      883.416      883.416
commit                         1    39742.083      314.628    39742.083    39742.083
counters_flush                 1       10.416       10.416       10.416       10.416
judgment_target                1        0.208        0.208        0.208        0.208
apply_deletes                  1        0.166        0.166        0.166        0.166
judgment_capacities            1        0.041        0.041        0.041        0.041
total wall 41342.000 us
```

### durable / delete_b1

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     4608.791     4608.791     4608.791     4608.791
apply_deletes                  1       91.500       91.500       91.500       91.500
commit                         1     4787.875       61.376     4787.875     4787.875
write_txn                      1     4844.625       56.750     4844.625     4844.625
counters_flush                 1       15.250       15.250       15.250       15.250
judgment_target                1        8.750        8.750        8.750        8.750
judgment_source                1        1.833        1.833        1.833        1.833
judgment_capacities            1        0.250        0.250        0.250        0.250
apply_inserts                  1        0.125        0.125        0.125        0.125
total wall 4844.625 us
```

### durable / delete_b10

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     9458.000     9458.000     9458.000     9458.000
apply_deletes                  1      605.458      605.458      605.458      605.458
write_txn                      1    10425.375      184.917    10425.375    10425.375
commit                         1    10240.458      118.127    10240.458    10240.458
judgment_target                1       31.916       31.916       31.916       31.916
counters_flush                 1       24.208       24.208       24.208       24.208
judgment_source                1        2.041        2.041        2.041        2.041
apply_inserts                  1        0.375        0.375        0.375        0.375
judgment_capacities            1        0.333        0.333        0.333        0.333
total wall 10425.375 us
```

### durable / delete_b100

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1    18375.041    18375.041    18375.041    18375.041
apply_deletes                  1     1888.375     1888.375     1888.375     1888.375
write_txn                      1    20852.125      447.625    20852.125    20852.125
commit                         1    20404.500       84.585    20404.500    20404.500
judgment_target                1       44.500       44.500       44.500       44.500
counters_flush                 1       10.500       10.500       10.500       10.500
judgment_source                1        1.250        1.250        1.250        1.250
judgment_capacities            1        0.208        0.208        0.208        0.208
apply_inserts                  1        0.041        0.041        0.041        0.041
total wall 20852.125 us
```

### durable / delete_b1000

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1    36678.291    36678.291    36678.291    36678.291
apply_deletes                  1     6032.083     6032.083     6032.083     6032.083
write_txn                      1    44908.666     1629.083    44908.666    44908.666
commit                         1    43279.583      300.003    43279.583    43279.583
judgment_target                1      264.666      264.666      264.666      264.666
counters_flush                 1        4.291        4.291        4.291        4.291
judgment_source                1        0.125        0.125        0.125        0.125
apply_inserts                  1        0.083        0.083        0.083        0.083
judgment_capacities            1        0.041        0.041        0.041        0.041
total wall 44908.666 us
```

