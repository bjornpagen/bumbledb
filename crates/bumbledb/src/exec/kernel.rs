//! Batch kernels: predicate scans, survivor compaction, Allen interval masks
//! and word folds. Portable bodies are generic over `S: Simd` on fixed-width
//! vectors, so chunking, tails and bitmasks are identical at every level; each
//! call dispatches once on the process-wide [`level`]. aarch64 Allen keeps a
//! hand-tuned NEON specialization. Every kernel has a scalar twin in
//! [`reference`] that the tests hold it bit-identical to at every level.
mod allen;
pub mod bench;
mod compact;
mod filter;
mod fold;
mod gather;
pub mod numeric;
mod prefetch;
pub mod reference;

#[cfg(target_arch = "aarch64")]
#[expect(
    unsafe_code,
    reason = "NEON intrinsics over windows proven in bounds by the dispatch"
)]
mod neon;

use std::sync::OnceLock;

use fearless_simd::Level;

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

/// The best SIMD level of this CPU, detected once per process. Miri
/// interprets the scalar fallback level, which has no intrinsics.
pub(crate) fn level() -> Level {
    static LEVEL: OnceLock<Level> = OnceLock::new();
    *LEVEL.get_or_init(detect)
}

#[cfg(not(miri))]
fn detect() -> Level {
    Level::new()
}

#[cfg(miri)]
fn detect() -> Level {
    Level::fallback()
}

/// The detected level and every lower level it implies, lowest first.
fn supported_levels() -> Vec<Level> {
    let best = Level::new();
    let mut levels = Vec::new();
    #[cfg(target_arch = "aarch64")]
    levels.extend(best.as_neon().map(Level::Neon));
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        levels.extend(best.as_sse2().map(Level::Sse2));
        levels.extend(best.as_sse4_2().map(Level::Sse4_2));
        levels.extend(best.as_avx2().map(Level::Avx2));
        levels.extend(best.as_avx512().map(Level::Avx512));
    }
    levels
}

fn level_name(level: Level) -> &'static str {
    if level.is_fallback() {
        return "fallback";
    }
    #[cfg(target_arch = "aarch64")]
    if level.as_neon().is_some() {
        return "neon";
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if level.as_avx512().is_some() {
            return "avx512";
        }
        if level.as_avx2().is_some() {
            return "avx2";
        }
        if level.as_sse4_2().is_some() {
            return "sse4.2";
        }
        if level.as_sse2().is_some() {
            return "sse2";
        }
    }
    "unknown"
}

/// [`supported_levels`] plus the scalar fallback level.
#[cfg(test)]
pub(crate) fn every_level() -> Vec<Level> {
    let mut levels = vec![Level::fallback()];
    levels.extend(supported_levels());
    levels
}

#[cfg(test)]
mod tests;
