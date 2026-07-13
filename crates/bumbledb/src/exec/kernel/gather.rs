// ---------------------------------------------------------------------
// The fold kernels. Two access shapes each:
// `_idx` gathers `values[idx as usize * stride + offset]` per index (the
// leaf batch's entry-major keys, and the scan pushdown's position
// gathers at stride 1 / offset 0); the contiguous form walks a strided slice
// directly (dense survivor runs — no index loads at all).
// ---------------------------------------------------------------------

/// The i64 biased-word sign flip (order-preserving storage form to
/// logical value).
#[inline]
pub(super) fn biased_to_i64(word: u64) -> i64 {
    (word ^ (1 << 63)).cast_signed()
}

/// Bounds proof for the `_idx` kernels, checked once per call in debug
/// builds; the unchecked interior relies on it.
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
/// i128, bit-identical to the naive fold.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub fn fold_sum_biased_i64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> i128 {
    debug_assert_idx_bounds(values, stride, offset, indices);
    // Four independent accumulators: the adds race down separate
    // dependency chains while the OoO window overlaps the gathers.
    let mut acc = [0i128; 4];
    let chunks = indices.chunks_exact(4);
    let tail = chunks.remainder();
    for chunk in chunks {
        for (lane, &idx) in chunk.iter().enumerate() {
            // SAFETY: debug-asserted above; indices are image/batch
            // positions produced against `values`' extent.
            let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
            acc[lane] += i128::from(biased_to_i64(word));
        }
    }
    for &idx in tail {
        let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
        acc[0] += i128::from(biased_to_i64(word));
    }
    acc[0] + acc[1] + acc[2] + acc[3]
}

/// Sum of u64 words at the indexed positions — exact u128.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub fn fold_sum_u64_idx(values: &[u64], stride: usize, offset: usize, indices: &[u32]) -> u128 {
    debug_assert_idx_bounds(values, stride, offset, indices);
    let mut acc = [0u128; 4];
    let chunks = indices.chunks_exact(4);
    let tail = chunks.remainder();
    for chunk in chunks {
        for (lane, &idx) in chunk.iter().enumerate() {
            // SAFETY: as in `fold_sum_biased_i64_idx`.
            let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
            acc[lane] += u128::from(word);
        }
    }
    for &idx in tail {
        let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
        acc[0] += u128::from(word);
    }
    acc[0] + acc[1] + acc[2] + acc[3]
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
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub fn fold_min_max_u64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    assert!(!indices.is_empty(), "non-empty batch");
    debug_assert_idx_bounds(values, stride, offset, indices);
    let mut mins = [u64::MAX; 4];
    let mut maxs = [u64::MIN; 4];
    let chunks = indices.chunks_exact(4);
    let tail = chunks.remainder();
    for chunk in chunks {
        for (lane, &idx) in chunk.iter().enumerate() {
            // SAFETY: as in `fold_sum_biased_i64_idx`.
            let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
            mins[lane] = mins[lane].min(word);
            maxs[lane] = maxs[lane].max(word);
        }
    }
    for &idx in tail {
        let word = unsafe { *values.get_unchecked(idx as usize * stride + offset) };
        mins[0] = mins[0].min(word);
        maxs[0] = maxs[0].max(word);
    }
    (
        mins.iter().copied().min().expect("four lanes"),
        maxs.iter().copied().max().expect("four lanes"),
    )
}
