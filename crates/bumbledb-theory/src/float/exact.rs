//! Exact, FPU-independent binary64 subtraction with one rounding.
//!
//! Every finite binary64 value is an integer multiple of 2⁻¹⁰⁷⁴, so the
//! difference of two finite values is exactly representable as a signed
//! integer at that scale; this module computes it in integer arithmetic
//! and rounds once, nearest with ties to even. The host thread's floating
//! rounding mode, FTZ/DAZ and FPCR flush controls cannot influence the
//! result: no floating instruction executes here. `Interval<F64>::length`
//! is the consumer; the engine's guarded scalar kernels are a separate,
//! differentially tested implementation of the same specification.
use super::{EXPONENT, F64, FRACTION, SIGN};

/// One finite operand as sign, exact integer mantissa and true exponent:
/// `value = (-1)^sign × mantissa × 2^exponent`, with `exponent` on the
/// shared 2⁻¹⁰⁷⁴-anchored scale (subnormals and the minimum normal align).
#[derive(Clone, Copy)]
struct Unpacked {
    negative: bool,
    mantissa: u64,
    exponent: i32,
}

fn unpack(value: F64) -> Unpacked {
    let bits = value.to_bits();
    let biased = ((bits & EXPONENT) >> 52) as i32;
    let fraction = bits & FRACTION;
    let (mantissa, exponent) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), biased - 1075)
    };
    Unpacked {
        negative: bits & SIGN != 0,
        mantissa,
        exponent,
    }
}

/// Packs a rounded magnitude back into canonical bits. `mantissa` is the
/// 53-bit (or subnormal) integer significand for `value = mantissa × 2^scale`.
/// Rounding overflow past the finite range returns the signed infinity —
/// the caller decides whether that is a value or a measure overflow.
fn pack(negative: bool, mantissa: u64, scale: i32) -> F64 {
    if mantissa == 0 {
        // The quotient domain has one zero; a negative exact zero cannot
        // arise from rounding a nonzero magnitude.
        return F64::ZERO;
    }
    debug_assert!(mantissa < (1 << 53));
    let biased = if mantissa < (1 << 52) {
        debug_assert_eq!(scale, -1074, "only the subnormal scale packs unnormalized");
        0
    } else {
        scale + 1075
    };
    if biased >= 2047 {
        return if negative {
            F64::NEG_INFINITY
        } else {
            F64::INFINITY
        };
    }
    let biased = u64::try_from(biased).expect("a packed exponent is nonnegative");
    F64::from_bits((u64::from(negative) << 63) | (biased << 52) | (mantissa & FRACTION))
}

/// Exact binary64 difference `a - b` for finite operands, rounded once to
/// the canonical domain (nearest, ties to even). A magnitude past the
/// finite range rounds to the signed infinity, exactly as the guarded
/// hardware subtraction would; the caller distinguishes value overflow
/// from measure overflow. Implemented entirely in integer arithmetic.
///
/// # Panics
/// If either operand is NaN or infinite (a programmer invariant — the
/// interval constructors and callers admit finite endpoints only).
#[must_use]
pub(crate) fn sub_rounded(a: F64, b: F64) -> F64 {
    assert!(a.is_finite() && b.is_finite(), "sub_rounded is finite-only");
    let lhs = unpack(a);
    let rhs = unpack(b);
    // a - b = a + (-b): flip the subtrahend's sign, then add magnitudes.
    add_magnitudes(
        lhs,
        Unpacked {
            negative: !rhs.negative,
            ..rhs
        },
    )
}

/// The magnitude order of two unpacked finite values, ignoring sign.
/// Exponent-then-mantissa comparison is exact because both mantissas of
/// one exponent class share the same scale.
fn magnitude_less(a: Unpacked, b: Unpacked) -> bool {
    // Compare as exact integers a.mantissa × 2^a.exponent. Both mantissas
    // are < 2^53; normalize by comparing (top-bit position + exponent).
    let key = |x: Unpacked| {
        if x.mantissa == 0 {
            i64::MIN
        } else {
            i64::from(x.exponent) + i64::from(x.mantissa.ilog2())
        }
    };
    match key(a).cmp(&key(b)) {
        core::cmp::Ordering::Less => true,
        core::cmp::Ordering::Greater => false,
        core::cmp::Ordering::Equal => {
            if a.mantissa == 0 {
                return false;
            }
            // Same leading-bit scale: align mantissas by exponent difference
            // (at most 52 here) and compare exactly.
            let shift = (a.exponent - b.exponent).unsigned_abs();
            if a.exponent >= b.exponent {
                (u128::from(a.mantissa) << shift) < u128::from(b.mantissa)
            } else {
                u128::from(a.mantissa) < (u128::from(b.mantissa) << shift)
            }
        }
    }
}

/// GUARD is the fixed left-shift of the dominant operand inside the u128
/// accumulator: 64 guard bits below the mantissa hold every alignment
/// shift up to the sticky horizon exactly.
const GUARD: u32 = 64;

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "shift amounts are proved < 128 and mantissas < 2^53 by construction"
)]
fn add_magnitudes(a: Unpacked, b: Unpacked) -> F64 {
    // Order the operands so `high` has the larger magnitude; the sum's
    // sign is high's sign, and effective subtraction never borrows.
    let (high, low) = if magnitude_less(a, b) { (b, a) } else { (a, b) };
    if high.mantissa == 0 {
        return F64::ZERO;
    }
    let effective_subtract = high.negative != low.negative;

    let acc_high = u128::from(high.mantissa) << GUARD;
    let delta = high.exponent - low.exponent; // ≥ 0 up to leading-bit ties
    debug_assert!(delta >= -52, "magnitude order bounds the exponent gap");
    let (acc_low, sticky) = if delta <= 0 {
        // Low has the smaller magnitude at a finer or equal scale only when
        // the leading-bit keys tied; the left shift stays within 53+64+52 bits.
        (u128::from(low.mantissa) << (GUARD as i32 - delta), false)
    } else {
        let delta = delta as u32;
        if delta <= GUARD {
            (u128::from(low.mantissa) << (GUARD - delta), false)
        } else if delta - GUARD < 64 {
            let shift = delta - GUARD;
            let kept = u128::from(low.mantissa >> shift);
            let lost = low.mantissa & ((1u64 << shift) - 1);
            (kept, lost != 0)
        } else {
            (0, low.mantissa != 0)
        }
    };

    let (magnitude, sticky) = if effective_subtract {
        if sticky {
            // The discarded tail is strictly positive: borrow one guard unit
            // and keep the sticky bit — the true value lies strictly between
            // the borrowed integer and its successor, so no exact tie exists.
            (acc_high - acc_low - 1, true)
        } else {
            (acc_high - acc_low, false)
        }
    } else {
        (acc_high + acc_low, sticky)
    };

    if magnitude == 0 {
        return F64::ZERO; // exact cancellation; sticky is false here
    }

    // The accumulator holds value = magnitude × 2^(scale_base) (+ sticky ulp
    // fraction), with scale_base anchored at high's exponent minus GUARD.
    let scale_base = high.exponent - GUARD as i32;
    let top = 127 - magnitude.leading_zeros() as i32;
    // Natural normalization places the leading bit at position 52; the
    // subnormal floor caps how far left the window may move.
    let natural_scale = scale_base + (top - 52);
    let target_scale = natural_scale.max(-1074);
    let shift = target_scale - scale_base;

    let (mantissa, round, sticky) = if shift <= 0 {
        debug_assert!(!sticky, "sticky implies a wide gap and a high leading bit");
        ((magnitude << (-shift) as u32) as u64, false, false)
    } else {
        let shift = shift as u32;
        debug_assert!(shift < 128);
        let kept = (magnitude >> shift) as u64;
        let round = (magnitude >> (shift - 1)) & 1 == 1;
        let below = magnitude & ((1u128 << (shift - 1)) - 1);
        (kept, round, below != 0 || sticky)
    };

    let mut mantissa = mantissa;
    let mut scale = target_scale;
    if round && (sticky || mantissa & 1 == 1) {
        mantissa += 1;
        if mantissa == 1 << 53 {
            mantissa >>= 1;
            scale += 1;
        }
        // A subnormal rounding into 2^52 becomes the minimum normal with the
        // same scale; `pack` reads that from the mantissa width directly.
    }
    pack(high.negative, mantissa, scale)
}

#[cfg(test)]
mod tests {
    use super::super::F64;
    use super::sub_rounded;

    fn f(bits: u64) -> F64 {
        F64::from_bits(bits)
    }

    fn of(value: f64) -> F64 {
        F64::from(value)
    }

    /// The integer implementation matches the host's default-mode IEEE
    /// subtraction bit for bit across structured operand classes. The host
    /// runs round-to-nearest-even here; this differential is an authored
    /// oracle for F-ARITH, independent of the engine's guarded kernels.
    #[test]
    fn differential_against_host_subtraction_over_structured_operands() {
        let atoms: &[u64] = &[
            0x0000_0000_0000_0000, // +0
            0x0000_0000_0000_0001, // min subnormal
            0x0000_0000_0000_0002,
            0x000f_ffff_ffff_ffff, // max subnormal
            0x0010_0000_0000_0000, // min normal
            0x0010_0000_0000_0001,
            0x3c00_0000_0000_0000, // 2^-63
            0x3fe0_0000_0000_0000, // 0.5
            0x3ff0_0000_0000_0000, // 1.0
            0x3ff0_0000_0000_0001, // nextUp(1.0)
            0x4000_0000_0000_0000, // 2.0
            0x4340_0000_0000_0000, // 2^53
            0x4340_0000_0000_0001,
            0x7fe0_0000_0000_0000, // 2^1023
            0x7fef_ffff_ffff_ffff, // MAX_FINITE
            0x4197_d784_0000_0000, // 1e8
            0x44b5_2d02_c7e1_4af6, // 1e23
        ];
        let mut checked = 0u32;
        for &ab in atoms {
            for &bb in atoms {
                for (sa, sb) in [(0u64, 0u64), (1, 0), (0, 1), (1, 1)] {
                    let a = f(ab | (sa << 63));
                    let b = f(bb | (sb << 63));
                    let expected = F64::from(a.to_f64() - b.to_f64());
                    assert_eq!(
                        sub_rounded(a, b),
                        expected,
                        "a={ab:#018x} sa={sa} b={bb:#018x} sb={sb}"
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(
            checked,
            u32::try_from(atoms.len() * atoms.len() * 4).unwrap()
        );
    }

    #[test]
    fn differential_against_host_subtraction_over_random_finite_pairs() {
        // SplitMix64-driven random finite pairs; nonfinite draws are skipped.
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let mut checked = 0;
        while checked < 4096 {
            let a = F64::from_bits(next());
            let b = F64::from_bits(next());
            if !a.is_finite() || !b.is_finite() {
                continue;
            }
            let expected = F64::from(a.to_f64() - b.to_f64());
            assert_eq!(sub_rounded(a, b), expected, "a={a:?} b={b:?}");
            checked += 1;
        }
    }

    #[test]
    fn golden_exact_and_rounded_differences() {
        // Exact small integers.
        assert_eq!(sub_rounded(of(1.5), of(0.5)), of(1.0));
        assert_eq!(sub_rounded(of(0.5), of(1.5)), of(-1.0));
        // Equal operands cancel to the canonical +0, including -x - -x.
        assert_eq!(sub_rounded(of(-3.25), of(-3.25)), F64::ZERO);
        assert_eq!(sub_rounded(F64::MAX_FINITE, F64::MAX_FINITE), F64::ZERO);
        // Signed-zero arithmetic collapses into the quotient's one zero.
        assert_eq!(sub_rounded(F64::ZERO, F64::ZERO), F64::ZERO);
        // Catastrophic cancellation is exact: nextUp(1) - 1 = 2^-52.
        assert_eq!(
            sub_rounded(f(0x3ff0_0000_0000_0001), of(1.0)),
            f(0x3cb0_0000_0000_0000)
        );
        // The 1e16-scale case that distinguishes rounded native addition.
        assert_eq!(sub_rounded(of(1e16_f64), of(-1.0)), of(1e16_f64 + 1.0));
        // Subnormal spacing survives: 2·min_sub - min_sub = min_sub.
        assert_eq!(
            sub_rounded(f(0x0000_0000_0000_0002), f(0x0000_0000_0000_0001)),
            F64::MIN_POSITIVE_SUBNORMAL
        );
        // Finite overflow rounds to the signed infinity.
        assert_eq!(sub_rounded(F64::MAX_FINITE, F64::MIN_FINITE), F64::INFINITY);
        assert_eq!(
            sub_rounded(F64::MIN_FINITE, F64::MAX_FINITE),
            F64::NEG_INFINITY
        );
    }

    #[test]
    #[should_panic(expected = "finite-only")]
    fn nonfinite_operands_are_a_programmer_invariant() {
        let _ = sub_rounded(F64::INFINITY, F64::ZERO);
    }
}
