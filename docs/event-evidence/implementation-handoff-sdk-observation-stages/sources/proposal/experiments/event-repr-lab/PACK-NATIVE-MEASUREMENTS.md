# Native Pack factorization measurements

Same executable, input Events, scalar rows and outputs. Each row holds the carrier
fixed and compares complete bindings with certified branch reductions.
Times are 7-sample medians in milliseconds; speedup is complete / factorized.
Raw samples and min/max ranges remain in the linked results.

| Carrier | Coup presentation | Fanout | Memo | Fresh complete | Fresh factored | Speedup | Warm complete | Warm factored | Speedup |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| dense | coup_4290 | 2 | False | 0.3195 | 0.1271 | 2.51× | 0.3243 | 0.1213 | 2.67× |
| dense | coup_4290 | 2 | True | 0.1631 | 0.0885 | 1.84× | 0.0424 | 0.0604 | 0.70× |
| dense | coup_4290 | 4 | False | 2.6371 | 0.2161 | 12.20× | 2.5909 | 0.1980 | 13.09× |
| dense | coup_4290 | 4 | True | 1.1035 | 0.1187 | 9.30× | 0.2760 | 0.0700 | 3.94× |
| dense | coup_4290 | 8 | False | 20.3508 | 0.2977 | 68.37× | 20.3736 | 0.2808 | 72.57× |
| dense | coup_4290 | 8 | True | 6.9977 | 0.1828 | 38.27× | 2.4123 | 0.1058 | 22.80× |
| dense | coup_product_65536 | 2 | False | 3.4217 | 0.9323 | 3.67× | 3.6013 | 0.8388 | 4.29× |
| dense | coup_product_65536 | 2 | True | 1.3580 | 0.3895 | 3.49× | 0.0558 | 0.0806 | 0.69× |
| dense | coup_product_65536 | 4 | False | 28.4007 | 1.4832 | 19.15× | 27.8245 | 1.5310 | 18.17× |
| dense | coup_product_65536 | 4 | True | 8.0999 | 0.6058 | 13.37× | 0.2641 | 0.0774 | 3.41× |
| dense | coup_product_65536 | 8 | False | 470.1020 | 2.3015 | 204.26× | 1085.6264 | 2.2685 | 478.57× |
| dense | coup_product_65536 | 8 | True | 154.1568 | 0.9551 | 161.41× | 7.3062 | 0.1162 | 62.87× |
| packed512 | coup_4290 | 2 | False | 0.7214 | 0.2409 | 2.99× | 0.0924 | 0.1422 | 0.65× |
| packed512 | coup_4290 | 2 | True | 0.6935 | 0.2220 | 3.12× | 0.0911 | 0.1495 | 0.61× |
| packed512 | coup_4290 | 4 | False | 3.4866 | 0.3000 | 11.62× | 0.4621 | 0.1110 | 4.16× |
| packed512 | coup_4290 | 4 | True | 3.5348 | 0.2966 | 11.92× | 0.3534 | 0.1054 | 3.35× |
| packed512 | coup_4290 | 8 | False | 18.2788 | 0.4547 | 40.20× | 3.3630 | 0.1519 | 22.15× |
| packed512 | coup_4290 | 8 | True | 18.8607 | 0.4244 | 44.44× | 2.7064 | 0.1544 | 17.53× |
| packed512 | coup_product_65536 | 2 | False | 2.3840 | 0.6540 | 3.65× | 0.2253 | 0.2301 | 0.98× |
| packed512 | coup_product_65536 | 2 | True | 2.3072 | 0.6972 | 3.31× | 0.2068 | 0.2247 | 0.92× |
| packed512 | coup_product_65536 | 4 | False | 10.4319 | 0.8101 | 12.88× | 0.5964 | 0.2217 | 2.69× |
| packed512 | coup_product_65536 | 4 | True | 10.3922 | 0.7227 | 14.38× | 0.4946 | 0.2173 | 2.28× |
| packed512 | coup_product_65536 | 8 | False | 45.4670 | 1.1651 | 39.02× | 3.5544 | 0.2346 | 15.15× |
| packed512 | coup_product_65536 | 8 | True | 43.9877 | 1.0632 | 41.37× | 2.7613 | 0.2241 | 12.32× |
| essential512 | coup_4290 | 2 | False | 5.0757 | 1.2054 | 4.21× | 0.0605 | 0.0674 | 0.90× |
| essential512 | coup_4290 | 2 | True | 5.2588 | 1.2342 | 4.26× | 0.0420 | 0.0617 | 0.68× |
| essential512 | coup_4290 | 4 | False | 30.7574 | 1.7876 | 17.21× | 0.4571 | 0.0805 | 5.68× |
| essential512 | coup_4290 | 4 | True | 31.0147 | 1.6923 | 18.33× | 0.2959 | 0.0709 | 4.17× |
| essential512 | coup_4290 | 8 | False | 138.7820 | 2.5779 | 53.84× | 3.6847 | 0.1163 | 31.69× |
| essential512 | coup_4290 | 8 | True | 140.4850 | 2.5812 | 54.43× | 2.5485 | 0.1071 | 23.79× |
| essential512 | coup_product_65536 | 2 | False | 17.2312 | 3.4061 | 5.06× | 0.0609 | 0.0645 | 0.94× |
| essential512 | coup_product_65536 | 2 | True | 16.7485 | 3.4436 | 4.86× | 0.0420 | 0.0602 | 0.70× |
| essential512 | coup_product_65536 | 4 | False | 79.1444 | 4.5884 | 17.25× | 0.4732 | 0.0778 | 6.08× |
| essential512 | coup_product_65536 | 4 | True | 78.5122 | 4.4098 | 17.80× | 0.2810 | 0.0703 | 3.99× |
| essential512 | coup_product_65536 | 8 | False | 283.0160 | 6.8549 | 41.29× | 3.7918 | 0.1175 | 32.26× |
| essential512 | coup_product_65536 | 8 | True | 282.2419 | 6.8192 | 41.39× | 2.5434 | 0.1070 | 23.78× |
| retraction512 | coup_4290 | 2 | False | 6.8795 | 1.2388 | 5.55× | 0.2943 | 0.1430 | 2.06× |
| retraction512 | coup_4290 | 2 | True | 5.8355 | 1.2655 | 4.61× | 0.1248 | 0.1399 | 0.89× |
| retraction512 | coup_4290 | 4 | False | 31.3431 | 1.9790 | 15.84× | 0.5001 | 0.1570 | 3.18× |
| retraction512 | coup_4290 | 4 | True | 31.1462 | 1.8355 | 16.97× | 0.3853 | 0.1530 | 2.52× |
| retraction512 | coup_4290 | 8 | False | 138.6085 | 2.7886 | 49.71× | 3.1844 | 0.1968 | 16.18× |
| retraction512 | coup_4290 | 8 | True | 139.2029 | 2.7267 | 51.05× | 2.2461 | 0.1825 | 12.31× |
| retraction512 | coup_product_65536 | 2 | False | 20.7544 | 5.4657 | 3.80× | 0.5940 | 0.5918 | 1.00× |
| retraction512 | coup_product_65536 | 2 | True | 20.3159 | 5.1937 | 3.91× | 0.5747 | 0.5929 | 0.97× |
| retraction512 | coup_product_65536 | 4 | False | 90.4865 | 6.2734 | 14.42× | 1.0293 | 0.6421 | 1.60× |
| retraction512 | coup_product_65536 | 4 | True | 89.1560 | 6.3183 | 14.11× | 0.8712 | 0.6390 | 1.36× |
| retraction512 | coup_product_65536 | 8 | False | 298.1099 | 9.0469 | 32.95× | 3.9110 | 0.6447 | 6.07× |
| retraction512 | coup_product_65536 | 8 | True | 298.7335 | 8.9360 | 33.43× | 2.8514 | 0.6455 | 4.42× |
| range-shannon | coup_4290 | 2 | False | 6.2295 | 1.8766 | 3.32× | 0.6226 | 0.6399 | 0.97× |
| range-shannon | coup_4290 | 2 | True | 6.1554 | 1.8607 | 3.31× | 0.6119 | 0.6258 | 0.98× |
| range-shannon | coup_4290 | 4 | False | 27.3242 | 2.2542 | 12.12× | 1.0521 | 0.6838 | 1.54× |
| range-shannon | coup_4290 | 4 | True | 26.3626 | 2.1580 | 12.22× | 0.9072 | 0.6621 | 1.37× |
| range-shannon | coup_4290 | 8 | False | 101.7253 | 2.8631 | 35.53× | 3.8409 | 0.5670 | 6.77× |
| range-shannon | coup_4290 | 8 | True | 98.5012 | 2.8432 | 34.64× | 3.1342 | 0.5478 | 5.72× |
| range-shannon | coup_product_65536 | 2 | False | 9.3775 | 2.8309 | 3.31× | 1.0519 | 1.0358 | 1.02× |
| range-shannon | coup_product_65536 | 2 | True | 9.2878 | 2.8893 | 3.21× | 1.0263 | 1.0370 | 0.99× |
| range-shannon | coup_product_65536 | 4 | False | 34.7621 | 3.2474 | 10.70× | 1.4545 | 1.0397 | 1.40× |
| range-shannon | coup_product_65536 | 4 | True | 34.5376 | 3.1538 | 10.95× | 1.3285 | 1.0183 | 1.30× |
| range-shannon | coup_product_65536 | 8 | False | 344.8514 | 4.7708 | 72.28× | 9.1136 | 0.8234 | 11.07× |
| range-shannon | coup_product_65536 | 8 | True | 353.5798 | 4.5693 | 77.38× | 3.6338 | 0.8212 | 4.42× |
| range-group | coup_4290 | 2 | False | 7.2791 | 2.0159 | 3.61× | 0.6235 | 0.6117 | 1.02× |
| range-group | coup_4290 | 2 | True | 6.7382 | 1.9678 | 3.42× | 0.6129 | 0.6114 | 1.00× |
| range-group | coup_4290 | 4 | False | 29.5722 | 2.4173 | 12.23× | 1.1214 | 0.6559 | 1.71× |
| range-group | coup_4290 | 4 | True | 31.7533 | 2.3418 | 13.56× | 0.9896 | 0.6514 | 1.52× |
| range-group | coup_4290 | 8 | False | 727.3537 | 3.6363 | 200.03× | 51.7185 | 0.5990 | 86.35× |
| range-group | coup_4290 | 8 | True | 624.8632 | 3.9938 | 156.46× | 5.5217 | 1.6735 | 3.30× |
| range-group | coup_product_65536 | 2 | False | 10.1769 | 3.0397 | 3.35× | 1.0141 | 1.0202 | 0.99× |
| range-group | coup_product_65536 | 2 | True | 9.7785 | 2.9949 | 3.27× | 1.0049 | 1.0296 | 0.98× |
| range-group | coup_product_65536 | 4 | False | 38.8449 | 3.5538 | 10.93× | 1.4407 | 1.0503 | 1.37× |
| range-group | coup_product_65536 | 4 | True | 36.9358 | 3.6571 | 10.10× | 1.3358 | 1.0026 | 1.33× |
| range-group | coup_product_65536 | 8 | False | 437.1608 | 10.7535 | 40.65× | 11.1525 | 1.1389 | 9.79× |
| range-group | coup_product_65536 | 8 | True | 354.8473 | 5.1974 | 68.27× | 3.9050 | 1.2492 | 3.13× |

## Execution and measurement contract

For 64 groups and branch fanout f, the complete schedule emits 64f³ bindings.
The factored schedule scans 192f branch rows and emits 64 final bindings;
its 192 summary rows retain group presence independently of Event nonemptiness.
Both report the same original 64f³ binding count. The summaries are rebuilt
on every query, including warm queries.

Input-image setup and cloning the input-only Event arena are outside query timing.
Validation, Event operations, staged root storage, summary planning/image/COLT
construction, final Free Join and exact output cardinalities are inside.
The input COLTs are primed for both schedules; fresh means a fresh Event arena.
Do not compare these absolute times with the earlier carrier-only harness.

Memory fields distinguish retained Event/memo estimates, input COLT retention,
summary COLT retention and staged root-vector capacities. They are not peak
query allocation totals: map nodes, relation images and planner temporaries
are not included in those estimates. Process RSS spans both schedules.

This fixture uses the proved Cartesian clover query. It does not establish a
general optimizer or license the same rewrite on triangle joins. Correctness
also covers absent branches, present empty results, duplicates and faults.
Timing fixtures use populated groups and published valid roots.

[Raw samples](results/pack-native-comparison-raw.json) · [Matched comparison](results/pack-native-analysis.json)
· [Rewrite and evidence](FACTORIZED-PACK.md)
