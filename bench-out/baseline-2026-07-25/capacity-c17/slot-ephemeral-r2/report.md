# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: e1d039fe75e34009247a93339aa734252af5d84b
- timestamp: 2026-08-01T23:38:17Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 1.94 2.39 2.30 (start) → 1.95 2.39 2.30 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `c1434ac8d28c935146685a6d20cd2243691d5d678631edfddb6a927ab0eca3de (families + 500 randomized cases)`

## Gate verdict

PARTIAL — filtered run; the ALL-WIN claim needs every family.
p99 budget (<= 10 ms warm): PASS (informational below scale L).

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_capacity_baseline | 18.2 | - | - |
| commit_capacity_sum | 32.3 | - | - |
| commit_capacity_duration | 30.8 | - | - |

## Allocations

(not captured — run with the alloc window)

## Execution digests

| family | worst est/actual | covers | emitted | absorbed |
|---|---|---|---|---|

## Store

- bumbledb file (compacted): 64274432 bytes
- sqlite file: 18432000 bytes
- image cache: 0 images, 0 bytes

## Clock proxy

| family | GHz pre | GHz post | status | norm p50 (us) |
|---|---|---|---|---|
| commit_capacity_baseline | 3.50 | 3.45 | clean | - |
| commit_capacity_sum | 3.50 | 3.50 | clean | - |
| commit_capacity_duration | 3.50 | 3.50 | clean | - |

## Flame summaries

(none captured — run with --trace)
