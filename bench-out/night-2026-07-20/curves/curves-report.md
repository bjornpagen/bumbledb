# Curves report

Scale curves, report-class. Every point is oracle-gated inline (value-identical multiset agreement against `SQLite`) before either engine is timed; a capped `SQLite` region is excluded-and-counted (`cap` names where it fired). `busy_scan` carries the hand-tuned twin beside the canonical OR-chain — both reported. p50 in ns; seed 1, 64 samples per point, cap 30000 ms per region.

| family | world | scale | facts | answers | ours p50 | sqlite p50 | hand p50 | cap |
|---|---|---|---:|---:|---:|---:|---:|---|
| triangle | ledger | S | 253264 | 3 | 9851917 | 36845292 | — | — |
| point | ledger | S | 253264 | 0 | 500 | 1416 | — | — |
| busy_scan | calendar | S | 192369 | 415 | 7292 | 3477125 | 1221583 | — |
| closure_fanout | closure | S | 17554 | 1316 | 1083 | 32542 | — | — |

capped points: 0 (excluded-and-counted)

## Warmth panel (cold/warm/memoized, p50 ns)

Reopen-cold is process-fresh but OS-page-cache-warm — as close as the harness allows. The engine side prices the (relation, generation) image cache and the resolved-filter view slots.

| family | engine | cold | warm | memoized |
|---|---|---:|---:|---:|
| triangle | bumbledb | 16684209 | 9825000 | 9827291 |
| triangle | sqlite | 37214083 | 36911750 | 36957500 |
| point | bumbledb | 3958 | 542 | 500 |
| point | sqlite | 6542 | 1625 | 1458 |
| busy_scan | bumbledb | 730666 | 7250 | 7167 |
| busy_scan | sqlite | 3611500 | 3479084 | 3471833 |
| closure_fanout | bumbledb | 378583 | 1000 | 1375 |
| closure_fanout | sqlite | 16417 | 9417 | 8542 |
