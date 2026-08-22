//! The configuration kernel: `Allen(mask)` over a batch of interval
//! pairs — branch-free, **flag-free**, table-driven; homogeneous
//! coordinates for time. One kernel pair serves every interval-pair
//! predicate that exists or ever will (8192 masks, one arithmetic): per
//! pair, 8 predicate lanes (`cmhi`/`cmeq` over the four endpoint words)
//! pack into a 6-bit signature; a 64-byte nibble table held in q

use bumbledb_theory::allen::AllenMask;

#[cfg(target_arch = "aarch64")]
use super::neon;
#[cfg(not(target_arch = "aarch64"))]
use super::reference;

#[cfg(target_arch = "aarch64")]
const CODE_LANES: usize = 8;

#[cfg(target_arch = "aarch64")]
const FILTER_LANES: usize = 16;

#[cfg(target_arch = "aarch64")]
fn classify_code(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> u8 {
    crate::allen::classify_bounds(&a_start, &a_end, &b_start, &b_end) as u8
}

/// Endpoint words — strided (whole columns) or gathered (per-survivor
/// scratch streams) — to 4-bit configuration codes: `codes[i]` is the
/// [`crate::allen::Basic`] discriminant of pair `i` (its bit index in
/// the mask coordinate system). `codes` is resized to the pair count
/// (capacity retained — pooled batch state); no `clear` first, so only
/// growth past the previous batch's count zero-fills — every byte of
/// the retained prefix is overwritten by the classify below (the full
/// per-batch refill was pure `_platform_memset` on the profile).
pub fn allen_code_batch(
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut Vec<u8>,
) {
    let n = a_starts.len();
    // Release-strength: the NEON core reads 8-word windows through raw

    // safety invariant — asserted here, outside the flag-free gated
    // symbols, like every sibling unsafe kernel's extent guard.
    assert_eq!(a_ends.len(), n, "four equal-length endpoint streams");
    assert_eq!(b_starts.len(), n, "four equal-length endpoint streams");
    assert_eq!(b_ends.len(), n, "four equal-length endpoint streams");
    codes.resize(n, 0);
    codes_into(a_starts, a_ends, b_starts, b_ends, codes);
}

/// residual's parent-constant side (`run_node`'s Allen pass: a
/// `Source::Slot` side reads the outer bindings, constant for the
/// whole call): the constant's two words broadcast into the b-side
/// predicate lanes, so the kernel streams two gathered arrays instead
/// of four (two of which would repeat one value per lane).
pub fn allen_code_batch_const(
    a_starts: &[u64],
    a_ends: &[u64],
    b_start: u64,
    b_end: u64,
    codes: &mut Vec<u8>,
) {
    let n = a_starts.len();
    // Release-strength, as `allen_code_batch`: the NEON windows read

    assert_eq!(a_ends.len(), n, "two equal-length endpoint streams");
    codes.resize(n, 0);
    codes_into_const(a_starts, a_ends, b_start, b_end, codes);
}

/// Configuration codes + the broadcast mask to keep bytes:
/// `keep[i] = 1` iff `(1 << codes[i]) & mask != 0` — the membership
/// test as a 16-byte `tbl` over the mask's per-code bit, broadcast once
/// per batch (literal or param alike). `keep` is resized to the code
/// count — like `codes` above, no `clear`: the membership test below
/// overwrites every retained byte, so only growth zero-fills;
/// survivors then feed the existing branchless cursor-write
/// ([`super::compact_u32_by_mask`], 1.00 cy/item).
pub fn allen_filter_batch(codes: &[u8], mask: AllenMask, keep: &mut Vec<u8>) {
    keep.resize(codes.len(), 0);
    keep_into(codes, mask, keep);
}

/// The dense filter-position composition (per-atom `Allen` between two
/// interval fields of one atom): stride-1 column pairs → surviving
/// positions, appended to `out` in ascending order like every filter
/// kernel. Chunked through stack scratch — codes, then the broadcast
/// mask's keep bytes, then the branchless cursor-write — so the view
/// path allocates nothing.
pub fn allen_filter_columns(
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    mask: AllenMask,
    out: &mut Vec<u32>,
) {
    filter_chunked(a_starts.len(), out, |base, len, codes| {
        codes_into(
            &a_starts[base..base + len],
            &a_ends[base..base + len],
            &b_starts[base..base + len],
            &b_ends[base..base + len],
            codes,
        );
        mask
    });
}

/// [`allen_filter_columns`] with a constant right operand (the per-atom
/// `Allen` against a literal/param interval — the filtered-view shape):
/// the constant's two words broadcast into the b-side predicate lanes.
pub fn allen_filter_columns_const(
    starts: &[u64],
    ends: &[u64],
    b_start: u64,
    b_end: u64,
    mask: AllenMask,
    out: &mut Vec<u32>,
) {
    filter_chunked(starts.len(), out, |base, len, codes| {
        codes_into_const(
            &starts[base..base + len],
            &ends[base..base + len],
            b_start,
            b_end,
            codes,
        );
        mask
    });
}

const SCAN_CHUNK: usize = 256;

#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
fn filter_chunked(
    n: usize,
    out: &mut Vec<u32>,
    fill: impl Fn(usize, usize, &mut [u8]) -> AllenMask,
) {
    let mut codes = [0u8; SCAN_CHUNK];
    let mut keep = [0u8; SCAN_CHUNK];
    let start = out.len();
    out.reserve(n);
    let mut write = start;
    let mut pos = super::filter::positions_fit_u32(n);
    let mut base = 0usize;
    while base < n {
        let len = SCAN_CHUNK.min(n - base);
        let mask = fill(base, len, &mut codes[..len]);
        keep_into(&codes[..len], mask, &mut keep[..len]);
        (write, pos) = super::filter::write_survivor_keeps(out, write, pos, &keep[..len]);
        base += len;
    }
    // SAFETY: every slot in `[start, write)` was cursor-written by the

    // capacity` (`u32` carries no drop obligation).
    unsafe { out.set_len(write) };
    crate::obs::event(
        crate::obs::names::KERNEL_ALLEN,
        crate::obs::TraceArgs::Pair(n as u64, (write - start) as u64),
    );
}

fn codes_into(
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut [u8],
) {
    #[cfg(target_arch = "aarch64")]
    {
        if codes.len() >= CODE_LANES {
            neon::allen_code_batch_neon(a_starts, a_ends, b_starts, b_ends, codes);
            return;
        }
        for (i, code) in codes.iter_mut().enumerate() {
            *code = classify_code(a_starts[i], a_ends[i], b_starts[i], b_ends[i]);
        }
    }
    #[cfg(not(target_arch = "aarch64"))]
    reference::allen_codes(a_starts, a_ends, b_starts, b_ends, codes);
}

fn codes_into_const(starts: &[u64], ends: &[u64], b_start: u64, b_end: u64, codes: &mut [u8]) {
    #[cfg(target_arch = "aarch64")]
    {
        if codes.len() >= CODE_LANES {
            neon::allen_code_batch_const_neon(starts, ends, b_start, b_end, codes);
            return;
        }
        for (i, code) in codes.iter_mut().enumerate() {
            *code = classify_code(starts[i], ends[i], b_start, b_end);
        }
    }
    #[cfg(not(target_arch = "aarch64"))]
    reference::allen_codes_const(starts, ends, b_start, b_end, codes);
}

fn keep_into(codes: &[u8], mask: AllenMask, keep: &mut [u8]) {
    #[cfg(target_arch = "aarch64")]
    {
        if codes.len() >= FILTER_LANES {
            neon::allen_filter_batch_neon(codes, mask.bits(), keep);
            return;
        }
        for (keep, &code) in keep.iter_mut().zip(codes) {
            *keep = ((mask.bits() >> u32::from(code)) & 1) as u8;
        }
    }
    #[cfg(not(target_arch = "aarch64"))]
    reference::allen_keep(codes, mask.bits(), keep);
}
