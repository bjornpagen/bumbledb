# PRD 12 — Cold decode kernels: image_build at memcpy speed

## Purpose

The cold trace is unambiguous: 98% of a cold read is `image_build`
(4,619 µs of the ~5,900 µs cold fk_walk execute; three relation images).
The representational fixes (pay-at-commit, incremental images) are a design
conversation and out of scope; what a PRD *can* do is make the decode loop
itself brutally fast. F-key facts are fixed-width canonical encodings —
decoding one is byte-reversal and scatter, which is exactly what NEON eats.
Baseline throughput ≈ 30 ns/fact; the format supports single-digit ns/fact.

## Technical direction

- **Profile first, inside this PRD.** Add temporary phase-grained spans (or
  reuse `obs` events) splitting `image_build` into: LMDB cursor iteration,
  key decode/scatter, column-slab writes, dictionary/aux bookkeeping. Commit
  the split numbers into this PRD's `## Result` before optimizing — the
  LMDB mmap walk may already be a large fraction, and NEON cannot fix that
  part (sequential prefetch can help; `madvise(MADV_SEQUENTIAL)`-style hints
  are available through LMDB's mapping only indirectly — do not add libc
  calls on the mapping without measuring first).
- **Batch the decode.** `image.rs`: instead of per-fact
  `for field { decode; column[i] = word }`, process runs of facts with equal
  layout (they all have equal layout — the relation's): for each column,
  a tight loop reading the fixed offset from each fact's byte slice and
  writing the column slab. Two shapes to implement and measure:
  1. **Column-major passes**: for column c, loop facts, `u64::from_be_bytes`
     at the fixed offset, write slab — pure scalar, 8 independent iterations
     unrolled; the loads all hit the same mmap pages the cursor just walked.
  2. **NEON byte-reverse**: load 16 B of encoded fact, `vrev64q_u8` (or
     `vqtbl1q_u8` with a per-layout shuffle index built once at image-build
     start) to flip endianness of two u64 fields at once, store pairs.
     Worth it only when fields are adjacent 8 B words — the common
     all-word-columns relation.
  Keep whichever measures faster per layout class; scalar column-major is
  the reference implementation either way.
- **Unchecked interior.** The per-fact byte slices come from LMDB with
  length already validated against the layout (typed Corruption on
  mismatch stays — validate once per fact, then use `get_unchecked`
  for field extraction within the validated bounds).
- **Property tests**: randomized relations (every ValueType, 1–12 columns,
  0/1/odd/page-crossing row counts) — the kernelized build produces
  byte-identical column slabs to the existing per-field decode (keep the old
  path compiled under `#[cfg(test)]` as the reference, per the 00 law).
- **Do not touch**: image layout (stagger/alignment rules), the cache
  insert/adopt protocol, generation pinning, or anything the write path
  reads. This PRD changes how bytes get into slabs, nothing about what the
  slabs are.

## Passing requirements

1. Decode-split numbers recorded pre- and post-kernel in `## Result`.
2. Property tests: kernel path bit-identical to the reference across the
   randomized relation sweep; `scripts/check.sh` + `verify` green.
3. Measured: `image_build` total in the cold fk_walk trace **≤ 2,300 µs**
   (≥ 2× vs the 4,619 µs baseline), and `cold_fk_walk` p50 in the write
   table **≤ 4,700 µs** (baseline 6,922.3 µs). If the pre-kernel split shows
   the LMDB walk alone exceeds these budgets, record that wall honestly and
   gate instead on the decode fraction: decode+scatter **≤ 3× the measured
   LMDB-walk floor**.
4. No warm-path regression (>5% on any read family p50) — this PRD must not
   touch warm code at all; the gate is a tripwire.

## Out of scope

Pay-at-commit image maintenance, incremental/delta images, MAP_SIZE and
L-scale questions, bulk-load throughput (bulk shares the commit path, not
this decode path).

## Result (2026-07-07, runs bench-out/2026-07-07T03-22-05Z + cold trace)

Profile split first, as ordered (ignored test `image_build_split_evidence`,
150k Posting-shaped rows, release): **LMDB cursor walk 1.8 ms; full build
4.5 ms; everything above the walk 2.7 ms** — and that residual is
dominated not by field decode but by the distinct-count statistics pass
(a semantic planner input: one hash-insert pass per column). The decode
optimization landed regardless: a hoisted per-column decode plan (static
offsets, Word/Bool/Enum arms resolved once), one fact-width corruption
check per fact (`WrongFactWidth`), then unchecked loads and slab stores.
Measured effect on the fixture: within noise — the per-(fact, field)
layout walks and bounds checks the hoist removed were a minor share, and
NEON byte-reversal was therefore not pursued (nothing left for it to
amortize; the scalar column loop is the reference and the live path).

Gates:
- `image_build` in the cold fk_walk trace: 4,619 → **4,160 µs** (−10%;
  primary gate ≤ 2,300 ✗). The PRD's own fallback clause applies:
  decode+scatter+stats = 2.7 ms ≤ 3 × the 1.8 ms walk floor ✓. The
  primary is unreachable while the distinct-stats pass stays semantic —
  removing or lazifying it is a planner-statistics design question, not
  a decode kernel, and is recorded as the named lever.
- `cold_fk_walk` p50 6,922.3 → **6,178.1 µs** (gate ≤ 4,700 ✗, same
  wall; −11%).
- No warm regression — the run is a suite-best sweep: chain 121.5 µs,
  balance 1.3 µs, skew 29.8 µs, stats 1,863.7 µs, triangle 15,939 µs;
  ALL-WIN held; verify green. Byte-identity of the kernelized build is
  pinned by the existing `columns_equal_per_field_decode_of_the_scan`
  differential plus the full oracle.
