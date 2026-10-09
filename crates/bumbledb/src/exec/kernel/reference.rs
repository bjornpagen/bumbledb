//! Scalar twins of every kernel, written in the definitional form: the
//! differential oracle the tests hold each kernel bit-identical to at every
//! SIMD level, and the bench's baseline. Allen twins use the `classify`
//! decision tree, never a signature table, so the tests cross-check the two.

/// Twin of [`super::filter_eq_u64`].
pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| col[i] == value);
}

/// Twin of [`super::filter_range_u64`].
pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| (lo..=hi).contains(&col[i]));
}

/// Twin of [`super::filter_eq_u8`].
pub fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| col[i] == value);
}

/// Twin of [`super::filter_point_in_u64`].
pub fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
    push_matching(starts.len(), out, |i| starts[i] <= point && point < ends[i]);
}

/// Twin of [`super::filter_any_point_in_u64`].
pub fn filter_any_point_in_u64(starts: &[u64], ends: &[u64], points: &[u64], out: &mut Vec<u32>) {
    push_matching(starts.len(), out, |i| {
        points.iter().any(|p| starts[i] <= *p && *p < ends[i])
    });
}

/// Twin of [`super::fold_sum_u64`].
#[must_use]
pub fn fold_sum_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> u128 {
    (0..count)
        .map(|i| u128::from(values[i * stride + offset]))
        .sum()
}

/// Twin of [`super::fold_min_max_u64`].
#[must_use]
pub fn fold_min_max_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> (u64, u64) {
    min_max((0..count).map(|i| values[i * stride + offset]))
}

/// Twin of [`super::fold_sum_u64_idx`].
#[must_use]
pub fn fold_sum_u64_idx(values: &[u64], stride: usize, offset: usize, indices: &[u32]) -> u128 {
    indices
        .iter()
        .map(|&i| u128::from(values[i as usize * stride + offset]))
        .sum()
}

/// Twin of [`super::fold_min_max_u64_idx`].
#[must_use]
pub fn fold_min_max_u64_idx(
    values: &[u64],
    stride: usize,
    offset: usize,
    indices: &[u32],
) -> (u64, u64) {
    min_max(
        indices
            .iter()
            .map(|&i| values[i as usize * stride + offset]),
    )
}

/// Twin of [`super::allen_code_batch`]: `codes[i]` is the
/// [`crate::allen::Basic`] discriminant of pair `i`.
pub fn allen_codes(
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut [u8],
) {
    for (i, code) in codes.iter_mut().enumerate() {
        *code =
            crate::allen::classify_bounds(&a_starts[i], &a_ends[i], &b_starts[i], &b_ends[i]) as u8;
    }
}

/// Twin of [`super::allen_code_batch_const`].
pub fn allen_codes_const(starts: &[u64], ends: &[u64], b_start: u64, b_end: u64, codes: &mut [u8]) {
    for (i, code) in codes.iter_mut().enumerate() {
        *code = crate::allen::classify_bounds(&starts[i], &ends[i], &b_start, &b_end) as u8;
    }
}

/// Twin of [`super::allen_filter_batch`]: `keep[i] = 1` iff the mask holds
/// `codes[i]`.
pub fn allen_keep(codes: &[u8], mask_bits: u16, keep: &mut [u8]) {
    for (keep, &code) in keep.iter_mut().zip(codes) {
        *keep = u8::from((mask_bits >> code) & 1 != 0);
    }
}

/// Twin of [`super::compact_u32_by_mask`]: safe indexing, keep judged as
/// `mask[i] != 0`.
///
/// # Panics
/// If `mask` is shorter than `items`.
pub fn compact_u32_by_mask(items: &mut Vec<u32>, mask: &[u8]) {
    assert!(mask.len() >= items.len());
    let mut write = 0usize;
    for read in 0..items.len() {
        items[write] = items[read];
        write += usize::from(mask[read] != 0);
    }
    items.truncate(write);
}

fn min_max(words: impl Iterator<Item = u64>) -> (u64, u64) {
    words.fold((u64::MAX, u64::MIN), |(lo, hi), w| (lo.min(w), hi.max(w)))
}

fn push_matching(len: usize, out: &mut Vec<u32>, keep: impl Fn(usize) -> bool) {
    out.extend(
        (0..len)
            .filter(|&i| keep(i))
            .map(|i| u32::try_from(i).expect("positions fit u32")),
    );
}
