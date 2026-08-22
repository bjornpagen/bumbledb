//! The predicate-scan kernels, one `std::simd` body per shape on every
//! are Miri-interpretable). The 256-bit width (4 × u64, two vectors per
//! chunk) amortizes the mask consumption: one `to_bitmask` vector→GPR
//! transfer per chunk, then GPR shifts — never a per-lane extract or a
//! flag-class increment (`m2max.core.flag-port-asymmetry`: flag µops
//! confine to 3 of 6 integer ALUs) — and the survivor writes go through
//! the reference host, delete the intrinsic dual and its `unsafe`, and
//! one hoisted capacity invariant instead of a per-lane bounds check
use std::simd::SimdElement;
use std::simd::prelude::*;

/// The u64 kernel width: two 128-bit vectors, four lanes per chunk.
const U64_LANES: usize = 4;

/// The u8 kernel width: one 128-bit vector, sixteen lanes.
const U8_LANES: usize = 16;

/// Positions in `col` equal to `value`, appended to `out` in ascending
/// order (branchless survivor writes).
pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
    let needle = Simd::splat(value);
    push_matching::<u64, U64_LANES>(col, out, |lanes| lanes.simd_eq(needle), |x| x == value);
}

/// Positions in `col` within `lo..=hi` (u64 word order — order-preserving
/// for I64's biased words too), appended to `out` in ascending order.
pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    let lo_v = Simd::splat(lo);
    let hi_v = Simd::splat(hi);
    push_matching::<u64, U64_LANES>(
        col,
        out,
        |lanes| lanes.simd_ge(lo_v) & lanes.simd_le(hi_v),
        |x| (lo..=hi).contains(&x),
    );
}

/// Positions in `col` equal to `value` (the bool byte-column arm,
/// 16 lanes), appended to `out` in ascending order.
pub fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
    let needle = Simd::splat(value);
    push_matching::<u8, U8_LANES>(col, out, |lanes| lanes.simd_eq(needle), |x| x == value);
}

/// Point membership over an interval column pair: positions where
/// `starts[i] <= point AND point < ends[i]` (the half-open rule), in
/// ascending order. The composition is the existing predicate-scan
/// .
pub fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    let p = Simd::splat(point);
    push_matching_pair(
        starts,
        ends,
        out,
        |s, e| s.simd_le(p) & e.simd_gt(p),
        |s, e| s <= point && point < e,
    );
}

/// Point-*set* membership over an interval column pair: positions where
/// ANY element of `points` lies in `[starts[i], ends[i])` — the OR over
/// per-point masks. An empty set keeps
/// nothing.
pub fn filter_any_point_in_u64(starts: &[u64], ends: &[u64], points: &[u64], out: &mut Vec<u32>) {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    push_matching_pair(
        starts,
        ends,
        out,
        |s, e| {
            let mut any = Mask::splat(false);
            for &point in points {
                let p = Simd::splat(point);
                any |= s.simd_le(p) & e.simd_gt(p);
            }
            any
        },
        |s, e| points.iter().any(|p| s <= *p && *p < e),
    );
}

#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
fn push_matching<T, const N: usize>(
    col: &[T],
    out: &mut Vec<u32>,
    keep: impl Fn(Simd<T, N>) -> Mask<T::Mask, N>,
    keep1: impl Fn(T) -> bool,
) where
    T: SimdElement,
{
    let start = out.len();
    out.reserve(col.len());
    let mut write = start;
    let mut pos = positions_fit_u32(col.len());
    let (chunks, tail) = col.as_chunks::<N>();
    for chunk in chunks {
        let bits = keep(Simd::from_array(*chunk)).to_bitmask();
        (write, pos) = write_survivor_bits::<N>(out, write, pos, bits);
    }
    for &item in tail {
        // SAFETY: the reserve above owns one slot per visited position

        unsafe { out.as_mut_ptr().add(write).write(pos) };
        write += usize::from(keep1(item));
        pos = pos.wrapping_add(1);
    }
    // SAFETY: every slot in `[start, write)` was cursor-written above

    // drop obligation).
    unsafe { out.set_len(write) };
    crate::obs::event(
        crate::obs::names::KERNEL_FILTER,
        crate::obs::TraceArgs::Pair(col.len() as u64, (write - start) as u64),
    );
}

#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
fn push_matching_pair(
    starts: &[u64],
    ends: &[u64],
    out: &mut Vec<u32>,
    keep: impl Fn(Simd<u64, U64_LANES>, Simd<u64, U64_LANES>) -> Mask<i64, U64_LANES>,
    keep1: impl Fn(u64, u64) -> bool,
) {
    let start = out.len();
    out.reserve(starts.len());
    let mut write = start;
    let mut pos = positions_fit_u32(starts.len());
    let (chunks, tail) = starts.as_chunks::<U64_LANES>();
    let tail_start = starts.len() - tail.len();
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let base = chunk_idx * U64_LANES;
        let s = Simd::from_array(*chunk);
        let e = Simd::<u64, U64_LANES>::from_slice(&ends[base..base + U64_LANES]);
        let bits = keep(s, e).to_bitmask();
        (write, pos) = write_survivor_bits::<U64_LANES>(out, write, pos, bits);
    }
    for i in tail_start..starts.len() {
        // SAFETY: the reserve above owns one slot per visited position

        unsafe { out.as_mut_ptr().add(write).write(pos) };
        write += usize::from(keep1(starts[i], ends[i]));
        pos = pos.wrapping_add(1);
    }
    // SAFETY: every slot in `[start, write)` was cursor-written above

    // drop obligation).
    unsafe { out.set_len(write) };
    crate::obs::event(
        crate::obs::names::KERNEL_FILTER,
        crate::obs::TraceArgs::Pair(starts.len() as u64, (write - start) as u64),
    );
}

/// The one hoisted position guard (the per-lane `u32::try_from` was a per-item
/// branch): a column of `len` rows writes positions `0..len`, so `len − 1` must
/// fit u32 — the same programmer invariant the per-lane guard asserted, checked
/// once.
pub(super) fn positions_fit_u32(len: usize) -> u32 {
    let _ = u32::try_from(len.saturating_sub(1)).expect("positions fit u32");
    0
}

/// The callers owe the capacity invariant asserted below: `out` has one
/// reserved slot per visited position past the initialized prefix and the
/// cursor advances at most once per position, so on entry `write + N <=
/// out.capacity` whenever a full chunk remains — every lane's store lands in
/// owned capacity, and the caller's final `set_len(write)` exposes only
/// cursor-written slots.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
fn write_survivor_bits<const N: usize>(
    out: &mut Vec<u32>,
    mut write: usize,
    mut pos: u32,
    bits: u64,
) -> (usize, u32) {
    debug_assert!(
        write + N <= out.capacity(),
        "the callers' reserve invariant"
    );
    let ptr = out.as_mut_ptr();
    for lane in 0..N {
        // SAFETY: `write + N <= out.capacity()` on entry (asserted

        unsafe {
            ptr.add(write).write(pos);
        }
        write += usize::from((bits >> lane) & 1 != 0);
        pos = pos.wrapping_add(1);
    }
    (write, pos)
}

/// Same reserve invariant, same hoisted position guard.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub(super) fn write_survivor_keeps(
    out: &mut Vec<u32>,
    mut write: usize,
    mut pos: u32,
    keep: &[u8],
) -> (usize, u32) {
    debug_assert!(
        write + keep.len() <= out.capacity(),
        "the callers' reserve invariant"
    );
    debug_assert!(
        keep.iter().all(|&k| k <= 1),
        "keep bytes are 0/1 by contract"
    );
    let ptr = out.as_mut_ptr();
    for &k in keep {
        // SAFETY: `write + keep.len() <= out.capacity()` on entry

        unsafe {
            ptr.add(write).write(pos);
        }
        write += usize::from(k & 1);
        pos = pos.wrapping_add(1);
    }
    (write, pos)
}
