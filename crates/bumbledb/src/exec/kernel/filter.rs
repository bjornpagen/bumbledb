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

/// Positions in `col` equal to `value` (the enum/bool column variant,
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

/// Interval overlap against a constant interval: positions where
/// `starts[i] < c_end AND c_start < ends[i]` (point-sets intersect).
pub fn filter_overlaps_u64(
    starts: &[u64],
    ends: &[u64],
    c_start: u64,
    c_end: u64,
    out: &mut Vec<u32>,
) {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    #[cfg(target_arch = "aarch64")]
    neon::filter_overlaps_u64(starts, ends, c_start, c_end, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_overlaps_u64(starts, ends, c_start, c_end, out);
}

/// Interval containment of a constant interval: positions where
/// `starts[i] <= c_start AND c_end <= ends[i]` (field ⊇ constant).
pub fn filter_contains_u64(
    starts: &[u64],
    ends: &[u64],
    c_start: u64,
    c_end: u64,
    out: &mut Vec<u32>,
) {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    #[cfg(target_arch = "aarch64")]
    neon::filter_contains_u64(starts, ends, c_start, c_end, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_contains_u64(starts, ends, c_start, c_end, out);
}

/// The reversed containment: positions whose interval lies within the
/// constant — `c_start <= starts[i] AND ends[i] <= c_end`
/// (constant ⊇ field).
pub fn filter_within_u64(
    starts: &[u64],
    ends: &[u64],
    c_start: u64,
    c_end: u64,
    out: &mut Vec<u32>,
) {
    debug_assert_eq!(starts.len(), ends.len(), "an interval span's column pair");
    #[cfg(target_arch = "aarch64")]
    neon::filter_within_u64(starts, ends, c_start, c_end, out);
    #[cfg(not(target_arch = "aarch64"))]
    reference::filter_within_u64(starts, ends, c_start, c_end, out);
}
