# Heap-arm first pins (issue 39)

2026-08-20, Apple M2 Max, release `bumbledb-bench heap`, scale S, seed 1, 8 samples.

Command: `heap --scale S --samples 8 --prefixes 256,1024,4096,16384`.

## Frozen vs LMDB point reads (same ledger corpus, NOSYNC publish)

| family | heap p50 ns | lmdb p50 ns | heap/lmdb |
| --- | ---: | ---: | ---: |
| get | 167 | 292 | **0.57×** |
| contains | 333 | 417 | **0.80×** |
| scan (500 accounts) | 19542 | 25500 | **0.77×** |

join 123_458 ns / 500 rows · `fromInstance` 250_776_958 ns.

Heap wins the point-read comparison (binary search over a packed map vs
the B-tree). Scan is the Account relation only.

## Admission prefixes (A, I, R, F, J)

| facts | wall ns | facts/s | ns/fact | A | I | R | F | J |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 693 | 535_375 | 1_294_420 | 772.5 | 188_768 | 4_352 | 131_072 | 138_139 | 140_760 |
| 2_633 | 2_059_917 | 1_278_207 | 782.3 | 414_048 | 16_640 | 393_216 | 521_823 | 544_312 |
| 10_392 | 8_947_458 | 1_161_447 | 861.0 | 1_249_632 | 65_792 | 1_310_720 | 2_056_158 | 2_158_464 |
| 41_432 | 40_360_958 | 1_026_537 | 974.1 | 4_788_576 | 262_656 | 5_242_880 | 8_194_547 | 8_615_296 |

ns/fact grew **1.26×** from 693 to 41_432 facts — no unexplained
superlinear term (gate threshold is 2×).

Bare-metal ramdisk row rides issue 18's release checklist beside this lane.
