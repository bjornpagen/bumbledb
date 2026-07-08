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

## Result

**Not shipped — the sweep is an isolation win that inverts in situ.
Measured three ways; the bucket-of-8 SWAR group walk from PRD 05 is
the shipped probe.**

**The pin passed, better than exp 16**: on the engine's own forced-map
geometry (100k arity-1 keys, 4.2 MB map), the NEON sweep probed
**2.91–3.37 ns/probe flat** (spread 0.46 ns) across hit rates
{10, 50, 90}% and **2.67×** over the scalar walk at 50% hits — after
two mechanism fixes over exp 16's kernel that are worth recording:
(1) the lane-index dot product false-matches legal zero-valued keys
against zeroed empty slots — replaced by a `vmovn`-narrowed byte mask
ANDed with the SWAR tag mask (empties can never carry the wanted tag);
(2) a hit/miss early-return branch mispredicts every other probe at
50% hits (+5 ns measured, 9.23 vs 3.10) — resolution must stay csel.

**The engine refuted it** (min-of-3 ledgers). First battery (sweep +
two collateral changes): triangle +6.6%, chain +18%, spread +7.8%.
The collateral decomposed and reverted: an UNCONDITIONAL child load in
`probe_child_at` (touches the child line on every miss — miss-heavy
chain paid a wasted line per probe) and the trimmed third
`prefetch_bucket` line (covered DRAM-tier passes have no latency
shadow for the hit-path child load to ride). Second battery (sweep
ONLY, collateral fixed): triangle 9,649 → 10,073 (+4.4%), chain 93.2 →
116.2 (+25%), spread +5.5%, skew p95 +6.6%, stats +1.2% — **no family
improved**. Verify green both times (stamps `89772158`, `013fc564`).

**Why the pin and the engine disagree** — two regimes, both against:
- Chain's probes are arity-1 FK hops over L2-hot maps at ~100% hit
  rate: retire-bound, perfectly predicted. The sweep's 2.5×
  instruction bill buys nothing there — the layer law's own dieting
  side.
- Triangle/spread's displaced-tier probes (exp 19's law: phases
  interleave, maps miss in situ regardless of footprint): the sweep
  LOADS THE KEY BLOCK ON EVERY PROBE, while the tag-gated walk's
  data-dependent key load never issues on a tag miss — one extra line
  per miss at exactly the tier where lines are the bill. Exp 13 saw
  this same inversion from the other side (branchy vs window at DRAM).
  The pin's isolated loop keeps the map resident, so neither
  mechanism exists there.
- Exp 16's bucket kept ctrl INSIDE the bucket line, so its sweep
  touched one line either way — its 3.5 ns flat does not transfer to
  the separate-ctrl layout that PRD 05 shipped (and that layout choice
  was right: it is what made the SWAR group walk and the −18% land).

**Per-arity cutover decisions** (requirement 4): arity 1 — measured,
REVERTED (above). Arity 2 — the NEON-first word-0 sweep measured
16.12 ns vs the scalar walk's 11.84 ns at 50% hits (ratio 0.73, far
under the 1.10 cutover bar); rejected, variant deleted (history:
b193567..this commit). Arities 3–4 follow a fortiori (more scalar
verification per candidate, same sweep overhead).

**Gate rulings**: `jp_probe_n1` ≤ 1,100 and triangle ≤ 8,000 are
**refuted premises, not misses** — they price a probe-layer mechanism
that measures as a strict in-situ loss. The waterfall stands at:
jp_probe_n1 2,445 µs = 8.2 ns/probe over the true 299k count
(post-05, −34% from PRD 05 itself), triangle 9,649 — under PRD 01's
twice-missed 9,800 gate at last. AMAC and gather-vectorization stay
rejected as drafted; this Result closes the exp-16 thread: the
instruction-buying law holds only where a mispredicted exit branch was
the bill AND the touched-lines count does not grow — inside one
resident bucket line (exp 16's layout), not across the shipped
split-slab layout.

30-execution.md's probe paragraph now carries the measured law; the
sanctioned-shapes list is unchanged (nothing ships).
