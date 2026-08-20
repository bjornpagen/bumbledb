# Heap-arm pin

2026-08-20 shared-machine night, Apple M2 Max, release `bumbledb-bench heap`,
scale S, seed 1, revision `4dd1ee96`.

Command: `heap --scale S --out bench-out/night-2026-08-20/heap`.

## Frozen vs LMDB point reads (same ledger corpus)

| family | heap p50 ns | lmdb p50 ns | heap/lmdb |
| --- | ---: | ---: | ---: |
| get | 208 | 291 | **0.71×** |
| contains | 375 | 417 | **0.90×** |
| scan (500 accounts) | 18583 | 25500 | **0.73×** |

join 59_584 ns / 500 rows · `fromInstance` 243_761_041 ns.

## Admission prefixes (A, I, R, F, J)

| facts | ns/fact |
| ---: | ---: |
| 693 | 784.5 |
| 2_633 | 810.3 |
| 10_392 | 851.6 |
| 41_432 | 910.4 |

ns/fact grew **1.16×** from 693 to 41_432 facts — no unexplained
superlinear term (gate threshold is 2×).
