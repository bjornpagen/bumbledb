# The silicon suite — remaining performance work, grounded in measured platform law

This suite turns the bumblebench findings (twelve disassembly-verified
experiments on the reference M2 Max, `~/Documents/bumblebench/docs/`) and
the Apple Silicon BrainLift into the complete remaining optimization
program for bumbledb. The previous suite (`docs/perf/`, PRDs 00–12) ended
with ALL-WIN and four named walls; this suite exists to demolish those
walls and to retune every mechanism the platform research proved we had
mis-modeled.

## The platform laws this suite executes against

Each PRD cites the law it exploits. The laws, with their evidence:

| law | evidence | PRDs |
|---|---|---|
| L2-resident control-dependent probe streams are fully overlapped at batch 1; the surviving cost class is instructions retired per probe (fix = instruction removal, 2–4×) | exp 01 | 02, 03, 14 |
| Open-addressing misses cost MORE than hits (9.2 vs 6.1 ns: walk + exit-branch mispredicts); load factor 0.38→0.05 takes misses to 2.8 ns; branchless probe 4.6× at hit-rate 0 | exp 01 | 03 |
| Probe-exit mispredict flushes expose hash latency across operations (45–85% of the 6.0-cycle mulxor chain); hash-ahead recovers 60–65% | exp 02 | 04 |
| False-tag rate, not probe length, is the hash-quality metric (foldmul: 19.4% false compares on strided keys vs 1/128 design) | exp 02 | 05 |
| Flag ops (`adds/adcs/cmp/csel/cinc`) live on 3 of 6 ALUs; NEON sums 19.6 vs 11.8 rows/ns at L1, exact-u128 NEON (cmhi carry-count) 8.8 vs 4.0–4.6; DRAM converges everything at ~7.5 rows/ns | exps 03, 04 | 06 |
| Dependent flag-µops stranded behind misses halve MLP (14→28 lanes at 4→0 flag-deps/miss); recoverable with `prfm` at DRAM tier only | exp 04 | 06, 09, 10 |
| Prefetch pays iff miss dependents clog the ~115-entry issue queue or a phase bubble idles memory; at L2 residency it is pure loss (+7–12%); sep-pass saturates at batch ~64 | exps 01, 06 | 10 |
| `array::from_fn` refuses to inline: ~34 ns per 8-entry Option table vs 3.4 ns prefix table; hoist crossover L* = build/saving → 4–8 with prefix tables | exp 05 | 08 |
| Always-branchless compaction is correct at every random selectivity (branchy 9× worse at 50%, ~18-cycle effective mispredict); small-footprint branchy wins are TAGE memorization | exp 07 | (validated — no change; harness law in 00) |
| Bounds checks cost zero cycles as instructions; the 1.7× L1 penalty is the second basic block blocking the ×4 `ldp/stp` unroll; `idx&(len-1)` does NOT elide; pre-loop asserts are pure loss | exp 08 | 09 |
| Per-regime ceilings are latency×MLP walls (random gather 3.98 ns/item = 28×111 ns); bumbledb gathers sit 2.6–4.5× above their walls — executor machinery, not memory | exp 09 | 02, 09 |
| 64 B L1D behind a 128 B memory system; set-aliasing maxes at 1.55× on lockstep scans; the REAL pathology is prefetch-tracker aliasing on 16 KB page-number bits: pow-2 pitch + 1–3-line stagger = 4–6× on DRAM scans; cure = pitch + 16 KB | exp 10 | 11 |
| `cntvct` read is 0.30 ns (not ~2 ns); unfenced slide ≤ ~50 ns (scheduler-bounded); `CNTVCTSS_EL0` is the single-shot stamp (4.6 ns); 41.67 ns quantum everywhere | exp 11 | 01 |
| A `bl/ret` boundary disables memory renaming; SLP-merged q-register sink state costs ~12 cycles/item un-renameable vs 1.00-cycle scalar; K independent accumulators overlap at 12/K | exp 12 | 02, 07 |
| Measurement hazards are real and quantified: co-tenant frequency drift (fake 2×), TAGE memorization (4.7×), placement lottery (35%), compiler substitution | exps 02, 03, 07, 08, 09 | 00 |

## Standing walls inherited from docs/perf

- **Triangle probe work**: `jp_probe_n1` ~5.5 ms held flat under 37× batching
  and prefetch (PRD 10's missed gates: p50 ≤ 8,000 µs, probe ≤ 1,500 µs).
  Now explained (retire-bound instruction weight) and owned by 02/03/04/14.
- **Point prologue**: p50 1.0 µs vs 0.8 gate; residual is the per-read LMDB
  txn begin (PRD 11's named lever). Owned by 12.
- **Distinct-stats pass**: ~1.8 ms walk floor on the cold path. Owned by 13.
- **Layout stagger rule**: actively harmful (tracker aliasing). Owned by 11.

## Doctrine

1. **Baseline is law.** PRD 00 commits `baseline.md` (min-of-5 full ledger
   runs + phase tables). Every later gate reads against it. Absolute targets
   in the PRDs below are written from the last committed campaign numbers;
   if PRD 00's re-anchor moves a denominator >10%, scale the gate
   proportionally and record the scaling in the PRD's `## Result`.
2. **Confirm-run protocol.** Any regression >5% on an untouched family
   requires a same-binary confirm run before it counts. Bimodal families
   gate on p95, not p50: fk_walk, balance, skew.
3. **Documented-miss protocol.** A missed numeric gate is not silently
   shipped: the `## Result` must name the mechanism, show the phase split,
   and name the lever that would close it. A documented miss with a named
   mechanism passes review; an undocumented miss does not.
4. **Functional invariants, every PRD:** verify oracle green (2,468 cases),
   batch-size equality, EXPLAIN `emits` digests byte-identical (unless the
   PRD says otherwise), zero-alloc-at-execute gate, `clippy -D warnings`,
   `scripts/check.sh` green at each PRD's own commit.
5. **Unsafe law (docs/architecture/00-product.md):** every new unsafe or
   NEON kernel ships with a portable reference implementation and a
   bit-identity differential test; unsafe lives only in the named-module
   allowlist.
6. **Measurement discipline (from the findings):** cross-run minima for
   throughput claims; the PRD-00 clock proxy brackets every gate run; no
   single-shot timing below 500 ns except via `CNTVCTSS`; any suspicious
   2×-class swing is presumed frequency contamination until the proxy
   clears it.
7. **Disassembly gates are real gates.** Where a PRD specifies an objdump
   grep, the gate is the machine code, not the source. `scripts/check-asm.sh`
   (created in PRD 02) is the vehicle.
8. **Execution protocol:** PRDs are work-organizational units, not atomic
   type-checking states. No transitional shims, no back-compat, no stable
   file format, no migrations (human-owned), no smoke/e2e PRDs
   (human-owned). Rip directly to the end state. Commit with `--no-verify`;
   never `git stash`.

## Order

00 baseline & harness discipline → 01 timer discipline → 02 probe
instruction diet → 03 map geometry → 04 hash-ahead → 05 hash-quality
gates → 06 NEON folds → 07 sink lanes & SLP audit → 08 prefix operand
tables → 09 gather-shape hardening → 10 prefetch tiering → 11 pitch
padding → 12 renewable reader → 13 on-demand stats → 14 endgame.

02–05 are one campaign (the probe wall) split so each lands and gates
independently. 06–09 are the execution-core campaign. 10–11 are the
memory-system campaign. 12–13 are the fixed-cost campaign. 14 closes the
inherited missed gates and records the final table.

## Appendix: round two

`docs/silicon2/` is the successor suite (fleet round two, exps 13–20):
per-rep clock normalization, the const-arity wordmap, the bucket-of-8
COLT layout, the batch-mean lever deletions, and the refutation Results
for key-ahead prefetch and the NEON sweep. `docs/silicon2/final2.md` is
the current denominator; this directory's final.md is the previous one.
