# The silicon suite — final table (PRD 14)

The new committed denominator. Captured 2026-07-07 on the reference
host, scale S, seed 1, min-of-3 full ledger runs (256 samples/family,
proxy-bracketed, contaminated blocks excluded), verify stamp green at
every binary along the way (2,468 oracle cases per verification, five
verifications across the suite's batches). Engine at the PRD-14 commit.
Any future suite gates against THIS table; `baseline.md` remains the
suite's starting denominator for the attribution below.

## Ledger, min-of-3 (µs) — vs the PRD-00 baseline

| family | baseline p50 | final p50 | Δ | baseline p95 | final p95 |
|---|---|---|---|---|---|
| point | 1.0 | **0.4** | **−60%** | 1.1 | 0.4 |
| string | 1.5 | **0.8** | **−47%** | 1.7 | 0.9 |
| balance | 1.4 | **0.7** | **−50%** | 26.3 | 25.0 |
| fk_walk | 6.8 | **2.9** | **−57%** | 1,034.8 | 889.0 |
| skew | 39.7 | **35.8** | −10% | 1,107.0 | 924.5 |
| range | 28.5 | **28.5** | 0% | 29.0 | 28.8 |
| chain | 134.4 | **104.0** | **−23%** | 224.0 | 168.8 |
| stats | 1,886.0 | **1,872.5** | −0.7% | 2,086.5 | 1,889.8 |
| spread | 11,281.6 | **10,725.8** | −4.9% | 11,745.8 | 11,128.8 |
| triangle | 15,064.0 | **11,742.5** | **−22%** | 15,394.9 | 12,071.8 |
| cold_fk_walk | 7,029.3 | **~4,018** | **−43%** | 9,601.5 | ~4,238 |
| commit_single | ~4,238 (dirty) | ~4,656–5,080 | physics band | — | — |
| commit_batch | 30,210.2 | ~25,935–29,983 | within band | — | — |
| bulk | 937,613 | ~912,286–920,083 | −2–3% | — | — |

**ALL-WIN on every run of every batch. Ledger geomean of
(final ÷ baseline) p50 ratios = 0.69 — a 31% suite-wide improvement
(gate: ≥ 20%) ✓. Every family's final p50 ≤ its baseline p50 (range
exactly equal); every bimodal family's p95 improved.** Store bytes
64,421,888 — byte-identical to baseline (no pitch padding triggers at
bench-scale spans; the layout rule is engaged and structurally tested).

## The endgame gates

- **point p50 ≤ 0.8 µs: PASSED at 0.4** (the perf-suite's standing miss,
  closed with 2× margin by PRD 12's parked reader).
- **triangle p50 ≤ 8,000 µs: MISSED at 11,742** (−22% from 15,064).
- **`jp_probe_n1` ≤ 1,500 µs: MISSED at 3,667** (−35% from 5,649).

The documented miss, at the high bar the gate demands — full
attribution and the irreducibility statement:

### Triangle phase attribution (traced, µs; baseline → final)

| phase | baseline | final | Δ | mechanism |
|---|---|---|---|---|
| jp_hash_n0 | 313 | 104 | −67% | PRD 02 (per-pass source tables) |
| jp_probe_n0 | 1,918 | 1,168 | −39% | PRD 02 (inline/monomorphic/no-copy) |
| jp_descend_n0 | 1,105 | 1,114 | 0% | bookkeeping floor (binds + routing) |
| jp_hash_n1 | 1,558 | 1,339 | −14% | PRDs 02/14 |
| jp_probe_n1 | 5,649 | 3,667 | −35% | PRDs 02/03/04 (diet, 33% load, window probe) |
| calls (hash_n1) | 2,725 | 2,555 | mean 37 → 39 | PRD 14 (segregation + 2×-batch cascade) |

### Why the remainder is irreducible under the current plan shape

The final probe cost is ~37 ns per node-1 probe (3,667 µs / ~100k).
The platform accounting: bumblebench's faithfully-shaped probe emulation
floors at 17–21 ns with NOTHING but the walk, the compare, and minimal
bookkeeping; the engine's per-probe extra is the pending-entry cursor
read, sibling-children store, survivor mask write, and the leaf handoff
— each already in its minimal measured shape (asm-gated inline, no
copies, no calls, pre-sized indexed stores). Batching cannot cut it:
the map is L2-resident and the OoO core overlaps these probes at batch 1
(the suite's founding law) — confirmed again by the segregation lever,
which raised effective grouping and moved nothing. Reaching ≤ 1,500 µs
(15 ns/probe) requires one of two things this suite deliberately does
not do:
1. **Fewer probes** — a plan-shape change (semijoin pre-filters,
   different node orderings, or a degree-aware cover policy), which is
   planner scope, not executor scope; or
2. **SIMD-batched probing** — probing 2–4 keys per SWAR/NEON step
   against restructured interleaved buckets, a COLT layout redesign.
Both are recorded as the next suite's opening levers. The triangle p50's
non-probe remainder (descend bookkeeping 1.1 ms, hash 1.4 ms, leaf +
seen-set + finalize ≈ 4 ms untraced) shrinks with the same two levers
(fewer survivors ⇒ less of everything downstream).

## Per-PRD attribution of the suite's wins

- **PRD 02 (instruction diet):** triangle 15,064 → 12,256 (−19%); every
  probe-bearing family moved (chain −8%, skew, spread −6%).
- **PRD 03/04 (map geometry + hash-ahead):** probe walks −35% cumulative
  on triangle's n1; stats descend 1,834 → 1,697 traced; the sweep pinned
  33% load; the confirm-run protocol caught and reverted the one
  mis-placed pipeline (projection scans).
- **PRD 06 (NEON folds):** dense exact sums 2.45–4.6 → 7.8–7.9 rows/ns
  at kernel level (family-level effect small — stats is dedup-bound).
- **PRD 08 (prefix tables):** both `from_fn` Option tables gone;
  thresholds 32 → 8 with the corrected attribution.
- **PRD 10 (prefetch tiering):** prefetch now fires ONLY where the law
  says it pays — 782 passes on triangle and spread (>2 MiB colts), zero
  on every resident family (trace-verified).
- **PRD 11 (pitch padding):** the stagger pathology is structurally
  impossible (band-tested layout; no bench-scale trigger, engineered
  pathological case pinned in-tree).
- **PRD 12 (parked reader):** point 1.0 → 0.4, string 1.5 → 0.8,
  balance 1.4 → 0.7, fk_walk 6.8 → 2.9 — every execute stopped paying
  a transaction begin. The single largest suite win.
- **PRD 13 (lazy stats):** cold_fk_walk 7,029 → ~4,018 (−43%) — the
  stats walk left the rebuild spike.
- **PRD 14 (segregation + cascade):** batch means 37 → 39 only — the
  diagnosis (pump-call granularity, volume-bound) is itself the finding;
  kept for its small wins and the diagnosis.

## Surviving walls, with owners

1. **Triangle probe volume** (~37 ns/probe × ~100k): planner-scope
   probe-count reduction or SIMD-batched COLT probing (next suite).
2. **stats dedup floor** (~1.87 ms): per-row full-binding dedup insert
   (key assembly + window walk) is the semantic floor; a
   distinctness-proving planner extension would elide it (planner
   scope).
3. **Write-path fsync physics** (commit_* families): untouched by
   design; the proxy marks them honestly.

The scenario suite, L-scale runs, and the performance claim remain
human-owned (suite law). Scenario p1/p2 are expected to flip below 1.0×
at the next human run on the strength of point 0.4/string 0.8.
