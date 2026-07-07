# The silicon-2 suite — round-two findings, turned into code

Fleet round two (`~/Documents/bumblebench/docs/`, experiments 13–20)
re-examined the surprises the first silicon campaign produced. Its
verdicts are sharper than round one's: two of the campaign's own
optimizations are measured dead weight, one of its "irreducible" walls
has a named, purchasable fix, and the biggest single lever in the
engine is a probe-map layout change with flat-across-hit-rates numbers
already measured. This suite executes all of it.

## The round-two laws this suite executes against

| law | evidence | PRDs |
|---|---|---|
| Residency is a property of phase interleaving, not structure footprint: in situ, inter-phase displacement makes "L2-resident" probe structures miss anyway, and FULL phase-1.5 prefetch coverage buys the whole residual back (34.7–40.9 → 11.4–12.1 ns/probe at every tier) | exp 19 | 01, 09 |
| Optimization composition is regime-dependent; the shipped sink hash-ahead is a strict loss in its shipped shape (+1.2–2.4 ns/row everywhere once the window probe and const-arity land), and isolation studies mis-rank levers | exps 13, 15 | 02 |
| Runtime-arity slice genericity costs 1.9× in a hot probe loop (length-ladder compares, `slot×arity` muls, blocked automatic hash hoisting/gather fusion — LLVM does prefix-hash hoisting FREE under const arity) | exp 15 | 03 |
| Branchless group probing is an in-cache optimization: SWAR resolution makes the key-load address data-dependent (1.4–1.7× DRAM-tier regression); key-ahead `prfm` recovers 21–29% of hit cost under pressure, free at rest | exps 13, 18 (found 3× independently) | 04 |
| The probe-walk layer is mispredict-serialization-bound, not instruction-bound: bucket-of-8 + NEON 8-key sweep probes at 3.5 ns FLAT across hit rates (2×), executes 2.5× more instructions, builds 22% cheaper, occupancy-invariant ≤ 0.4 load | exp 16 | 05, 06 |
| Interleaved `&mut`-scratch stores force `Vec`-header reloads that cost 32% in executor-shaped gather/probe loops (alias analysis, not bounds arithmetic) | exp 19 | 07 |
| Per-pass fixed overhead is 11–30 ns (20× below the estimate that motivated the batching levers): segregation, cascade doubling, and fill carry are ~1% effects — complexity without payoff | exp 14 | 08 |
| Post-fsync cores sit at 1.05–1.46 GHz P-core DVFS (never E-migration); sleeping after I/O costs 2.0–2.6× plus a 25–40% E-core wake lottery; measure immediately or spin | exp 17 | 09 (bench protocol) |
| The unfenced-slide bound is min(remaining payload latency, scheduler drain): µop-poor latency-bound spans mis-attribute up to −99.6%; `CNTVCTSS`/libsystem (commpage kind-3) closes hold ±7% | exp 20 | (doctrine folded into 01/09 doc updates; obs already compliant) |

## The denominator

`docs/silicon/final.md` (min-of-3, 2026-07-07): point 0.4 µs, string
0.8, balance 0.7, fk_walk 2.9 (p95 889), skew 35.8 (p95 924.5), range
28.5, chain 104.0, stats 1,872.5, spread 10,725.8, triangle 11,742.5
with `jp_probe_n1` 3,667 µs / `jp_probe_n0` 1,168 / `jp_hash_n1` 1,339
(traced), cold_fk_walk ~4,018, store 64,421,888 B. PRD 00 re-confirms
it on the current tree before work starts. Absolute gates below are
written from these numbers; if PRD 00's confirmation moves a
denominator > 10%, scale the gate and record the scaling.

## The inherited targets

- **`jp_probe_n1` ≤ 1,500 µs** — missed twice (5,649 → 3,667); exp 19
  predicts full prefetch coverage alone lands ~1,300–1,600. PRD 01
  owns it; PRD 05/06 (bucket map) stack on top.
- **triangle p50 ≤ 8,000 µs** — missed twice (15,064 → 11,742). The
  stack: coverage (−2.1–2.4 ms predicted), bucket map (probe layer
  2×), const-arity dedup (leaf side). PRD 09 measures the sum.
- **stats ≤ 1,200 µs** — never gated this low; exp 15's −0.8 ms
  projection makes it honest now. PRD 03 owns it.

## Doctrine (inherits docs/silicon/README.md rules 1–8, plus)

9. **Per-rep paired-proxy normalization** (exp 15): co-tenant
   contamination arrives as seconds-long 2.0–2.4 GHz spans that
   SURVIVE min-of-reps between clean block-bracket proxies. Any
   suspicious structural finding must be checked against per-rep
   normalized numbers before it is believed. (The bench harness gains
   this in PRD 09.)
10. **Never sleep near I/O in measurement code** (exp 17): post-commit
   settle is a spin; a sleep hands the thread to the E-core lottery.
11. **Serial-chain stalls must be non-associative** (exp 14): LLVM
   reassociates constant-multiplier mul chains into parallel chains.
   The shipped clock proxy uses register-only asm and is safe; any
   future Rust-level calibrated delay uses xor-shift-mul recurrences.
12. **Deletion is a first-class result.** Two PRDs in this suite
   remove shipped optimizations (sink hash-ahead; segregation +
   cascade). The gate for a deletion is the same as for an addition:
   measured, no-regress, documented.

## Order

00 re-anchor → 01 prefetch coverage (the inherited-gate lever) →
02 delete sink hash-ahead → 03 const-arity wordmap → 04 key-ahead
under pressure → 05 bucket-of-8 COLT layout+build → 06 bucket probe +
NEON sweep integration → 07 alias-hoisted executor loops →
08 delete the dead batching levers → 09 endgame: bench protocol,
doctrine docs, final2.md → **10 re-simplify: the whole-crate audit
where every optimization defends its complexity with a measured
citation or is deleted** (runs last, gated measurement-neutral, diff
deletion-dominated — the suite ends with the tree SIMPLER than it
found it everywhere a change didn't pay).

01–04 are independent of 05–06 (wordmap vs colt); they are ordered by
predicted value ÷ risk. 05 and 06 are one campaign split so the layout
lands (with its own differential gates) before the probe cutover.
10 exists because three campaigns of sediment violate the 00-product
simplicity law wherever a lever measured at noise — and rounds one and
two both proved deletion is a measurable discipline.
