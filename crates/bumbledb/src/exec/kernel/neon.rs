use std::arch::aarch64::{
    uint64x2_t, vandq_u64, vceqq_u64, vceqq_u8, vcgeq_u64, vcgtq_u64, vcleq_u64, vdupq_n_u64,
    vdupq_n_u8, vgetq_lane_u64, vld1q_u64, vld1q_u8, vorrq_u64, vst1q_u8,
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

/// The shared two-column compare-and-mask pass (docs/architecture/
/// 40-execution.md, § access paths — interval predicates lower to word
/// comparisons over the start/end column pair; the fixed 8-byte shape,
/// no new NEON width): `mask` maps a (starts, ends) lane pair to the
/// combined survivor mask; the branchless writes are the predicate-scan
/// pattern verbatim, and `tail` is its scalar twin over the remainder.
#[allow(clippy::inline_always)]
// the fused pass exists to keep the mask
// closures from becoming outlined calls
// per two-lane chunk (the
// instruction-diet law)
#[inline(always)]
unsafe fn filter_pair_u64(
    starts: &[u64],
    ends: &[u64],
    out: &mut Vec<u32>,
    mask: impl Fn(uint64x2_t, uint64x2_t) -> uint64x2_t,
    tail: impl Fn(u64, u64) -> bool,
) {
    debug_assert_eq!(starts.len(), ends.len());
    let base = out.len();
    out.resize(base + starts.len(), 0);
    let mut write = base;
    // SAFETY (caller's contract): every vld1q_u64 reads exactly two u64s
    // from within `chunks_exact(2)` of equal-length columns; unaligned
    // loads are legal for vld1q.
    unsafe {
        let chunks = starts.chunks_exact(2);
        let tail_start = starts.len() - chunks.remainder().len();
        for (chunk_idx, chunk) in chunks.enumerate() {
            let s = vld1q_u64(chunk.as_ptr());
            let e = vld1q_u64(ends.as_ptr().add(chunk_idx * 2));
            let m = mask(s, e);
            let out_base = u32::try_from(chunk_idx * 2).expect("positions fit u32");
            out[write] = out_base;
            write += usize::from(vgetq_lane_u64(m, 0) != 0);
            out[write] = out_base + 1;
            write += usize::from(vgetq_lane_u64(m, 1) != 0);
        }
        for i in tail_start..starts.len() {
            out[write] = u32::try_from(i).expect("positions fit u32");
            write += usize::from(tail(starts[i], ends[i]));
        }
    }
    out.truncate(write);
}

pub(super) fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
    // SAFETY: equal-length columns, chunked loads in bounds — the
    // `filter_pair_u64` contract.
    unsafe {
        let p = vdupq_n_u64(point);
        filter_pair_u64(
            starts,
            ends,
            out,
            // `start <= p` AND `p < end` — the half-open membership rule
            // as two predicate-scan masks ANDed.
            |s, e| vandq_u64(vcleq_u64(s, p), vcgtq_u64(e, p)),
            |s, e| s <= point && point < e,
        );
    }
}

pub(super) fn filter_any_point_in_u64(
    starts: &[u64],
    ends: &[u64],
    points: &[u64],
    out: &mut Vec<u32>,
) {
    // SAFETY: as in `filter_point_in_u64`; the OR accumulates per-point
    // membership masks (k small — docs/architecture/20-query-ir.md,
    // § param sets).
    unsafe {
        filter_pair_u64(
            starts,
            ends,
            out,
            |s, e| {
                let mut any = vdupq_n_u64(0);
                for &point in points {
                    let p = vdupq_n_u64(point);
                    any = vorrq_u64(any, vandq_u64(vcleq_u64(s, p), vcgtq_u64(e, p)));
                }
                any
            },
            |s, e| points.iter().any(|p| s <= *p && *p < e),
        );
    }
}

// The old interval-vs-constant comparison kernels lived here; they left
// with their operators (interval-pair predicates are Allen masks now —
// the configuration kernel is PRD 04's).

/// Dense exact-u128 sum via carry counting: four
/// 2-lane accumulators take wrapping `vaddq_u64` adds while a
/// parallel counter lane counts carries — unsigned overflow iff
/// `new < old`, i.e. `vcgtq_u64(old, new)` all-ones, subtracted to
/// count +1. Total = Σ lane lo + (Σ lane carries << 64): exact, and
/// bit-identical to any-association i128/u128 folding. Flag ports
/// untouched — the scalar exact loop's `adds/adcs` are confined to
/// 3 of 6 ALUs, which was the whole wall (measured 8.8 vs 4.0–4.6
/// rows/ns at L1).
pub(super) fn fold_sum_u64_dense(values: &[u64]) -> u128 {
    // SAFETY: every vld1q_u64 reads exactly two u64s from within
    // `chunks_exact(8)` of `values`.
    unsafe {
        use std::arch::aarch64::{vaddq_u64, vcgtq_u64, vsubq_u64};
        let mut lows = [vdupq_n_u64(0); 4];
        let mut carries = [vdupq_n_u64(0); 4];
        let chunks = values.chunks_exact(8);
        let tail = chunks.remainder();
        for chunk in chunks {
            for lane in 0..4 {
                let v = vld1q_u64(chunk.as_ptr().add(lane * 2));
                let new = vaddq_u64(lows[lane], v);
                // Overflowed lanes read all-ones; subtracting adds 1.
                let carry = vcgtq_u64(lows[lane], new);
                carries[lane] = vsubq_u64(carries[lane], carry);
                lows[lane] = new;
            }
        }
        let mut total: u128 = 0;
        for lane in 0..4 {
            for half in 0..2 {
                let lo = if half == 0 {
                    vgetq_lane_u64(lows[lane], 0)
                } else {
                    vgetq_lane_u64(lows[lane], 1)
                };
                let carry = if half == 0 {
                    vgetq_lane_u64(carries[lane], 0)
                } else {
                    vgetq_lane_u64(carries[lane], 1)
                };
                total += u128::from(lo) + (u128::from(carry) << 64);
            }
        }
        for &v in tail {
            total += u128::from(v);
        }
        total
    }
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
