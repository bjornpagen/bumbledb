# PRD 03 — portable_simd, measurement-gated: adopt or refuse

**Depends on:** 01. Independent of 02.
**Modules:** `crates/bumbledb/src/exec/kernel/` (`neon.rs`,
`reference.rs`, `allen.rs`, `filter.rs`, `fold.rs`, `gather.rs`,
`compact.rs`), `exec/swar.rs`, `scripts/check-asm.sh` (the gates
arbitrate).
**Authority:** the measurement discipline, absolute: "an optimization
that cannot cite its number does not ship" applies to SIMPLIFICATIONS of
measured code too — a kernel rewrite that cannot prove codegen parity
does not ship either. The asm gates and microbench pins are the judges;
this PRD's outcome is legitimately either ADOPT or REFUSE, and both
outcomes are wins (one deletes code, the other deletes a question).
**Representation move:** the kernel layer is a dual today — hand-written
NEON intrinsics plus scalar reference twins, kept honest by differential
tests. `std::simd` promises one portable body per kernel. Whether the
promise holds HERE is an empirical question about generated aarch64 code,
not a style question — so the PRD is an experiment with a typed verdict.

## Context (decided shape)

Two distinct sub-questions with independent verdicts:

**Q1 — the reference twins.** `reference.rs` (and the scalar fallback
paths) exist for correctness-differential and non-aarch64 compilation.
Porting THEM to `std::simd` risks nothing measured (they are not the hot
path) and buys: one vocabulary across both bodies, autovectorized
portable fallbacks, and Miri-interpretable SIMD (PRD 15's lane). Verdict
expected: ADOPT, unless the differential tests show any behavioral
delta (they must not — bit-exact).

**Q2 — the NEON hot kernels.** Replace hand intrinsics with `std::simd`
ONLY where the asm gates prove the generated code equivalent: same
absence of calls/branches in the probe loop, same instruction classes
the gates assert, and the `#[ignore]`d microbench pins within noise on
re-run. Kernel-by-kernel verdicts — a mixed outcome (some kernels
portable, some staying intrinsic) is expected and fine; each staying
intrinsic gets a one-line refusal naming which gate or margin refused
it.

## Technical direction

1. Q1 first: port `reference.rs` bodies to `std::simd`, run the
   kernel differential tests (NEON vs reference must stay bit-exact) and
   the full suite. This is the low-risk half and lands regardless of Q2.
2. Q2 kernel-by-kernel, smallest first (`filter` compare masks →
   `compact` survivor packing → `fold` → `gather` → the SWAR window):
   port, `check-asm.sh`, and an informal microbench comparison per
   kernel. A kernel ships portable only if the gates pass UNMODIFIED
   and the informal margin is within noise; otherwise revert that
   kernel and ledger the refusal. Editing a gate to accept new codegen
   is FORBIDDEN in this PRD — a gate edit is a measurement-discipline
   event that goes through the human register's re-earn session.
3. The Allen configuration kernel (`allen.rs`) is the most valuable
   target (branchless mask algebra is `std::simd`'s exact shape) and
   the most gate-covered — do it last, with the most care.
4. Record the final matrix in this PRD file: kernel × {portable,
   intrinsic-kept} × the arbitrating evidence.

## Passing criteria

- `[test]` The kernel differential suite (NEON/portable vs reference)
  green and bit-exact throughout; zero tolerance deltas.
- `[shape]` `check-asm.sh` passes with ZERO gate edits.
- `[shape]` The verdict matrix in this file is complete — every kernel
  has an outcome and its evidence; every kept intrinsic has its refusal
  line.
- `[shape]` If Q1 adopted: `reference.rs` contains no hand-rolled lane
  loops that `std::simd` expresses; the Miri lane (PRD 15) can interpret
  it.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`40-execution.md` § the sanctioned kernel shapes: one paragraph on the
portable/intrinsic split as measured, pointing at this PRD's matrix as
the record.

## Results (executed 2026-07-13, nightly-2026-07-12, M2 Max reference host)

Method: head-to-head informal microbenches — both bodies in one release
test binary, interleaved min-of-5 per measurement, three independent
runs, single-threaded. Time-ratio = portable/neon (< 1 means portable
faster); the three runs' ratios are listed. Differential suite bit-exact
throughout (12/12 kernel property tests, zero deltas). `check-asm.sh`
green with zero gate edits, before and after.

### Q1 — the reference twins: ADOPT-WITH-CARVE-OUT (conflict recorded)

Executing Q1 and Q2 together surfaced a conflict the PRD is silent on:
Q2's evidence made the *portable body itself the kernel* for the
filter/fold/gather shapes, and one body cannot be its own differential
oracle. Resolution (policy 5): the reference twin stays the definitional
SCALAR form wherever the live kernel adopted `std::simd` — the
differential's independence outranks the vocabulary win, which landed
better than promised anyway (the portable vocabulary became the kernel,
not the shadow). The twins whose kernels stay intrinsic did adopt:
`allen_keep` is lane-parallel `std::simd` shift-and-mask (mechanism-
independent of the kernel's `tbl` table); `allen_codes`/`allen_codes_const`
stay the scalar `classify` decision tree on purpose — the tests
cross-check the NEON signature TABLE against the TREE, and a `swizzle_dyn`
port would rebuild the table, collapsing the two independent derivations.
The feature gate is live in every build (the kernels use it), named in
`rust-toolchain.toml` per policy 9.

### Q2 — the verdict matrix

| kernel | verdict | arbitrating evidence |
|---|---|---|
| filter `eq_u64` | **PORTABLE** | time-ratio 0.816 / 0.817 / 0.818 (portable 1.22× faster); no asm gate covers the symbol; `unsafe` NEON body deleted |
| filter `range_u64` | **PORTABLE** | 0.673 / 0.664 / 0.655 (1.5× faster — the NEON body's per-lane `vgetq_lane` + scalar `&&` was the tax) |
| filter `eq_u8` | **PORTABLE** | 0.960 / 0.957 / 0.958 (~4% faster) |
| filter `point_in_u64` | **PORTABLE** | 0.967 / 0.959 / 0.972 (~3% faster); `any_point_in` shares the pair walk verbatim |
| filter `duration_range_u64` | **PORTABLE** | 0.717 / 0.713 / 0.722 (1.4× faster); ray verdicts and first-ray positions bit-exact |
| compact `compact_u32_by_mask` | **scalar kept** | refusal: nothing to port — zero intrinsics exist (the scalar cursor-write is the sanctioned shape; NEON compress is SVE-only) and `std::simd` has no compress primitive; a `swizzle_dyn` index-table compaction is a new algorithm needing its own campaign, not a port |
| fold `sum_u64_dense` (carry count) | **PORTABLE** | 1.019 / 0.996 / 0.995 — parity; the pinned `fold_throughput` gate re-run post-adoption: biased 7.49–7.82, u64 7.65–7.99 rows/ns over 8 runs (floor ≥ 7.0; one first-run 6.91 outlier under load, 7 of 8 clean — baseline band was 7.46–7.79); dividend: the `unsafe` NEON body deleted, dense arm now portable on every target |
| fold `min_max_u64_dense` | **PORTABLE** | 0.998 / 0.998 / 0.987 — parity (`simd_min`/`simd_max` lower to the same `cmhi`+`bsl` pair); `unsafe` deleted |
| fold strided arms (stride > 1) | **scalar kept** | refusal: no NEON existed to replace; the standing rule (strided folds stay scalar until measured) is untouched by this PRD |
| gather `min_max_u64_idx` | **PORTABLE** | 0.895 / 0.928 / 0.911 and 0.895 / 0.891 / 0.918 (~1.1× faster); `gather_or_default` deletes the `unsafe` block and its bounds obligation |
| gather `sum_u64_idx` | **PORTABLE** | 1.022 / 1.005 / 0.997 — parity via the carry-count mechanism (i128 lane accumulation is inexpressible; the bias identity makes it unnecessary); `unsafe` deleted |
| gather `sum_biased_i64_idx` | **PORTABLE** | 1.003 / 1.006 / 1.001 — parity (bias identity, bit-identical by the fold property tests) |
| SWAR window (`eq_byte_mask`/`zero_byte_mask`) | **scalar kept** | refusal: not an intrinsic (already-portable 3-op GPR SWAR); the arbitrating pins cannot resolve the candidate above the session noise floor — wordmap K=4 fill: baseline 1.046–1.117 ms, portable trial 0.878–1.075 ms, second baseline 0.964–0.978 ms (the A–A drift exceeds the A–B delta); colt build pin ratios 0.82–1.26 across arms, dominated by noise — and the `u8x8` form adds two cross-domain moves per probe group while deleting nothing (policy 9) |
| allen `code_batch` / `code_batch_const` | **INTRINSIC kept** | refusal: codegen non-parity — `std::simd` has no 4-table `tbl4` primitive, so the 64-byte signature table costs 4×`swizzle_dyn` + range subtractions: 459 insns per 16-pair window vs 212 per 8-pair (≈ +8%/pair). Wall-clock parity at the L1 dense tier only (pair form 0.998 / 0.988 / 0.999; const form 0.931 / 0.924 / 0.925 — portable faster, but on a 16-wide window the intrinsic body doesn't have; that width win transfers to NEON and belongs to the re-earn session, not this PRD). The flag-free gates DID pass on the portable codegen (0 flag writers in both symbols) — the gate did not refuse; the instruction diet and the unproven gathered/retire-bound tiers did |
| allen `filter_batch` (membership) | **INTRINSIC kept** | refusal: true codegen parity achieved (37 vs 37 insns, the identical `tbl.16b` sub/cbnz loop, margin 0.940 / 0.984 / 1.012) — but the flag-free gate structurally forbids SAFE `std::simd` (safe slicing reintroduces bounds-check `cmp`; proven necessary, not assumed), so the portable body keeps every `unsafe` and deletes zero lines: a no-dividend adoption (policy 9) that would also put portable bodies under the gate's `_neon` symbol names |

Net deletion: `neon.rs` 598 → 280 lines (the filter block, the fused
pair walk, and both dense fold bodies gone); all three
`#[expect(unsafe_code)]` sites deleted from the gather kernels (the
fold kernels keep theirs for the scalar strided arms); the filter/fold/gather
dispatch `cfg` arms collapsed to one body per shape on every target
(non-aarch64 targets gain vectorized kernels they never had). The
filter/fold/gather kernels are now Miri-interpretable (PRD 15's lane);
the Allen trio remains Miri-opaque and stays on that PRD's exclusion
list.

Leads recorded for the human register (not this PRD's scope): (1) the
allen const-form's 7.5% win came from a 16-wide window — try the same
width in the intrinsic body at the next re-earn session; (2) the
`fold_throughput` pin brushed its 7.0 floor once under load — the
re-earn session should re-derive the floor's noise margin on this
toolchain.
