//! Predicate scans: each chunk compares fixed-width lanes into one bitmask,
//! then a branchless cursor writes every position and advances only past
//! survivors. Positions land in `out` in ascending order.
#![expect(
    clippy::inline_always,
    reason = "SIMD bodies inline into the dispatched target-feature context"
)]

use std::mem::MaybeUninit;

use fearless_simd::{Level, Simd, SimdBase, SimdMask, dispatch, u8x16, u64x4};

/// Positions in `col` equal to `value`.
pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
    eq_u64(super::level(), col, value, out);
}

/// Positions in `col` within `lo..=hi` in word order (order-preserving for
/// biased I64 and F64 order keys).
pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    range_u64(super::level(), col, lo, hi, out);
}

/// Positions in the byte column `col` equal to `value`.
pub fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
    eq_u8(super::level(), col, value, out);
}

/// Positions where `starts[i] <= point < ends[i]`.
pub fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
    point_in_u64(super::level(), starts, ends, point, out);
}

/// Positions where some element of `points` lies in `[starts[i], ends[i])`;
/// an empty set keeps nothing.
pub fn filter_any_point_in_u64(starts: &[u64], ends: &[u64], points: &[u64], out: &mut Vec<u32>) {
    any_point_in_u64(super::level(), starts, ends, points, out);
}

pub(super) fn eq_u64(level: Level, col: &[u64], value: u64, out: &mut Vec<u32>) {
    dispatch!(level, simd => push_matching_u64(
        simd,
        col,
        out,
        #[inline(always)]
        |lanes| lanes.simd_eq(value),
        #[inline(always)]
        |word| word == value,
    ));
}

pub(super) fn range_u64(level: Level, col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    dispatch!(level, simd => push_matching_u64(
        simd,
        col,
        out,
        #[inline(always)]
        |lanes| lanes.simd_ge(lo) & lanes.simd_le(hi),
        #[inline(always)]
        |word| (lo..=hi).contains(&word),
    ));
}

pub(super) fn eq_u8(level: Level, col: &[u8], value: u8, out: &mut Vec<u32>) {
    dispatch!(level, simd => push_matching_u8(simd, col, value, out));
}

pub(super) fn point_in_u64(
    level: Level,
    starts: &[u64],
    ends: &[u64],
    point: u64,
    out: &mut Vec<u32>,
) {
    dispatch!(level, simd => push_matching_pair(
        simd,
        starts,
        ends,
        out,
        #[inline(always)]
        |s, e| s.simd_le(point) & e.simd_gt(point),
        #[inline(always)]
        |s, e| s <= point && point < e,
    ));
}

pub(super) fn any_point_in_u64(
    level: Level,
    starts: &[u64],
    ends: &[u64],
    points: &[u64],
    out: &mut Vec<u32>,
) {
    dispatch!(level, simd => push_matching_pair(
        simd,
        starts,
        ends,
        out,
        #[inline(always)]
        |s, e| {
            let mut any = SimdMask::splat(simd, false);
            for &point in points {
                any |= s.simd_le(point) & e.simd_gt(point);
            }
            any
        },
        #[inline(always)]
        |s, e| points.iter().any(|&p| s <= p && p < e),
    ));
}

#[inline(always)]
fn push_matching_u64<S: Simd>(
    simd: S,
    col: &[u64],
    out: &mut Vec<u32>,
    keep: impl Fn(u64x4<S>) -> <u64x4<S> as SimdBase<S>>::Mask,
    keep1: impl Fn(u64) -> bool,
) {
    let mut cursor = Cursor::open(out, col.len());
    let (chunks, tail) = col.as_chunks::<4>();
    for chunk in chunks {
        cursor.push_bits::<4>(keep(u64x4::load_array(simd, *chunk)).to_bitmask());
    }
    for &word in tail {
        cursor.push(keep1(word));
    }
    cursor.close();
}

#[inline(always)]
fn push_matching_u8<S: Simd>(simd: S, col: &[u8], value: u8, out: &mut Vec<u32>) {
    let mut cursor = Cursor::open(out, col.len());
    let (chunks, tail) = col.as_chunks::<16>();
    for chunk in chunks {
        cursor.push_bits::<16>(u8x16::load_array(simd, *chunk).simd_eq(value).to_bitmask());
    }
    for &byte in tail {
        cursor.push(byte == value);
    }
    cursor.close();
}

#[inline(always)]
fn push_matching_pair<S: Simd>(
    simd: S,
    starts: &[u64],
    ends: &[u64],
    out: &mut Vec<u32>,
    keep: impl Fn(u64x4<S>, u64x4<S>) -> <u64x4<S> as SimdBase<S>>::Mask,
    keep1: impl Fn(u64, u64) -> bool,
) {
    assert_eq!(starts.len(), ends.len(), "an interval column pair");
    let mut cursor = Cursor::open(out, starts.len());
    let (start_chunks, start_tail) = starts.as_chunks::<4>();
    let (end_chunks, end_tail) = ends.as_chunks::<4>();
    for (s, e) in start_chunks.iter().zip(end_chunks) {
        let bits = keep(u64x4::load_array(simd, *s), u64x4::load_array(simd, *e)).to_bitmask();
        cursor.push_bits::<4>(bits);
    }
    for (&s, &e) in start_tail.iter().zip(end_tail) {
        cursor.push(keep1(s, e));
    }
    cursor.close();
}

/// The survivor cursor over `out`'s spare capacity: every visited position
/// is written at the cursor, which advances only past kept positions.
pub(super) struct Cursor<'a> {
    out: &'a mut Vec<u32>,
    write: usize,
    position: u32,
}

impl<'a> Cursor<'a> {
    /// Reserves one slot per position of a `len`-row column.
    ///
    /// # Panics
    /// If the last position `len - 1` does not fit `u32`.
    pub(super) fn open(out: &'a mut Vec<u32>, len: usize) -> Self {
        let _ = u32::try_from(len.saturating_sub(1)).expect("positions fit u32");
        out.reserve(len);
        Self {
            out,
            write: 0,
            position: 0,
        }
    }

    #[inline]
    pub(super) fn push(&mut self, keep: bool) {
        self.out.spare_capacity_mut()[self.write].write(self.position);
        self.write += usize::from(keep);
        self.position = self.position.wrapping_add(1);
    }

    /// Pushes `N` positions whose keep flags are bits `0..N` of `bits`.
    #[inline]
    pub(super) fn push_bits<const N: usize>(&mut self, bits: u64) {
        const { assert!(N.is_power_of_two()) };
        let window: &mut [MaybeUninit<u32>; N] = (&mut self.out.spare_capacity_mut()
            [self.write..self.write + N])
            .try_into()
            .expect("one window per chunk");
        let mut local = 0;
        for lane in 0..N {
            // `local <= lane < N`, so the mask never changes the index; it
            // only lets the compiler drop the per-lane bounds check.
            window[local & (N - 1)].write(self.position);
            local += usize::from((bits >> lane) & 1 != 0);
            self.position = self.position.wrapping_add(1);
        }
        self.write += local;
    }

    /// Pushes one position per keep byte; a byte keeps its position iff it
    /// is nonzero.
    pub(super) fn push_keeps(&mut self, keep: &[u8]) {
        for &k in keep {
            self.push(k != 0);
        }
    }

    /// Publishes the kept positions.
    #[expect(unsafe_code, reason = "publishes the cursor-written prefix")]
    pub(super) fn close(self) {
        let len = self.out.len() + self.write;
        // SAFETY: every slot in `[len - write, len)` was written through
        // `spare_capacity_mut` (which starts at the current length) before
        // the cursor moved past it; `u32` has no drop obligation.
        unsafe { self.out.set_len(len) };
    }
}
