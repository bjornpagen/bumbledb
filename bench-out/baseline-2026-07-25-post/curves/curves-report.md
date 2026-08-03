# Curves report

Scale curves, report-class. Every point is oracle-gated inline (value-identical multiset agreement against `SQLite`) before either engine is timed; a capped `SQLite` region is excluded-and-counted (`cap` names where it fired). `busy_scan` carries the hand-tuned twin beside the canonical OR-chain — both reported. p50 in ns; seed 1, 64 samples per point, cap 30000 ms per region.

| family | world | scale | facts | answers | ours p50 | sqlite p50 | hand p50 | cap |
|---|---|---|---:|---:|---:|---:|---:|---|
| triangle | ledger | S | 253264 | 3 | 2570750 | 37092541 | — | — |
| point | ledger | S | 253264 | 0 | 292 | 1500 | — | — |
| busy_scan | calendar | S | 192369 | 410 | 8291 | 3379833 | 1217875 | — |
| closure_fanout | closure | S | 17554 | 1316 | 459 | 8459 | — | — |

capped points: 0 (excluded-and-counted)

## Warmth panel (cold/warm/memoized, p50 ns)

Reopen-cold is process-fresh but OS-page-cache-warm — as close as the harness allows. The engine side prices the (relation, generation) image cache and the resolved-filter view slots.

| family | engine | cold | warm | memoized |
|---|---|---:|---:|---:|
| triangle | bumbledb | 15385958 | 2560042 | 2546875 |
| triangle | sqlite | 37693375 | 37115916 | 37055167 |
| point | bumbledb | 3042 | 333 | 292 |
| point | sqlite | 6292 | 1750 | 1500 |
| busy_scan | bumbledb | 2611000 | 8375 | 8083 |
| busy_scan | sqlite | 3501583 | 3392041 | 3409250 |
| closure_fanout | bumbledb | 556167 | 708 | 459 |
| closure_fanout | sqlite | 20291 | 12542 | 8833 |
