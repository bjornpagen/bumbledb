use std::arch::aarch64::{
    uint64x2_t, vandq_u64, vceqq_u64, vceqq_u8, vcgeq_u64, vcgtq_u64, vcleq_u64, vdupq_n_u64,
    vdupq_n_u8, vgetq_lane_u64, vld1q_u64, vld1q_u8, vorrq_u64, vst1q_u8, vsubq_u64,
};

pub(super) fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
    let start = out.len();
    out.resize(start + col.len(), 0);
    let mut write = start;
    // SAFETY: every vld1q_u64 reads exactly two u64s from within
    // `chunks_exact(2)` of `col`; unaligned loads are legal for vld1q.
    unsafe {
        let needle = vdupq_n_u64(value);
        let chunks = col.chunks_exact(2);
        let tail_start = col.len() - chunks.remainder().len();
        for (chunk_idx, chunk) in chunks.enumerate() {
            let lanes = vld1q_u64(chunk.as_ptr());
            let mask = vceqq_u64(lanes, needle);
            let base = u32::try_from(chunk_idx * 2).expect("positions fit u32");
            out[write] = base;
            write += usize::from(vgetq_lane_u64(mask, 0) != 0);
            out[write] = base + 1;
            write += usize::from(vgetq_lane_u64(mask, 1) != 0);
        }
        for (i, item) in col[tail_start..].iter().enumerate() {
            out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
            write += usize::from(*item == value);
        }
    }
    out.truncate(write);
}

pub(super) fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
    let start = out.len();
    out.resize(start + col.len(), 0);
    let mut write = start;
    // SAFETY: as in `filter_eq_u64` — two-lane loads within bounds.
    unsafe {
        let lo_v = vdupq_n_u64(lo);
        let hi_v = vdupq_n_u64(hi);
        let chunks = col.chunks_exact(2);
        let tail_start = col.len() - chunks.remainder().len();
        for (chunk_idx, chunk) in chunks.enumerate() {
            let lanes = vld1q_u64(chunk.as_ptr());
            let ge = vcgeq_u64(lanes, lo_v);
            let le = vcleq_u64(lanes, hi_v);
            let base = u32::try_from(chunk_idx * 2).expect("positions fit u32");
            out[write] = base;
            write += usize::from(vgetq_lane_u64(ge, 0) != 0 && vgetq_lane_u64(le, 0) != 0);
            out[write] = base + 1;
            write += usize::from(vgetq_lane_u64(ge, 1) != 0 && vgetq_lane_u64(le, 1) != 0);
        }
        for (i, item) in col[tail_start..].iter().enumerate() {
            out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
            write += usize::from((lo..=hi).contains(item));
        }
    }
    out.truncate(write);
}

/// The shared two-column compare-and-mask pass (docs/architecture/
/// 40-execution.md, § access paths — interval predicates lower to word
/// comparisons over the start/end column pair; the fixed 8-byte shape,
/// no new NEON width): `mask` maps a (starts, ends) lane pair to the
/// combined survivor mask; the branchless writes are the predicate-scan
/// pattern verbatim, and `tail` is its scalar twin over the remainder.
#[expect(
    clippy::inline_always,
    reason = "measured kernel inlining is machine-checked and load-bearing"
)]
// the fused pass exists to keep the mask
// closures from becoming outlined calls
// per two-lane chunk (the
// instruction-diet law)
#[inline(always)]
unsafe fn filter_pair_u64(
    starts: &[u64],
    ends: &[u64],
    out: &mut Vec<u32>,
    mask: impl Fn(uint64x2_t, uint64x2_t) -> uint64x2_t,
    tail: impl Fn(u64, u64) -> bool,
) {
    debug_assert_eq!(starts.len(), ends.len());
    let base = out.len();
    out.resize(base + starts.len(), 0);
    let mut write = base;
    // SAFETY (caller's contract): every vld1q_u64 reads exactly two u64s
    // from within `chunks_exact(2)` of equal-length columns; unaligned
    // loads are legal for vld1q.
    unsafe {
        let chunks = starts.chunks_exact(2);
        let tail_start = starts.len() - chunks.remainder().len();
        for (chunk_idx, chunk) in chunks.enumerate() {
            let s = vld1q_u64(chunk.as_ptr());
            let e = vld1q_u64(ends.as_ptr().add(chunk_idx * 2));
            let m = mask(s, e);
            let out_base = u32::try_from(chunk_idx * 2).expect("positions fit u32");
            out[write] = out_base;
            write += usize::from(vgetq_lane_u64(m, 0) != 0);
            out[write] = out_base + 1;
            write += usize::from(vgetq_lane_u64(m, 1) != 0);
        }
        for i in tail_start..starts.len() {
            out[write] = u32::try_from(i).expect("positions fit u32");
            write += usize::from(tail(starts[i], ends[i]));
        }
    }
    out.truncate(write);
}

/// The measure scan (the one gather+subtract shape, 20-query-ir § the
/// measure): NEON on the dense stride-1 column pair — `vsubq_u64` runs on the vector pipes, so
/// the subtraction competes with no flag port (the port-topology law) —
/// with the ray test fused as one more lane compare. The ray branch is
/// predicted never-taken; on a hit the scalar re-scan of the two-lane
/// chunk names the first offending position in scan order.
pub(super) fn filter_duration_range_u64(
    starts: &[u64],
    ends: &[u64],
    lo: u64,
    hi: u64,
    out: &mut Vec<u32>,
) -> Result<(), usize> {
    let base = out.len();
    out.resize(base + starts.len(), 0);
    let mut write = base;
    // SAFETY: every vld1q_u64 reads exactly two u64s from within
    // `chunks_exact(2)` of equal-length columns; unaligned loads are
    // legal for vld1q.
    unsafe {
        let lo_v = vdupq_n_u64(lo);
        let hi_v = vdupq_n_u64(hi);
        let inf = vdupq_n_u64(u64::MAX);
        let chunks = starts.chunks_exact(2);
        let tail_start = starts.len() - chunks.remainder().len();
        for (chunk_idx, chunk) in chunks.enumerate() {
            let s = vld1q_u64(chunk.as_ptr());
            let e = vld1q_u64(ends.as_ptr().add(chunk_idx * 2));
            let ray = vceqq_u64(e, inf);
            if (vgetq_lane_u64(ray, 0) | vgetq_lane_u64(ray, 1)) != 0 {
                return Err(chunk_idx * 2 + usize::from(vgetq_lane_u64(ray, 0) == 0));
            }
            let duration = vsubq_u64(e, s);
            let ge = vcgeq_u64(duration, lo_v);
            let le = vcleq_u64(duration, hi_v);
            let out_base = u32::try_from(chunk_idx * 2).expect("positions fit u32");
            out[write] = out_base;
            write += usize::from(vgetq_lane_u64(ge, 0) != 0 && vgetq_lane_u64(le, 0) != 0);
            out[write] = out_base + 1;
            write += usize::from(vgetq_lane_u64(ge, 1) != 0 && vgetq_lane_u64(le, 1) != 0);
        }
        for i in tail_start..starts.len() {
            if ends[i] == u64::MAX {
                return Err(i);
            }
            let duration = ends[i] - starts[i];
            out[write] = u32::try_from(i).expect("positions fit u32");
            write += usize::from((lo..=hi).contains(&duration));
        }
    }
    out.truncate(write);
    Ok(())
}

pub(super) fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
    // SAFETY: equal-length columns, chunked loads in bounds — the
    // `filter_pair_u64` contract.
    unsafe {
        let p = vdupq_n_u64(point);
        filter_pair_u64(
            starts,
            ends,
            out,
            // `start <= p` AND `p < end` — the half-open membership rule
            // as two predicate-scan masks ANDed.
            |s, e| vandq_u64(vcleq_u64(s, p), vcgtq_u64(e, p)),
            |s, e| s <= point && point < e,
        );
    }
}

pub(super) fn filter_any_point_in_u64(
    starts: &[u64],
    ends: &[u64],
    points: &[u64],
    out: &mut Vec<u32>,
) {
    // SAFETY: as in `filter_point_in_u64`; the OR accumulates per-point
    // membership masks (k small — docs/architecture/20-query-ir.md,
    // § param sets).
    unsafe {
        filter_pair_u64(
            starts,
            ends,
            out,
            |s, e| {
                let mut any = vdupq_n_u64(0);
                for &point in points {
                    let p = vdupq_n_u64(point);
                    any = vorrq_u64(any, vandq_u64(vcleq_u64(s, p), vcgtq_u64(e, p)));
                }
                any
            },
            |s, e| points.iter().any(|p| s <= *p && *p < e),
        );
    }
}

// The old interval-vs-constant comparison kernels lived here; they left
// with their operators — interval-pair predicates are Allen masks now,
// evaluated by the configuration kernel below (`super::allen` hosts the
// dispatch and the doctrine).

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
    debug_assert!(keep.len() == n);
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
    // countdown passes through `black_box` so LLVM keeps the
    // flag-free `sub`+`cbnz` back edge instead of re-deriving a
    // `cmp`-shaped trip count while unrolling (the gate is the machine
    // code — `scripts/check-asm.sh`).
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

/// Dense exact-u128 sum via carry counting: four
/// 2-lane accumulators take wrapping `vaddq_u64` adds while a
/// parallel counter lane counts carries — unsigned overflow iff
/// `new < old`, i.e. `vcgtq_u64(old, new)` all-ones, subtracted to
/// count +1. Total = Σ lane lo + (Σ lane carries << 64): exact, and
/// bit-identical to any-association i128/u128 folding. Flag ports
/// untouched — the scalar exact loop's `adds/adcs` are confined to
/// 3 of 6 ALUs, which was the whole wall (measured 8.8 vs 4.0–4.6
/// rows/ns at L1).
pub(super) fn fold_sum_u64_dense(values: &[u64]) -> u128 {
    // SAFETY: every vld1q_u64 reads exactly two u64s from within
    // `chunks_exact(8)` of `values`.
    unsafe {
        use std::arch::aarch64::{vaddq_u64, vcgtq_u64, vsubq_u64};
        let mut lows = [vdupq_n_u64(0); 4];
        let mut carries = [vdupq_n_u64(0); 4];
        let chunks = values.chunks_exact(8);
        let tail = chunks.remainder();
        for chunk in chunks {
            for lane in 0..4 {
                let v = vld1q_u64(chunk.as_ptr().add(lane * 2));
                let new = vaddq_u64(lows[lane], v);
                // Overflowed lanes read all-ones; subtracting adds 1.
                let carry = vcgtq_u64(lows[lane], new);
                carries[lane] = vsubq_u64(carries[lane], carry);
                lows[lane] = new;
            }
        }
        let mut total: u128 = 0;
        for lane in 0..4 {
            for half in 0..2 {
                let lo = if half == 0 {
                    vgetq_lane_u64(lows[lane], 0)
                } else {
                    vgetq_lane_u64(lows[lane], 1)
                };
                let carry = if half == 0 {
                    vgetq_lane_u64(carries[lane], 0)
                } else {
                    vgetq_lane_u64(carries[lane], 1)
                };
                total += u128::from(lo) + (u128::from(carry) << 64);
            }
        }
        for &v in tail {
            total += u128::from(v);
        }
        total
    }
}

/// Dense (min, max) over a contiguous u64 slice: compare-select
/// lanes (`vcgtq_u64` + `vbslq_u64` — NEON has no 64-bit lane
/// min/max), four vector accumulators to break the dependency
/// chains, scalar tail.
pub(super) fn fold_min_max_u64_dense(values: &[u64]) -> (u64, u64) {
    let mut min_scalar = u64::MAX;
    let mut max_scalar = u64::MIN;
    // SAFETY: every vld1q_u64 reads exactly two u64s from within
    // `chunks_exact(8)` of `values`.
    unsafe {
        use std::arch::aarch64::{vbslq_u64, vcgtq_u64};
        let mut mins = [vdupq_n_u64(u64::MAX); 4];
        let mut maxs = [vdupq_n_u64(u64::MIN); 4];
        let chunks = values.chunks_exact(8);
        let tail = chunks.remainder();
        for chunk in chunks {
            for lane in 0..4 {
                let v = vld1q_u64(chunk.as_ptr().add(lane * 2));
                mins[lane] = vbslq_u64(vcgtq_u64(mins[lane], v), v, mins[lane]);
                maxs[lane] = vbslq_u64(vcgtq_u64(v, maxs[lane]), v, maxs[lane]);
            }
        }
        for lane in 0..4 {
            min_scalar = min_scalar
                .min(vgetq_lane_u64(mins[lane], 0))
                .min(vgetq_lane_u64(mins[lane], 1));
            max_scalar = max_scalar
                .max(vgetq_lane_u64(maxs[lane], 0))
                .max(vgetq_lane_u64(maxs[lane], 1));
        }
        for &v in tail {
            min_scalar = min_scalar.min(v);
            max_scalar = max_scalar.max(v);
        }
    }
    (min_scalar, max_scalar)
}

pub(super) fn filter_eq_u8(col: &[u8], value: u8, out: &mut Vec<u32>) {
    let start = out.len();
    out.resize(start + col.len(), 0);
    let mut write = start;
    // SAFETY: every vld1q_u8 reads exactly sixteen bytes from within
    // `chunks_exact(16)`; the mask store writes a stack array.
    unsafe {
        let needle = vdupq_n_u8(value);
        let chunks = col.chunks_exact(16);
        let tail_start = col.len() - chunks.remainder().len();
        let mut mask_bytes = [0u8; 16];
        for (chunk_idx, chunk) in chunks.enumerate() {
            let lanes = vld1q_u8(chunk.as_ptr());
            let mask = vceqq_u8(lanes, needle);
            vst1q_u8(mask_bytes.as_mut_ptr(), mask);
            let base = chunk_idx * 16;
            for (lane, m) in mask_bytes.iter().enumerate() {
                out[write] = u32::try_from(base + lane).expect("positions fit u32");
                write += usize::from(*m != 0);
            }
        }
        for (i, item) in col[tail_start..].iter().enumerate() {
            out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
            write += usize::from(*item == value);
        }
    }
    out.truncate(write);
}
