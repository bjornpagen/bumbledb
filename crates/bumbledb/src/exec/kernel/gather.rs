// ---------------------------------------------------------------------
// The fold kernels. Two access shapes each:
// `_idx` gathers `values[idx as usize * stride + offset]` per index (the
// leaf batch's entry-major keys, and the scan pushdown's position
// gathers at stride 1 / offset 0); the contiguous form walks a strided slice
// directly (dense survivor runs — no index loads at all).
//
// The `_idx` kernels are `std::simd` gathers on every target
// (docs/prd-crucible/03-portable-simd.md, Q2 — ADOPT, measured: min/max
// ~9% faster than the retired scalar-unrolled bodies, sums at parity
// via the same carry-count trick as the dense fold; three `unsafe`
// blocks and their bounds obligations deleted — `gather_or_default`
// masks lanes safely). Four lanes: the adds race down separate
// dependency chains while the OoO window overlaps the gathers.
// ---------------------------------------------------------------------

use std::simd::prelude::*;

/// The i64 biased-word sign flip (order-preserving storage form to
/// logical value).
#[inline]
pub(super) fn biased_to_i64(word: u64) -> i64 {
    (word ^ (1 << 63)).cast_signed()
}

/// The gather lane width: four index lanes per wave.
const IDX_LANES: usize = 4;

/// The invariant the callers owe (checked in debug builds): every
/// strided index lands inside `values`. The safe gathers below would
/// otherwise read the lane default, never out of bounds.
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

/// Sum of sign-flip-decoded i64 words at the indexed positions — exact
/// i128, bit-identical to the naive fold: `Σ value = Σ word −
/// count·2^63` exactly (the bias identity, as in the dense fold).
#[must_use]
pub fn fold_sum_biased_i64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> i128 {
    let total = fold_sum_u64_idx(values, stride, offset, indices);
    let bias = u128::from(indices.len() as u64) << 63;
    i128::try_from(total).expect("sum of u32-counted words fits i128")
        - i128::try_from(bias).expect("bias fits i128")
}

/// Sum of u64 words at the indexed positions — exact u128 via carry
/// counting (see the dense fold's doctrine; same mechanism, gathered).
#[must_use]
pub fn fold_sum_u64_idx(values: &[u64], stride: usize, offset: usize, indices: &[u32]) -> u128 {
    debug_assert_idx_bounds(values, stride, offset, indices);
    let mut lows = Simd::<u64, IDX_LANES>::splat(0);
    let mut carries = Simd::<u64, IDX_LANES>::splat(0);
    let stride_v = Simd::<usize, IDX_LANES>::splat(stride);
    let offset_v = Simd::<usize, IDX_LANES>::splat(offset);
    let (chunks, tail) = indices.as_chunks::<IDX_LANES>();
    for chunk in chunks {
        let idx = Simd::<u32, IDX_LANES>::from_array(*chunk).cast::<usize>() * stride_v + offset_v;
        let v = Simd::gather_or_default(values, idx);
        let new = lows + v;
        // Overflowed lanes read all-ones; subtracting adds 1.
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
///
/// # Panics
///
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
    let stride_v = Simd::<usize, IDX_LANES>::splat(stride);
    let offset_v = Simd::<usize, IDX_LANES>::splat(offset);
    let (chunks, tail) = indices.as_chunks::<IDX_LANES>();
    for chunk in chunks {
        let idx = Simd::<u32, IDX_LANES>::from_array(*chunk).cast::<usize>() * stride_v + offset_v;
        let v = Simd::gather_or_default(values, idx);
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
