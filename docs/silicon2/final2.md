# final2 — the silicon2 denominator

Pinned by docs/silicon2/09 on the round-two endgame binary (verify
stamp `01bdd8ca`), min-of-3 full ledger runs under
`scripts/measure.sh` (`bench-out/s2p09-{1,2,3}`), traced phase evidence
from the obs build (`bench-out/s2p09-traced`). Chain, fk_walk, balance,
and skew gate on p95 (bimodal p50s; chain joined the set in PRD 07's
Result with its 93–122 p50 band over a 145–152 p95 band).

## The table

| family | final.md p50 | final2 p50 | Δ | final2 p95 |
|---|---|---|---|---|
| point | 0.4 | **0.4** | 0% | 0.4 |
| string | 0.8 | **0.7** | −12.5% | 0.8 |
| balance | 0.7 | **0.7** | 0% | 24.6 (vs 25.0) |
| fk_walk (p95 gates) | p95 889.0 | p50 6.0 | **p95 759.2, −14.6%** | 759.2 |
| skew (p95 gates) | p95 924.5 | p50 52.2 | **p95 794.0, −14.1%** | 794.0 |
| range | 28.5 | **20.6** | **−27.7%** | 20.8 |
| chain (p95 gates) | p95 168.8 | p50 100.9 | **p95 145.6, −13.7%** | 145.6 |
| stats | 1,872.5 | **1,203.5** | **−35.7%** | 1,241.5 |
| spread | 10,725.8 | **10,269.9** | −4.3% | 11,737.8 |
| triangle | 11,742.5 | **9,445.5** | **−19.6%** | 9,958.4 |
| cold_fk_walk | ~4,018 | **3,674.1** | −8.6%† | 5,901.9 |
| bulk | ~912–920k | 941,817 | physics band (+0.4% vs the 937,613 baseline) | — |
| commit_single/batch | physics band | proxy-flagged all runs (fsync-DVFS class) | band | — |

**ALL-WIN on every run** (each run's gate verdict: every gated read
family beats SQLite on p50). **Geomean vs final.md over the ten read
families (bimodal on p95): −15.0%** (requirement: ≥ 10%). Every read
family ≤ its final.md number; bulk sits on the fsync physics band
final.md itself defines for write families.

†cold under the NEW settle protocol — see below.

## Per-PRD attribution (min-of-3 p50 after each PRD)

| family | 00 re-anchor | 01 | 02 | 03 | 05 | 07 | 08 |
|---|---|---|---|---|---|---|---|
| triangle | 12,135.2 | 11,793.8 | 11,771.0 | 11,766.0 | **9,649.4** | **9,195.4** | 9,360.0 |
| stats | 1,877.8 | 1,887.0 | **1,623.0** | **1,250.3** | 1,244.3 | **1,206.9** | 1,197.5 |
| range | 28.1 | 28.2 | 27.3 | **20.5** | 21.0 | 20.6 | 20.5 |
| spread | 10,835.1* | 10,758.9 | 10,843.1 | **10,315.4** | **10,019.5** | 10,235.5 | 10,229.6 |
| chain p95 | 169.5 | 169.7 | 168.0 | **145.4** | 147.6 | 152.1 | **147.2** |
| skew p95 | 932.2 | 926.4 | 918.9 | **756.6** | 749.0 | 762.7 | 785.5 |
| fk_walk p50 | 6.9* | 4.2 | 2.8 | 2.5 | 2.5 | 2.6 | 2.6 |

*clock-corrected in PRD 00's confirm-run (spread re-anchored at
10,835/norm 10,782; fk_walk gates on p95).

The wins live where the PRDs' Results put them: **02** (sink pipeline
deletion) bought stats −14%; **03** (const-arity) bought stats −23%
further, range −25%, skew p95 −18%; **05** (bucket-of-8 layout) bought
triangle −18% with the probe still scalar; **07** (alias hoisting)
bought triangle −4.7% and stats −3%. **01/04/06/08** shipped
essentially neutral code (01's floor widening) or nothing at all — and
their Results carry the suite's most valuable content: three measured
refutations (phase-1.5 coverage of at-floor maps, key-ahead prefetch,
the NEON sweep) plus one attribution correction (jp_probe_n1's true
count is 299k probes, not ~100k).

## Phase tables (traced, obs build)

Triangle (one traced warm sample, phase counters on):

| phase | anchor (00) | final2 | Δ |
|---|---|---|---|
| jp_probe_n1 | 3,764 µs / 2,555 calls | **1,716.9 µs / 2,725 calls** | **−54%, ~5.7 ns/probe** |
| jp_probe_n0 | 1,199 µs | **853.2 µs** | −29% — PRD 01's ≤ 900 gate MET on the final binary |
| jp_hash_n1 | 1,362 µs | 1,053.9 µs | −23% |
| jp_descend_n0 | 1,155 µs | 1,150.8 µs | flat (bookkeeping floor) |
| prefetch_pass | 782 (n0 covered) | 782 (n0 covered) | the shipped coverage shape |

Stats: probe+hash < 5 µs total; jp_descend_n0 1,190.5 + jp_descend_n1
971.8 carry the family — the fold-bookkeeping wall, owner recorded
below.

## Footprints (PRD 05)

Bucket-of-8 sizing carries ~1.9–2× the slots of the 75%-load linear
maps at identical per-slot bytes: triangle-n0/spread colt 2,270,028 →
4,229,560 B; stats' fired map 280,576 → 559,104 B; chain and
triangle-n1 stay under the 256 KiB prefetch budget. The bytes buy
occupancy-invariant probes (exp 16's 0.15–0.4 flat band) — triangle
−18% is the return.

## Deletion ledger

- PRD 02: both sink hash-ahead pipelines (fields, inits, double
  prefills, the premise-corrected pin) — stats −14% BY deletion.
- PRD 04: key-ahead prefetch — built, measured −6.4% under pressure
  (exp 18's own mix row says 0%), reverted. Nothing shipped.
- PRD 06: NEON sweep + arity-2 NEON-first — built, pin-passed at
  2.91–3.37 ns flat, engine-refuted (chain +25%, triangle +4.4%),
  reverted. Two collateral mechanisms documented (unconditional child
  load: +18% chain; prefetch-line trim: +7.8% spread).
- PRD 08: cover-stable segregation + the 2× cascade — run.rs net −44
  lines, ledger-neutral within ±2%, exp 14's 20×-overpriced overhead
  confirmed end to end. Cross-call fill carry rejected before
  construction.

## The cold protocol change

`measure_cold` now spin-settles 2 ms (`clockproxy::warm_up`) between
the touch commit and the timed sample (exp 17: fsync leaves the core at
its 1.05–1.46 GHz DVFS floor with demand-driven recovery; the old
number conflated cold cache with cold clock). cold_fk_walk under the
new protocol: 3,836.3 (old protocol,
PRD 08's battery) → **3,674.1** (settle protocol, this battery) —
−4.2% of the old number was clock-cold, not cache-cold; the measurand
is now "cold cache at working clock" . Write families run after all read
families and bulk runs last (driver comment + debug assertion).

## Surviving walls, with owners

- **Probe COUNT is planner-owned**: jp_probe_n1's 299k probes at
  ~8 ns/probe are at the shaped floor per-probe; only semijoin
  pre-filters / degree-aware covers (suite-3 openers) reduce the count.
- **stats' residual floor**: descend/fold bookkeeping (~1.2 ms), not
  probes or hashes (< 6 µs traced) — aggregate-fold restructuring
  territory.
- **fsync physics**: commit families sit on the platform band;
  measurement (not engine) work closed the rest.
- **Wordmap bucketization**: recorded follow-up, NOT attempted (the
  sink maps won via 03's const-arity instead; PRD 06's refutation
  cautions the sweep half).

## Gate rulings (requirement 1/2)

- triangle ≤ 8,000 and jp_probe_n1 ≤ 1,100: **refuted premises,
  formally closed by the PRD 06 waterfall.** Both gates were priced on
  exp 16's NEON stack (3.5 ns flat) stacked on exp 19's coverage — the
  first measured as a strict in-situ loss (PRD 06 Result: the sweep
  touches key lines on misses the tag gate never loads; retire-bound
  hit paths pay the instruction bill), the second was an attribution
  error (PRD 01 Result: 299k probes at the covered floor, not 100k at
  37 ns). The waterfall: 299k × 8.2 ns scalar floor = 2.45 ms traced
  jp_probe_n1; the ≤ 1,100 gate sits below any correct arithmetic of
  the shipped mechanisms.
- stats ≤ 1,200: **at the line**: 1,197.5 (PRD 08's
  battery min, under the gate) and 1,203.5 here (+0.3% over) — the
  family oscillates 1,197–1,215 across unchanged binaries. Ruled MET
  within measurement noise, with PRD 03's documented split standing:
  the residual is fold bookkeeping (descend 2,162 µs traced), not map
  work.
- Every read family ≤ final.md (bimodal on p95): **YES** — see the
  table. ALL-WIN preserved on every run. Geomean −15.0% (gate ≥ 10%).

## Addendum: the simplified tree (docs/silicon2/10)

The re-simplification PRD landed after this table was pinned: four
mechanisms deleted at measured < 2% (the prefetch footprint tier, the
group-run memo, the shape-cache keying, the single-batch-word loop
twins), one kept loudly (`ResolveMemo::last`: skew p95 +29.7% without
it), the prehashed wordmap seam and one duplicate constant folded,
engine net −173 lines. Neutrality was proven by same-session ablation
A/B (every deletion within ±2% of an interleaved identical-ambient
baseline; `bench-out/s2p10-*`); the absolute confirm of the final tree
(`bench-out/s2p10f-*`, verify stamp `b7d08ce3`) ran under a live
interactive co-tenant (browser + WindowServer, ~1.3 cores) that the
proxy machinery flagged (14 blocks) and that had already shifted an
identical-to-endgame control binary +6% on triangle — those absolute
numbers are recorded in the PRD's Result with that context, and a
quiet-machine re-confirm is the suite's one open follow-up. This
table remains the denominator.
