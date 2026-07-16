//! The hand-NEON residue: the configuration kernel trio, the kernels
//! the portable_simd experiment REFUSED (the crucible packet (git ecec1dc3)
//! 03-portable-simd.md — the verdict matrix records the arbitrating
//! evidence; the filter/fold/gather kernels adopted `std::simd` and
//! left this module).

use std::arch::aarch64::{
    uint64x2_t, vandq_u64, vceqq_u64, vcgtq_u64, vdupq_n_u64, vld1q_u8, vld1q_u64, vorrq_u64,
    vst1q_u8,
};

/// The configuration kernel's 64-byte signature → basic-code nibble
/// table (held in q registers via `tbl` — the Allen decision tree as
/// in-register data). The 6-bit signature packs the 8 predicate lanes:
///
/// - bit 0: `a.s == b.s` — bit 1: `a.s > b.s`
/// - bit 2: `a.e == b.e` — bit 3: `a.e > b.e`
/// - bit 4: `a.e == b.s  OR  b.e == a.s` (the meets-type adjacency)
/// - bit 5: `a.e > b.s  AND  b.e > a.s` (strict nonempty intersection)
///
/// Strict nonemptiness (`start < end`, the [`crate::Interval`] parse)
/// admits exactly 13 valid signatures; every other index is
/// unreachable and filled with `0xFF` (past the mask table's range, so
/// a table bug drops rows in the bit-identity tests instead of passing
/// silently). The entries are the [`crate::allen::Basic`]
/// discriminants — the property tests cross-check this table against
/// PRD 03's `classify` decision tree, bit for bit.
const ALLEN_SIG_TABLE: [u8; 64] = {
    let mut table = [0xFFu8; 64];
    table[0b00_0000] = 0; // before:        a.e < b.s, no adjacency
    table[0b01_0000] = 1; // meets:         a.e == b.s
    table[0b10_0000] = 2; // overlaps:      s <, e <, strict ∩
    table[0b10_0001] = 3; // starts:        s ==, e <
    table[0b10_0010] = 4; // during:        s >, e <
    table[0b10_0110] = 5; // finishes:      s >, e ==
    table[0b10_0101] = 6; // equals:        s ==, e ==
    table[0b10_0100] = 7; // finished-by:   s <, e ==
    table[0b10_1000] = 8; // contains:      s <, e >
    table[0b10_1001] = 9; // started-by:    s ==, e >
    table[0b10_1010] = 10; // overlapped-by: s >, e >, strict ∩
    table[0b01_1010] = 11; // met-by:        b.e == a.s
    table[0b00_1010] = 12; // after:         b.e < a.s, no adjacency
    table
};

/// Two lanes' signatures from the 8 predicate lanes (4 `cmhi`/`cmeq`
/// pairs over the endpoint words), packed by masked constant bits.
#[expect(
    clippy::inline_always,
    reason = "measured kernel inlining is machine-checked and load-bearing"
)]
// the window loops exist to keep this
// arithmetic in registers; an outlined call per two lanes would spill it
#[inline(always)]
unsafe fn allen_sig2(
    a_s: uint64x2_t,
    a_e: uint64x2_t,
    b_s: uint64x2_t,
    b_e: uint64x2_t,
) -> uint64x2_t {
    // SAFETY (caller's contract): NEON-only lane arithmetic.
    unsafe {
        let bit = |m: uint64x2_t, w: u64| vandq_u64(m, vdupq_n_u64(w));
        let s_eq = bit(vceqq_u64(a_s, b_s), 1);
        let s_gt = bit(vcgtq_u64(a_s, b_s), 2);
        let e_eq = bit(vceqq_u64(a_e, b_e), 4);
        let e_gt = bit(vcgtq_u64(a_e, b_e), 8);
        let adjacent = bit(vorrq_u64(vceqq_u64(a_e, b_s), vceqq_u64(b_e, a_s)), 16);
        let intersects = bit(vandq_u64(vcgtq_u64(a_e, b_s), vcgtq_u64(b_e, a_s)), 32);
        vorrq_u64(
            vorrq_u64(vorrq_u64(s_eq, s_gt), vorrq_u64(e_eq, e_gt)),
            vorrq_u64(adjacent, intersects),
        )
    }
}

/// One 8-pair window: 4×2 signature lanes narrowed to 8 index bytes,
/// mapped through the 64-byte table in q registers via `tbl`, stored as
/// 8 code bytes.
#[inline(always)]
unsafe fn allen_code_window(
    table: std::arch::aarch64::uint8x16x4_t,
    load_b: impl Fn(usize) -> (uint64x2_t, uint64x2_t),
    a_s: *const u64,
    a_e: *const u64,
    codes: *mut u8,
) {
    // SAFETY (caller's contract): all four streams hold ≥ 8 words at
    // the given pointers; `codes` holds ≥ 8 bytes.
    unsafe {
        use std::arch::aarch64::{vcombine_u16, vcombine_u32, vmovn_u16, vmovn_u32, vmovn_u64};
        let sig = |lane: usize| {
            let (b_s, b_e) = load_b(lane);
            allen_sig2(vld1q_u64(a_s.add(lane)), vld1q_u64(a_e.add(lane)), b_s, b_e)
        };
        let (s0, s1, s2, s3) = (sig(0), sig(2), sig(4), sig(6));
        let lo = vmovn_u32(vcombine_u32(vmovn_u64(s0), vmovn_u64(s1)));
        let hi = vmovn_u32(vcombine_u32(vmovn_u64(s2), vmovn_u64(s3)));
        let indices = vmovn_u16(vcombine_u16(lo, hi));
        let mapped = std::arch::aarch64::vqtbl4_u8(table, indices);
        std::arch::aarch64::vst1_u8(codes, mapped);
    }
}

/// The 64-byte nibble table, loaded into four q registers.
#[expect(
    clippy::inline_always,
    reason = "measured kernel inlining is machine-checked and load-bearing"
)] // as `allen_sig2`
#[inline(always)]
unsafe fn allen_table() -> std::arch::aarch64::uint8x16x4_t {
    // SAFETY (caller's contract): four 16-byte loads within the 64-byte
    // table.
    unsafe {
        std::arch::aarch64::uint8x16x4_t(
            vld1q_u8(ALLEN_SIG_TABLE.as_ptr()),
            vld1q_u8(ALLEN_SIG_TABLE.as_ptr().add(16)),
            vld1q_u8(ALLEN_SIG_TABLE.as_ptr().add(32)),
            vld1q_u8(ALLEN_SIG_TABLE.as_ptr().add(48)),
        )
    }
}

/// The configuration code kernel over four endpoint streams (`super::
/// allen_code_batch`'s NEON core; the dispatch guarantees `len ≥ 8`).
/// The tail is the overlapped last window — codes are idempotent per
/// position, so re-classifying up to 7 pairs is free of both branches
/// and a scalar tail — and the loops are countdown-shaped so no `cmp`
/// reaches the back edge: this symbol is the asm gate's flag-free
/// subject (`scripts/check-asm.sh`), never inlined away.
#[inline(never)]
pub(super) fn allen_code_batch_neon(
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut [u8],
) {
    let n = codes.len();
    debug_assert!(n >= 8, "the dispatch owns the small-batch fallback");
    debug_assert!(
        a_starts.len() == n && a_ends.len() == n && b_starts.len() == n && b_ends.len() == n
    );
    // SAFETY: every window reads 8 words from within the four n-length
    // streams and writes 8 bytes into `codes` — full windows at k*8 with
    // k*8+8 <= n, plus one overlapped window at n-8 (n >= 8).
    unsafe {
        let (a_s, a_e) = (a_starts.as_ptr(), a_ends.as_ptr());
        let (b_s, b_e) = (b_starts.as_ptr(), b_ends.as_ptr());
        let out = codes.as_mut_ptr();
        let table = allen_table();
        let mut left = n / 8;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;
            allen_code_window(
                table,
                |lane| {
                    (
                        vld1q_u64(b_s.add(base + lane)),
                        vld1q_u64(b_e.add(base + lane)),
                    )
                },
                a_s.add(base),
                a_e.add(base),
                out.add(base),
            );
            base += 8;
        }
        let tail = n - 8;
        allen_code_window(
            table,
            |lane| {
                (
                    vld1q_u64(b_s.add(tail + lane)),
                    vld1q_u64(b_e.add(tail + lane)),
                )
            },
            a_s.add(tail),
            a_e.add(tail),
            out.add(tail),
        );
    }
}

/// [`allen_code_batch_neon`] with a broadcast constant right operand —
/// the filter-position shape (per-atom `Allen` against a literal/param
/// interval). Same window walk, the b-side lanes `dup`ed once; a gated
/// flag-free symbol like its sibling.
#[inline(never)]
pub(super) fn allen_code_batch_const_neon(
    starts: &[u64],
    ends: &[u64],
    b_start: u64,
    b_end: u64,
    codes: &mut [u8],
) {
    let n = codes.len();
    debug_assert!(n >= 8, "the dispatch owns the small-batch fallback");
    debug_assert!(starts.len() == n && ends.len() == n);
    // SAFETY: as `allen_code_batch_neon`, with the b side broadcast.
    unsafe {
        let (a_s, a_e) = (starts.as_ptr(), ends.as_ptr());
        let out = codes.as_mut_ptr();
        let table = allen_table();
        let (b_s, b_e) = (vdupq_n_u64(b_start), vdupq_n_u64(b_end));
        let mut left = n / 8;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;
            allen_code_window(
                table,
                |_| (b_s, b_e),
                a_s.add(base),
                a_e.add(base),
                out.add(base),
            );
            base += 8;
        }
        let tail = n - 8;
        allen_code_window(
            table,
            |_| (b_s, b_e),
            a_s.add(tail),
            a_e.add(tail),
            out.add(tail),
        );
    }
}

/// The membership kernel (`super::allen_filter_batch`'s NEON core; the
/// dispatch guarantees `len ≥ 16`): the mask's 13 per-code keep bits
/// expand once into a 16-byte table — **the mask broadcast in a vector
/// register for the whole batch** — and every 16 codes map through one
/// `tbl1` to their keep bytes. Overlapped tail, countdown loop: the
/// asm gate's second flag-free subject.
#[inline(never)]
pub(super) fn allen_filter_batch_neon(codes: &[u8], mask_bits: u16, keep: &mut [u8]) {
    let n = codes.len();
    debug_assert!(n >= 16, "the dispatch owns the small-batch fallback");
    debug_assert_eq!(keep.len(), n);
    // The broadcast mask table: byte c is code c's keep bit (1/0);
    // indices 13..=15 are unreachable codes and keep nothing. The
    // expansion is a fixed 13-step shift-and-mask — fully unrolled,
    // flag-free.
    let mut table = [0u8; 16];
    let mut code = 0usize;
    while code < 13 {
        table[code] = ((mask_bits >> code) & 1) as u8;
        code += 1;
    }
    // SAFETY: every window reads 16 bytes from within `codes` and
    // writes 16 within `keep` — full windows plus one overlapped window
    // at n-16 (n >= 16); keep bytes are idempotent per position. The
    // countdown passes through an empty register-pinned `asm!` identity
    // so LLVM keeps the flag-free `sub`+`cbnz` back edge instead of
    // re-deriving a `cmp`-shaped trip count while unrolling (the gate
    // is the machine code — `scripts/check-asm.sh`). `black_box` would
    // do the same opaquing but routes the counter through a stack slot
    // — a spill+reload per 16-code window, 2 extra memory µops in a
    // 5-µop payload (m2max.core.scalar-memory-rename: the renamed
    // round trip is bimodal, medians ~4.8 cy); the empty asm keeps the
    // counter in its register and emits zero instructions.
    unsafe {
        use std::arch::aarch64::vqtbl1q_u8;
        let mask_table = vld1q_u8(table.as_ptr());
        let src = codes.as_ptr();
        let dst = keep.as_mut_ptr();
        let mut left = n / 16;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;
            // The opaque back edge: an empty asm block whose only
            // effect is that `left`'s value is no longer known to LLVM.
            std::arch::asm!(
                "/* {c} */",
                c = inout(reg) left,
                options(nomem, nostack, preserves_flags)
            );
            vst1q_u8(
                dst.add(base),
                vqtbl1q_u8(mask_table, vld1q_u8(src.add(base))),
            );
            base += 16;
        }
        let tail = n - 16;
        vst1q_u8(
            dst.add(tail),
            vqtbl1q_u8(mask_table, vld1q_u8(src.add(tail))),
        );
    }
}

/// The T7 falsifier's arm A: [`allen_filter_batch_neon`] verbatim as it
/// shipped before the counter-spill fix, its countdown routed through
/// `std::hint::black_box` — which LLVM materializes as a stack
/// spill+reload of the counter per 16-code window (`str x,[sp,#8]` /
/// `ldr x,[sp,#8]` inside the 5-µop loop). Test-only: it exists so the
/// `allen_filter_counter_spill_ab` timing pin can interleave the two
/// back-edge shapes inside one process forever.
#[cfg(test)]
#[inline(never)]
pub(super) fn allen_filter_batch_neon_spill_arm(codes: &[u8], mask_bits: u16, keep: &mut [u8]) {
    let n = codes.len();
    debug_assert!(n >= 16, "the dispatch owns the small-batch fallback");
    debug_assert_eq!(keep.len(), n);
    let mut table = [0u8; 16];
    let mut code = 0usize;
    while code < 13 {
        table[code] = ((mask_bits >> code) & 1) as u8;
        code += 1;
    }
    // SAFETY: as `allen_filter_batch_neon` — same windows, same
    // overlapped tail; only the countdown's opaquing differs.
    unsafe {
        use std::arch::aarch64::vqtbl1q_u8;
        let mask_table = vld1q_u8(table.as_ptr());
        let src = codes.as_ptr();
        let dst = keep.as_mut_ptr();
        let mut left = n / 16;
        let mut base = 0usize;
        while left != 0 {
            left = std::hint::black_box(left - 1);
            vst1q_u8(
                dst.add(base),
                vqtbl1q_u8(mask_table, vld1q_u8(src.add(base))),
            );
            base += 16;
        }
        let tail = n - 16;
        vst1q_u8(
            dst.add(tail),
            vqtbl1q_u8(mask_table, vld1q_u8(src.add(tail))),
        );
    }
}
