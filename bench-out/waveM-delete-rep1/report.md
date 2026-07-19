# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: 5f6c746ac76e3e051ecbf22712152c599bbf69ff
- timestamp: 2026-07-19T10:05:46Z
- host: Apple M2 Max
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `06e9620d6ec88418e5150de76c47996db8bc62a8b4add74b11633bb8f257a428`
- verify stamp: `1f665e0c0795d265257473fd68e39da443c5806deb9065539617f330dc29d37f (families + 500 randomized cases)`

## Gate verdict

PARTIAL — filtered run; the ALL-WIN claim needs every family.
p99 budget (<= 10 ms warm): PASS (informational below scale L).

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| cold_containment_walk | 1416.5 | 56.7 | - |
| cold_containment_walk_delete | 3540.6 | 59.2 | - |

## Allocations

(not captured — run with the alloc window)

## Execution digests

| family | worst est/actual | covers | emitted | absorbed |
|---|---|---|---|---|

## Store

- bumbledb file (compacted): 77955072 bytes
- sqlite file: 18464768 bytes
- image cache: 0 images, 0 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| cold_containment_walk | 3.26 | 3.26 | clean | - |
| cold_containment_walk_delete | 3.26 | 3.26 | clean | - |

## Flame summaries

(none captured — run with --trace)
