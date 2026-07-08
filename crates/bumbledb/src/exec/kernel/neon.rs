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

/// Dense exact-u128 sum via carry counting (docs/silicon/06): four
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
