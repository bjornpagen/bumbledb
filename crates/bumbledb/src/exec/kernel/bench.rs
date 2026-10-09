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

/// [`super::filter_eq_u64`] at `level`.
pub fn filter_eq_u64(level: SimdLevel, col: &[u64], value: u64, out: &mut Vec<u32>) {
    super::filter::eq_u64(level.0, col, value, out);
}

/// [`super::filter_range_u64`] at `level`.
pub fn filter_range_u64(level: SimdLevel, col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    super::filter::range_u64(level.0, col, lo, hi, out);
}

/// [`super::filter_eq_u8`] at `level`.
pub fn filter_eq_u8(level: SimdLevel, col: &[u8], value: u8, out: &mut Vec<u32>) {
    super::filter::eq_u8(level.0, col, value, out);
}

/// [`super::filter_point_in_u64`] at `level`.
pub fn filter_point_in_u64(
    level: SimdLevel,
    starts: &[u64],
    ends: &[u64],
    point: u64,
    out: &mut Vec<u32>,
) {
    super::filter::point_in_u64(level.0, starts, ends, point, out);
}

/// [`super::filter_any_point_in_u64`] at `level`.
pub fn filter_any_point_in_u64(
    level: SimdLevel,
    starts: &[u64],
    ends: &[u64],
    points: &[u64],
    out: &mut Vec<u32>,
) {
    super::filter::any_point_in_u64(level.0, starts, ends, points, out);
}

/// [`super::fold_sum_u64`] at `level`.
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

/// [`super::fold_min_max_u64`] at `level`.
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

/// [`super::fold_sum_u64_idx`] at `level`.
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

/// [`super::fold_min_max_u64_idx`] at `level`.
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
