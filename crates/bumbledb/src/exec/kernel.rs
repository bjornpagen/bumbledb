//! The explicit-SIMD and unrolled-fold kernels (docs/architecture/30-execution.md;
//! sanctioned shapes amended by docs/perf/): fixed-width predicate scans,
//! survivor compaction, and — PRD 03 — the fold/accumulate kernels behind
//! the aggregate sink's batch path, all behind scalar-identical
//! signatures.
//!
//! `unsafe` is sanctioned here per the 00-product policy. The NEON paths
//! are `cfg(target_arch = "aarch64")`; every other 64-bit platform
//! compiles and runs the scalar fallback correctly, with no performance
//! promises. The scalar implementations are compiled wherever they have a
//! reader — the non-aarch64 live path and every target's test builds,
//! where they are the reference the property tests compare the kernels
//! against, bit for bit (an aarch64 release build omits them: dead code
//! is disallowed).
//!
//! Fold doctrine (30-execution, amended): **scalar-ILP first** — the fold
//! kernels are unrolled multi-accumulator scalar loops (the M2 runs
//! 6-wide integer ALU; 2-lane NEON adds on 64-bit data lose to it), and
//! NEON earns a slot only where measured faster (min/max via
//! `vcgtq_u64` + `vbslq_u64` — there is no `vmaxq_u64`). Sum semantics
//! are exact i128/u128 accumulation — bit-identical to the naive fold at
//! any association, since fewer than 2^64 i64 terms cannot wrap i128.
//!
//! Survivor compaction is the scalar cursor-write on every target: NEON has
//! no compress instruction (that is SVE, which Apple Silicon lacks), and
//! 30-execution names "NEON compress **or scalar cursor-write**" as the
//! sanctioned shapes. No x86 intrinsics exist anywhere (doctrine).
//!
//! Fallback verification command (run where a cross std is installed):
//! `cargo check --workspace --target x86_64-unknown-linux-gnu`. The
//! non-aarch64 dispatch arms are one-line calls into [`reference`], which
//! compiles and is property-tested on every target.

/// Positions in `col` equal to `value`, appended to `out` in ascending
/// order (branchless survivor writes).
pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
    #[cfg(target_arch = "aarch64")]
    neon::filter_eq_u64(col, value, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_eq_u64(col, value, out);
}

/// Positions in `col` within `lo..=hi` (u64 word order — order-preserving
/// for I64's biased words too), appended to `out` in ascending order.
pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    #[cfg(target_arch = "aarch64")]
    neon::filter_range_u64(col, lo, hi, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_range_u64(col, lo, hi, out);
}

/// Positions in `col` equal to `value` (the enum/bool column variant,
/// 16 lanes), appended to `out` in ascending order.
pub fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
    #[cfg(target_arch = "aarch64")]
    neon::filter_eq_u8(col, value, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_eq_u8(col, value, out);
}

/// Compacts `items` in place, keeping `items[i]` where `mask[i] != 0` —
/// the survivor-compaction kernel (scalar cursor-write on every target;
/// see the module docs).
///
/// # Panics
///
/// Only on a programmer-invariant violation: `mask` shorter than `items`.
pub fn compact_u32_by_mask(items: &mut Vec<u32>, mask: &[u8]) {
    assert!(mask.len() >= items.len());
    let mut write = 0usize;
    for read in 0..items.len() {
        items[write] = items[read];
        write += usize::from(mask[read] != 0);
    }
    items.truncate(write);
}

/// Best-effort read prefetch into L1 (`prfm pldl1keep`); a no-op off
/// aarch64. Purely a scheduling hint — no architectural effect, no
/// safety obligations on the pointer beyond being a valid address to
/// hint about (a stale hint is harmless).
#[inline]
#[allow(unsafe_code)]
pub fn prefetch_read<T>(ptr: *const T) {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: prfm is a hint; it cannot fault and has no memory effects.
    unsafe {
        core::arch::asm!("prfm pldl1keep, [{p}]", p = in(reg) ptr, options(readonly, nostack));
    }
    #[cfg(not(target_arch = "aarch64"))]
    let _ = ptr;
}

// ---------------------------------------------------------------------
// The fold kernels (docs/perf/ PRD 03). Two access shapes each:
// `_idx` gathers `values[idx as usize * stride + offset]` per index (the
// leaf batch's entry-major keys, and PRD 05's position gathers at
// stride 1 / offset 0); the contiguous form walks a strided slice
// directly (dense survivor runs — no index loads at all).
// ---------------------------------------------------------------------

/// The i64 biased-word sign flip (order-preserving storage form to
/// logical value).
#[inline]
fn biased_to_i64(word: u64) -> i64 {
    (word ^ (1 << 63)).cast_signed()
}

/// Bounds proof for the `_idx` kernels, checked once per call in debug
/// builds; the unchecked interior relies on it.
#[inline]
fn debug_assert_idx_bounds(values: &[u64], stride: usize, offset: usize, indices: &[u32]) {
    debug_assert!(stride > 0);
    debug_assert!(
        indices
            .iter()
            .all(|&i| i as usize * stride + offset < values.len()),
        "leaf-batch indices are in-bounds by construction"
    );
}

/// Sum of sign-flip-decoded i64 words at the indexed positions — exact
/// i128, bit-identical to the naive fold.
#[must_use]
#[allow(unsafe_code)]
pub fn fold_sum_biased_i64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> i128 {
    debug_assert_idx_bounds(values, stride, offset, indices);
    // Four independent accumulators: the adds race down separate
    // dependency chains while the OoO window overlaps the gathers.
    let mut acc = [0i128; 4];
    let chunks = indices.chunks_exact(4);
    let tail = chunks.remainder();
    for chunk in chunks {
        for (lane, &idx) in chunk.iter().enumerate() {
            // SAFETY: debug-asserted above; indices are image/batch
            // positions produced against `values`' extent.
            let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
            acc[lane] += i128::from(biased_to_i64(word));
        }
    }
    for &idx in tail {
        let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
        acc[0] += i128::from(biased_to_i64(word));
    }
    acc[0] + acc[1] + acc[2] + acc[3]
}

/// Sum of u64 words at the indexed positions — exact u128.
#[must_use]
#[allow(unsafe_code)]
pub fn fold_sum_u64_idx(values: &[u64], stride: usize, offset: usize, indices: &[u32]) -> u128 {
    debug_assert_idx_bounds(values, stride, offset, indices);
    let mut acc = [0u128; 4];
    let chunks = indices.chunks_exact(4);
    let tail = chunks.remainder();
    for chunk in chunks {
        for (lane, &idx) in chunk.iter().enumerate() {
            // SAFETY: as in `fold_sum_biased_i64_idx`.
            let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
            acc[lane] += u128::from(word);
        }
    }
    for &idx in tail {
        let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
        acc[0] += u128::from(word);
    }
    acc[0] + acc[1] + acc[2] + acc[3]
}

/// Word-order (min, max) at the indexed positions in one pass — biased
/// i64 words are order-preserving, so one kernel serves both
/// signednesses.
///
/// # Panics
///
/// Only on a programmer-invariant violation: an empty index list (the
/// executor never emits empty batches).
#[must_use]
#[allow(unsafe_code)]
pub fn fold_min_max_u64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    assert!(!indices.is_empty(), "non-empty batch");
    debug_assert_idx_bounds(values, stride, offset, indices);
    let mut mins = [u64::MAX; 4];
    let mut maxs = [u64::MIN; 4];
    let chunks = indices.chunks_exact(4);
    let tail = chunks.remainder();
    for chunk in chunks {
        for (lane, &idx) in chunk.iter().enumerate() {
            // SAFETY: as in `fold_sum_biased_i64_idx`.
            let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
            mins[lane] = mins[lane].min(word);
            maxs[lane] = maxs[lane].max(word);
        }
    }
    for &idx in tail {
        let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
        mins[0] = mins[0].min(word);
        maxs[0] = maxs[0].max(word);
    }
    (
        mins.iter().copied().min().expect("four lanes"),
        maxs.iter().copied().max().expect("four lanes"),
    )
}

/// Contiguous strided sum of biased-i64 words over
/// `values[offset], values[offset + stride], ..` for `count` elements —
/// the dense-survivor fast form (no index loads).
///
/// # Panics
///
/// Only on a programmer-invariant violation: the strided extent
/// exceeding `values`.
#[must_use]
#[allow(unsafe_code)]
pub fn fold_sum_biased_i64(values: &[u64], stride: usize, offset: usize, count: usize) -> i128 {
    assert!(stride > 0 && (count == 0 || (count - 1) * stride + offset < values.len()));
    let mut acc = [0i128; 4];
    let mut i = 0;
    while i + 4 <= count {
        for (lane, slot) in acc.iter_mut().enumerate() {
            // SAFETY: the extent assert above covers every index.
            let word = unsafe { *values.get_unchecked((i + lane) * stride + offset) };
            *slot += i128::from(biased_to_i64(word));
        }
        i += 4;
    }
    while i < count {
        let word = unsafe { *values.get_unchecked(i * stride + offset) };
        acc[0] += i128::from(biased_to_i64(word));
        i += 1;
    }
    acc[0] + acc[1] + acc[2] + acc[3]
}

/// Contiguous strided sum of u64 words (see [`fold_sum_biased_i64`]).
///
/// # Panics
///
/// Only on a programmer-invariant violation: the strided extent
/// exceeding `values`.
#[must_use]
#[allow(unsafe_code)]
pub fn fold_sum_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> u128 {
    assert!(stride > 0 && (count == 0 || (count - 1) * stride + offset < values.len()));
    let mut acc = [0u128; 4];
    let mut i = 0;
    while i + 4 <= count {
        for (lane, slot) in acc.iter_mut().enumerate() {
            // SAFETY: the extent assert above covers every index.
            let word = unsafe { *values.get_unchecked((i + lane) * stride + offset) };
            *slot += u128::from(word);
        }
        i += 4;
    }
    while i < count {
        let word = unsafe { *values.get_unchecked(i * stride + offset) };
        acc[0] += u128::from(word);
        i += 1;
    }
    acc[0] + acc[1] + acc[2] + acc[3]
}

/// Contiguous strided (min, max) in one pass. Stride 1 takes the NEON
/// lane path on aarch64 (`vcgtq_u64` + `vbslq_u64` — the compare-select
/// pair; there is no 64-bit lane min/max instruction).
///
/// # Panics
///
/// Only on a programmer-invariant violation: zero `count` or the strided
/// extent exceeding `values`.
#[must_use]
pub fn fold_min_max_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> (u64, u64) {
    assert!(count > 0 && stride > 0 && (count - 1) * stride + offset < values.len());
    #[cfg(target_arch = "aarch64")]
    if stride == 1 {
        return neon::fold_min_max_u64_dense(&values[offset..offset + count]);
    }
    fold_min_max_u64_strided(values, stride, offset, count)
}

/// The scalar strided (min, max) — the live path wherever the NEON dense
/// form does not apply.
#[allow(unsafe_code)]
fn fold_min_max_u64_strided(
    values: &[u64],
    stride: usize,
    offset: usize,
    count: usize,
) -> (u64, u64) {
    let mut mins = [u64::MAX; 4];
    let mut maxs = [u64::MIN; 4];
    let mut i = 0;
    while i + 4 <= count {
        for lane in 0..4 {
            // SAFETY: the caller asserted the strided extent.
            let word = unsafe { *values.get_unchecked((i + lane) * stride + offset) };
            mins[lane] = mins[lane].min(word);
            maxs[lane] = maxs[lane].max(word);
        }
        i += 4;
    }
    while i < count {
        let word = unsafe { *values.get_unchecked(i * stride + offset) };
        mins[0] = mins[0].min(word);
        maxs[0] = maxs[0].max(word);
        i += 1;
    }
    (
        mins.iter().copied().min().expect("four lanes"),
        maxs.iter().copied().max().expect("four lanes"),
    )
}

/// The scalar reference implementations: the fallback on non-aarch64
/// targets and the comparison oracle the aarch64 property tests assert
/// bit-identity against (absent only in aarch64 non-test builds, where
/// they would be dead code).
#[cfg(any(not(target_arch = "aarch64"), test))]
pub mod reference {
    /// Scalar reference of [`super::filter_eq_u64`].
    pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
        push_matching(col.len(), out, |i| col[i] == value);
    }

    /// Scalar reference of [`super::filter_range_u64`].
    pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
        push_matching(col.len(), out, |i| (lo..=hi).contains(&col[i]));
    }

    /// Scalar reference of [`super::filter_eq_u8`].
    pub fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
        push_matching(col.len(), out, |i| col[i] == value);
    }

    /// Branchless cursor-write over the whole column.
    fn push_matching(len: usize, out: &mut Vec<u32>, keep: impl Fn(usize) -> bool) {
        let start = out.len();
        out.resize(start + len, 0);
        let mut write = start;
        for i in 0..len {
            out[write] = u32::try_from(i).expect("positions fit u32");
            write += usize::from(keep(i));
        }
        out.truncate(write);
    }
}

/// The NEON kernels (128-bit: 2 x u64 or 16 x u8 lanes).
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code)] // the 30-execution doc: the one sanctioned unsafe module
mod neon {
    use std::arch::aarch64::{
        vceqq_u64, vceqq_u8, vcgeq_u64, vcleq_u64, vdupq_n_u64, vdupq_n_u8, vgetq_lane_u64,
        vld1q_u64, vld1q_u8, vst1q_u8,
    };

    pub(super) fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
        let start = out.len();
        out.resize(start + col.len(), 0);
        let mut write = start;
        // SAFETY: every vld1q_u64 reads exactly two u64s from within
        // `chunks_exact(2)` of `col`; unaligned loads are legal for vld1q.
        unsafe {
            let needle = vdupq_n_u64(value);
            let chunks = col.chunks_exact(2);
            let tail_start = col.len() - chunks.remainder().len();
            for (chunk_idx, chunk) in chunks.enumerate() {
                let lanes = vld1q_u64(chunk.as_ptr());
                let mask = vceqq_u64(lanes, needle);
                let base = u32::try_from(chunk_idx * 2).expect("positions fit u32");
                out[write] = base;
                write += usize::from(vgetq_lane_u64(mask, 0) != 0);
                out[write] = base + 1;
                write += usize::from(vgetq_lane_u64(mask, 1) != 0);
            }
            for (i, item) in col[tail_start..].iter().enumerate() {
                out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
                write += usize::from(*item == value);
            }
        }
        out.truncate(write);
    }

    pub(super) fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
        let start = out.len();
        out.resize(start + col.len(), 0);
        let mut write = start;
        // SAFETY: as in `filter_eq_u64` — two-lane loads within bounds.
        unsafe {
            let lo_v = vdupq_n_u64(lo);
            let hi_v = vdupq_n_u64(hi);
            let chunks = col.chunks_exact(2);
            let tail_start = col.len() - chunks.remainder().len();
            for (chunk_idx, chunk) in chunks.enumerate() {
                let lanes = vld1q_u64(chunk.as_ptr());
                let ge = vcgeq_u64(lanes, lo_v);
                let le = vcleq_u64(lanes, hi_v);
                let base = u32::try_from(chunk_idx * 2).expect("positions fit u32");
                out[write] = base;
                write += usize::from(vgetq_lane_u64(ge, 0) != 0 && vgetq_lane_u64(le, 0) != 0);
                out[write] = base + 1;
                write += usize::from(vgetq_lane_u64(ge, 1) != 0 && vgetq_lane_u64(le, 1) != 0);
            }
            for (i, item) in col[tail_start..].iter().enumerate() {
                out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
                write += usize::from((lo..=hi).contains(item));
            }
        }
        out.truncate(write);
    }

    /// Dense (min, max) over a contiguous u64 slice: compare-select
    /// lanes (`vcgtq_u64` + `vbslq_u64` — NEON has no 64-bit lane
    /// min/max), four vector accumulators to break the dependency
    /// chains, scalar tail.
    pub(super) fn fold_min_max_u64_dense(values: &[u64]) -> (u64, u64) {
        let mut min_scalar = u64::MAX;
        let mut max_scalar = u64::MIN;
        // SAFETY: every vld1q_u64 reads exactly two u64s from within
        // `chunks_exact(8)` of `values`.
        unsafe {
            use std::arch::aarch64::{vbslq_u64, vcgtq_u64};
            let mut mins = [vdupq_n_u64(u64::MAX); 4];
            let mut maxs = [vdupq_n_u64(u64::MIN); 4];
            let chunks = values.chunks_exact(8);
            let tail = chunks.remainder();
            for chunk in chunks {
                for lane in 0..4 {
                    let v = vld1q_u64(chunk.as_ptr().add(lane * 2));
                    mins[lane] = vbslq_u64(vcgtq_u64(mins[lane], v), v, mins[lane]);
                    maxs[lane] = vbslq_u64(vcgtq_u64(v, maxs[lane]), v, maxs[lane]);
                }
            }
            for lane in 0..4 {
                min_scalar = min_scalar
                    .min(vgetq_lane_u64(mins[lane], 0))
                    .min(vgetq_lane_u64(mins[lane], 1));
                max_scalar = max_scalar
                    .max(vgetq_lane_u64(maxs[lane], 0))
                    .max(vgetq_lane_u64(maxs[lane], 1));
            }
            for &v in tail {
                min_scalar = min_scalar.min(v);
                max_scalar = max_scalar.max(v);
            }
        }
        (min_scalar, max_scalar)
    }

    pub(super) fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
        let start = out.len();
        out.resize(start + col.len(), 0);
        let mut write = start;
        // SAFETY: every vld1q_u8 reads exactly sixteen bytes from within
        // `chunks_exact(16)`; the mask store writes a stack array.
        unsafe {
            let needle = vdupq_n_u8(value);
            let chunks = col.chunks_exact(16);
            let tail_start = col.len() - chunks.remainder().len();
            let mut mask_bytes = [0u8; 16];
            for (chunk_idx, chunk) in chunks.enumerate() {
                let lanes = vld1q_u8(chunk.as_ptr());
                let mask = vceqq_u8(lanes, needle);
                vst1q_u8(mask_bytes.as_mut_ptr(), mask);
                let base = chunk_idx * 16;
                for (lane, m) in mask_bytes.iter().enumerate() {
                    out[write] = u32::try_from(base + lane).expect("positions fit u32");
                    write += usize::from(*m != 0);
                }
            }
            for (i, item) in col[tail_start..].iter().enumerate() {
                out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
                write += usize::from(*item == value);
            }
        }
        out.truncate(write);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic LCG so the property sweeps are reproducible.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }
    }

    /// Lengths that stress lane boundaries: empty, single, odd, lane
    /// multiples +/- 1.
    const LENGTHS: &[usize] = &[0, 1, 2, 3, 15, 16, 17, 31, 32, 33, 100, 257];

    #[test]
    fn u64_kernels_match_the_scalar_reference_bit_for_bit() {
        let mut rng = Lcg(42);
        for &len in LENGTHS {
            // Narrow value range forces plenty of matches; extremes too.
            let col: Vec<u64> = (0..len)
                .map(|_| match rng.next() % 8 {
                    0 => 0,
                    1 => u64::MAX,
                    n => n % 4,
                })
                .collect();
            for needle in [0u64, 1, 2, 3, u64::MAX] {
                let (mut kernel, mut reference) = (Vec::new(), Vec::new());
                filter_eq_u64(&col, needle, &mut kernel);
                super::reference::filter_eq_u64(&col, needle, &mut reference);
                assert_eq!(kernel, reference, "eq len {len} needle {needle}");
            }
            for (lo, hi) in [(0u64, 2u64), (1, 1), (3, u64::MAX), (u64::MAX, 0)] {
                let (mut kernel, mut reference) = (Vec::new(), Vec::new());
                filter_range_u64(&col, lo, hi, &mut kernel);
                super::reference::filter_range_u64(&col, lo, hi, &mut reference);
                assert_eq!(kernel, reference, "range len {len} {lo}..={hi}");
            }
        }
    }

    #[test]
    fn u8_kernel_matches_the_scalar_reference() {
        let mut rng = Lcg(7);
        for &len in LENGTHS {
            let col: Vec<u8> = (0..len)
                .map(|_| u8::try_from(rng.next() % 3).expect("small"))
                .collect();
            for needle in [0u8, 1, 2, 255] {
                let (mut kernel, mut reference) = (Vec::new(), Vec::new());
                filter_eq_u8(&col, needle, &mut kernel);
                super::reference::filter_eq_u8(&col, needle, &mut reference);
                assert_eq!(kernel, reference, "u8 eq len {len} needle {needle}");
            }
        }
    }

    #[test]
    fn results_preserve_ascending_position_order() {
        let col: Vec<u64> = (0..1000).map(|i| i % 5).collect();
        let mut out = Vec::new();
        filter_eq_u64(&col, 3, &mut out);
        assert!(out.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(out.len(), 200);
    }

    /// PRD 03 (docs/perf/): the fold kernels are bit-identical to naive
    /// folds across strides, boundary words, duplicate and reversed
    /// indices, and lane-boundary lengths.
    #[test]
    fn fold_kernels_match_the_naive_folds_bit_for_bit() {
        let mut rng = Lcg(99);
        for &len in LENGTHS {
            for &stride in &[1usize, 2, 3, 5] {
                for &offset in &[0usize, 1] {
                    if stride == 1 && offset > 0 {
                        continue;
                    }
                    let slots = len * stride + offset + 1;
                    let values: Vec<u64> = (0..slots)
                        .map(|_| match rng.next() % 6 {
                            0 => 0,
                            1 => u64::MAX,
                            2 => 1 << 63,       // i64 0
                            3 => (1 << 63) - 1, // i64 -1's neighbor
                            _ => rng.next(),
                        })
                        .collect();
                    // Indices with duplicates, reversed order, and gaps.
                    let mut indices: Vec<u32> =
                        (0..len).map(|i| u32::try_from(i).expect("small")).collect();
                    indices.reverse();
                    if len > 2 {
                        indices.push(1);
                        indices.push(1);
                    }

                    let at = |i: u32| values[i as usize * stride + offset];
                    let naive_sum_i: i128 = indices
                        .iter()
                        .map(|&i| i128::from(super::biased_to_i64(at(i))))
                        .sum();
                    let naive_sum_u: u128 = indices.iter().map(|&i| u128::from(at(i))).sum();
                    assert_eq!(
                        fold_sum_biased_i64_idx(&values, stride, offset, &indices),
                        naive_sum_i,
                        "len {len} stride {stride} offset {offset}"
                    );
                    assert_eq!(
                        fold_sum_u64_idx(&values, stride, offset, &indices),
                        naive_sum_u
                    );
                    if !indices.is_empty() {
                        let naive_min = indices.iter().map(|&i| at(i)).min().expect("nonempty");
                        let naive_max = indices.iter().map(|&i| at(i)).max().expect("nonempty");
                        assert_eq!(
                            fold_min_max_u64_idx(&values, stride, offset, &indices),
                            (naive_min, naive_max)
                        );
                    }

                    // Contiguous forms over the dense prefix.
                    let naive_dense_i: i128 = (0..len)
                        .map(|i| i128::from(super::biased_to_i64(values[i * stride + offset])))
                        .sum();
                    let naive_dense_u: u128 = (0..len)
                        .map(|i| u128::from(values[i * stride + offset]))
                        .sum();
                    assert_eq!(
                        fold_sum_biased_i64(&values, stride, offset, len),
                        naive_dense_i
                    );
                    assert_eq!(fold_sum_u64(&values, stride, offset, len), naive_dense_u);
                    if len > 0 {
                        let dmin = (0..len)
                            .map(|i| values[i * stride + offset])
                            .min()
                            .expect("nonempty");
                        let dmax = (0..len)
                            .map(|i| values[i * stride + offset])
                            .max()
                            .expect("nonempty");
                        assert_eq!(fold_min_max_u64(&values, stride, offset, len), (dmin, dmax));
                    }
                }
            }
        }
    }

    /// PRD 03's throughput evidence (ignored: a timing test runs only by
    /// hand — `cargo test -p bumbledb --release fold_throughput -- --ignored --nocapture`).
    /// The gate: ≥ 1 row/ns on a contiguous stride-1 biased-i64 sum.
    #[test]
    #[ignore = "timing evidence, run by hand on the reference host"]
    fn fold_throughput_contiguous_sum() {
        let values: Vec<u64> = (0..1_000_000u64).map(|i| i ^ (1 << 63)).collect();
        // Warm.
        let mut sink = 0i128;
        for _ in 0..3 {
            sink += fold_sum_biased_i64(&values, 1, 0, values.len());
        }
        let start = std::time::Instant::now();
        let reps = 100;
        for _ in 0..reps {
            sink += fold_sum_biased_i64(&values, 1, 0, values.len());
        }
        let elapsed = start.elapsed();
        #[allow(clippy::cast_precision_loss)] // both values are far below 2^52
        let rate = (values.len() as u64 * reps) as f64
            / u64::try_from(elapsed.as_nanos().max(1)).expect("short run") as f64;
        println!("fold_sum_biased_i64 dense: {rate:.2} rows/ns (sink {sink})");
        assert!(rate >= 1.0, "≥1 row/ns on the reference host");
    }

    #[test]
    fn compaction_keeps_exactly_the_masked_items_in_order() {
        // Empty and full survivor sets, plus a mixed mask.
        let mut items: Vec<u32> = (0..10).collect();
        compact_u32_by_mask(&mut items, &[0; 10]);
        assert!(items.is_empty());

        let mut items: Vec<u32> = (0..10).collect();
        compact_u32_by_mask(&mut items, &[1; 10]);
        assert_eq!(items, (0..10).collect::<Vec<u32>>());

        let mut items: Vec<u32> = (0..10).collect();
        let mask = [1u8, 0, 1, 0, 0, 1, 1, 0, 0, 1];
        compact_u32_by_mask(&mut items, &mask);
        assert_eq!(items, vec![0, 2, 5, 6, 9]);
    }
}
