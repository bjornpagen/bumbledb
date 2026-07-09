/// Scalar reference of [`super::filter_eq_u64`].
pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| col[i] == value);
}

/// Scalar reference of [`super::filter_range_u64`].
pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| (lo..=hi).contains(&col[i]));
}

/// Scalar reference of [`super::filter_eq_u8`].
pub fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| col[i] == value);
}

/// Scalar reference of [`super::filter_point_in_u64`]: the half-open
/// membership rule, `start <= p AND p < end`.
pub fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
    push_matching(starts.len(), out, |i| starts[i] <= point && point < ends[i]);
}

/// Scalar reference of [`super::filter_any_point_in_u64`]: the OR over
/// per-point membership masks.
pub fn filter_any_point_in_u64(starts: &[u64], ends: &[u64], points: &[u64], out: &mut Vec<u32>) {
    push_matching(starts.len(), out, |i| {
        points.iter().any(|p| starts[i] <= *p && *p < ends[i])
    });
}

/// Scalar reference of [`super::filter_overlaps_u64`].
pub fn filter_overlaps_u64(
    starts: &[u64],
    ends: &[u64],
    c_start: u64,
    c_end: u64,
    out: &mut Vec<u32>,
) {
    push_matching(starts.len(), out, |i| {
        starts[i] < c_end && c_start < ends[i]
    });
}

/// Scalar reference of [`super::filter_contains_u64`].
pub fn filter_contains_u64(
    starts: &[u64],
    ends: &[u64],
    c_start: u64,
    c_end: u64,
    out: &mut Vec<u32>,
) {
    push_matching(starts.len(), out, |i| {
        starts[i] <= c_start && c_end <= ends[i]
    });
}

/// Scalar reference of [`super::filter_within_u64`].
pub fn filter_within_u64(
    starts: &[u64],
    ends: &[u64],
    c_start: u64,
    c_end: u64,
    out: &mut Vec<u32>,
) {
    push_matching(starts.len(), out, |i| {
        c_start <= starts[i] && ends[i] <= c_end
    });
}

/// Branchless cursor-write over the whole column.
fn push_matching(len: usize, out: &mut Vec<u32>, keep: impl Fn(usize) -> bool) {
    let start = out.len();
    out.resize(start + len, 0);
    let mut write = start;
    for i in 0..len {
        out[write] = u32::try_from(i).expect("positions fit u32");
        write += usize::from(keep(i));
    }
    out.truncate(write);
}
