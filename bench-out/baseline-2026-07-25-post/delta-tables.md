# DELTA — baseline-2026-07-25-post vs baseline-2026-07-25

### bench-durable-r1 — 38 common cells
geomean(ratio_p50): post 0.0794 vs baseline 0.0845 → **0.940**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| read/containment_walk | 8416 → 1917 | 0.1593 → 0.0334 | 0.21 |
| read/conflict_pairs | 31375 → 24417 | 0.0112 → 0.0041 | 0.37 |
| read/postings_without_tag | 6958 → 2542 | 0.1470 → 0.0552 | 0.38 |
| read/closure_depth | 7459 → 5208 | 0.4476 → 0.3049 | 0.68 |
| read/free_busy | 4208 → 3083 | 0.0159 → 0.0115 | 0.72 |
| read/entries_for_account_set | 10000 → 7750 | 0.8421 → 0.6739 | 0.80 |
| read/conflict_free | 583 → 625 | 0.0245 → 0.0410 | 1.67 |
| read/slot_booking_overlap | 6709 → 12042 | 0.0101 → 0.0197 | 1.95 |
| write/cold_containment_walk_delete | 3521584 → 11369625 | 38.4176 → 119.4702 | 3.11 |

### bench-durable-r2 — 38 common cells
geomean(ratio_p50): post 0.0735 vs baseline 0.0846 → **0.869**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| read/closure_depth | 9834 → 1458 | 0.5210 → 0.0408 | 0.08 |
| read/containment_walk | 10042 → 2417 | 0.2049 → 0.0347 | 0.17 |
| read/postings_without_tag | 6834 → 2541 | 0.1553 → 0.0547 | 0.35 |
| read/entries_for_account_set | 4625 → 3500 | 0.5022 → 0.2675 | 0.53 |
| read/slot_booking_overlap | 13792 → 7667 | 0.0199 → 0.0111 | 0.56 |
| read/deep_chain | 395958 → 375584 | 0.1240 → 0.0827 | 0.67 |
| read/free_busy | 4042 → 3083 | 0.0146 → 0.0112 | 0.77 |
| read/skew | 1647542 → 1917458 | 0.2155 → 0.2507 | 1.16 |
| read/conflict_pairs | 27125 → 34375 | 0.0097 → 0.0120 | 1.24 |
| write/cold_containment_walk_delete | 3455333 → 11559333 | 41.4021 → 134.2813 | 3.24 |

### bench-durable-r3 — 38 common cells
geomean(ratio_p50): post 0.0713 vs baseline 0.0759 → **0.939**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| read/closure_fanout | 1000 → 583 | 0.1165 → 0.0043 | 0.04 |
| read/postings_without_tag | 7209 → 2459 | 0.1651 → 0.0556 | 0.34 |
| read/closure_depth | 2833 → 4166 | 0.2152 → 0.1098 | 0.51 |
| read/deep_chain | 373750 → 372542 | 0.1164 → 0.0820 | 0.70 |
| read/conflict_pairs | 23584 → 29084 | 0.0083 → 0.0100 | 1.20 |
| read/conflict_free | 625 → 625 | 0.0263 → 0.0396 | 1.51 |
| read/entries_for_account_set | 1292 → 3000 | 0.1240 → 0.2903 | 2.34 |
| write/cold_containment_walk_delete | 3456041 → 11461250 | 41.1840 → 130.8616 | 3.18 |

### bench-ephemeral-r1 — 38 common cells
geomean(ratio_p50): post 0.0719 vs baseline 0.0809 → **0.889**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| read/closure_fanout | 4583 → 500 | 0.4435 → 0.0625 | 0.14 |
| read/closure_depth | 2833 → 917 | 0.2030 → 0.0388 | 0.19 |
| read/entries_for_account_set | 5708 → 2875 | 0.7568 → 0.2760 | 0.36 |
| read/slot_booking_overlap | 10709 → 7458 | 0.0161 → 0.0109 | 0.68 |
| read/free_busy | 4208 → 3000 | 0.0149 → 0.0115 | 0.77 |
| read/postings_without_tag | 3291 → 2584 | 0.0748 → 0.0580 | 0.78 |
| write/bulk | 759799917 → 622524000 | 1.7070 → 1.3996 | 0.82 |
| write/cold_containment_walk_delete | 3474625 → 11601375 | 41.6542 → 140.1979 | 3.37 |

### bench-ephemeral-r2 — 38 common cells
geomean(ratio_p50): post 0.0796 vs baseline 0.0737 → **1.081**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| read/closure_fanout | 1084 → 500 | 0.1321 → 0.0411 | 0.31 |
| read/chain | 247458 → 192959 | 0.1333 → 0.1061 | 0.80 |
| read/conflict_free | 583 → 583 | 0.0300 → 0.0247 | 0.82 |
| write/bulk | 850398792 → 670126666 | 1.7808 → 1.4889 | 0.84 |
| read/containment_walk | 2333 → 1958 | 0.0471 → 0.0396 | 0.84 |
| read/free_busy | 2959 → 4125 | 0.0112 → 0.0155 | 1.38 |
| read/slot_booking_overlap | 6750 → 17166 | 0.0115 → 0.0268 | 2.33 |
| read/closure_depth | 4666 → 8875 | 0.1854 → 0.4561 | 2.46 |
| write/cold_containment_walk_delete | 3461250 → 11450875 | 42.7099 → 124.2419 | 2.91 |
| read/entries_for_account_set | 1291 → 6250 | 0.1802 → 0.8621 | 4.78 |

### bench-ephemeral-r3 — 38 common cells
geomean(ratio_p50): post 0.0723 vs baseline 0.0740 → **0.976**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| read/closure_depth | 4542 → 1042 | 0.2180 → 0.0476 | 0.22 |
| read/containment_walk | 6042 → 1959 | 0.1232 → 0.0316 | 0.26 |
| read/postings_without_tag | 6667 → 2667 | 0.1459 → 0.0510 | 0.35 |
| read/free_busy | 4166 → 3917 | 0.0167 → 0.0130 | 0.78 |
| read/deep_chain | 469333 → 382625 | 0.1453 → 0.1133 | 0.78 |
| read/conflict_free | 583 → 584 | 0.0305 → 0.0239 | 0.78 |
| write/bulk | 764709667 → 653945333 | 1.7286 → 1.4359 | 0.83 |
| read/entries_for_account_set | 1250 → 2792 | 0.1316 → 0.1530 | 1.16 |
| read/conflict_pairs | 33666 → 30417 | 0.0078 → 0.0107 | 1.37 |
| write/cold_containment_walk_delete | 3645417 → 11752875 | 38.2890 → 119.7245 | 3.13 |
| read/closure_fanout | 1000 → 4208 | 0.0293 → 0.2463 | 8.41 |

scenario DNFs: baseline ['rings/r4_bomb_t2', 'temporal/t2_overlap_join'] / post ['rings/r4_bomb_t2', 'temporal/t2_overlap_join']

### scenarios — 31 common cells
geomean(ratio_p50): post 0.0478 vs baseline 0.0542 → **0.882**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| graph/g6_weighted_hop | 750 → 334 | 0.1071 → 0.0353 | 0.33 |
| joins/j3_keyword_kind | 3625 → 1625 | 0.2544 → 0.0892 | 0.35 |
| temporal/t1_stab | 1125 → 459 | 0.2015 → 0.0847 | 0.42 |
| graph/g5_triangles_from | 625 → 625 | 0.0459 → 0.0259 | 0.56 |
| graph/g3_three_hop_count | 1292 → 1583 | 0.0682 → 0.0424 | 0.62 |
| joins/j1_filmography | 250 → 209 | 0.0423 → 0.0274 | 0.65 |
| graph/g1_neighbors | 250 → 209 | 0.0822 → 0.0627 | 0.76 |
| olap/o2_category_window | 405042 → 377458 | 0.0226 → 0.0178 | 0.79 |
| olap/o6_brand_drill | 1750 → 1792 | 0.0036 → 0.0031 | 0.86 |
| joins/j4_five_way | 1369125 → 1674500 | 0.3428 → 0.3051 | 0.89 |
| points/p5_keyed_get | 1041 → 958 | 0.7571 → 0.6761 | 0.89 |
| joins/j6_keyword_neighborhood | 26625 → 39792 | 0.0210 → 0.0239 | 1.14 |
| graph/g4_mutual | 2938750 → 4101833 | 0.1126 → 0.1311 | 1.16 |
| temporal/t4_ray_stab | 41042 → 45375 | 0.0092 → 0.0108 | 1.17 |
| rings/r6_two_path_count | 131327417 → 196619375 | 0.1999 → 0.2920 | 1.46 |
| graph/g2_two_hop | 250 → 750 | 0.0245 → 0.0545 | 2.22 |

### crud — 22 common cells
geomean(ratio_p50): post 1.6737 vs baseline 1.6382 → **1.022**

### lawful — 12 common cells
geomean(ratio_p50): post 2.8773 vs baseline 2.8123 → **1.023**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| durable/law_reject_scope | 20417 → 15834 | 2.8993 → 2.3900 | 0.82 |
| durable/law_reject_key | 4562500 → 4142666 | 311.9658 → 494.6467 | 1.59 |

### writes — 18 common cells
geomean(ratio_p50): post 1.2349 vs baseline 1.1188 → **1.104**

| cell | ours p50 base → post (ns) | ratio base → post | Δratio |
|---|---:|---:|---:|
| nosync/bulk_append | 742829833 → 649263916 | 1.7112 → 1.4163 | 0.83 |
| durable/delete_b100 | 13201292 → 14363083 | 1.5221 → 1.7514 | 1.15 |
| nosync/delete_b1000 | 10201375 → 11613792 | 0.7230 → 0.8464 | 1.17 |
| nosync/delete_b1 | 43416 → 54375 | 1.0698 → 1.3155 | 1.23 |
| nosync/commit_b1 | 51208 → 68000 | 1.4970 → 1.8567 | 1.24 |
| nosync/commit_b10 | 185959 → 270000 | 0.6894 → 0.9778 | 1.42 |
| nosync/commit_b100 | 1302000 → 1920292 | 0.6551 → 0.9409 | 1.44 |
