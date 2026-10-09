//! Folds over `count` strided words `values[offset + i * stride]`. Stride 1
//! runs fixed-width lanes; other strides run four scalar accumulators.
#![expect(
    clippy::inline_always,
    reason = "SIMD bodies inline into the dispatched target-feature context"
)]

use fearless_simd::{Level, Simd, SimdBase, SimdMask, dispatch, i64x4, u64x4};

/// `(count - 1) * stride + offset < len`, computed without wrapping so the
/// guard is total over its inputs.
fn strided_extent_in(len: usize, stride: usize, offset: usize, count: usize) -> bool {
    stride > 0
        && count.checked_sub(1).is_none_or(|c| {
            c.checked_mul(stride)
                .and_then(|span| span.checked_add(offset))
                .is_some_and(|last| last < len)
        })
}

/// Exact sum of `count` strided words. Signed callers sum the biased words
/// and remove the bias once.
///
/// # Panics
/// If `stride` is zero or the extent runs past `values`.
#[must_use]
pub fn fold_sum_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> u128 {
    sum_u64(super::level(), values, stride, offset, count)
}

/// Word-order `(min, max)` of `count` strided words.
///
/// # Panics
/// If `count` or `stride` is zero or the extent runs past `values`.
#[must_use]
pub fn fold_min_max_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> (u64, u64) {
    min_max_u64(super::level(), values, stride, offset, count)
}

pub(super) fn sum_u64(
    level: Level,
    values: &[u64],
    stride: usize,
    offset: usize,
    count: usize,
) -> u128 {
    assert!(strided_extent_in(values.len(), stride, offset, count));
    if stride == 1 {
        let dense = &values[offset..offset + count];
        return dispatch!(level, simd => sum_dense(simd, dense));
    }
    let mut acc = [0u128; 4];
    let mut i = 0;
    while i + 4 <= count {
        for (lane, slot) in acc.iter_mut().enumerate() {
            *slot += u128::from(strided_word(values, stride, offset, i + lane));
        }
        i += 4;
    }
    while i < count {
        acc[0] += u128::from(strided_word(values, stride, offset, i));
        i += 1;
    }
    acc[0] + acc[1] + acc[2] + acc[3]
}

pub(super) fn min_max_u64(
    level: Level,
    values: &[u64],
    stride: usize,
    offset: usize,
    count: usize,
) -> (u64, u64) {
    assert!(count > 0 && strided_extent_in(values.len(), stride, offset, count));
    if stride == 1 {
        let dense = &values[offset..offset + count];
        return dispatch!(level, simd => min_max_dense(simd, dense));
    }
    let mut mins = [u64::MAX; 4];
    let mut maxs = [u64::MIN; 4];
    let mut i = 0;
    while i + 4 <= count {
        for lane in 0..4 {
            let word = strided_word(values, stride, offset, i + lane);
            mins[lane] = mins[lane].min(word);
            maxs[lane] = maxs[lane].max(word);
        }
        i += 4;
    }
    while i < count {
        let word = strided_word(values, stride, offset, i);
        mins[0] = mins[0].min(word);
        maxs[0] = maxs[0].max(word);
        i += 1;
    }
    (
        mins.into_iter().fold(u64::MAX, u64::min),
        maxs.into_iter().fold(u64::MIN, u64::max),
    )
}

#[inline]
#[expect(
    unsafe_code,
    reason = "strided loads under the callers' release-strength extent assert"
)]
fn strided_word(values: &[u64], stride: usize, offset: usize, i: usize) -> u64 {
    debug_assert!(i * stride + offset < values.len());
    // SAFETY: every caller asserted `strided_extent_in(values.len(), stride,
    // offset, count)` and passes `i < count`, so `i * stride + offset <=
    // (count - 1) * stride + offset < values.len()` without overflow.
    unsafe { *values.get_unchecked(i * stride + offset) }
}

/// Lane sums with per-lane carry counts: a wrapped lane add (`new < old`)
/// adds one to that lane's carry (subtracting the all-ones mask lane), which
/// keeps the u128 total exact.
#[inline(always)]
fn sum_dense<S: Simd>(simd: S, values: &[u64]) -> u128 {
    let mut lows = [u64x4::splat(simd, 0); 2];
    let mut carries = [i64x4::splat(simd, 0); 2];
    let (chunks, tail) = values.as_chunks::<8>();
    for chunk in chunks {
        for (half, (low, carry)) in lows.iter_mut().zip(&mut carries).enumerate() {
            let v = u64x4::from_slice(simd, &chunk[half * 4..half * 4 + 4]);
            let new = *low + v;
            *carry -= low.simd_gt(new).to_vector();
            *low = new;
        }
    }
    let mut total: u128 = 0;
    for (low, carry) in lows.iter().zip(&carries) {
        for lane in 0..4 {
            total += u128::from(low[lane]) + (u128::from(carry[lane].cast_unsigned()) << 64);
        }
    }
    for &v in tail {
        total += u128::from(v);
    }
    total
}

#[inline(always)]
fn min_max_dense<S: Simd>(simd: S, values: &[u64]) -> (u64, u64) {
    let mut mins = [u64x4::splat(simd, u64::MAX); 2];
    let mut maxs = [u64x4::splat(simd, u64::MIN); 2];
    let (chunks, tail) = values.as_chunks::<8>();
    for chunk in chunks {
        for half in 0..2 {
            let v = u64x4::from_slice(simd, &chunk[half * 4..half * 4 + 4]);
            mins[half] = mins[half].min(v);
            maxs[half] = maxs[half].max(v);
        }
    }
    let mut min = mins[0].min(mins[1]).reduce_min();
    let mut max = maxs[0].max(maxs[1]).reduce_max();
    for &v in tail {
        min = min.min(v);
        max = max.max(v);
    }
    (min, max)
}
