# bumbledb bench report

## Provenance

- crate version: 0.9.0
- engine rev: 35766d9b34f28c33bc644e6b1b1ad83ab89bf913
- timestamp: 2026-08-02T00:21:52Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 1.98 2.19 2.59 (start) → 2.14 2.22 2.60 (end)
- config: scale S, seed 1, 256 samples, ephemeral stores
- corpus digest: `fa73e680324f9b26dd1c8504899c43beec8eef48953ca4bdf4ca432623caaca8`
- verify stamp: `5c3b2c1d056c180caa921ba7be8f8eb2a1f3da0d8133532cf045f2f3a429c243 (families + 500 randomized cases)`

## Gate verdict

PARTIAL — filtered run; the ALL-WIN claim needs every family.
p99 budget (<= 10 ms warm): PASS (informational below scale L).
clock proxy: 1 block(s) still contaminated after retry — treat their percentiles as dirty: commit_window_exclusion.

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_window_baseline | 27.8 | - | - |
| commit_window_admission | 34.4 | - | - |
| commit_window_exclusion | 32.7 | - | - |

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
| commit_window_baseline | 3.51 | 3.45 | clean | - |
| commit_window_admission | 3.44 | 3.50 | clean | - |
| commit_window_exclusion | 3.50 | 2.97 | CONTAMINATED | - |

## Flame summaries

(none captured — run with --trace)
