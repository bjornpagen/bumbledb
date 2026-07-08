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
