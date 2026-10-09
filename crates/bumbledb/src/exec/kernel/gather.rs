//! Folds over strided words picked by a survivor index list. Every lane's
//! address is bounds-checked: out-of-range lanes load zero but set a sticky
//! flag that panics after the loop, so a wrong word never reaches a total.
use std::simd::prelude::*;

const IDX_LANES: usize = 4;

/// Loads `values[index * stride + offset]` for four indices. A lane whose
/// address overflows or falls outside `values` loads zero and sets `*bad`.
#[inline]
fn gather_words(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32; IDX_LANES],
    bad: &mut bool,
) -> Simd<u64, IDX_LANES> {
    Simd::from_array(indices.map(|index| {
        let address = (index as usize)
            .saturating_mul(stride)
            .saturating_add(offset);
        *bad |= address >= values.len();
        // Selecting the reference before the load keeps the lane branch-free.
        *values.get(address).unwrap_or(&0)
    }))
}

#[track_caller]
fn assert_in_bounds(bad: bool) {
    assert!(!bad, "gathered index out of bounds");
}

/// Exact sum of `values[i * stride + offset]` over `indices`.
///
/// # Panics
/// If any index addresses past `values`.
#[must_use]
pub fn fold_sum_u64_idx(values: &[u64], stride: usize, offset: usize, indices: &[u32]) -> u128 {
    let mut bad = false;
    let mut lows = Simd::<u64, IDX_LANES>::splat(0);
    let mut carries = Simd::<u64, IDX_LANES>::splat(0);
    let (chunks, tail) = indices.as_chunks::<IDX_LANES>();
    for chunk in chunks {
        let v = gather_words(values, stride, offset, chunk, &mut bad);
        let new = lows + v;
        carries -= lows.simd_gt(new).to_simd().cast::<u64>();
        lows = new;
    }
    assert_in_bounds(bad);
    let mut total: u128 = 0;
    for lane in 0..IDX_LANES {
        total += u128::from(lows.as_array()[lane]) + (u128::from(carries.as_array()[lane]) << 64);
    }
    for &i in tail {
        total += u128::from(values[i as usize * stride + offset]);
    }
    total
}

/// Word-order `(min, max)` of `values[i * stride + offset]` over `indices`.
/// Biased I64 and F64 order keys are order-preserving, so one kernel serves
/// every scalar type.
///
/// # Panics
/// If `indices` is empty or any index addresses past `values`.
#[must_use]
pub fn fold_min_max_u64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    assert!(!indices.is_empty(), "non-empty batch");
    let mut bad = false;
    let mut mins = Simd::<u64, IDX_LANES>::splat(u64::MAX);
    let mut maxs = Simd::<u64, IDX_LANES>::splat(u64::MIN);
    let (chunks, tail) = indices.as_chunks::<IDX_LANES>();
    for chunk in chunks {
        let v = gather_words(values, stride, offset, chunk, &mut bad);
        mins = mins.simd_min(v);
        maxs = maxs.simd_max(v);
    }
    assert_in_bounds(bad);
    let mut min_scalar = mins.reduce_min();
    let mut max_scalar = maxs.reduce_max();
    for &i in tail {
        let word = values[i as usize * stride + offset];
        min_scalar = min_scalar.min(word);
        max_scalar = max_scalar.max(word);
    }
    (min_scalar, max_scalar)
}
