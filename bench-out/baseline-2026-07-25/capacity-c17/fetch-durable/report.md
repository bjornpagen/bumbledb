# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: e1d039fe75e34009247a93339aa734252af5d84b
- timestamp: 2026-08-01T23:32:56Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.67 2.45 2.25 (start) → 2.54 2.43 2.24 (end)
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `b39fc086bdbd09c006f017c3172bbb4f342a84773f36b81ccedcd682b2282daf (families + 500 randomized cases)`

## Gate verdict

PARTIAL — filtered run; the ALL-WIN claim needs every family.
p99 budget (<= 10 ms warm): PASS (informational below scale L).
clock proxy: 3 block(s) still contaminated after retry — treat their percentiles as dirty: commit_capacity_baseline, commit_capacity_sum, commit_capacity_duration.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_capacity_baseline | 5052.2 | - | - |
| commit_capacity_sum | 5365.1 | - | - |
| commit_capacity_duration | 6007.1 | - | - |

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
| commit_capacity_baseline | 3.34 | 1.60 | CONTAMINATED | - |
| commit_capacity_sum | 1.62 | 0.91 | CONTAMINATED | - |
| commit_capacity_duration | 0.91 | 1.75 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
