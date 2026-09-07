use std::simd::prelude::*;

const IDX_LANES: usize = 4;

/// The invariant the callers owe (checked in debug builds): every strided index
/// lands inside `values`.
#[inline]
fn debug_assert_idx_bounds(values: &[u64], stride: usize, offset: usize, indices: &[u32]) {
    debug_assert!(stride > 0);
    debug_assert!(
        indices
            .iter()
            .all(|&i| i as usize * stride + offset < values.len()),
        "leaf-batch indices are in-bounds by construction"
    );
}

/// The same wrapping-address, zero-default gather as portable SIMD, built
/// from scalar loads. Keep defaults for invalid vector lanes; the folds'
/// debug preconditions and scalar-tail behavior are unchanged.
/// Select the default reference before loading: on Apple Silicon this lets
/// LLVM interleave checked loads without a branch around each value load.
#[inline]
pub(super) fn gather_words(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32; IDX_LANES],
) -> Simd<u64, IDX_LANES> {
    Simd::from_array(indices.map(|index| {
        let address = (index as usize).wrapping_mul(stride).wrapping_add(offset);
        *values.get(address).unwrap_or(&0)
    }))
}

/// Sum of u64 words at the indexed positions — exact u128 via carry
/// counting (see the dense fold's doctrine; same mechanism, gathered).
#[must_use]
pub fn fold_sum_u64_idx(values: &[u64], stride: usize, offset: usize, indices: &[u32]) -> u128 {
    debug_assert_idx_bounds(values, stride, offset, indices);
    let mut lows = Simd::<u64, IDX_LANES>::splat(0);
    let mut carries = Simd::<u64, IDX_LANES>::splat(0);
    let (chunks, tail) = indices.as_chunks::<IDX_LANES>();
    for chunk in chunks {
        let v = gather_words(values, stride, offset, chunk);
        let new = lows + v;

        carries -= lows.simd_gt(new).to_simd().cast::<u64>();
        lows = new;
    }
    let mut total: u128 = 0;
    for lane in 0..IDX_LANES {
        total += u128::from(lows.as_array()[lane]) + (u128::from(carries.as_array()[lane]) << 64);
    }
    for &i in tail {
        total += u128::from(values[i as usize * stride + offset]);
    }
    total
}

/// Word-order (min, max) at the indexed positions in one pass — biased
/// i64 words are order-preserving, so one kernel serves both
/// signednesses.
/// # Panics
/// Only on a programmer-invariant violation: an empty index list (the
/// executor never emits empty batches).
#[must_use]
pub fn fold_min_max_u64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    assert!(!indices.is_empty(), "non-empty batch");
    debug_assert_idx_bounds(values, stride, offset, indices);
    let mut mins = Simd::<u64, IDX_LANES>::splat(u64::MAX);
    let mut maxs = Simd::<u64, IDX_LANES>::splat(u64::MIN);
    let (chunks, tail) = indices.as_chunks::<IDX_LANES>();
    for chunk in chunks {
        let v = gather_words(values, stride, offset, chunk);
        mins = mins.simd_min(v);
        maxs = maxs.simd_max(v);
    }
    let mut min_scalar = mins.reduce_min();
    let mut max_scalar = maxs.reduce_max();
    for &i in tail {
        let word = values[i as usize * stride + offset];
        min_scalar = min_scalar.min(word);
        max_scalar = max_scalar.max(word);
    }
    (min_scalar, max_scalar)
}
