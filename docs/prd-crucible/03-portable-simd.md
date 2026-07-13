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
