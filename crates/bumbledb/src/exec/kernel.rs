//! The only explicit SIMD in the system (PRD 22): fixed-width predicate
//! scans and survivor compaction, NEON 128-bit, behind scalar-identical
//! signatures (`docs/architecture/30-execution.md` D4, `00-product.md`).
//!
//! This is the one module where `unsafe` is sanctioned. The NEON paths are
//! `cfg(target_arch = "aarch64")`; every other 64-bit platform compiles and
//! runs the scalar fallback correctly, with no performance promises. The
//! scalar implementations are compiled wherever they have a reader — the
//! non-aarch64 live path and every target's test builds, where they are
//! the reference the property tests compare the kernels against, bit for
//! bit (an aarch64 release build omits them: dead code is disallowed).
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
#[allow(unsafe_code)] // PRD 22: the one sanctioned unsafe module
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
