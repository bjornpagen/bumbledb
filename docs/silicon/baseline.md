# The silicon-suite baseline (PRD 00)

> **Superseded as the live denominator by [`final.md`](final.md)**
> (PRD 14, 2026-07-07): any future suite gates against the final table.
> This file remains the PRD-00 starting point every silicon-suite delta
> is attributed against.

Committed denominator for every gate in docs/silicon/. Captured
2026-07-07 on the reference host (Apple M2 Max), scale S, seed 1,
256 samples/family, verify stamp
`e5f4edcc…` (families + 500 randomized cases, 2,468 oracle cases green).
Engine at commit `d2fc6ef` (bench harness carries the PRD-00 clock proxy;
zero engine changes vs the docs/perf endgame).

Protocol: five full ledger runs under `scripts/measure.sh`, min-of-5 via
`bumbledb-bench merge`; every family block proxy-bracketed (GHz pre/post,
contamination = min < 3.2 GHz, one bounded retry, still-dirty blocks
excluded from minima). Phase tables from one traced run of the same
commit (obs build, 32 samples + 1 traced).

## Ledger, min-of-5 (µs)

| family | run p50s (1..5) | min p50 | min p95 |
|---|---|---|---|
| point | 1.0 / 1.0 / 1.1 / 1.1 / 1.0 | **1.0** | 1.1 |
| fk_walk | 7.3 / 10.9 / 6.8 / 8.8 / 17.2 | **6.8** | 1,034.8 |
| chain | 161.4 / 136.8 / 151.8 / 152.4 / 134.4 | **134.4** | 224.0 |
| range | 28.7 / 28.6 / 28.5 / 28.6 / 28.9 | **28.5** | 29.0 |
| balance | ~~2.2~~ / 1.4 / 2.0 / 1.5 / 1.8 | **1.4** | 26.3 |
| stats | 2,084.3 / 1,888.5 / 1,925.8 / 1,919.3 / 1,886.0 | **1,886.0** | 2,086.5 |
| string | 1.5 / 1.5 / 1.5 / 1.7 / 1.6 | **1.5** | 1.7 |
| skew | ~~170.0~~ / 49.5 / ~~52.1~~ / 39.7 / ~~35.7~~ | **39.7** | 1,107.0 |
| spread | 12,483.0 / 11,631.9 / ~~11,441.9~~ / 11,893.6 / 11,281.6 | **11,281.6** | 11,745.8 |
| triangle | 15,260.1 / ~~15,375.4~~ / 15,064.0 / 15,347.2 / 15,267.0 | **15,064.0** | 15,394.9 |
| commit_single | all five contaminated (fsync-DVFS, see below) | (4,238.0 dirty-min) | — |
| commit_batch | ~~30,225.9~~ / 32,869.3 / 34,207.2 / 30,210.2 / 32,169.2 | **30,210.2** | 34,435.0 |
| cold_fk_walk | 7,792.2 / 7,504.4 / 8,134.2 / 7,395.3 / 7,029.3 | **7,029.3** | 9,601.5 |
| bulk | 954,039 / 1,013,944 / 1,044,601 / 937,613 / 997,631 | **937,613** | 957,884 |

~~struck~~ = the block's clock-proxy bracket read contaminated after the
bounded retry; excluded from minima (12 blocks total across 5 runs).
ALL-WIN held on every run.

Store: bumbledb 64,421,888 B (compacted) vs sqlite 13,611,008 B.

## Clock-proxy findings

- Read-family blocks sit at 3.26–3.41 GHz clean; the detector's ignored
  test fires under 24-way spin load (machinery gate green).
- **Write families read low honestly**: `commit_single` bracketed at
  1.6–2.4 GHz in all five runs — during fsync waits the core clocks
  down, and the proxy reports the DVFS state around the block. For
  write families the contamination mark is a *physics annotation*, not
  a dirty measurement: both engines pay the same clock. Gates on writes
  therefore read the table values (ratio vs SQLite), never the proxy.

## Run-to-run variance (gate 2, investigated)

Non-bimodal families agree within 5% p50 across consecutive runs with
two named exceptions, both understood:
- **point/string/balance (~1–2 µs)**: ±1 timer tick (41.67 ns ≈ 4% of
  1 µs) — quantization wiggle, not variance. The quantum guard leaves
  them un-batched at 24+ ticks/sample; sub-tick precision is not
  claimed.
- **chain (134–161 µs, ±10%)**: run-scoped mode structure (adjacent
  runs 3↔4 agree at 0.4%; the mode moves between runs). Chain gates in
  this suite read min-of-5 and apply the confirm-run protocol.
Bimodal-exempt families (fk_walk, balance, skew) gate on p95: fk_walk
p95 1,034.8, skew p95 1,107.0 — the hot-parameter mode, as documented
in docs/perf.

## Phase tables (traced run, µs)

### triangle (p50 15,064; traced execute 15,625)
```
phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              782      313.083        400      313.083
jp_probe_n0             782     1918.416       2453     1918.416
jp_residual_n0          782        1.000          1        1.000
jp_descend_n0           782     1104.791       1412        0.000
jp_hash_n1             2725     1558.083        571     1558.083
jp_probe_n1            2725     5649.000       2073     5649.000
jp_residual_n1         2725        2.291          0        2.291
jp_descend_n1          2725       36.375         13       26.918
jp_iter_n2               62        2.708         43        2.708
jp_residual_n2           62        0.208          3        0.208
jp_descend_n2           464        6.541         14        6.541
```
The wall, itemized: `jp_probe_n1` **5,649 µs** (the docs/perf standing
wall at ~5.5 ms), `jp_probe_n0` 1,918, `jp_hash_n1` 1,558,
`jp_descend_n0` 1,105 (bookkeeping).

### chain (p50 134.4)
```
phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                1        0.416        416        0.416
jp_probe_n0               1        2.166       2166        2.166
jp_residual_n0            1        0.041         41        0.041
jp_descend_n0             1        1.666       1666        0.000
jp_hash_n1                4        1.541        385        1.541
jp_probe_n1               4       10.208       2552       10.208
jp_residual_n1            4        0.041         10        0.041
jp_descend_n1             4       46.875      11718        8.667
jp_descend_n2           381       38.208        100       38.208
```

### stats (p50 1,886.0)
```
phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0                4        1.791        447        1.791
jp_probe_n0               4        6.333       1583        6.333
jp_residual_n0            4        0.125         31        0.125
jp_descend_n0             4     2075.875     518968       46.125
jp_iter_n1             1536      194.750        126      194.750
jp_residual_n1         1024        0.500          0        0.500
jp_descend_n1          1024     1834.500       1791     1834.500
```
`jp_descend_n1` 1,834 µs = the leaf scan-folds + dedup seen-set — the
PRD 04/06 target.

### spread (p50 11,281.6)
```
phase                 calls     total_us     avg_ns      excl_us
jp_hash_n0              782      319.166        408      319.166
jp_probe_n0             782     1813.000       2318     1813.000
jp_residual_n0          782        0.750          0        0.750
jp_descend_n0           782     8544.125      10925     1929.542
jp_descend_n1        100000     6614.583         66     6614.583
```
`jp_descend_n1` 6,615 µs over 100k calls (66 ns/leaf-entry) — the
fanout-1.4 leaf path (PRD 08's per-run costs live here).

### skew (traced sample = hot parameter; p50 gate reads p95)
```
phase                 calls     total_us     avg_ns      excl_us
jp_descend_n1             1      696.458     696458        0.542
jp_descend_n2             4      695.916     173979      695.916
```
(abridged rows < 1 µs) — the leaf dedup + finalize dominate.

### range (p50 28.5)
```
phase                 calls     total_us     avg_ns      excl_us
jp_descend_n0             1       22.583      22583       22.583
```
Single-node scan-fold: all leaf-scan machinery.

## Notes for later PRDs

- The traced run is branded UNVERIFIED (filtered + `--i-am-lying` from
  the archived-tree binary); the stamp above belongs to the five timing
  runs, which refused to run without it. Phase tables are work-direction
  artifacts, not gates (50-validation law).
- `cold_fk_walk` at 7.0–8.1 ms vs SQLite's ~74 µs is the stats-walk +
  image-rebuild cost (PRD 13's denominator).
- The 12 contaminated blocks split: 5× commit_single, 3× commit_batch
  (writes, physics), and 4 read blocks (balance r1, skew r1/r3/r5,
  spread r3, triangle r2 — co-tenant windows the retry did not clear;
  the min-of-5 protocol absorbed them exactly as designed).
