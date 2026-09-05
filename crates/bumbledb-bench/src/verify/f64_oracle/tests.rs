//! P11-authored oracle tests (gates `F-CANON`, `F-ORDER`, `F-ARITH`,
//! `F-AGG`; audit ASS-001/ASS-002 assurance routing). Authored in F1,
//! executed only after the F3 barrier.
//!
//! Decimal spellings are always PAIRED with exact bit patterns so a shared
//! parser cannot mask an arithmetic bug; where host `f64` appears it is the
//! differential SUBJECT, never the expected-value source.

use super::{
    Acc, Class, INF, MAX_FINITE, NAN, NEG_INF, SIGN, Total, Wide, canonical, classify, fold,
    is_canonical, mean_bits, order_key, ref_add, ref_div, ref_mul, ref_neg, ref_sub, round_dyadic,
    scaled_magnitude, sum_bits,
};

// Golden payloads, hardcoded AND cross-checked against host literals below.
const ONE: u64 = 0x3ff0_0000_0000_0000;
const TWO: u64 = 0x4000_0000_0000_0000;
const TEN_POW_16: u64 = 0x4341_c379_37e0_8000;
const MIN_SUBNORMAL: u64 = 0x0000_0000_0000_0001;
const NEG_ZERO: u64 = 0x8000_0000_0000_0000;

#[test]
fn golden_bits_match_host_literals() {
    // The pairing rule: decimal spelling AND exact bits, together.
    assert_eq!(1.0f64.to_bits(), ONE);
    assert_eq!(2.0f64.to_bits(), TWO);
    assert_eq!(1e16f64.to_bits(), TEN_POW_16);
    assert_eq!(f64::MAX.to_bits(), MAX_FINITE);
    assert_eq!(f64::INFINITY.to_bits(), INF);
    assert_eq!(f64::NEG_INFINITY.to_bits(), NEG_INF);
    assert_eq!((-0.0f64).to_bits(), NEG_ZERO);
}

#[test]
fn canonicalization_collapses_zeros_and_nans() {
    assert_eq!(canonical(NEG_ZERO), 0, "negative zero collapses");
    assert_eq!(canonical(0), 0);
    // Every NaN class: quiet/signaling payloads, both signs.
    for nan_bits in [
        NAN,
        0x7ff0_0000_0000_0001, // signaling, positive
        0x7fff_ffff_ffff_ffff, // quiet payload, positive
        0xfff8_0000_0000_0000, // canonical pattern with sign
        0xfff0_0000_0000_0001, // signaling, negative
        0xffff_ffff_ffff_ffff, // all ones
    ] {
        assert_eq!(canonical(nan_bits), NAN, "{nan_bits:#x}");
    }
    // Finite values and infinities are fixed points.
    for keep in [
        0u64,
        MIN_SUBNORMAL,
        ONE,
        MAX_FINITE,
        INF,
        NEG_INF,
        SIGN | ONE,
    ] {
        assert_eq!(canonical(keep), keep, "{keep:#x}");
        assert!(is_canonical(keep));
    }
    // Idempotence over a structured sweep of all field classes.
    for exponent in [0u64, 1, 2, 1022, 1023, 2045, 2046, 2047] {
        for fraction in [0u64, 1, 0xf_ffff_ffff_ffff, 0x8_0000_0000_0000] {
            for sign in [0u64, SIGN] {
                let bits = sign | (exponent << 52) | fraction;
                assert_eq!(canonical(canonical(bits)), canonical(bits));
            }
        }
    }
}

#[test]
fn order_key_total_order_matches_numeric_classes() {
    // -inf < -MAX < -min_sub < 0 < min_sub < 1 < MAX < +inf < NaN.
    let ascending = [
        NEG_INF,
        SIGN | MAX_FINITE,
        SIGN | ONE,
        SIGN | MIN_SUBNORMAL,
        0,
        MIN_SUBNORMAL,
        ONE,
        0x3ff0_0000_0000_0001, // nextUp(1.0)
        TWO,
        MAX_FINITE,
        INF,
        NAN,
    ];
    for pair in ascending.windows(2) {
        assert!(
            order_key(pair[0]) < order_key(pair[1]),
            "{:#x} < {:#x}",
            pair[0],
            pair[1]
        );
    }
    // The key is injective on the roster.
    for (i, &a) in ascending.iter().enumerate() {
        for &b in &ascending[i + 1..] {
            assert_ne!(order_key(a), order_key(b));
        }
    }
}

#[test]
fn scaled_magnitude_decomposition_is_exact() {
    assert!(scaled_magnitude(0).is_zero());
    assert_eq!(scaled_magnitude(MIN_SUBNORMAL), Wide::from_u64(1));
    // 1.0 = 2^52 * 2^1022 scaled units = 2^1074 * 2^-1074.
    assert_eq!(scaled_magnitude(ONE), Wide::from_u64(1).shl(1074));
    // MAX_FINITE = (2^53 - 1) * 2^2045 scaled units.
    assert_eq!(
        scaled_magnitude(MAX_FINITE),
        Wide::from_u64((1u64 << 53) - 1).shl(2045)
    );
}

#[test]
fn exact_sum_matches_rational_oracle() {
    // {1e16, 1, -1e16}: exact accumulation cancels to exactly 1.0 — the
    // fixture that refutes repeated native addition (which loses the 1:
    // 1e16 + 1 rounds back to 1e16).
    assert_eq!(sum_bits(&[TEN_POW_16, ONE, SIGN | TEN_POW_16]), ONE);
    let host = ((1e16f64 + 1.0) - 1e16f64).to_bits();
    assert_ne!(
        host, ONE,
        "the order-dependent native fold must actually lose the 1 for \
         this fixture to bite"
    );
    // {MAX, MAX}: sum overflows to +inf after ONE rounding.
    assert_eq!(sum_bits(&[MAX_FINITE, MAX_FINITE]), INF);
    // {MIN_SUBNORMAL, MIN_SUBNORMAL}: exact 2 scaled units.
    assert_eq!(sum_bits(&[MIN_SUBNORMAL, MIN_SUBNORMAL]), 2);
    // Exact zero total is canonical +0.
    assert_eq!(sum_bits(&[ONE, SIGN | ONE]), 0);
    // The fixed special-case table.
    assert_eq!(sum_bits(&[NAN, ONE]), NAN);
    assert_eq!(sum_bits(&[INF, NEG_INF]), NAN);
    assert_eq!(sum_bits(&[INF, ONE, TWO]), INF);
    assert_eq!(sum_bits(&[NEG_INF, ONE]), NEG_INF);
}

#[test]
fn sum_is_permutation_and_partition_independent() {
    let values = [
        TEN_POW_16,
        ONE,
        SIGN | TEN_POW_16,
        MIN_SUBNORMAL,
        SIGN | TWO,
        MAX_FINITE,
        SIGN | MAX_FINITE,
        0,
    ];
    let expected = sum_bits(&values);
    // Every rotation and the reversal.
    for rot in 0..values.len() {
        let mut rotated = values.to_vec();
        rotated.rotate_left(rot);
        assert_eq!(sum_bits(&rotated), expected, "rotation {rot}");
    }
    let mut reversed = values.to_vec();
    reversed.reverse();
    assert_eq!(sum_bits(&reversed), expected);
    // Every two-way partition point: fold the halves separately and merge.
    for split in 0..=values.len() {
        let (left, right) = values.split_at(split);
        let merged = fold(left).merge(&fold(right));
        assert_eq!(
            super::sum_bits(&values),
            match &merged.total {
                Total::Finite { neg, mag } => super::signed_bits(*neg, round_dyadic(mag, 1, 0)),
                Total::Nan => NAN,
                Total::PosInf => INF,
                Total::NegInf => NEG_INF,
            },
            "partition at {split}"
        );
        assert_eq!(merged.count, u64::try_from(values.len()).expect("small"));
    }
    assert_eq!(expected, sum_bits(&values));
}

#[test]
fn partial_state_replay_is_not_idempotent() {
    // Merging one finite partial state with itself doubles contribution and
    // count: the accumulator carries no binding provenance, so exact set
    // deduplication MUST precede accumulation (chapter 11 §4).
    let acc = fold(&[ONE, TWO]);
    let replayed = acc.merge(&acc);
    assert_ne!(replayed, acc);
    assert_eq!(replayed.count, 2 * acc.count);
    let Total::Finite { neg: false, mag } = &replayed.total else {
        panic!("finite replay stays finite");
    };
    let Total::Finite {
        neg: false,
        mag: once,
    } = &acc.total
    else {
        panic!("finite fold");
    };
    assert_eq!(*mag, once.mul_u64(2), "the contribution doubled exactly");
}

#[test]
fn mean_divides_exact_rational_not_rounded_sum() {
    // {MAX, MAX}: the once-rounded SUM is +inf, but the exact mean is
    // exactly MAX — dividing the rounded sum would answer +inf.
    assert_eq!(sum_bits(&[MAX_FINITE, MAX_FINITE]), INF);
    assert_eq!(mean_bits(&[MAX_FINITE, MAX_FINITE]), Some(MAX_FINITE));
    // {MIN_SUBNORMAL, MIN_SUBNORMAL}: mean is exactly one scaled unit.
    assert_eq!(
        mean_bits(&[MIN_SUBNORMAL, MIN_SUBNORMAL]),
        Some(MIN_SUBNORMAL)
    );
    // Empty input forms no group: no fabricated zero/NaN row.
    assert_eq!(mean_bits(&[]), None);
    // Nonfinite groups keep the table.
    assert_eq!(mean_bits(&[INF, NEG_INF]), Some(NAN));
    assert_eq!(mean_bits(&[NAN, ONE]), Some(NAN));
    // 1/3 in scaled units: the denominator enters the ONE rounding
    // exactly (no intermediate rounding exists to disagree with).
    assert_eq!(
        mean_bits(&[MIN_SUBNORMAL, 0, 0]),
        Some(0),
        "one scaled unit over three rounds to zero"
    );
    assert_eq!(
        mean_bits(&[2, 0, 0]),
        Some(MIN_SUBNORMAL),
        "two scaled units over three rounds to one unit"
    );
}

#[test]
fn ties_round_to_even_at_binade_boundaries() {
    // 2^53 + 1 scaled units ties between 2^53 and 2^53 + 2: even wins low.
    let exactly = |units: u64| round_dyadic(&Wide::from_u64(units), 1, 0);
    assert_eq!(exactly((1 << 53) + 1), exactly(1 << 53));
    // 2^53 + 3 ties between 2^53 + 2 and 2^53 + 4: even wins high.
    assert_eq!(exactly((1 << 53) + 3), exactly((1 << 53) + 4));
    // Representable magnitudes are fixed points.
    for units in [0u64, 1, 2, (1 << 52) - 1, 1 << 52, (1 << 53) - 1] {
        assert_eq!(exactly(units), units, "sub-2^53 grid identity");
    }
}

#[test]
fn reference_arithmetic_matches_host_on_goldens() {
    // F-ARITH differential: the oracle is the expected side; host f64 is
    // the qualified-hardware subject. Bitwise, never epsilon.
    let cases: &[(u64, u64)] = &[
        (ONE, TWO),
        (TEN_POW_16, ONE),
        (MAX_FINITE, MAX_FINITE),
        (MIN_SUBNORMAL, MIN_SUBNORMAL),
        (MIN_SUBNORMAL, ONE),
        (SIGN | ONE, ONE),
        (MAX_FINITE, MIN_SUBNORMAL),
        (TWO, 0x4008_0000_0000_0000), // 2.0, 3.0 — 2/3 is inexact
        (ONE, 0x4008_0000_0000_0000), // 1/3
        (0, ONE),
        (INF, ONE),
        (NEG_INF, INF),
        (NAN, ONE),
        (0, 0),
        (INF, 0),
    ];
    for &(a, b) in cases {
        let fa = f64::from_bits(a);
        let fb = f64::from_bits(b);
        assert_eq!(
            ref_add(a, b),
            canonical((fa + fb).to_bits()),
            "add {a:#x} {b:#x}"
        );
        assert_eq!(
            ref_sub(a, b),
            canonical((fa - fb).to_bits()),
            "sub {a:#x} {b:#x}"
        );
        assert_eq!(
            ref_mul(a, b),
            canonical((fa * fb).to_bits()),
            "mul {a:#x} {b:#x}"
        );
        assert_eq!(
            ref_div(a, b),
            canonical((fa / fb).to_bits()),
            "div {a:#x} {b:#x}"
        );
    }
    // Signed-zero collapse is the DELIBERATE quotient-domain divergence:
    // 1 / neg(0) is +Infinity here because -0 does not exist canonically.
    assert_eq!(ref_div(ONE, ref_neg(0)), INF);
    assert_eq!(ref_neg(0), 0);
}

#[test]
fn reference_arithmetic_ieee_special_table() {
    assert_eq!(ref_div(ONE, 0), INF, "1/0 = +inf");
    assert_eq!(ref_div(0, 0), NAN, "0/0 = NaN");
    assert_eq!(ref_sub(INF, INF), NAN, "inf - inf = NaN");
    assert_eq!(ref_mul(0, INF), NAN, "0 * inf = NaN");
    assert_eq!(ref_mul(MAX_FINITE, TWO), INF, "overflow to +inf");
    assert_eq!(ref_mul(SIGN | MAX_FINITE, TWO), NEG_INF, "overflow to -inf");
    assert_eq!(
        ref_div(MIN_SUBNORMAL, TWO),
        0,
        "half the smallest subnormal rounds to +0 (ties to even)"
    );
    assert_eq!(classify(ref_add(NAN, NAN)), Class::Nan);
}

#[test]
fn accumulator_count_tracks_cardinality_exactly() {
    let acc = fold(&[ONE, TWO, NAN, INF]);
    assert_eq!(acc.count, 4, "nonfinite members still count");
    assert_eq!(Acc::empty().count, 0);
    assert_eq!(Acc::empty().merge(&acc), acc, "empty is the identity");
}
