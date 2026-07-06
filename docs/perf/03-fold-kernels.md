# PRD 03 — Fold kernels: accumulation at ALU width

## Purpose

PRD 02 gave the aggregate sink a batch to fold; this PRD makes the fold loop
itself run at the machine's arithmetic width. The M2 core sustains ~6 integer
ALU ops/cycle with a ~630-entry ROB — a fold that runs one row per several
cycles is leaving most of the core idle. Target shape: one row per cycle or
better on register-resident batches.

## Technical direction

All kernels live in `exec/kernel.rs` (already the sanctioned unsafe module),
behind scalar-identical signatures, with portable references and bit-identity
property tests per the 00 law.

- **Kernel set.** Over `(values: &[u64], stride: usize)` (stride covers
  entry-major batch key layouts; stride 1 is the contiguous case the
  compiler should reduce to plain loads):
  - `fold_sum_biased_i64(values, stride) -> (i128, u64)` — sum of
    sign-flip-decoded i64 words (`word ^ (1 << 63)` as i64) plus the count.
    **Semantics: exactly equal to the scalar i128 fold** — no intermediate
    overflow is representable (i128 summing < 2^64 terms of i64 cannot wrap).
    Shape: scalar, 4–8 independent accumulators (each `i128` or split
    lo/hi u64+carry — measure both), reduced at the end. Do not use NEON
    here unless it measures faster; 64-bit lane adds at 2/vector rarely beat
    6-wide scalar.
  - `fold_sum_u64(values, stride) -> (u128, u64)` — same discipline.
  - `fold_min_max_u64(values, stride) -> (u64, u64)` — word-order min and
    max in one pass (biased i64 words are order-preserving, so one kernel
    serves both signedness). NEON is legitimate here: `vcgtq_u64` + `vbslq`
    (there is no `vmaxq_u64`/`vminq_u64` — do not look for one), two lanes,
    horizontal reduce at the end; keep 2–4 vector accumulators to break the
    dependency chain. Property-test against the scalar reference.
  - `count` needs no kernel (it is `survivors.len()` after PRD 02's
    constant-group path; keep it arithmetic, not a loop).
- **Gathered variant.** `fold_*_gather(col: &[u64], positions: &[u32])`
  for PRD 05's scan pushdown: position-indexed loads. Unroll by 8; the loads
  are independent — let the OoO window overlap them; add a `prfm pldl1keep`
  on `col[positions[i + 16]]` when `positions.len() > 32` (measure the
  distance; 8–32 ahead is the sane range). Bounds: positions are u32 image
  positions, invariant `< col.len()` — use `get_unchecked` with a
  `debug_assert!` sweep at entry (`positions.iter().all(|p| (*p as usize) < col.len())`
  under `debug_assertions` only).
- **Wiring.** PRD 02's `AggregateSink::fold_batch` calls these kernels per
  `FindSpec::Agg` instead of its per-row match loop. The per-op dispatch
  happens **once per batch**, not once per row: match on the op outside,
  call the kernel inside.
- **Property tests** (kernel.rs test module, extending the existing LENGTHS
  discipline): randomized values including `0`, `u64::MAX`, `i64::MIN/MAX`
  boundary words, strides {1, 2, 3, 5}, lengths {0, 1, 2, 3, 15, 16, 17,
  100, 257, 1000}; assert bit-identity of sums (including the i128), counts,
  min/max against the scalar reference. For the gather variant: positions
  randomized with duplicates and reverse order.

## Passing requirements

1. Kernels + references + property tests as above; `scripts/check.sh` green;
   `verify` green (the oracle re-proves Sum/Min/Max/Count values end to end).
2. Measured (S/seed 1, untraced timing table): `stats` p50 improves further
   from PRD 02's recorded number, reaching **≤ 900 µs** (baseline 4,130.9 µs)
   unless PRD 02 already passed that bar — in which case record the kernel
   delta and require it non-negative outside noise.
3. Throughput evidence: a `#[cfg(test)]`-gated micro-throughput check or a
   recorded phase-table row demonstrating the fused fold sustains
   **≥ 1 row/ns on contiguous stride-1 input** (i.e., ~3 GHz × ~3+ rows per
   3 cycles) on the reference host; record the number in this PRD's
   `## Result` section.
4. Zero-alloc warm gate stays green.

## Out of scope

Group-key hashing (PRD 02 owns the group path), suffix gather into the sink
without materialized batches (PRD 05), non-aarch64 performance.
