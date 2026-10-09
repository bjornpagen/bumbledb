//! Folds over strided words picked by a survivor index list. Every lane's
//! address is bounds-checked: out-of-range lanes load zero but set a sticky
//! flag that panics after the loop, so a wrong word never reaches a total.
#![expect(
    clippy::inline_always,
    reason = "SIMD bodies inline into the dispatched target-feature context"
)]

use fearless_simd::{Level, Simd, SimdBase, SimdMask, dispatch, i64x4, u64x4};

/// Exact sum of `values[i * stride + offset]` over `indices`.
///
/// # Panics
/// If any index addresses past `values`.
#[must_use]
pub(crate) fn fold_sum_u64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> u128 {
    sum_u64_idx(super::level(), values, stride, offset, indices)
}

/// Word-order `(min, max)` of `values[i * stride + offset]` over `indices`.
/// Biased I64 and F64 order keys are order-preserving, so one kernel serves
/// every scalar type.
///
/// # Panics
/// If `indices` is empty or any index addresses past `values`.
#[must_use]
pub(crate) fn fold_min_max_u64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    min_max_u64_idx(super::level(), values, stride, offset, indices)
}

pub(super) fn sum_u64_idx(
    level: Level,
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> u128 {
    dispatch!(level, simd => sum_idx(simd, values, stride, offset, indices))
}

pub(super) fn min_max_u64_idx(
    level: Level,
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    assert!(!indices.is_empty(), "non-empty batch");
    dispatch!(level, simd => min_max_idx(simd, values, stride, offset, indices))
}

/// Loads `values[index * stride + offset]` for four indices. A lane whose
/// address overflows or falls outside `values` loads zero and sets `*bad`.
#[inline(always)]
fn gather_words<S: Simd>(
    simd: S,
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32; 4],
    bad: &mut bool,
) -> u64x4<S> {
    u64x4::load_array(
        simd,
        indices.map(|index| {
            let address = (index as usize)
                .saturating_mul(stride)
                .saturating_add(offset);
            *bad |= address >= values.len();
            // Selecting the reference before the load keeps the lane branch-free.
            *values.get(address).unwrap_or(&0)
        }),
    )
}

#[track_caller]
fn assert_in_bounds(bad: bool) {
    assert!(!bad, "gathered index out of bounds");
}

#[inline(always)]
fn sum_idx<S: Simd>(
    simd: S,
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> u128 {
    let mut bad = false;
    let mut lows = u64x4::splat(simd, 0);
    let mut carries = i64x4::splat(simd, 0);
    let (chunks, tail) = indices.as_chunks::<4>();
    for chunk in chunks {
        let v = gather_words(simd, values, stride, offset, chunk, &mut bad);
        let new = lows + v;
        carries -= lows.simd_gt(new).to_vector();
        lows = new;
    }
    assert_in_bounds(bad);
    let mut total: u128 = 0;
    for lane in 0..4 {
        total += u128::from(lows[lane]) + (u128::from(carries[lane].cast_unsigned()) << 64);
    }
    for &i in tail {
        total += u128::from(values[i as usize * stride + offset]);
    }
    total
}

#[inline(always)]
fn min_max_idx<S: Simd>(
    simd: S,
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    let mut bad = false;
    let mut mins = u64x4::splat(simd, u64::MAX);
    let mut maxs = u64x4::splat(simd, u64::MIN);
    let (chunks, tail) = indices.as_chunks::<4>();
    for chunk in chunks {
        let v = gather_words(simd, values, stride, offset, chunk, &mut bad);
        mins = mins.min(v);
        maxs = maxs.max(v);
    }
    assert_in_bounds(bad);
    let mut min = mins.reduce_min();
    let mut max = maxs.reduce_max();
    for &i in tail {
        let word = values[i as usize * stride + offset];
        min = min.min(word);
        max = max.max(word);
    }
    (min, max)
}
