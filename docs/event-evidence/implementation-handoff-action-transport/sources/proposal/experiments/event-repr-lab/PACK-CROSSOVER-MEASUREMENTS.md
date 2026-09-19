# Native Pack factorization measurements

Same executable, input Events, scalar rows and outputs. Each row holds the carrier
fixed and compares complete bindings with certified branch reductions.
Times are 11-sample medians in milliseconds; speedup is complete / factorized.
Raw samples and min/max ranges remain in the linked results.

| Carrier | Coup presentation | Fanout | Memo | Fresh complete | Fresh factored | Speedup | Warm complete | Warm factored | Speedup |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| dense | coup_4290 | 1 | False | 0.0316 | 0.0932 | 0.34× | 0.0331 | 0.0811 | 0.41× |
| dense | coup_4290 | 1 | True | 0.0226 | 0.0713 | 0.32× | 0.0075 | 0.0558 | 0.13× |
| dense | coup_4290 | 2 | False | 0.3109 | 0.1286 | 2.42× | 0.3174 | 0.1199 | 2.65× |
| dense | coup_4290 | 2 | True | 0.1606 | 0.0931 | 1.73× | 0.0427 | 0.0594 | 0.72× |
| dense | coup_4290 | 8 | False | 20.3905 | 0.2840 | 71.81× | 20.4379 | 0.2753 | 74.23× |
| dense | coup_4290 | 8 | True | 7.3647 | 0.1716 | 42.91× | 2.3649 | 0.1055 | 22.42× |
| dense | coup_product_65536 | 1 | False | 0.2899 | 0.3669 | 0.79× | 0.3082 | 0.3577 | 0.86× |
| dense | coup_product_65536 | 1 | True | 0.1698 | 0.2212 | 0.77× | 0.0156 | 0.0639 | 0.24× |
| dense | coup_product_65536 | 2 | False | 3.2136 | 0.7881 | 4.08× | 3.3499 | 0.7955 | 4.21× |
| dense | coup_product_65536 | 2 | True | 1.2514 | 0.3740 | 3.35× | 0.0518 | 0.0685 | 0.76× |
| dense | coup_product_65536 | 8 | False | 218.0463 | 2.2149 | 98.44× | 212.1823 | 2.2268 | 95.29× |
| dense | coup_product_65536 | 8 | True | 47.5123 | 0.8783 | 54.09× | 2.1863 | 0.1132 | 19.31× |
| packed512 | coup_4290 | 1 | False | 0.0913 | 0.1660 | 0.55× | 0.0385 | 0.0875 | 0.44× |
| packed512 | coup_4290 | 1 | True | 0.0936 | 0.1555 | 0.60× | 0.0368 | 0.0843 | 0.44× |
| packed512 | coup_4290 | 2 | False | 0.5957 | 0.2476 | 2.41× | 0.0904 | 0.0980 | 0.92× |
| packed512 | coup_4290 | 2 | True | 0.5791 | 0.2099 | 2.76× | 0.0758 | 0.0950 | 0.80× |
| packed512 | coup_4290 | 8 | False | 16.1050 | 0.3835 | 42.00× | 3.0298 | 0.1484 | 20.42× |
| packed512 | coup_4290 | 8 | True | 16.2096 | 0.3918 | 41.37× | 2.5579 | 0.1381 | 18.52× |
| packed512 | coup_product_65536 | 1 | False | 0.3899 | 0.4555 | 0.86× | 0.1085 | 0.1625 | 0.67× |
| packed512 | coup_product_65536 | 1 | True | 0.3856 | 0.4531 | 0.85× | 0.1090 | 0.1617 | 0.67× |
| packed512 | coup_product_65536 | 2 | False | 2.2497 | 0.6720 | 3.35× | 0.2230 | 0.2220 | 1.00× |
| packed512 | coup_product_65536 | 2 | True | 2.2345 | 0.6057 | 3.69× | 0.2048 | 0.2183 | 0.94× |
| packed512 | coup_product_65536 | 8 | False | 44.1167 | 1.1115 | 39.69× | 3.4700 | 0.2247 | 15.45× |
| packed512 | coup_product_65536 | 8 | True | 43.4014 | 1.0806 | 40.16× | 2.7808 | 0.2180 | 12.75× |

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

[Raw samples](results/pack-native-crossover-raw.json) · [Matched comparison](results/pack-native-crossover-analysis.json)
· [Rewrite and evidence](FACTORIZED-PACK.md)
