# PRD 06 — NEON folds: the doctrine flips

## Purpose

The scalar-ILP-first fold doctrine is measured wrong at cache speeds. On
this core every flag-writing op (`adds/adcs/cmp/csel`) is confined to 3 of
the 6 ALUs; NEON escapes the triad and multiplies load-port width. At L1:
NEON wrapping u64 sum 19.6 rows/ns (the 3×16 B load-port ceiling, 5.8
rows/cycle) vs 11.8 for the best scalar (frontend-bound); EXACT u128 NEON
via `cmhi` carry-counting 8.8 vs 4.0–4.6 for exact scalar i128 (the
`adcs` chain is latency-free — the flag-port triad is the wall). From
DRAM everything converges at ~7.5 rows/ns, so scan-scale folds are
unaffected either way — the win is precisely the L1/L2-resident batch
folds our aggregate families run. The doctrine's founding measurement
(2.45 rows/ns) reproduces on no core/cache combination of the reference
host; it was contamination. min/max already went NEON in the perf suite
(2.65× at every tier, port math exact); sums now follow.

## Technical direction

`crates/bumbledb/src/exec/kernel.rs` (+ its `neon` module),
`crates/bumbledb/src/exec/sink.rs` (fold dispatch),
`docs/architecture/30-execution.md`, `docs/architecture/00-product.md`.

- **`neon::fold_sum_u64_dense`** — wrapping sum over a contiguous `&[u64]`:
  4 × 2-lane `uint64x2_t` accumulators (8 lanes total), `ldp q, q`-shaped
  loads (LLVM emits these from paired `vld1q_u64`; verify in disassembly —
  the load-port ceiling is the target: ≥ 5 rows/cycle), lanes reduced at
  the end. Wrapping semantics == scalar wrapping (bit-identity test
  against the portable reference).
- **`neon::fold_sum_exact_u128_dense`** — exact 128-bit sum via carry
  counting: per step, `new_lo = old_lo + x` (`vaddq_u64`), carry mask
  = `vcgtq_u64(old_lo, new_lo)` (unsigned overflow iff new < old), carry
  accumulator `-= mask` (mask is all-ones = −1 per overflowed lane; keep a
  separate `uint64x2_t` carry counter). Final: u128 = Σ(lo lanes as u128)
  + (Σ carry counts << 64). MUST be bit-identical to the scalar i128/u128
  kernels on the full property-test corpus including overflow boundaries
  (u64::MAX runs, alternating extremes).
- **Biased-i64 variant** rides the same kernels in the biased-u64 domain
  exactly as `fold_sum_biased_i64` does today (sum in wrapped/biased
  space, adjust by `count × bias` at the end) — reuse that arithmetic, do
  not invent a second bias scheme.
- **Dispatch**: `dense_run` detector already exists — dense runs go NEON,
  strided/positioned folds keep the scalar `_idx` kernels (gather-fed
  folds are latency×MLP-bound; NEON gathers are PRD 09's business, not
  this one). No size gate is needed: NEON ≥ scalar at every residency for
  dense input (DRAM converges, L1/L2 wins 2×).
- **Unsafe law**: both kernels get portable references + differential
  bit-identity tests; `neon` stays in the named-module allowlist; extend
  the sanctioned-shapes list in `00-product.md` (sum-wrapping,
  sum-exact-carry-count join min/max there).
- **Doctrine text**: rewrite `30-execution.md`'s scalar-ILP-first section
  into the port-topology law (flag triad, load-port ceiling, DRAM
  convergence), citing bumblebench exps 03/04. The old doctrine's
  evidence (2.45 rows/ns) is recorded as a contamination artifact — keep
  the correction visible, do not erase it.

## Passing requirements

1. `fold_throughput_contiguous_sum` (`#[ignore]`d, proxy-bracketed,
   min-of-9): wrapping ≥ 12 rows/ns, exact-u128 ≥ 7 rows/ns on the
   reference host (baselines 2.45–4.6; bumblebench upper bounds 19.6/8.8 —
   the gates leave engine-overhead headroom).
2. Bit-identity differential tests green for both kernels across the
   property corpus (including overflow boundaries) AND the 2,468-case
   verify oracle (values must be identical end-to-end, not just
   kernel-level).
3. Measured (vs post-05, min-of-5): stats p50 ≤ 1,550 µs; balance p95
   holds (bimodal family — p95 gate); spread p50 −3% or documented (its
   folds are enum-heavy; the win may be small — document honestly).
4. Disassembly gate: the NEON sum loop shows `ldp q`-class paired loads
   and no scalar `adds/adcs` in the loop body.
5. No family regresses >5% (confirm-run); zero-alloc holds; docs greps:
   "scalar-ILP-first" no longer stated as doctrine in `30-execution.md`.

## Out of scope

Gathered (non-dense) NEON folds (09 owns gather shape; a NEON gather fold
is a follow-up only if 09's measurements justify it); f64 folds (none
exist); SME/AMX anything.
