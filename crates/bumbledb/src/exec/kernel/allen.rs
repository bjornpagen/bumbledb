//! The configuration kernel (docs/architecture/40-execution.md, § the
//! sanctioned kernel shapes): `Allen(mask)` over a batch of interval
//! pairs — branch-free, **flag-free**, table-driven; homogeneous
//! coordinates for time. One kernel pair serves every interval-pair
//! predicate that exists or ever will (8192 masks, one arithmetic): per
//! pair, 8 predicate lanes (`cmhi`/`cmeq` over the four endpoint words)
//! pack into a 6-bit signature; a 64-byte nibble table held in q
//! registers maps signature → 4-bit basic code via `tbl` — the Allen
//! decision tree as in-register data: zero memory traffic, zero
//! branches, zero flags; membership is `(1 << code) & mask` with the
//! mask broadcast in a vector register for the whole batch. Uniform
//! cost for all 8192 masks — no per-mask codegen, no 13-way dispatch,
//! no indirect branch; mask-as-param is free by uniformity.
//!
//! The i64 sign-flip encoding makes every stored endpoint word
//! unsigned-order-faithful (docs/architecture/50-storage.md), so the
//! one unsigned kernel serves both element types, rays included: a
//! ray's `end == MAX` is just the largest word (the point-domain law).
//!
//! **The flag-free law is load-bearing, not style**
//! (`m2max.core.flag-port-asymmetry`, `m2max.core.flag-strand-mlp`): a
//! scalar `cmp`/`csel` classify carries 4–5 flag µops per pair — capped
//! at ~2.8 flag-µops/cycle on the 3-port triad dense, and **halving
//! sustainable miss lanes (~28 → ~14) when the pairs are gathered** at
//! DRAM tier. The NEON route keeps dependents on the vector schedulers
//! and preserves the lanes — `m2max.simd.minmax-universal`'s mechanism
//! (2.65×, port-arithmetic-predicted) applied to Allen. Zero scalar
//! flag µops exist in the kernel symbols, enforced structurally by
//! `scripts/check-asm.sh` on the release disassembly (`cmp`/`csel`/
//! `adds`/`ccmp` forbidden — the gate is the machine code, because LLVM
//! substitutes).
//!
//! **`tbl` vs the 256×u16 one-hot table is a sweep, not a doctrine**:
//! the measured alternative (512 B, permanently L1-hot, one pipelined
//! load per pair) trades `tbl` arithmetic for load-port pressure —
//! prefer `tbl` in filter position (load ports busy streaming columns),
//! the table load in residual position (gather context, load ports
//! idle). Shipped: `tbl` in both positions; the sweep waits for the
//! calendar family's numbers.
//!
//! NOTE (falsifier-shaped performance pins — recorded until the calendar family
//! earns numbers; no benchmark has run):
//! - *dense-uniform-within-2×-of-hand-`INTERSECTS` at L1*: the uniform
//!   kernel stays within 2× of a hand-written two-compare `INTERSECTS`
//!   loop over L1-resident columns, else the signature packing is fat;
//! - *gathered-within-15%-of-xor-gather-floor at DRAM*: a gathered
//!   Allen residual stays within 15% of the flag-free xor-gather floor,
//!   else a flag µop leaked into the miss shadow — read the
//!   disassembly.

use bumbledb_theory::allen::AllenMask;

#[cfg(target_arch = "aarch64")]
use super::neon;
#[cfg(not(target_arch = "aarch64"))]
use super::reference;

/// The NEON code kernel's window width in pairs (8 = one `tbl` of 8
/// narrowed signatures); shorter batches take the scalar classify.
#[cfg(target_arch = "aarch64")]
const CODE_LANES: usize = 8;

/// The NEON membership kernel's window width in codes (one q register);
/// shorter batches take the scalar bit test (itself flag-free: one
/// shift, one and).
#[cfg(target_arch = "aarch64")]
const FILTER_LANES: usize = 16;

/// One pair's configuration code — [`crate::allen::classify_bounds`]'s
/// decision tree, the scalar fallback below the NEON window width (the
/// [`super::reference`] module is absent in aarch64 non-test builds).
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
///
/// NOTE (bind-time mask simplification — a recorded lever, not
/// shipped): the workload composites collapse (`INTERSECTS` =
/// `a.s < b.e ∧ b.s < a.e` — two compares; `DISJOINT` its complement;
/// `COVERS` three), and on L2-resident retire-bound filters
/// (`m2max.mem.l2-resident-retire-bound`) a two-compare kernel beats
/// the uniform one on µop count alone. The structure, if earned, is
/// bind-time monomorphized selection (the sink-dispatch precedent — no
/// hot-loop indirection). *Trigger:* the calendar family showing
/// the filter phase owning enough of a family budget to buy it — pin
/// the fraction before building the lever
/// (`m2max.probe.pass-overhead`'s lesson).
pub fn allen_code_batch(
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut Vec<u8>,
) {
    let n = a_starts.len();
    debug_assert_eq!(a_ends.len(), n, "four equal-length endpoint streams");
    debug_assert_eq!(b_starts.len(), n, "four equal-length endpoint streams");
    debug_assert_eq!(b_ends.len(), n, "four equal-length endpoint streams");
    codes.resize(n, 0);
    codes_into(a_starts, a_ends, b_starts, b_ends, codes);
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

/// The dense scans' chunk width (stack scratch; well past both NEON
/// window widths so the overlap tails stay a per-chunk constant).
const SCAN_CHUNK: usize = 256;

/// The shared chunk walk of the two dense scans: `fill(base, len,
/// codes)` computes the chunk's codes and returns the (batch-constant)
/// mask; positions compact through the branchless cursor-write.
fn filter_chunked(
    n: usize,
    out: &mut Vec<u32>,
    fill: impl Fn(usize, usize, &mut [u8]) -> AllenMask,
) {
    let mut codes = [0u8; SCAN_CHUNK];
    let mut keep = [0u8; SCAN_CHUNK];
    let mut base = 0usize;
    while base < n {
        let len = SCAN_CHUNK.min(n - base);
        let mask = fill(base, len, &mut codes[..len]);
        keep_into(&codes[..len], mask, &mut keep[..len]);
        let start = out.len();
        out.resize(start + len, 0);
        let mut write = start;
        for (i, &keep) in keep[..len].iter().enumerate() {
            out[write] = u32::try_from(base + i).expect("positions fit u32");
            write += usize::from(keep != 0);
        }
        out.truncate(write);
        base += len;
    }
}

/// [`allen_code_batch`]'s core over pre-sized slices (the dense scans'
/// chunk form).
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

/// [`codes_into`] with the constant right operand.
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

/// [`allen_filter_batch`]'s core over pre-sized slices.
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
