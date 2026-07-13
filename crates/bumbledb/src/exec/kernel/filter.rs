#[cfg(target_arch = "aarch64")]
use super::neon;
#[cfg(not(target_arch = "aarch64"))]
use super::reference;

/// Positions in `col` equal to `value`, appended to `out` in ascending
/// order (branchless survivor writes).
pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
    #[cfg(target_arch = "aarch64")]
    neon::filter_eq_u64(col, value, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_eq_u64(col, value, out);
}

/// Positions in `col` within `lo..=hi` (u64 word order — order-preserving
/// for I64's biased words too), appended to `out` in ascending order.
pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    #[cfg(target_arch = "aarch64")]
    neon::filter_range_u64(col, lo, hi, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_range_u64(col, lo, hi, out);
}

/// Positions in `col` equal to `value` (the bool byte-column arm,
/// 16 lanes), appended to `out` in ascending order.
pub fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
    #[cfg(target_arch = "aarch64")]
    neon::filter_eq_u8(col, value, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_eq_u8(col, value, out);
}

/// Point membership over an interval column pair: positions where
/// `starts[i] <= point AND point < ends[i]` (the half-open rule), in
/// ascending order. The composition is the existing predicate-scan
/// shape applied to two columns with an AND — no new kernel shape
/// (docs/architecture/40-execution.md, § access paths).
pub fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    #[cfg(target_arch = "aarch64")]
    neon::filter_point_in_u64(starts, ends, point, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_point_in_u64(starts, ends, point, out);
}

/// Point-*set* membership over an interval column pair: positions where
/// ANY element of `points` lies in `[starts[i], ends[i])` — the OR over
/// per-point masks (k small by the documented set assumption,
/// `docs/architecture/20-query-ir.md` § param sets). An empty set keeps
/// nothing.
pub fn filter_any_point_in_u64(starts: &[u64], ends: &[u64], points: &[u64], out: &mut Vec<u32>) {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    #[cfg(target_arch = "aarch64")]
    neon::filter_any_point_in_u64(starts, ends, points, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_any_point_in_u64(starts, ends, points, out);
}

/// The measure scan — the one gather+subtract shape
/// (docs/architecture/20-query-ir.md, § the measure): positions whose
/// duration `ends[i] − starts[i]` lies within `lo..=hi`, appended
/// in ascending order. The subtraction feeds the existing range shape —
/// one fused stride-1 pass, NEON on the dense case per the port-topology
/// law (subtraction is not flag-bound); strided/gathered callers stay
/// scalar until measured, per the standing rule. Encoded-word
/// subtraction is exact for both element types: the encodings are
/// unit-spaced order-preserving maps onto u64 words (u64 the identity,
/// I64 the +2⁶³ bias, which cancels), and the constructor invariant
/// `end > start` keeps the difference positive.
///
/// # Errors
///
/// The first ray in scan order (`ends[i] == u64::MAX` — ∞ in both
/// element encodings): a ray has no finite measure, and the caller
/// raises the typed [`crate::Error::MeasureOfRay`]. `out`'s contents are
/// unspecified after an error.
pub fn filter_duration_range_u64(
    starts: &[u64],
    ends: &[u64],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) -> Result<(), usize> {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    #[cfg(target_arch = "aarch64")]
    return neon::filter_duration_range_u64(starts, ends, lo, hi, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_duration_range_u64(starts, ends, lo, hi, out)
}

// The old interval-vs-constant comparison kernels (overlaps, contains,
// within-over-pairs) are gone with their operators: interval-pair
// predicates are Allen masks, evaluated by the configuration kernel
// (`super::allen` — one branchless, flag-free kernel for all 8192
// masks).
