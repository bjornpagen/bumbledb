# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: e1d039fe75e34009247a93339aa734252af5d84b
- timestamp: 2026-08-01T23:37:07Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.44 2.54 2.34 (start) → 2.49 2.55 2.34 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `b39fc086bdbd09c006f017c3172bbb4f342a84773f36b81ccedcd682b2282daf (families + 500 randomized cases)`

## Gate verdict

PARTIAL — filtered run; the ALL-WIN claim needs every family.
p99 budget (<= 10 ms warm): PASS (informational below scale L).

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_capacity_baseline | 18.6 | - | - |
| commit_capacity_sum | 35.2 | - | - |
| commit_capacity_duration | 34.2 | - | - |

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
| commit_capacity_baseline | 3.50 | 3.50 | clean | - |
| commit_capacity_sum | 3.50 | 3.50 | clean | - |
| commit_capacity_duration | 3.51 | 3.50 | clean | - |

## Flame summaries

(none captured — run with --trace)
