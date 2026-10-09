//! Kernel entry points at an explicit SIMD level, for the bench crate's micro
//! report; the scalar twins are [`super::reference`]. Not embedding API.
use fearless_simd::Level;

/// One SIMD level this CPU supports.
#[derive(Clone, Copy, Debug)]
pub struct SimdLevel(Level);

impl SimdLevel {
    /// The detected level and every lower level it implies, lowest first.
    #[must_use]
    pub fn available() -> Vec<Self> {
        super::supported_levels().into_iter().map(Self).collect()
    }

    /// The level's instruction-set name.
    #[must_use]
    pub fn name(self) -> &'static str {
        super::level_name(self.0)
    }
}

/// `filter_eq_u64` at `level`.
pub fn filter_eq_u64(level: SimdLevel, col: &[u64], value: u64, out: &mut Vec<u32>) {
    super::filter::eq_u64(level.0, col, value, out);
}

/// `filter_range_u64` at `level`.
pub fn filter_range_u64(level: SimdLevel, col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    super::filter::range_u64(level.0, col, lo, hi, out);
}

/// `filter_eq_u8` at `level`.
pub fn filter_eq_u8(level: SimdLevel, col: &[u8], value: u8, out: &mut Vec<u32>) {
    super::filter::eq_u8(level.0, col, value, out);
}

/// `filter_point_in_u64` at `level`.
pub fn filter_point_in_u64(
    level: SimdLevel,
    starts: &[u64],
    ends: &[u64],
    point: u64,
    out: &mut Vec<u32>,
) {
    super::filter::point_in_u64(level.0, starts, ends, point, out);
}

/// `filter_any_point_in_u64` at `level`.
pub fn filter_any_point_in_u64(
    level: SimdLevel,
    starts: &[u64],
    ends: &[u64],
    points: &[u64],
    out: &mut Vec<u32>,
) {
    super::filter::any_point_in_u64(level.0, starts, ends, points, out);
}

/// `fold_sum_u64` at `level`.
#[must_use]
pub fn fold_sum_u64(
    level: SimdLevel,
    values: &[u64],
    stride: usize,
    offset: usize,
    count: usize,
) -> u128 {
    super::fold::sum_u64(level.0, values, stride, offset, count)
}

/// `fold_min_max_u64` at `level`.
#[must_use]
pub fn fold_min_max_u64(
    level: SimdLevel,
    values: &[u64],
    stride: usize,
    offset: usize,
    count: usize,
) -> (u64, u64) {
    super::fold::min_max_u64(level.0, values, stride, offset, count)
}

/// `fold_sum_u64_idx` at `level`.
#[must_use]
pub fn fold_sum_u64_idx(
    level: SimdLevel,
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> u128 {
    super::gather::sum_u64_idx(level.0, values, stride, offset, indices)
}

/// `fold_min_max_u64_idx` at `level`.
#[must_use]
pub fn fold_min_max_u64_idx(
    level: SimdLevel,
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    super::gather::min_max_u64_idx(level.0, values, stride, offset, indices)
}

/// `allen_code_batch` at `level`.
pub fn allen_code_batch(
    level: SimdLevel,
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut Vec<u8>,
) {
    super::allen::code_batch(level.0, a_starts, a_ends, b_starts, b_ends, codes);
}

/// `allen_code_batch_const` at `level`.
pub fn allen_code_batch_const(
    level: SimdLevel,
    a_starts: &[u64],
    a_ends: &[u64],
    b_start: u64,
    b_end: u64,
    codes: &mut Vec<u8>,
) {
    super::allen::code_batch_const(level.0, a_starts, a_ends, b_start, b_end, codes);
}

/// `allen_filter_batch` at `level`.
pub fn allen_filter_batch(
    level: SimdLevel,
    codes: &[u8],
    mask: crate::AllenMask,
    keep: &mut Vec<u8>,
) {
    super::allen::filter_batch(level.0, codes, mask, keep);
}

/// `allen_filter_columns` at `level`.
pub fn allen_filter_columns(
    level: SimdLevel,
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    mask: crate::AllenMask,
    out: &mut Vec<u32>,
) {
    super::allen::filter_columns(level.0, a_starts, a_ends, b_starts, b_ends, mask, out);
}

/// `allen_filter_columns_const` at `level`.
pub fn allen_filter_columns_const(
    level: SimdLevel,
    starts: &[u64],
    ends: &[u64],
    b_start: u64,
    b_end: u64,
    mask: crate::AllenMask,
    out: &mut Vec<u32>,
) {
    super::allen::filter_columns_const(level.0, starts, ends, b_start, b_end, mask, out);
}

/// `compact_u32_by_mask` at `level`.
pub fn compact_u32_by_mask(level: SimdLevel, items: &mut Vec<u32>, mask: &[u8]) {
    super::compact::compact(level.0, items, mask);
}
