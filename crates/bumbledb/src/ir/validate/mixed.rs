//! Exact integer ↔ F64 comparison against a literal, rewritten into the
//! variable's own type with integer-only arithmetic on the binary64 fields.
//! The rewrite is exact: no rounding through host floating point.
use crate::ir::{Value, WordCmp};
use bumbledb_theory::F64;
use bumbledb_theory::schema::ValueType;

/// One comparison `var op literal` restated over the variable's own type.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Rewritten {
    /// Every value of the variable satisfies it.
    Always,
    /// No value of the variable satisfies it.
    Never,
    Compare(WordCmp, Value),
}

/// Restate `var op literal` where exactly one side is F64 and the other an
/// integer type. `None` when the pair is not an integer/F64 mix.
pub(super) fn rewrite(var: ValueType, op: WordCmp, literal: &Value) -> Option<Rewritten> {
    match (var, literal) {
        (ValueType::U64 | ValueType::I64, Value::F64(value)) => {
            Some(integer_against_float(var, op, *value))
        }
        (ValueType::F64, Value::U64(value)) => Some(float_against_integer(op, i128::from(*value))),
        (ValueType::F64, Value::I64(value)) => Some(float_against_integer(op, i128::from(*value))),
        _ => None,
    }
}

/// The comparison no value of `var` satisfies: below the bottom of its
/// domain (`-Infinity` is the bottom of the F64 order).
pub(super) fn never(var: ValueType) -> (WordCmp, Value) {
    let bottom = match var {
        ValueType::U64 => Value::U64(0),
        ValueType::I64 => Value::I64(i64::MIN),
        ValueType::F64 => Value::F64(F64::NEG_INFINITY),
        _ => unreachable!("mixed comparisons are numeric"),
    };
    (WordCmp::Lt, bottom)
}

/// A binary64 value on the integer line: its floor and whether it is
/// integral. Magnitudes past `2^70` saturate; they lie outside every integer
/// domain either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Real {
    NegInf,
    Finite { floor: i128, integral: bool },
    PosInf,
    NaN,
}

const SATURATE: i128 = 1 << 70;

fn real(value: F64) -> Real {
    if value.is_nan() {
        return Real::NaN;
    }
    if value.is_infinite() {
        return if value.to_bits() >> 63 == 0 {
            Real::PosInf
        } else {
            Real::NegInf
        };
    }
    let bits = value.to_bits();
    let negative = bits >> 63 == 1;
    let exponent = i32::try_from((bits >> 52) & 0x7ff).expect("eleven bits");
    let fraction = bits & ((1 << 52) - 1);
    let (mantissa, scale) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), exponent - 1075)
    };
    let (whole, fractional) = if scale >= 0 {
        if scale > 64 {
            (SATURATE, false)
        } else {
            ((i128::from(mantissa) << scale).min(SATURATE), false)
        }
    } else {
        let shift = scale.unsigned_abs();
        if shift >= 64 {
            (0, mantissa != 0)
        } else {
            (
                i128::from(mantissa >> shift),
                mantissa & ((1 << shift) - 1) != 0,
            )
        }
    };
    let floor = match (negative, fractional) {
        (false, _) => whole,
        (true, false) => -whole,
        (true, true) => -whole - 1,
    };
    Real::Finite {
        floor,
        integral: !fractional,
    }
}

fn domain(var: ValueType) -> (i128, i128) {
    match var {
        ValueType::U64 => (0, i128::from(u64::MAX)),
        ValueType::I64 => (i128::from(i64::MIN), i128::from(i64::MAX)),
        _ => unreachable!("integer domains only"),
    }
}

fn integer_value(var: ValueType, value: i128) -> Value {
    match var {
        ValueType::U64 => Value::U64(u64::try_from(value).expect("within the U64 domain")),
        ValueType::I64 => Value::I64(i64::try_from(value).expect("within the I64 domain")),
        _ => unreachable!("integer domains only"),
    }
}

/// `x op f` for an integer `x`: `x < f ⟺ x <= ceil(f) - 1`,
/// `x <= f ⟺ x <= floor(f)`, `x > f ⟺ x >= floor(f) + 1`,
/// `x >= f ⟺ x >= ceil(f)`; equality needs an integral `f`.
fn integer_against_float(var: ValueType, op: WordCmp, value: F64) -> Rewritten {
    let (lo, hi) = domain(var);
    let at_most = |bound: i128| {
        if bound < lo {
            Rewritten::Never
        } else if bound >= hi {
            Rewritten::Always
        } else {
            Rewritten::Compare(WordCmp::Le, integer_value(var, bound))
        }
    };
    let at_least = |bound: i128| {
        if bound > hi {
            Rewritten::Never
        } else if bound <= lo {
            Rewritten::Always
        } else {
            Rewritten::Compare(WordCmp::Ge, integer_value(var, bound))
        }
    };
    match (real(value), op) {
        (Real::NaN, WordCmp::Ne)
        | (Real::PosInf, WordCmp::Lt | WordCmp::Le | WordCmp::Ne)
        | (Real::NegInf, WordCmp::Gt | WordCmp::Ge | WordCmp::Ne) => Rewritten::Always,
        (Real::NaN | Real::PosInf | Real::NegInf, _) => Rewritten::Never,
        (Real::Finite { floor, integral }, op) => {
            let ceil = if integral { floor } else { floor + 1 };
            let exact = integral && (lo..=hi).contains(&floor);
            match op {
                WordCmp::Lt => at_most(ceil - 1),
                WordCmp::Le => at_most(floor),
                WordCmp::Gt => at_least(floor + 1),
                WordCmp::Ge => at_least(ceil),
                WordCmp::Eq if exact => Rewritten::Compare(WordCmp::Eq, integer_value(var, floor)),
                WordCmp::Eq => Rewritten::Never,
                WordCmp::Ne if exact => Rewritten::Compare(WordCmp::Ne, integer_value(var, floor)),
                WordCmp::Ne => Rewritten::Always,
            }
        }
    }
}

/// The binary64 neighbours of an integer: `(below, above)` with
/// `below <= n <= above`, equal exactly when `n` is representable.
fn neighbours(n: i128) -> (F64, F64) {
    let magnitude = n.unsigned_abs();
    if magnitude == 0 {
        return (F64::ZERO, F64::ZERO);
    }
    let width = 128 - magnitude.leading_zeros();
    let (truncated, lost) = if width > 53 {
        let shift = width - 53;
        (magnitude >> shift, magnitude & ((1 << shift) - 1) != 0)
    } else {
        (magnitude, false)
    };
    let toward_zero = magnitude_to_f64(truncated, width);
    let away = if lost {
        magnitude_to_f64(truncated + 1, width)
    } else {
        toward_zero
    };
    if n > 0 {
        (toward_zero, away)
    } else {
        (negate(away), negate(toward_zero))
    }
}

/// `mantissa × 2^(width - 53)` (or the plain integer when `width <= 53`) as
/// exact binary64 bits; a carry to 2^53 bumps the exponent.
fn magnitude_to_f64(mantissa: u128, width: u32) -> F64 {
    let (mantissa, width) = if mantissa == 1 << 53 && width > 53 {
        (1u128 << 52, width + 1)
    } else {
        (mantissa, width)
    };
    let exponent = u64::from(width - 1 + 1023);
    let normalized = if width > 53 {
        mantissa
    } else {
        mantissa << (53 - width)
    };
    let fraction = u64::try_from(normalized).expect("53 bits") & ((1 << 52) - 1);
    F64::from_bits((exponent << 52) | fraction)
}

fn negate(value: F64) -> F64 {
    F64::from_bits(value.to_bits() ^ (1 << 63))
}

/// `x op n` for an F64 `x`: when `n` lies strictly between binary64
/// neighbours `below < n < above`, `x < n ⟺ x <= below` and
/// `x > n ⟺ x >= above`, and no F64 equals `n`.
fn float_against_integer(op: WordCmp, n: i128) -> Rewritten {
    let (below, above) = neighbours(n);
    if below == above {
        return Rewritten::Compare(op, Value::F64(below));
    }
    match op {
        WordCmp::Lt | WordCmp::Le => Rewritten::Compare(WordCmp::Le, Value::F64(below)),
        WordCmp::Gt | WordCmp::Ge => Rewritten::Compare(WordCmp::Ge, Value::F64(above)),
        WordCmp::Eq => Rewritten::Never,
        WordCmp::Ne => Rewritten::Always,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(var: ValueType, op: WordCmp, value: f64) -> Rewritten {
        integer_against_float(var, op, F64::from(value))
    }

    #[test]
    fn integer_columns_round_fractional_bounds_exactly() {
        use Rewritten::{Always, Compare, Never};
        let i = ValueType::I64;
        assert_eq!(
            int(i, WordCmp::Lt, 2.5),
            Compare(WordCmp::Le, Value::I64(2))
        );
        assert_eq!(
            int(i, WordCmp::Le, 2.5),
            Compare(WordCmp::Le, Value::I64(2))
        );
        assert_eq!(
            int(i, WordCmp::Gt, 2.5),
            Compare(WordCmp::Ge, Value::I64(3))
        );
        assert_eq!(
            int(i, WordCmp::Ge, 2.5),
            Compare(WordCmp::Ge, Value::I64(3))
        );
        assert_eq!(
            int(i, WordCmp::Lt, -2.5),
            Compare(WordCmp::Le, Value::I64(-3))
        );
        assert_eq!(
            int(i, WordCmp::Ge, -2.5),
            Compare(WordCmp::Ge, Value::I64(-2))
        );
        assert_eq!(
            int(i, WordCmp::Lt, 3.0),
            Compare(WordCmp::Le, Value::I64(2))
        );
        assert_eq!(int(i, WordCmp::Eq, 2.5), Never);
        assert_eq!(int(i, WordCmp::Ne, 2.5), Always);
        assert_eq!(
            int(i, WordCmp::Eq, -0.0),
            Compare(WordCmp::Eq, Value::I64(0))
        );
        assert_eq!(
            int(i, WordCmp::Lt, 1e-300),
            Compare(WordCmp::Le, Value::I64(0))
        );
        assert_eq!(
            int(i, WordCmp::Gt, -1e-300),
            Compare(WordCmp::Ge, Value::I64(0))
        );
    }

    #[test]
    fn integer_columns_saturate_at_their_domain_and_refuse_nan_orders() {
        use Rewritten::{Always, Compare, Never};
        let u = ValueType::U64;
        assert_eq!(
            int(u, WordCmp::Lt, 0.5),
            Compare(WordCmp::Le, Value::U64(0))
        );
        assert_eq!(int(u, WordCmp::Lt, 0.0), Never);
        assert_eq!(int(u, WordCmp::Ge, -0.5), Always);
        assert_eq!(int(u, WordCmp::Le, 1.9e19), Always);
        assert_eq!(int(u, WordCmp::Gt, 1.9e19), Never);
        assert_eq!(int(u, WordCmp::Eq, 2f64.powi(64)), Never);
        for op in [
            WordCmp::Lt,
            WordCmp::Le,
            WordCmp::Gt,
            WordCmp::Ge,
            WordCmp::Eq,
        ] {
            assert_eq!(int(u, op, f64::NAN), Never);
        }
        assert_eq!(int(u, WordCmp::Ne, f64::NAN), Always);
        assert_eq!(int(u, WordCmp::Lt, f64::INFINITY), Always);
        assert_eq!(int(u, WordCmp::Gt, f64::INFINITY), Never);
        assert_eq!(int(u, WordCmp::Gt, f64::NEG_INFINITY), Always);
        assert_eq!(int(u, WordCmp::Le, f64::NEG_INFINITY), Never);
        assert_eq!(int(ValueType::I64, WordCmp::Ge, f64::MAX), Never);
        assert_eq!(int(ValueType::I64, WordCmp::Ge, -f64::MAX), Always);
    }

    #[test]
    fn float_columns_compare_against_the_binary64_neighbours_of_an_integer() {
        use Rewritten::{Always, Compare, Never};
        let two53 = 1i128 << 53;
        let f = |v: f64| Value::F64(F64::from(v));
        assert_eq!(
            float_against_integer(WordCmp::Lt, two53),
            Compare(WordCmp::Lt, f(9_007_199_254_740_992.0))
        );
        assert_eq!(
            float_against_integer(WordCmp::Lt, two53 + 1),
            Compare(WordCmp::Le, f(9_007_199_254_740_992.0))
        );
        assert_eq!(
            float_against_integer(WordCmp::Gt, two53 + 1),
            Compare(WordCmp::Ge, f(9_007_199_254_740_994.0))
        );
        assert_eq!(float_against_integer(WordCmp::Eq, two53 + 1), Never);
        assert_eq!(float_against_integer(WordCmp::Ne, two53 + 1), Always);
        assert_eq!(
            float_against_integer(WordCmp::Eq, two53 - 1),
            Compare(WordCmp::Eq, f(9_007_199_254_740_991.0))
        );
        assert_eq!(
            float_against_integer(WordCmp::Gt, -(two53 + 1)),
            Compare(WordCmp::Ge, f(-9_007_199_254_740_992.0))
        );
        assert_eq!(
            float_against_integer(WordCmp::Lt, -(two53 + 1)),
            Compare(WordCmp::Le, f(-9_007_199_254_740_994.0))
        );
        assert_eq!(
            float_against_integer(WordCmp::Le, i128::from(u64::MAX)),
            Compare(WordCmp::Le, f(18_446_744_073_709_549_568.0))
        );
        assert_eq!(
            float_against_integer(WordCmp::Ge, i128::from(u64::MAX)),
            Compare(WordCmp::Ge, f(18_446_744_073_709_551_616.0))
        );
        assert_eq!(
            float_against_integer(WordCmp::Eq, i128::from(i64::MIN)),
            Compare(WordCmp::Eq, f(-9_223_372_036_854_775_808.0))
        );
        assert_eq!(
            float_against_integer(WordCmp::Eq, 0),
            Compare(WordCmp::Eq, f(0.0))
        );
    }

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "host conversions are the independent oracle"
    )]
    fn neighbours_bracket_every_integer_and_match_host_conversion_when_exact() {
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..10_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            for n in [
                i128::from(state),
                -i128::from(state >> 1),
                i128::from(state >> (state % 64)),
            ] {
                let (below, above) = neighbours(n);
                assert!(below <= above);
                let exact = (n as f64) as i128 == n;
                if exact {
                    assert_eq!(below, F64::from(n as f64), "{n}");
                    assert_eq!(below, above);
                } else {
                    assert!(below.to_f64() < above.to_f64(), "{n}");
                    assert!(
                        (below.to_f64() as i128) < n && n < (above.to_f64() as i128),
                        "{n}"
                    );
                }
            }
        }
    }
}
