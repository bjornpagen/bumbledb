//! The explicit-SIMD and unrolled-fold kernels (docs/architecture/40-execution.md;
//! sanctioned shapes amended by docs/perf/): fixed-width predicate scans,
//! survivor compaction, and — PRD 03 — the fold/accumulate kernels behind
//! the aggregate sink's batch path, all behind scalar-identical
//! signatures.
//!
//! `unsafe` is sanctioned here per the 00-product policy. The NEON paths
//! are `cfg(target_arch = "aarch64")`; every other 64-bit platform
//! compiles and runs the scalar fallback correctly, with no performance
//! promises. The scalar implementations are compiled wherever they have a
//! reader — the non-aarch64 live path and every target's test builds,
//! where they are the reference the property tests compare the kernels
//! against, bit for bit (an aarch64 release build omits them: dead code
//! is disallowed).
//!
//! Fold doctrine (30-execution, rewritten by docs/silicon/06 from
//! bumblebench exps 03/04): **port topology decides, not lane count.**
//! Every flag-writing scalar op (`adds/adcs/cmp/csel`) is confined to 3
//! of the M2's 6 integer ALUs, so exact scalar summation caps at ~2.8
//! flag-µops/cycle while the frontend idles; NEON escapes the triad and
//! multiplies load-port width (3 × 16 B). Measured on the reference
//! host: exact-u128 NEON via carry counting 8.8 rows/ns vs 4.0–4.6 for
//! the 4-accumulator scalar i128 loop at L1; min/max 2.65× at every
//! tier (`cmhi`/`bsl` run on all 4 vector pipes — there is no
//! `vmaxq_u64`). From DRAM all parallel kernels converge (~7.5 rows/ns
//! at 60.6 GB/s single-core), so dense folds take NEON unconditionally.
//! The prior scalar-ILP-first doctrine rested on a 2.45 rows/ns scalar
//! measurement that reproduces on no core/cache combination of the
//! reference host — a frequency-contamination artifact, kept on record
//! (docs/silicon/README, law table) rather than erased. Sum semantics
//! are exact i128/u128 accumulation — bit-identical to the naive fold at
//! any association, since fewer than 2^64 i64 terms cannot wrap i128;
//! the NEON sum counts unsigned carries (`vcgtq_u64(old, new)`) into a
//! parallel lane so exactness costs vector ops, not flag ports.
//!
//! Survivor compaction is the scalar cursor-write on every target: NEON has
//! no compress instruction (that is SVE, which Apple Silicon lacks), and
//! 30-execution names "NEON compress **or scalar cursor-write**" as the
//! sanctioned shapes. No x86 intrinsics exist anywhere (doctrine).
//!
//! Fallback verification command (run where a cross std is installed):
//! `cargo check --workspace --target x86_64-unknown-linux-gnu`. The
//! non-aarch64 dispatch arms are one-line calls into [`reference`], which
//! compiles and is property-tested on every target.

mod compact;
mod filter;
mod fold;
mod gather;
mod prefetch;

/// The scalar reference implementations: the fallback on non-aarch64
/// targets and the comparison oracle the aarch64 property tests assert
/// bit-identity against (absent only in aarch64 non-test builds, where
/// they would be dead code).
#[cfg(any(not(target_arch = "aarch64"), test))]
pub mod reference;

/// The NEON kernels (128-bit: 2 x u64 or 16 x u8 lanes).
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code)] // the 30-execution doc: the one sanctioned unsafe module
mod neon;

pub use compact::compact_u32_by_mask;
pub use filter::{
    filter_any_point_in_u64, filter_contains_u64, filter_eq_u64, filter_eq_u8, filter_overlaps_u64,
    filter_point_in_u64, filter_range_u64, filter_within_u64,
};
pub use fold::{fold_min_max_u64, fold_sum_biased_i64, fold_sum_u64};
pub use gather::{fold_min_max_u64_idx, fold_sum_biased_i64_idx, fold_sum_u64_idx};
pub use prefetch::prefetch_read;

#[cfg(test)]
use gather::biased_to_i64;

#[cfg(test)]
mod tests;
