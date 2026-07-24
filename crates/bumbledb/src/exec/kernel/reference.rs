//! The reference twins — the differential oracle the property tests
//! compare every kernel against, bit for bit. Deliberately SCALAR for
//! every shape whose live kernel is `std::simd` (the crucible packet (git ecec1dc3)
//! 03-portable-simd.md, Q1×Q2 resolution: one body cannot be its own
//! oracle, so where the kernel adopted the portable lane form, the twin
//! keeps the definitional scalar form — the differential's independence
//! outranks the vocabulary win). The Allen twins likewise stay the
//! `classify` decision tree (never the signature table) so the tests
//! cross-check table against tree; [`allen_keep`] alone speaks
//! `std::simd`, because its live kernel is the NEON `tbl` mechanism and
//! the lane-parallel shift-and-mask remains an independent derivation.
//!
//! The Allen twins are also the live non-aarch64 dispatch arm; the
//! filter twins are test-only (their portable kernels run everywhere).

/// Scalar reference of [`super::filter_eq_u64`].
#[cfg(test)]
pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| col[i] == value);
}

/// Scalar reference of [`super::filter_range_u64`].
#[cfg(test)]
pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| (lo..=hi).contains(&col[i]));
}

/// Scalar reference of [`super::filter_eq_u8`].
#[cfg(test)]
pub fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
    push_matching(col.len(), out, |i| col[i] == value);
}

/// Scalar reference of [`super::filter_point_in_u64`]: the half-open
/// membership rule, `start <= p AND p < end`.
#[cfg(test)]
pub fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
    push_matching(starts.len(), out, |i| starts[i] <= point && point < ends[i]);
}

/// Scalar reference of [`super::filter_any_point_in_u64`]: the OR over
/// per-point membership masks.
#[cfg(test)]
pub fn filter_any_point_in_u64(starts: &[u64], ends: &[u64], points: &[u64], out: &mut Vec<u32>) {
    push_matching(starts.len(), out, |i| {
        points.iter().any(|p| starts[i] <= *p && *p < ends[i])
    });
}

/// Scalar reference of [`super::allen_code_batch`]'s core: PRD 03's
/// `classify` decision tree per pair — deliberately **never** the
/// signature table, so the property tests cross-check the NEON table
/// against the tree, bit for bit. The code is the [`crate::allen::Basic`]
/// discriminant (its bit index in the mask coordinate system).
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

/// [`allen_codes`] with a constant right operand.
pub fn allen_codes_const(starts: &[u64], ends: &[u64], b_start: u64, b_end: u64, codes: &mut [u8]) {
    for (i, code) in codes.iter_mut().enumerate() {
        *code = crate::allen::classify_bounds(&starts[i], &ends[i], &b_start, &b_end) as u8;
    }
}

/// Reference of [`super::allen_filter_batch`]'s core:
/// `keep[i] = 1` iff the mask holds `codes[i]` — one shift, one and
/// (lane-parallel here; still never the kernel's `tbl` table).
pub fn allen_keep(codes: &[u8], mask_bits: u16, keep: &mut [u8]) {
    use std::simd::prelude::*;
    const N: usize = 16;
    let mask = Simd::<u16, N>::splat(mask_bits);
    let one = Simd::<u16, N>::splat(1);
    let (chunks, tail) = codes.as_chunks::<N>();
    let (keep_chunks, keep_tail) = keep.as_chunks_mut::<N>();
    for (chunk, keep_chunk) in chunks.iter().zip(keep_chunks) {
        let codes: Simd<u16, N> = Simd::from_array(*chunk).cast();
        *keep_chunk = ((mask >> codes) & one).cast::<u8>().to_array();
    }
    for (keep, &code) in keep_tail.iter_mut().zip(tail) {
        *keep = ((mask_bits >> u32::from(code)) & 1) as u8;
    }
}

/// Scalar reference of [`super::filter_duration_range_u64`]: the ray
/// test first (`end == MAX` never survives — its verdict is Ray, the
/// ray-probe pass's territory, ruled 2026-07-23, R6), then the exact
/// encoded-word subtraction against the inclusive range.
#[cfg(test)]
pub fn filter_duration_range_u64(
    starts: &[u64],
    ends: &[u64],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) {
    let start = out.len();
    out.resize(start + starts.len(), 0);
    let mut write = start;
    for i in 0..starts.len() {
        let keep = ends[i] != u64::MAX && {
            let duration = ends[i] - starts[i];
            (lo..=hi).contains(&duration)
        };
        out[write] = u32::try_from(i).expect("positions fit u32");
        write += usize::from(keep);
    }
    out.truncate(write);
}

/// Scalar reference of [`super::compact_u32_by_mask`]: the fully
/// safe-indexed cursor-write (the pre-diet shape — `items[write]`
/// bounds-checked, keep judged as `mask[i] != 0`). The property test
/// asserts bit-identity on 0/1 masks, the live kernel's contract.
#[cfg(test)]
pub fn compact_u32_by_mask(items: &mut Vec<u32>, mask: &[u8]) {
    assert!(mask.len() >= items.len());
    let mut write = 0usize;
    for read in 0..items.len() {
        items[write] = items[read];
        write += usize::from(mask[read] != 0);
    }
    items.truncate(write);
}

/// Branchless cursor-write over the whole column.
#[cfg(test)]
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
