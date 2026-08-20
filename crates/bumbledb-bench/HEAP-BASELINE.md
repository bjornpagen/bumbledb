# Heap-arm first pins (issue 39)

2026-08-20, Apple M2 Max, release `bumbledb-bench heap`, scale S, seed 1, 8 samples.

Command: `heap --scale S --samples 8 --prefixes 256,1024,4096,16384` with
`--primer-spec /Users/bjorn/Documents/primer-spec` and
`--primer-snapshot /Users/bjorn/Documents/knowledge-graph-data/v1.11.0`.

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

## Primer scaling gate

**blocked** (corpus reachable; opener missing).

- Source JSONL present at `knowledge-graph-data/v1.11.0` (sizes
  292_652_341 / 520_406_049, the pinned 1.11.0 pair).
- Completed store
  `primer-spec/.primer/builds/2026-08-18T21-07-04.800Z-a0ee7a25-b34c-4940-9127-ce6228a3cedc/standards-evidence-ir.bumbledb`
  (1_680_998_400 bytes).
- **Ask:** land a fingerprint-matching Rust `SchemaDescriptor` (or
  `schema!` transcription) of StandardsEvidenceIR so the bench can open
  that store and run four prefixes through load → complete admit →
  keyed reads → representative joins → `fromInstance`. Grade handles
  include `"1"`..`"12"` and `"source-normalized"`, which `schema!`
  identifiers cannot spell.

Bare-metal ramdisk row rides issue 18's release checklist beside this lane.
