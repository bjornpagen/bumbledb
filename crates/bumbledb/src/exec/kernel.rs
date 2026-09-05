//! The explicit-SIMD and unrolled-fold kernels:
//! fixed-width predicate scans,
//! survivor compaction, the configuration kernel (Allen mask
//! [`allen_filter_batch`]), and the fold/accumulate kernels behind the
//! aggregate sink's batch path, all behind scalar-identical signatures.
//! **The portable/intrinsic split is measured, not stylistic**
//! intrinsic dual and most of the layer's `unsafe`, and are
//! unsafe module): its 64-byte `tbl4` signature table has no `std::simd`
mod allen;
mod compact;
mod filter;
mod fold;
mod gather;
pub mod numeric;
mod prefetch;

/// The reference twins: the differential oracle the property tests
/// assert bit-identity against on every target, and the live Allen
/// fallback on non-aarch64 targets (absent only in aarch64 non-test
/// builds, where they would be dead code; the filter twins are
/// test-only everywhere — their portable kernels run on every target).
#[cfg(any(not(target_arch = "aarch64"), test))]
pub mod reference;

#[cfg(target_arch = "aarch64")]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)] // the 40-execution doc: the one sanctioned unsafe module
mod neon;

pub use allen::{
    allen_code_batch, allen_code_batch_const, allen_filter_batch, allen_filter_columns,
    allen_filter_columns_const,
};
pub use compact::compact_u32_by_mask;
pub use filter::{
    filter_any_point_in_u64, filter_eq_u8, filter_eq_u64, filter_point_in_u64, filter_range_u64,
};
pub use fold::{fold_min_max_u64, fold_sum_biased_i64, fold_sum_u64};
pub use gather::{fold_min_max_u64_idx, fold_sum_biased_i64_idx, fold_sum_u64_idx};
pub use prefetch::prefetch_read;

#[cfg(test)]
use gather::biased_to_i64;

#[cfg(test)]
mod tests;
