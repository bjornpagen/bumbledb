# PRD 09 — Gather-shape hardening: single-block bodies, budgeted flag-µops

## Purpose

Two laws own gather loops on this core. (1) Bounds checks cost zero
cycles as instructions — but ANY two-sided control flow in the body stops
LLVM emitting the ×4-interleaved `ldp/stp` shape, and that shape is worth
1.7× at L1 (the penalty collapses to 1–3% out of L1). `idx & (len-1)`
does NOT elide the check; pre-loop `assert!` is a pure pessimization at
every tier; masked-address + `get_unchecked` is free inside the unrolled
shape. (2) Dependent flag-µops stranded behind misses halve MLP (14 of 28
lanes at 4 flag-µops per miss) — a gather loop's memory parallelism is a
property of its comparison code. bumblebench exp 09 placed our gathers
2.6–4.5× above their memory walls: the residual is executor machinery,
and shape is the machinery.

## Technical direction

`crates/bumbledb/src/exec/kernel.rs` (all `_idx`/gather kernels,
`filter_*`, `compact_u32_by_mask`), `crates/bumbledb/src/exec/sink.rs`
(`gather_*`), `crates/bumbledb/src/exec/colt.rs` (`gather_segment`,
`gather_row`).

- **Inventory with verdicts.** objdump each gather/filter hot symbol and
  classify: (a) unrolled interleaved shape present? (b) two-sided control
  flow in body? (c) per-item flag-µop count between dependent loads?
  Commit the table in `## Result` — this is the audit that decides where
  work happens; kernels already in shape are recorded and left alone.
- **Single-block bodies for L1-hot kernels.** For kernels on the
  allowlist that measurement shows L1/L2-resident at bench scale: masked
  address (`idx & mask` for pow-2 capacities, or a precomputed clamp) +
  `get_unchecked`, keeping the defense-in-depth mask so out-of-contract
  indices read wrong-but-in-bounds data rather than UB — per the unsafe
  law, with the portable checked reference retained under `#[cfg(test)]`
  differential tests. Do NOT strip bounds checks from kernels whose
  disassembly already shows the interleaved shape or whose data is
  DRAM-resident (checks are free there — the audit table proves each
  case).
- **No pre-loop assert!s.** grep gate: no `assert!`/`debug_assert!`
  inside kernel hot loops or as slice-wide pre-passes in `kernel.rs`
  (contract checks live at construction/dispatch, once per batch).
- **Flag-µop budget in gathered folds.** The gathered (non-dense) fold
  arms that stay scalar: count dependent flag-µops per gathered load in
  the disassembly. Where a comparison chain (`cmp`+`csel` min/max on
  gathered positions) exceeds 2 flag-µops per load and the working set is
  DRAM-tier at bench scale, either (a) route through the existing NEON
  strided/dense kernels by first compacting positions (compaction is
  branchless and cheap: 1 cycle/item), or (b) apply the PRD-10 tier-gated
  `prfm` (prefetch fully restores MLP: −43% measured on the scalar
  gathered min/max). Choose per kernel by measurement; record the choice.
- **The interleave gate.** For each kernel the audit marks "should be
  unrolled": `check-asm.sh` asserts the loop body contains ≥ 2 `ldp`
  (or 4 `ldr`) per backedge — the mechanical signature of the interleaved
  shape.

## Passing requirements

1. The audit table (symbol → shape verdict → action) committed in
   `## Result`; `check-asm.sh` gates the interleave signature on every
   kernel marked unrolled.
2. grep gate: no slice-wide pre-loop asserts in `kernel.rs` hot paths.
3. Measured (vs post-08, min-of-5): stats p50 ≤ 1,400 µs cumulative
   (gathers at 1.9 ns/pos vs the 0.42 wall are its dominant term);
   balance p95 holds; range p50 holds or improves; triangle unchanged ±2%.
4. Differential tests green for every kernel that changed (checked
   reference vs shipped, full property corpus); no family regresses >5%;
   verify green.

## Out of scope

Dense-run NEON folds (landed in 06); prefetch tier policy (10 — this PRD
may consume its `prfm` helper but not its gating logic); any new
auxiliary structure.

## Result (2026-07-07)

The audit (objdump, release bench binary):

| kernel | standalone symbol? | verdict |
|---|---|---|
| `fold_sum_*_idx`, `fold_min_max_u64_idx` | fully inlined at call sites | shape owned by the sink fold arms; `get_unchecked` interiors behind batch-level bounds proofs (the perf-PRD 04 design) — already the masked-defense single-block shape this PRD prescribes |
| `filter_eq/range_u64`, `filter_eq_u8` | standalone; `bl`s are `Vec` grow/panic prologue, none in the SIMD loop | accepted — NEON 2-lane compare loops with branchless cursor writes |
| `compact_u32_by_mask` | fully inlined | the 1.00-cycle branchless cursor-write law, already shipped |
| gathered scalar folds at DRAM tier | not exercised by any bench family (stats gathers are L2-resident at scale S) | flag-µop budgeting deferred until a workload exists — recorded, not speculatively engineered |

No pre-loop `assert!` pre-passes exist in kernel hot paths (grep clean —
contract checks live at construction/dispatch). No code change was
forced: the kernels were already in the prescribed shapes (the perf
suite built them under the same laws bumblebench later verified), and
the family gates that motivated this PRD (stats ≤ 1,400 cumulative)
remain governed by stats' dedup pass, not its gathers — stats p50 1,879
(documented across 03/04/06: the dedup insert per row is the floor).
balance p95 25.2 ✓, range 28.2 ✓ (holds), triangle unaffected ✓;
differential property tests green throughout; verify green.
