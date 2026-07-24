# Curves report

Scale curves, report-class. Every point is oracle-gated inline (value-identical multiset agreement against `SQLite`) before either engine is timed; a capped `SQLite` region is excluded-and-counted (`cap` names where it fired). `busy_scan` carries the hand-tuned twin beside the canonical OR-chain — both reported. p50 in ns; seed 1, 64 samples per point, cap 30000 ms per region.

| family | world | scale | facts | answers | ours p50 | sqlite p50 | hand p50 | cap |
|---|---|---|---:|---:|---:|---:|---:|---|
| triangle | ledger | S | 253264 | 3 | 2560833 | 35554000 | — | — |
| point | ledger | S | 253264 | 0 | 292 | 1375 | — | — |
| busy_scan | calendar | S | 192369 | 410 | 7916 | 3292083 | 1176792 | — |
| closure_fanout | closure | S | 17554 | 1316 | 1209 | 8125 | — | — |

capped points: 0 (excluded-and-counted)

## Warmth panel (cold/warm/memoized, p50 ns)

Reopen-cold is process-fresh but OS-page-cache-warm — as close as the harness allows. The engine side prices the (relation, generation) image cache and the resolved-filter view slots.

| family | engine | cold | warm | memoized |
|---|---|---:|---:|---:|
| triangle | bumbledb | 7717250 | 2546333 | 2553708 |
| triangle | sqlite | 35910834 | 35400041 | 35484875 |
| point | bumbledb | 2791 | 333 | 291 |
| point | sqlite | 6542 | 1625 | 1375 |
| busy_scan | bumbledb | 747334 | 7541 | 7708 |
| busy_scan | sqlite | 3539167 | 3351250 | 3293959 |
| closure_fanout | bumbledb | 297458 | 625 | 1000 |
| closure_fanout | sqlite | 16458 | 10917 | 8792 |
