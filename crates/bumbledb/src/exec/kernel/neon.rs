//! The hand-NEON residue: the configuration kernel trio, the kernels
//! evidence; the filter/fold/gather kernels adopted `std::simd` and
//! left this module).
use std::arch::aarch64::{
    uint64x2_t, vandq_u64, vceqq_u64, vcgtq_u64, vdupq_n_u64, vld1q_u8, vld1q_u64, vorrq_u64,
    vst1q_u8,
};

const ALLEN_SIG_TABLE: [u8; 64] = {
    let mut table = [0xFFu8; 64];
    table[0b00_0000] = 0;
    table[0b01_0000] = 1;
    table[0b10_0000] = 2;
    table[0b10_0001] = 3;
    table[0b10_0010] = 4;
    table[0b10_0110] = 5;
    table[0b10_0101] = 6;
    table[0b10_0100] = 7;
    table[0b10_1000] = 8;
    table[0b10_1001] = 9;
    table[0b10_1010] = 10;
    table[0b01_1010] = 11;
    table[0b00_1010] = 12;
    table
};

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

#[inline(always)]
unsafe fn allen_code_window(
    table: std::arch::aarch64::uint8x16x4_t,
    load_b: impl Fn(usize) -> (uint64x2_t, uint64x2_t),
    a_s: *const u64,
    a_e: *const u64,
    codes: *mut u8,
) {
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

    unsafe {
        let (a_s, a_e) = (a_starts.as_ptr(), a_ends.as_ptr());
        let (b_s, b_e) = (b_starts.as_ptr(), b_ends.as_ptr());
        let out = codes.as_mut_ptr();
        let table = allen_table();
        let mut left = (n - 1) / 8;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;

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
        let mut left = (n - 1) / 8;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;

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
pub(super) fn allen_filter_batch_neon(codes: &[u8], mask_bits: u16, keep: &mut [u8]) {
    let n = codes.len();
    debug_assert!(n >= 16, "the dispatch owns the small-batch fallback");
    debug_assert_eq!(keep.len(), n);

    let mut table = [0u8; 16];
    let mut code = 0usize;
    while code < 13 {
        table[code] = ((mask_bits >> code) & 1) as u8;
        code += 1;
    }
    // SAFETY: every window reads 16 bytes from within `codes` and

    unsafe {
        use std::arch::aarch64::vqtbl1q_u8;
        let mask_table = vld1q_u8(table.as_ptr());
        let src = codes.as_ptr();
        let dst = keep.as_mut_ptr();
        let mut left = (n - 1) / 16;
        let mut base = 0usize;
        while left != 0 {
            left -= 1;

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

/// The T7 falsifier's arm A: [`allen_filter_batch_neon`] verbatim as it shipped
/// before the counter-spill fix, its countdown routed through
/// `std::hint::black_box` — which LLVM materializes as a stack spill+reload of
/// the counter per 16-code window (`str x,[sp,#8]` / `ldr x,[sp,#8]` inside the
/// 5-µop loop).
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

    unsafe {
        use std::arch::aarch64::vqtbl1q_u8;
        let mask_table = vld1q_u8(table.as_ptr());
        let src = codes.as_ptr();
        let dst = keep.as_mut_ptr();
        let mut left = (n - 1) / 16;
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
