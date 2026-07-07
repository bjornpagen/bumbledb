# PRD 06 — Bucket-of-8 COLT maps, part 2: the NEON sweep probe cutover

## Purpose

With the layout landed (05), this PRD ships the probe that exp 16
measured: load the home bucket's 8 ctrl bytes as one word, sweep the
matching slots' keys with NEON compares, resolve hit/miss and the child
word branchlessly — 3.5 ns/probe flat across hit rates, IPC 0.8 → 4.1,
2× over the dieted scalar walk at mixed hit rates. Adding instructions
to delete branches: the round-two law the campaign's instruction-diet
doctrine has to share the stage with. Stacked on PRD 01's prefetch
coverage, the predicted triangle probe layer approaches the shaped
floor.

## Technical direction

`crates/bumbledb/src/exec/colt.rs` (`probe_hashed` and the
`probe_walk` monomorphs), following exp 16's disassembly-verified
kernel (`~/Documents/bumblebench/src/bin/simd_probe.rs` `bucket8_neon`).

- **Arity-1 (the dominant shape — triangle, chain, most joins)**: one
  `u64` ctrl-group load + SWAR tag mask; load the bucket's 8 key words
  as 4×`uint64x2_t` (`vld1q_u64` pairs — the column-major key layout
  from 05 makes this contiguous); `vceqq_u64` against the broadcast
  key; merge the NEON match mask with the SWAR tag mask; `rbit/clz`
  the combined candidate mask; select the child word with a branchless
  index (exp 16's micro-mechanism: the miss-slot clamp decides which
  probes pay a third cache line — replicate its clamp choice, and its
  one-`csel`-costs-0.7cy note says keep the child-address chain
  csel-free where possible). Miss = no candidate AND the bucket has an
  empty ctrl (else continue to the next bucket).
- **Arity 2–4**: NEON-compare key word 0 across the 8 slots (same as
  arity-1) to get a candidate mask, then verify remaining words
  scalar per candidate (expected candidates ≤ 1 at 7-bit tags — the
  verification is one straight-line compare, exactly the campaign's
  monomorphic `probe_walk::<A>` body per candidate). Exp 16's caveat
  (multi-word keys shift the ranking) is handled by gating: keep the
  05 scalar bucket walk as the arity ≥ 2 path INITIALLY, and cut
  arities ≥ 2 over only if the in-tree microbench shows the NEON-first
  variant ≥ 10% better at arity 2 — record the measured choice per
  arity in `## Result`.
- **`prefetch_bucket`** updated: one bucket = ctrl group + key block +
  child block, contiguous — a single `prfm` per 64 B line spanned by
  the bucket's stride (arity-1 bucket = 8+64+64 = 136 B ≈ 3 lines;
  prefetch the first two — the child line only matters on hits and
  rides the sweep's latency).
- **Unsafe law**: the NEON sweep lives in the colt module (allowlisted)
  with a portable scalar reference (the 05 walk IS the reference) and
  bit-identity differential tests across the adversarial corpus,
  including the overflow-chain fixture; extend `00-product.md`'s
  sanctioned-shape list ("bucketized probe sweep").
- **check-asm**: the arity-1 probe monomorph shows `vceqq`-class NEON
  compares (`cmeq.2d`) and no `bl` (probe-class list); the campaign's
  probe gates stay green.
- **Doc**: `30-execution.md`'s probe paragraph gains the layer law:
  retire-bound loops diet instructions; flush-bound walks buy
  instructions to delete branches (exps 02→16 arc, both cited).

## Passing requirements

1. In-tree `#[ignore]`d probe microbench (the exp-16 shape, engine map
   geometry): NEON bucket sweep ≤ 4.5 ns/probe at hit rates {10, 50,
   90}% with spread ≤ 1 ns across them (exp 16: 3.48–3.58; headroom
   for engine-shape differences), ≥ 1.6× over the PRD-05 scalar bucket
   walk at 50% hits.
2. Measured (vs post-05, min-of-3): **`jp_probe_n1` ≤ 1,100 µs**
   (stacked on PRD 01's ≤ 1,500 — exp 16 prices the probe layer at
   half the scalar cost; documented-miss with the traced split if the
   stack interferes); `jp_probe_n0` improves further; **triangle p50
   ≤ 8,000 µs** — the twice-inherited gate, now gated HERE where the
   stack completes (documented-miss protocol, high bar: per-phase
   waterfall vs the exp 16 + exp 19 predictions).
3. chain/skew/spread hold or improve; no family regresses > 5%; verify
   green; emits digests unchanged; batch-size equality + D2 (200)
   green; zero-alloc holds; check-asm green.
4. `## Result` records the per-arity cutover decisions with their
   microbench rows, and the final probe disassembly excerpt.

## Out of scope

Wordmap bucketization (recorded follow-up); AMAC-style interleaving
(exp 16: floors at 5.1 ns, dominated — considered-and-rejected);
vertical/gather vectorization (Polychroniou-Ross needs gather hardware
NEON lacks — rejected with citation).
