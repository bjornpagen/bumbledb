//! The aarch64 Allen specialization: compare endpoint lanes, pack their 6-bit
//! signatures, then map them through the 64-byte signature table held in four
//! q registers (`tbl4`). Windows read 8 pairs (16 codes for keep bytes); the
//! last window overlaps the previous one instead of running a scalar tail.
use std::arch::aarch64::{
    uint64x2_t, vandq_u64, vceqq_u64, vcgtq_u64, vdupq_n_u64, vld1q_u8, vld1q_u64, vorrq_u64,
    vst1q_u8,
};

use fearless_simd::aarch64::Neon;

use super::allen::SIGNATURE_CODES as ALLEN_SIG_TABLE;

/// Pairs per classification window; shorter batches take the portable path.
pub(super) const CODE_LANES: usize = 8;

/// Codes per keep window; shorter batches take the portable path.
pub(super) const FILTER_LANES: usize = 16;

#[expect(
    clippy::inline_always,
    reason = "measured kernel inlining is machine-checked and load-bearing"
)]
#[inline(always)]
unsafe fn allen_sig2(
    a_s: uint64x2_t,
    a_e: uint64x2_t,
    b_s: uint64x2_t,
    b_e: uint64x2_t,
) -> uint64x2_t {
    // SAFETY: lane arithmetic on registers only; NEON is the aarch64 baseline.
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

#[inline(always)]
unsafe fn allen_code_window(
    table: std::arch::aarch64::uint8x16x4_t,
    load_b: impl Fn(usize) -> (uint64x2_t, uint64x2_t),
    a_s: *const u64,
    a_e: *const u64,
    codes: *mut u8,
) {
    // SAFETY: the caller guarantees `a_s`, `a_e` and every `load_b` lane are
    // readable for 8 words and `codes` is writable for 8 bytes.
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

#[expect(
    clippy::inline_always,
    reason = "measured kernel inlining is machine-checked and load-bearing"
)]
#[inline(always)]
unsafe fn allen_table() -> std::arch::aarch64::uint8x16x4_t {
    // SAFETY: the four loads read the four 16-byte quarters of a 64-byte table.
    unsafe {
        std::arch::aarch64::uint8x16x4_t(
            vld1q_u8(ALLEN_SIG_TABLE.as_ptr()),
            vld1q_u8(ALLEN_SIG_TABLE.as_ptr().add(16)),
            vld1q_u8(ALLEN_SIG_TABLE.as_ptr().add(32)),
            vld1q_u8(ALLEN_SIG_TABLE.as_ptr().add(48)),
        )
    }
}

#[inline(never)]
pub(super) fn allen_code_batch_neon(
    _neon: Neon,
    a_starts: &[u64],
    a_ends: &[u64],
    b_starts: &[u64],
    b_ends: &[u64],
    codes: &mut [u8],
) {
    let n = codes.len();
    assert!(n >= CODE_LANES, "the dispatch owns short batches");
    assert!(
        a_starts.len() == n && a_ends.len() == n && b_starts.len() == n && b_ends.len() == n,
        "four equal-length endpoint streams"
    );
    // SAFETY: NEON is the aarch64 baseline (and `_neon` proves it). Window
    // `base` reads words `base..base + 8` of each stream and writes codes
    // `base..base + 8`: the loop runs `(n - 1) / 8` windows with
    // `base + 8 <= n - 1`, and the final window starts at `n - 8 >= 0`, so
    // every access lies inside the four asserted n-length streams and `codes`.
    unsafe {
        let (a_s, a_e) = (a_starts.as_ptr(), a_ends.as_ptr());
        let (b_s, b_e) = (b_starts.as_ptr(), b_ends.as_ptr());
        let out = codes.as_mut_ptr();
        let table = allen_table();
        let mut left = (n - 1) / 8;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;
            // An empty asm that owns the countdown keeps it in a register;
            // without it LLVM spills and reloads it in every window.
            std::arch::asm!(
                "/* {c} */",
                c = inout(reg) left,
                options(nomem, nostack, preserves_flags)
            );
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

#[inline(never)]
pub(super) fn allen_code_batch_const_neon(
    _neon: Neon,
    starts: &[u64],
    ends: &[u64],
    b_start: u64,
    b_end: u64,
    codes: &mut [u8],
) {
    let n = codes.len();
    assert!(n >= CODE_LANES, "the dispatch owns short batches");
    assert!(
        starts.len() == n && ends.len() == n,
        "two equal-length endpoint streams"
    );
    // SAFETY: as `allen_code_batch_neon`, with the b side broadcast.
    unsafe {
        let (a_s, a_e) = (starts.as_ptr(), ends.as_ptr());
        let out = codes.as_mut_ptr();
        let table = allen_table();
        let (b_s, b_e) = (vdupq_n_u64(b_start), vdupq_n_u64(b_end));
        let mut left = (n - 1) / 8;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;
            // An empty asm that owns the countdown keeps it in a register;
            // without it LLVM spills and reloads it in every window.
            std::arch::asm!(
                "/* {c} */",
                c = inout(reg) left,
                options(nomem, nostack, preserves_flags)
            );
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

#[inline(never)]
pub(super) fn allen_filter_batch_neon(_neon: Neon, codes: &[u8], mask_bits: u16, keep: &mut [u8]) {
    let n = codes.len();
    assert!(n >= FILTER_LANES, "the dispatch owns short batches");
    assert_eq!(keep.len(), n, "one keep byte per code");

    let mut table = [0u8; 16];
    let mut code = 0usize;
    while code < 13 {
        table[code] = ((mask_bits >> code) & 1) as u8;
        code += 1;
    }
    // SAFETY: NEON is the aarch64 baseline. Window `base` reads codes and
    // writes keep bytes `base..base + 16`: the loop's windows end at most at
    // `n - 1` and the final window starts at `n - 16 >= 0`, all inside the two
    // asserted n-length slices.
    unsafe {
        use std::arch::aarch64::vqtbl1q_u8;
        let mask_table = vld1q_u8(table.as_ptr());
        let src = codes.as_ptr();
        let dst = keep.as_mut_ptr();
        let mut left = (n - 1) / 16;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;
            // An empty asm that owns the countdown keeps it in a register;
            // without it LLVM spills and reloads it in every window.
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
