# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## temporal (geomean ratio 0.02 over 4 timed, 1 DNF > cap — excluded and counted)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| t1_stab | sqlite | 796 | 0.5 | 5.6 | 0.09 | interval stabbing: point-in-span membership probe |
| t2_overlap_join | sqlite | 1 | 48609.8 | DNF>1000ms | — | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t2_overlap_join | sqlite-tuned | 1 | 48609.8 | 484810.6 | 0.10 | pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite |
| t3_mixed_mask | sqlite | 26543 | 18.5 | 1194.5 | 0.02 | mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data |
| t4_ray_stab | sqlite | 2247 | 45.5 | 4179.8 | 0.01 | open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter |
| t5_pack_key | sqlite-hand | 7 | 2.0 | 95.0 | 0.02 | Pack/coalesce: Snodgrass coalescing per key — SQLite's lane is the hand-written islands SQL (the free_busy precedent) |

Overall geomean ratio across 5 queries: **0.02**; 1 lane(s) DNF > cap (excluded, counted).

## Flame summaries (per query, --trace)

### temporal / t1_stab

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       15.083       15.083       15.083       15.083
finalize                       1        2.708        2.708        2.708        2.708
execute                        1       18.375        0.168       18.375       18.375
views                          1        0.166        0.166        0.166        0.166
rule_0                         1       15.458        0.127       15.458       15.458
selections                     1        0.041        0.041        0.041        0.041
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 18.375 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0               12        3.583        298        3.583
jp_residual_n0           10        0.000          0        0.000
jp_descend_n0            10       10.791       1079       10.791
```

### temporal / t2_overlap_join

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1    47358.958    47358.958    47358.958    47358.958
execute                        1    47360.875        1.168    47360.875    47360.875
views                          1        0.375        0.375        0.375        0.375
rule_0                         1    47359.666        0.250    47359.666    47359.666
selections                     1        0.083        0.083        0.083        0.083
finalize                       1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
prefetch_pass               1173        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 47360.875 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0             1173      223.333        190      223.333
jp_probe_n0            1173      961.125        819      961.125
jp_residual_n0         1173        4.666          3        4.666
jp_descend_n0          1173    45276.375      38598    14824.418
jp_force_n0            1173        2.833          2        2.833
jp_gather_n0           3519      379.541        107      379.541
jp_iter_n1           450888    22932.000         50    22932.000
jp_residual_n1       150820     5623.041         37     5623.041
jp_descend_n1         64017     1896.916         29     1896.916
```

### temporal / t3_mixed_mask

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       21.750       21.750       21.750       21.750
execute                        1       23.208        0.709       23.208       23.208
finalize                       1        0.250        0.250        0.250        0.250
rule_0                         1       22.208        0.168       22.208       22.208
views                          1        0.166        0.166        0.166        0.166
selections                     1        0.083        0.083        0.083        0.083
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  2        0.000        0.000        0.000        0.000
select_probe                   2        0.000        0.000        0.000        0.000
total wall 23.208 us

phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.166        166        0.166
jp_probe_n0               1        0.625        625        0.625
jp_residual_n0            1        0.125        125        0.125
jp_descend_n0             1       19.250      19250        7.918
jp_force_n0               1        0.166        166        0.166
jp_gather_n0              3        0.541        180        0.541
jp_iter_n1              234        7.541         32        7.541
jp_residual_n1           78        2.916         37        2.916
jp_descend_n1            11        0.875         79        0.875
```

### temporal / t4_ray_stab

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1       40.000       40.000       40.000       40.000
finalize                       1        8.916        8.916        8.916        8.916
views                          1        0.166        0.166        0.166        0.166
rule_0                         1       40.333        0.126       40.333       40.333
execute                        1       49.416        0.126       49.416       49.416
resolve_filters                1        0.041        0.041        0.041        0.041
bind_params                    1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
selections                     1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
total wall 49.416 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0               26        8.625        331        8.625
jp_residual_n0           24        0.125          5        0.125
jp_descend_n0            24       30.583       1274       30.583
```

### temporal / t5_pack_key

```text
span                       calls     total_us      self_us       p50_us       max_us
join                           1        1.500        1.500        1.500        1.500
finalize                       1        0.541        0.541        0.541        0.541
execute                        1        2.583        0.251        2.583        2.583
rule_0                         1        1.791        0.126        1.791        1.791
views                          1        0.083        0.083        0.083        0.083
selections                     1        0.041        0.041        0.041        0.041
resolve_filters                1        0.041        0.041        0.041        0.041
view_memo_hit                  1        0.000        0.000        0.000        0.000
select_probe                   1        0.000        0.000        0.000        0.000
bind_params                    1        0.000        0.000        0.000        0.000
total wall 2.583 us

phase                 calls     total_us     avg_ns      excl_us
jp_iter_n0                3        0.083         27        0.083
jp_residual_n0            1        0.041         41        0.041
jp_descend_n0             1        1.083       1083        1.083
```

