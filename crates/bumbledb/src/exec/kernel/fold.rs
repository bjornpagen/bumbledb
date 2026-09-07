use std::simd::prelude::*;

/// The strided extent guard the fold kernels' `get_unchecked` bodies cite:
/// `(count − 1) · stride + offset < len`, computed checked so the guard is
/// total over the input type — a wrapping product cannot forge an in-bounds
/// extent in release (overflow-checks default off there; the guard IS the
/// safety invariant, so it must not wrap).
fn strided_extent_in(len: usize, stride: usize, offset: usize, count: usize) -> bool {
    stride > 0
        && count.checked_sub(1).is_none_or(|c| {
            c.checked_mul(stride)
                .and_then(|span| span.checked_add(offset))
                .is_some_and(|last| last < len)
        })
}

/// Sum `values[offset + i * stride]` for `count` elements without index
/// loads. Stride 1 uses the SIMD lane carry-count path. Signed sums use
/// these same raw totals and subtract the encoding bias once at the sink.
/// # Panics
/// Only on a programmer-invariant violation: zero stride or an extent
/// exceeding `values`.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub fn fold_sum_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> u128 {
    assert!(strided_extent_in(values.len(), stride, offset, count));
    if stride == 1 {
        return fold_sum_u64_dense(&values[offset..offset + count]);
    }
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

/// Contiguous strided (min, max) in one pass. Stride 1 takes the lane
/// path (`simd_min`/`simd_max` — on aarch64 the compare-select pair;
/// there is no 64-bit lane min/max instruction).
/// # Panics
/// extent exceeding `values`.
/// Only on a programmer-invariant violation: zero `count` or the strided
#[must_use]
pub fn fold_min_max_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> (u64, u64) {
    assert!(count > 0 && strided_extent_in(values.len(), stride, offset, count));
    if stride == 1 {
        return fold_min_max_u64_dense(&values[offset..offset + count]);
    }
    fold_min_max_u64_strided(values, stride, offset, count)
}

/// Count carries per SIMD lane to retain the exact u128 sum.
fn fold_sum_u64_dense(values: &[u64]) -> u128 {
    let mut lows = [Simd::<u64, 2>::splat(0); 4];
    let mut carries = [Simd::<u64, 2>::splat(0); 4];
    let (chunks, tail) = values.as_chunks::<8>();
    for chunk in chunks {
        for lane in 0..4 {
            let v = Simd::<u64, 2>::from_slice(&chunk[lane * 2..lane * 2 + 2]);
            let new = lows[lane] + v;

            let carry = lows[lane].simd_gt(new).to_simd().cast::<u64>();
            carries[lane] -= carry;
            lows[lane] = new;
        }
    }
    let mut total: u128 = 0;
    for lane in 0..4 {
        for half in 0..2 {
            total += u128::from(lows[lane].as_array()[half])
                + (u128::from(carries[lane].as_array()[half]) << 64);
        }
    }
    for &v in tail {
        total += u128::from(v);
    }
    total
}

fn fold_min_max_u64_dense(values: &[u64]) -> (u64, u64) {
    let mut mins = [Simd::<u64, 2>::splat(u64::MAX); 4];
    let mut maxs = [Simd::<u64, 2>::splat(u64::MIN); 4];
    let (chunks, tail) = values.as_chunks::<8>();
    for chunk in chunks {
        for lane in 0..4 {
            let v = Simd::<u64, 2>::from_slice(&chunk[lane * 2..lane * 2 + 2]);
            mins[lane] = mins[lane].simd_min(v);
            maxs[lane] = maxs[lane].simd_max(v);
        }
    }
    let mut min_scalar = u64::MAX;
    let mut max_scalar = u64::MIN;
    for lane in 0..4 {
        min_scalar = min_scalar.min(mins[lane].reduce_min());
        max_scalar = max_scalar.max(maxs[lane].reduce_max());
    }
    for &v in tail {
        min_scalar = min_scalar.min(v);
        max_scalar = max_scalar.max(v);
    }
    (min_scalar, max_scalar)
}

#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
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
