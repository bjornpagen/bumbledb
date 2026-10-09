//! `Allen(mask)` over batches of interval pairs, branch-free and table driven: six
//! endpoint predicates pack into a 6-bit signature whose 13 valid values are the 13
//! basic relations, coded as [`crate::allen::Basic`] discriminants. The portable
//! kernel maps a signature through an injective 4-bit hash into one nibble-packed
//! `u64`; aarch64 indexes a 64-byte table with NEON `tbl`.
#![expect(
    clippy::inline_always,
    reason = "SIMD bodies inline into the dispatched target-feature context"
)]

use bumbledb_theory::allen::AllenMask;
use fearless_simd::{Level, Select, Simd, SimdBase, dispatch, mask64x4, u8x16, u64x4};

/// Signature bits, one per endpoint predicate of the pair `(a, b)`.
const START_EQ: u64 = 1;
const START_GT: u64 = 2;
const END_EQ: u64 = 4;
const END_GT: u64 = 8;
/// `a.end == b.start || b.end == a.start`.
const ADJACENT: u64 = 16;
/// `a.end > b.start && b.end > a.start`.
const INTERSECTS: u64 = 32;

/// Basic-relation code per 6-bit signature; `0xFF` marks the 51
/// signatures no pair of nonempty intervals can produce.
#[expect(clippy::cast_possible_truncation, reason = "signatures are below 64")]
pub(super) const SIGNATURE_CODES: [u8; 64] = {
    let relations: [u64; 13] = [
        0,
        ADJACENT,
        INTERSECTS,
        INTERSECTS | START_EQ,
        INTERSECTS | START_GT,
        INTERSECTS | START_GT | END_EQ,
        INTERSECTS | START_EQ | END_EQ,
        INTERSECTS | END_EQ,
        INTERSECTS | END_GT,
        INTERSECTS | START_EQ | END_GT,
        INTERSECTS | START_GT | END_GT,
        ADJACENT | START_GT | END_GT,
        START_GT | END_GT,
    ];
    let mut table = [0xFFu8; 64];
    let mut code = 0;
    while code < relations.len() {
        table[relations[code] as usize] = code as u8;
        code += 1;
    }
    table
};

/// The 4-bit hash `(15 * signature) / 4 mod 16`, injective over the 13 valid
/// signatures.
const fn hash(signature: u64) -> u64 {
    (((signature << 4) - signature) >> 2) & 15
}

/// Nibble `hash(s)` holds `SIGNATURE_CODES[s]` for every valid signature `s`.
const HASHED_CODES: u64 = {
    let mut table = 0u64;
    let mut seen = 0u16;
    let mut signature = 0;
    while signature < 64 {
        let code = SIGNATURE_CODES[signature];
        if code != 0xFF {
            let slot = hash(signature as u64);
            assert!(seen & (1 << slot) == 0, "the hash is injective");
            seen |= 1 << slot;
            table |= (code as u64) << (slot * 4);
        }
        signature += 1;
    }
    table
};

/// Configuration codes of the pairs `(a[i], b[i])` into `codes`, resized to
/// the pair count. Only growth zero-fills; every retained byte is
/// overwritten.
///
/// # Panics
/// If the four endpoint streams differ in length.
pub(crate) fn allen_code_batch(
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut Vec<u8>,
) {
    code_batch(super::level(), a_starts, a_ends, b_starts, b_ends, codes);
}

/// [`allen_code_batch`] against one constant right operand `[b_start, b_end)`.
///
/// # Panics
/// If the two endpoint streams differ in length.
pub(crate) fn allen_code_batch_const(
    a_starts: &[u64],
    a_ends: &[u64],
    b_start: u64,
    b_end: u64,
    codes: &mut Vec<u8>,
) {
    code_batch_const(super::level(), a_starts, a_ends, b_start, b_end, codes);
}

/// `keep[i] = 1` iff `mask` holds `codes[i]`; `keep` is resized like `codes`
/// in [`allen_code_batch`].
pub(crate) fn allen_filter_batch(codes: &[u8], mask: AllenMask, keep: &mut Vec<u8>) {
    filter_batch(super::level(), codes, mask, keep);
}

/// Positions `i` whose pair `(a[i], b[i])` satisfies `mask`, appended to
/// `out` in ascending order without allocating.
///
/// # Panics
/// If the four endpoint columns differ in length.
pub(crate) fn allen_filter_columns(
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    mask: AllenMask,
    out: &mut Vec<u32>,
) {
    filter_columns(
        super::level(),
        a_starts,
        a_ends,
        b_starts,
        b_ends,
        mask,
        out,
    );
}

/// [`allen_filter_columns`] against one constant right operand.
///
/// # Panics
/// If the two endpoint columns differ in length.
pub(crate) fn allen_filter_columns_const(
    starts: &[u64],
    ends: &[u64],
    b_start: u64,
    b_end: u64,
    mask: AllenMask,
    out: &mut Vec<u32>,
) {
    filter_columns_const(super::level(), starts, ends, b_start, b_end, mask, out);
}

pub(super) fn code_batch(
    level: Level,
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut Vec<u8>,
) {
    let n = a_starts.len();
    assert_eq!(a_ends.len(), n, "four equal-length endpoint streams");
    assert_eq!(b_starts.len(), n, "four equal-length endpoint streams");
    assert_eq!(b_ends.len(), n, "four equal-length endpoint streams");
    codes.resize(n, 0);
    codes_into(level, a_starts, a_ends, b_starts, b_ends, codes);
}

pub(super) fn code_batch_const(
    level: Level,
    a_starts: &[u64],
    a_ends: &[u64],
    b_start: u64,
    b_end: u64,
    codes: &mut Vec<u8>,
) {
    let n = a_starts.len();
    assert_eq!(a_ends.len(), n, "two equal-length endpoint streams");
    codes.resize(n, 0);
    codes_into_const(level, a_starts, a_ends, b_start, b_end, codes);
}

pub(super) fn filter_batch(level: Level, codes: &[u8], mask: AllenMask, keep: &mut Vec<u8>) {
    keep.resize(codes.len(), 0);
    keep_into(level, codes, mask, keep);
}

pub(super) fn filter_columns(
    level: Level,
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    mask: AllenMask,
    out: &mut Vec<u32>,
) {
    let n = a_starts.len();
    assert_eq!(a_ends.len(), n, "four equal-length endpoint streams");
    assert_eq!(b_starts.len(), n, "four equal-length endpoint streams");
    assert_eq!(b_ends.len(), n, "four equal-length endpoint streams");
    filter_chunked(level, n, mask, out, |range, codes| {
        codes_into(
            level,
            &a_starts[range.clone()],
            &a_ends[range.clone()],
            &b_starts[range.clone()],
            &b_ends[range],
            codes,
        );
    });
}

pub(super) fn filter_columns_const(
    level: Level,
    starts: &[u64],
    ends: &[u64],
    b_start: u64,
    b_end: u64,
    mask: AllenMask,
    out: &mut Vec<u32>,
) {
    assert_eq!(
        ends.len(),
        starts.len(),
        "two equal-length endpoint streams"
    );
    filter_chunked(level, starts.len(), mask, out, |range, codes| {
        codes_into_const(
            level,
            &starts[range.clone()],
            &ends[range],
            b_start,
            b_end,
            codes,
        );
    });
}

const SCAN_CHUNK: usize = 256;

/// Codes, keep bytes, then the survivor cursor, one stack chunk at a time.
fn filter_chunked(
    level: Level,
    n: usize,
    mask: AllenMask,
    out: &mut Vec<u32>,
    fill: impl Fn(std::ops::Range<usize>, &mut [u8]),
) {
    let mut codes = [0u8; SCAN_CHUNK];
    let mut keep = [0u8; SCAN_CHUNK];
    let mut cursor = super::filter::Cursor::open(out, n);
    let mut base = 0usize;
    while base < n {
        let len = SCAN_CHUNK.min(n - base);
        fill(base..base + len, &mut codes[..len]);
        keep_into(level, &codes[..len], mask, &mut keep[..len]);
        cursor.push_keeps(&keep[..len]);
        base += len;
    }
    cursor.close();
}

/// Callers assert the four streams match `codes` in length.
fn codes_into(
    level: Level,
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut [u8],
) {
    #[cfg(target_arch = "aarch64")]
    if let Some(neon) = level.as_neon()
        && codes.len() >= super::neon::CODE_LANES
    {
        super::neon::allen_code_batch_neon(neon, a_starts, a_ends, b_starts, b_ends, codes);
        return;
    }
    dispatch!(level, simd => portable_codes(
        simd,
        codes,
        #[inline(always)]
        |i| {
            (
                u64x4::from_slice(simd, &a_starts[i..i + 4]),
                u64x4::from_slice(simd, &a_ends[i..i + 4]),
                u64x4::from_slice(simd, &b_starts[i..i + 4]),
                u64x4::from_slice(simd, &b_ends[i..i + 4]),
            )
        },
        #[inline(always)]
        |i| code_of(a_starts[i], a_ends[i], b_starts[i], b_ends[i]),
    ));
}

/// Callers assert the two streams match `codes` in length.
fn codes_into_const(
    level: Level,
    starts: &[u64],
    ends: &[u64],
    b_start: u64,
    b_end: u64,
    codes: &mut [u8],
) {
    #[cfg(target_arch = "aarch64")]
    if let Some(neon) = level.as_neon()
        && codes.len() >= super::neon::CODE_LANES
    {
        super::neon::allen_code_batch_const_neon(neon, starts, ends, b_start, b_end, codes);
        return;
    }
    dispatch!(level, simd => portable_codes(
        simd,
        codes,
        #[inline(always)]
        |i| {
            (
                u64x4::from_slice(simd, &starts[i..i + 4]),
                u64x4::from_slice(simd, &ends[i..i + 4]),
                u64x4::splat(simd, b_start),
                u64x4::splat(simd, b_end),
            )
        },
        #[inline(always)]
        |i| code_of(starts[i], ends[i], b_start, b_end),
    ));
}

fn keep_into(level: Level, codes: &[u8], mask: AllenMask, keep: &mut [u8]) {
    #[cfg(target_arch = "aarch64")]
    if let Some(neon) = level.as_neon()
        && codes.len() >= super::neon::FILTER_LANES
    {
        super::neon::allen_filter_batch_neon(neon, codes, mask.bits(), keep);
        return;
    }
    dispatch!(level, simd => portable_keep(simd, codes, mask.bits(), keep));
}

#[inline(always)]
#[expect(clippy::cast_possible_truncation, reason = "codes are below 13")]
fn portable_codes<S: Simd>(
    simd: S,
    codes: &mut [u8],
    load: impl Fn(usize) -> (u64x4<S>, u64x4<S>, u64x4<S>, u64x4<S>),
    scalar: impl Fn(usize) -> u8,
) {
    let full = codes.len() / 4 * 4;
    let (chunks, tail) = codes.as_chunks_mut::<4>();
    for (chunk, i) in chunks.iter_mut().zip((0..full).step_by(4)) {
        let (a_s, a_e, b_s, b_e) = load(i);
        let lanes = codes4(simd, a_s, a_e, b_s, b_e);
        for (code, lane) in chunk.iter_mut().zip(lanes.to_array()) {
            *code = lane as u8;
        }
    }
    for (code, i) in tail.iter_mut().zip(full..) {
        *code = scalar(i);
    }
}

#[inline(always)]
fn codes4<S: Simd>(
    simd: S,
    a_s: u64x4<S>,
    a_e: u64x4<S>,
    b_s: u64x4<S>,
    b_e: u64x4<S>,
) -> u64x4<S> {
    let zero = u64x4::splat(simd, 0);
    let bit =
        |predicate: mask64x4<S>, weight: u64| predicate.select(u64x4::splat(simd, weight), zero);
    let signature = bit(a_s.simd_eq(b_s), START_EQ)
        | bit(a_s.simd_gt(b_s), START_GT)
        | bit(a_e.simd_eq(b_e), END_EQ)
        | bit(a_e.simd_gt(b_e), END_GT)
        | bit(a_e.simd_eq(b_s) | b_e.simd_eq(a_s), ADJACENT)
        | bit(a_e.simd_gt(b_s) & b_e.simd_gt(a_s), INTERSECTS);
    let slot = (((signature << 4u32) - signature) >> 2u32) & 15u64;
    (u64x4::splat(simd, HASHED_CODES) >> (slot << 2u32)) & 15u64
}

/// The portable kernel's formula for one pair.
fn code_of(a_s: u64, a_e: u64, b_s: u64, b_e: u64) -> u8 {
    let bit = |predicate: bool, weight: u64| u64::from(predicate) * weight;
    let signature = bit(a_s == b_s, START_EQ)
        | bit(a_s > b_s, START_GT)
        | bit(a_e == b_e, END_EQ)
        | bit(a_e > b_e, END_GT)
        | bit(a_e == b_s || b_e == a_s, ADJACENT)
        | bit(a_e > b_s && b_e > a_s, INTERSECTS);
    ((HASHED_CODES >> (hash(signature) << 2)) & 15) as u8
}

#[inline(always)]
fn portable_keep<S: Simd>(simd: S, codes: &[u8], mask_bits: u16, keep: &mut [u8]) {
    let table = u8x16::from_fn(simd, |code| u8::from((mask_bits >> code) & 1 != 0));
    let (chunks, tail) = codes.as_chunks::<16>();
    let (keep_chunks, keep_tail) = keep.as_chunks_mut::<16>();
    for (chunk, keep) in chunks.iter().zip(keep_chunks) {
        *keep = table
            .swizzle_dyn(u8x16::load_array(simd, *chunk))
            .to_array();
    }
    for (keep, &code) in keep_tail.iter_mut().zip(tail) {
        *keep = u8::from((mask_bits >> code) & 1 != 0);
    }
}
