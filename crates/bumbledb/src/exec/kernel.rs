//! SIMD and unrolled kernels for predicate scans, survivor compaction,
//! Allen interval masks and aggregate reductions, checked against scalar
//! references. Predicate and fold kernels use portable SIMD; aarch64 Allen
//! classification uses NEON's 64-byte table lookup (`tbl4`).
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
)]
mod neon;

pub use allen::{
    allen_code_batch, allen_code_batch_const, allen_filter_batch, allen_filter_columns,
    allen_filter_columns_const,
};
pub use compact::compact_u32_by_mask;
pub use filter::{
    filter_any_point_in_u64, filter_eq_u8, filter_eq_u64, filter_point_in_u64, filter_range_u64,
};
pub use fold::{fold_min_max_u64, fold_sum_u64};
pub use gather::{fold_min_max_u64_idx, fold_sum_u64_idx};
pub use prefetch::prefetch_read;

#[cfg(test)]
mod tests;
