//! The predicate-scan kernels, one `std::simd` body per shape on every
//! target (the crucible packet (git ecec1dc3), Q2 — ADOPT, measured:
//! the portable bodies beat the retired hand-NEON twins 1.03–1.5× on
//! the reference host, delete the intrinsic dual and its `unsafe`, and
//! are Miri-interpretable). The 128-bit width (2 × u64 / 16 × u8) is
//! the shape every 64-bit target has; the scalar tail twin per kernel
//! covers the remainder lanes. The scalar reference twins
//! ([`super::reference`]) remain the independent differential oracle.

use std::simd::prelude::*;
use std::simd::{MaskElement, SimdElement};

/// The u64 kernel width: one 128-bit vector, two lanes.
const U64_LANES: usize = 2;

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
/// shape applied to two columns with an AND — no new kernel shape
/// (docs/architecture/40-execution.md, § access paths).
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
/// per-point masks (k small by the documented set assumption,
/// `docs/architecture/20-query-ir.md` § param sets). An empty set keeps
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

/// The measure scan — the one gather+subtract shape
/// (docs/architecture/20-query-ir.md, § the measure): positions whose
/// duration `ends[i] − starts[i]` lies within `lo..=hi`, appended
/// in ascending order. The subtraction feeds the existing range shape —
/// one fused stride-1 pass, lane-parallel on the dense case per the
/// port-topology law (subtraction is not flag-bound); strided/gathered
/// callers stay scalar until measured, per the standing rule.
/// Encoded-word subtraction is exact for both element types: the
/// encodings are unit-spaced order-preserving maps onto u64 words (u64
/// the identity, I64 the +2⁶³ bias, which cancels), and the constructor
/// invariant `end > start` keeps the difference positive. The ray test
/// is fused as one more lane compare; on a hit the mask's first set
/// lane names the first offending position in scan order.
///
/// # Errors
///
/// The first ray in scan order (`ends[i] == u64::MAX` — ∞ in both
/// element encodings): a ray has no finite measure, and the caller
/// raises the typed [`crate::Error::MeasureOfRay`]. `out`'s contents are
/// unspecified after an error.
pub fn filter_duration_range_u64(
    starts: &[u64],
    ends: &[u64],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) -> Result<(), usize> {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    let start = out.len();
    out.resize(start + starts.len(), 0);
    let mut write = start;
    let lo_v = Simd::<u64, U64_LANES>::splat(lo);
    let hi_v = Simd::<u64, U64_LANES>::splat(hi);
    let inf = Simd::<u64, U64_LANES>::splat(u64::MAX);
    let (chunks, tail) = starts.as_chunks::<U64_LANES>();
    let tail_start = starts.len() - tail.len();
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let base = chunk_idx * U64_LANES;
        let s = Simd::from_array(*chunk);
        let e = Simd::<u64, U64_LANES>::from_slice(&ends[base..base + U64_LANES]);
        let ray = e.simd_eq(inf);
        if ray.any() {
            let lane = usize::try_from(ray.to_bitmask().trailing_zeros()).expect("lane index");
            return Err(base + lane);
        }
        let duration = e - s;
        let mask = duration.simd_ge(lo_v) & duration.simd_le(hi_v);
        write = write_survivors(out, write, base, mask);
    }
    for i in tail_start..starts.len() {
        if ends[i] == u64::MAX {
            return Err(i);
        }
        let duration = ends[i] - starts[i];
        out[write] = u32::try_from(i).expect("positions fit u32");
        write += usize::from((lo..=hi).contains(&duration));
    }
    out.truncate(write);
    Ok(())
}

/// Branchless cursor-write over the whole column: lane chunks through
/// the `keep` mask, the remainder through its scalar twin `keep1`.
fn push_matching<T, const N: usize>(
    col: &[T],
    out: &mut Vec<u32>,
    keep: impl Fn(Simd<T, N>) -> Mask<T::Mask, N>,
    keep1: impl Fn(T) -> bool,
) where
    T: SimdElement,
{
    let start = out.len();
    out.resize(start + col.len(), 0);
    let mut write = start;
    let (chunks, tail) = col.as_chunks::<N>();
    let tail_start = col.len() - tail.len();
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let mask = keep(Simd::from_array(*chunk));
        write = write_survivors(out, write, chunk_idx * N, mask);
    }
    for (i, &item) in tail.iter().enumerate() {
        out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
        write += usize::from(keep1(item));
    }
    out.truncate(write);
}

/// [`push_matching`] over an interval span's (starts, ends) column pair.
fn push_matching_pair(
    starts: &[u64],
    ends: &[u64],
    out: &mut Vec<u32>,
    keep: impl Fn(Simd<u64, U64_LANES>, Simd<u64, U64_LANES>) -> Mask<i64, U64_LANES>,
    keep1: impl Fn(u64, u64) -> bool,
) {
    let start = out.len();
    out.resize(start + starts.len(), 0);
    let mut write = start;
    let (chunks, tail) = starts.as_chunks::<U64_LANES>();
    let tail_start = starts.len() - tail.len();
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let base = chunk_idx * U64_LANES;
        let s = Simd::from_array(*chunk);
        let e = Simd::<u64, U64_LANES>::from_slice(&ends[base..base + U64_LANES]);
        write = write_survivors(out, write, base, keep(s, e));
    }
    for i in tail_start..starts.len() {
        out[write] = u32::try_from(i).expect("positions fit u32");
        write += usize::from(keep1(starts[i], ends[i]));
    }
    out.truncate(write);
}

/// The per-lane branchless survivor writes: position `base + lane`
/// lands at the cursor, which advances by the lane's mask bit.
fn write_survivors<M, const N: usize>(
    out: &mut [u32],
    mut write: usize,
    base: usize,
    mask: Mask<M, N>,
) -> usize
where
    M: MaskElement,
{
    for lane in 0..N {
        out[write] = u32::try_from(base + lane).expect("positions fit u32");
        write += usize::from(mask.test(lane));
    }
    write
}

// The old interval-vs-constant comparison kernels (overlaps, contains,
// within-over-pairs) are gone with their operators: interval-pair
// predicates are Allen masks, evaluated by the configuration kernel
// (`super::allen` — one branchless, flag-free kernel for all 8192
// masks).
