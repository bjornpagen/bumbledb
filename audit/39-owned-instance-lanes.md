# 39 — New lanes: the heap arm gets measured like the store arm

- **Status:** **fixed this pass** — two lanes run under `heap`; first
  pins in `crates/bumbledb-bench/HEAP-BASELINE.md`.
- **Severity:** performance coverage — the heap arm shipped without its
  ladder.

## Principle

The store arm's perf truths are pinned by lanes; the heap arm has focused
tests but no measured characteristics. "Aggressively cleaning up the
performance characteristics" starts with measuring the surface that never
had numbers.

## The lanes

1. **Frozen vs LMDB point reads**: warm `get`/`contains`/`scan` on the same
   canonical corpus, heap arm vs store arm — the binary-search-vs-B-tree
   truth, recorded.
2. **Admission throughput**: facts/second and the five phase quantities
   (`A`, `I`, `R`, `F`, `J`) across corpus prefixes — the proposal's
   telemetry, promoted into a lane.
3. The bare-metal ramdisk row rides the release checklist (closed issue
   18's suggested one-liner) beside this lane.

## First pin (2026-08-20, Apple M2 Max, release, scale S, 8 samples)

| family | heap p50 | lmdb p50 | ratio |
| --- | ---: | ---: | ---: |
| get | 167 ns | 292 ns | 0.57× |
| contains | 333 ns | 417 ns | 0.80× |
| scan | 19_542 ns | 25_500 ns | 0.77× |

Admission (four prefixes): 693 → 41_432 facts, 1.29M → 1.03M facts/s,
ns/fact 773 → 974 (**1.26×**, no superlinear term). Telemetry columns
A/I/R/F/J are on every row (`InstanceBuilder::admit_measured`).

join 123 µs / 500 rows · `fromInstance` 251 ms.

## Acceptance

- The two lanes exist, run under the bench driver, and their first pins
  are recorded in the bench docs.
