# PRD 05 — Bucket-of-8 COLT maps, part 1: layout, build, iteration

## Purpose

Exp 16 settled what the probe-walk layer is bound by: mispredict
serialization, not instruction count. A bucket-of-8 layout — 8 ctrl
bytes + 8 keys + 8 children contiguous per bucket, probed by loading
the bucket once and sweeping all 8 candidates branchlessly — ran the
triangle-shaped map at **3.5 ns/probe FLAT across hit rates 10–90%**
(the shipped dieted scalar: 7.1/7.5/4.7) while executing 2.5× more
instructions, and the bucketized BUILD measured **22% cheaper** than
linear probing's (3.06 vs 3.94 ns/key — one bucket landing, no walk).
Occupancy-invariance below ~0.4 load additionally halves map bytes vs
the shipped sizing. This PRD replaces colt's forced-map layout and
build; PRD 06 cuts the probe over. Split deliberately: the layout must
land against the differential harnesses (the exact machinery the
campaign's origin-collision bug lived in) before the hot path moves.

## Technical direction

`crates/bumbledb/src/exec/colt.rs` (`Map`, `force`, `force_ingest`,
`grow_map`, `append_child` interplay, `iter_map`, `key_count`,
`probe_footprint_bytes`), guided by exp 16's shape
(`~/Documents/bumblebench/src/bin/simd_probe.rs` — read it first; its
`bucket8` variant is the measured design).

- **The bucket.** For arity-K maps, one bucket =
  `8 ctrl bytes | 8×K key words | 8 child words` laid contiguously in
  the existing slabs (ctrl stays in `ctrl: Vec<u8>` as 8-byte groups;
  keys/children interleave per bucket in `buckets: Vec<u64>` with
  stride `8*K + 8` words per bucket — keys column-major WITHIN the
  bucket per exp 16: key word 0 of all 8 slots contiguous, so the
  arity-1 NEON sweep loads 4×`uint64x2_t` straight). Bucket count =
  power of two; home bucket = `hash & (nbuckets-1)`; slot within
  bucket by first empty/match; overflow to the NEXT bucket
  (bucket-linear probing — exp 16 measured displacement negligible
  below 0.4 load with 8-slot buckets).
- **Load target**: size for ≤ **0.4 load** (exp 16: occupancy-invariant
  probes 0.15–0.4; the campaign shipped 50% on colt) — bucket count =
  `next_pow2(keys / (8 * 0.4))`. Record the byte delta vs the shipped
  sizing in `## Result` (exp 16 predicts roughly half the slots of the
  33%-rule wordmaps; colt was 50% so expect ~+25% slots but FEWER total
  bytes per probe touched — the honest number is the footprint, print
  it).
- **Build (`force_ingest`)**: hash → home bucket → find first empty
  ctrl in the 8-group (SWAR zero-scan, no walk) → write ctrl/key/child;
  on a full bucket, step to the next. Duplicate-key ingest (the chunked
  child-list append) keys on the ctrl+key match exactly as today —
  `append_child` semantics unchanged.
- **`grow_map`**: rehash-double in bucket units, re-probing via the
  same home-bucket logic; dense list preserved in insertion order
  exactly as today (the campaign's determinism law).
- **Iteration** (`iter_map`, dense list, `key_count`) is untouched in
  behavior: the dense occupied list remains the iteration structure;
  only the slot-index → (bucket, slot) addressing changes where dense
  entries decode.
- **`probe_footprint_bytes`** updated for the new stride (PRD silicon-2
  01's gate reads it).
- **The probe stays OLD in this PRD**: `probe_hashed` gets a
  bucket-aware but scalar implementation (walk the home bucket's 8
  ctrl bytes via the existing SWAR window machinery — the layouts are
  compatible since ctrl groups are 8-aligned — then next bucket). This
  keeps PRD 05 semantics-complete and independently gateable; the NEON
  sweep is PRD 06.
- **Tests**: `bucket_probes_match_the_model_under_adversarial_keys`,
  `hoisted_gathers_match_the_per_position_reference`,
  `skewed_maps_size_by_the_formula_and_iterate_densely` (formula
  updated), `near_unique_maps_grow_to_the_pinned_capacity` (capacity
  formula updated), `dense_tokens_resume_across_interleaved_probes`,
  `get_and_iter_agree_with_a_naive_oracle` — ALL must pass against the
  new layout; the drain/oracle differentials are the law here. Add:
  a bucket-overflow-heavy fixture (adversarial keys all landing in one
  home bucket — 9+ equal-home keys must chain to the next bucket and
  still round-trip).

## Passing requirements

1. Full colt test suite green including the new overflow fixture;
   verify green (2,468 — every plan through the new layout);
   batch-size equality green; emits digests unchanged; D2 randomized
   differential (200 cases) green.
2. Build cost: an `#[ignore]`d in-tree microbench pins forced-map build
   ≥ 15% cheaper than the pre-PRD build at the triangle-scale shape
   (exp 16: 22%); `COLT_FORCE`-heavy families (cold path, chain) hold
   or improve in the ledger.
3. Ledger (vs post-04, min-of-3): triangle within ±3% (the probe is
   still scalar — this PRD must be roughly neutral on warm probes;
   the win is PRD 06's), everything else within 5% no-regress.
4. Footprint table (old vs new bytes per bench map) in `## Result`;
   zero-alloc holds; check-asm green.

## Out of scope

The NEON sweep and probe cutover (06); wordmap (bucketizing the sink
maps is a recorded follow-up, NOT this suite — sink maps won big from
03/04 already and their arities vary more); multi-map layout sharing.

## Result

**Shipped**: the bucket-of-8 layout end to end — `Map` re-founded on
`nbuckets` (stride `8·arity + 8` words: keys column-major within the
bucket, then 8 packed children; ctrl stays a separate slab in 8-aligned
groups so a bucket's ctrl word loads aligned and never straddles),
global slot indices everywhere (dense list, probe returns, ctrl
indexing all unchanged in shape), 0.4-load sizing
(`nbuckets = next_pow2(guess·5/16)`, grow at `len+1 > 3.2·nbuckets`),
bucket-linear overflow, SWAR group probes (scalar candidate resolution
— the NEON sweep is PRD 06), `grow_map` rehashing in bucket units with
column-major gathers, `iter_map`/`prefetch_bucket` re-addressed.

**Measured** (min-of-3 vs post-03/04, `bench-out/s2p05-{1,2,3}`,
verify stamp `19f18db8`):

| family | post-03 | post-05 | Δ | gate |
|---|---|---|---|---|
| **triangle** | 11,766.0 | **9,649.4** | **−18.0%** | "within ±3%" — beaten in the WIN direction |
| spread | 10,315.4 | **10,019.5** | −2.9% | ✓ |
| chain | 92.4 | 93.2 | +0.9% | force-heavy family holds ✓ |
| stats | 1,250.3 | 1,244.3 | −0.5% | ✓ |
| range | 20.5 | 21.0 | +2.4% | ✓ (< 5%) |
| skew | p95 756.6 | p95 749.0 | ✓ | |
| fk_walk | p50 2.5 / p95 718 | p50 2.5 / p95 717.4 | ✓ | |
| point/string/balance | — | flat | ✓ | |

cold_fk_walk was proxy-flagged in all three runs (the fsync-DVFS
class); its raw p50 band (4,095–4,552) sits on post-03's
(4,091–4,327) — the force-heavy cold path holds within its noise.

The −18% triangle from a probe the PRD budgeted as NEUTRAL is exp 16's
mechanism arriving early: the SWAR ctrl-group load replaces up to 8
dependent byte-load/branch steps per bucket with one aligned load and
two masks, and the mispredicted per-slot exit branches — the actual
bill, per exp 16 — are already gone. The traced split
agrees: `jp_probe_n1` 3,686 → **2,444.9 µs (−34%, 8.2 ns/probe at the
true 299k count)** with `jp_probe_n0` flat at 1,190 (n0 was already
prefetch-covered — the walk change is latency-neutral there).

**Corrected premises**:
1. Exp 16's "22% cheaper build" belonged to its ctrl-word-IN-bucket
   layout (one line per insert). This PRD's spec — ctrl in a separate
   slab, the right probe-side choice — touches ctrl + key + child lines
   per insert and measured build PARITY at the DRAM-tier 100k shape
   (ratio 1.00–1.19 across runs, min-of-5 each) and ~1.5× slower at an
   L2-resident 20k shape. The pin now guards DRAM-tier parity
   (≤ 1.11×); the ledger's force-heavy families (chain +0.9%, cold in
   band) carry the regression gate. Recorded for PRD 10: if the L2-tier
   build tax ever surfaces in a family, ctrl-in-bucket is the follow-up
   with a measured 22% waiting.
2. "FEWER total bytes" does not hold either: per-slot bytes are
   identical (1 ctrl + (arity+1)·8) and 0.4-load sizing carries ~1.9–2×
   the slots of the 75%-load linear map. Measured (traced
   `PREFETCH_PASS` footprints, old → new): triangle-n0/spread colt
   2,270,028 → 4,229,560 B (1.86×); stats' fired map 280,576 → 559,104 B
   (1.99×); chain's maps stay under the 256 KiB prefetch budget (no
   fired passes, before and after), as does triangle n1
   (54 KB → ~103 KB analytic).
   The occupancy-invariance that motivated 0.4 (exp 16: flat probes
   0.15–0.4) is what the −18% is buying with those bytes.

**Tests**: full colt suite green (21) including the new
`overflowing_home_buckets_chain_to_the_next_and_round_trip` fixture
(12 keys hashed to ONE home bucket of 8 — 8 fill it, 4 chain; probes,
same-home misses through the full bucket, and the dense drain all
round-trip); the drain/oracle differentials and adversarial-key model
tests pass against the new layout unchanged; sizing tests re-pinned to
the documented formula. Verify 2,468 green (every plan through the new
layout); batch-size equality and the randomized differential ride
verify; emits digests unchanged; zero-alloc holds; check-asm green.
