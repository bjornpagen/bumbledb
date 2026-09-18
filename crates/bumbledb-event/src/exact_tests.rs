use crate::{ArithmeticLimits, Capacity, Control, Error, ExactArithmetic, ExactRational};
use std::cell::Cell;

fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn ratio(n: i64, d: i64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}

#[test]
fn exact_arithmetic_matches_small_integer_cross_products() {
    let mut w = work();
    for a in -7..=7 {
        for b in 1..=7 {
            for c in -7..=7 {
                for d in 1..=7 {
                    let x = ratio(a, b);
                    let y = ratio(c, d);
                    assert_eq!(x.add(&y, &mut w).unwrap(), ratio(a * d + c * b, b * d));
                    assert_eq!(x.sub(&y, &mut w).unwrap(), ratio(a * d - c * b, b * d));
                    assert_eq!(x.mul(&y, &mut w).unwrap(), ratio(a * c, b * d));
                    if c != 0 {
                        assert_eq!(x.div(&y, &mut w).unwrap(), ratio(a * d, b * c));
                    } else {
                        assert_eq!(x.div(&y, &mut w), Err(Error::DivisionByZero));
                    }
                }
            }
        }
    }
    let giant = "12345678901234567890123456789012345678901234567890";
    assert_eq!(
        ExactRational::fraction(giant, giant, &mut w).unwrap(),
        ExactRational::one()
    );
}

#[test]
fn decimal_and_ieee_import_have_distinct_exact_meanings() {
    let mut w = work();
    assert_eq!(
        ExactRational::decimal("-12.50e-2", &mut w).unwrap(),
        ratio(-1, 8)
    );
    assert_eq!(
        ExactRational::decimal("+0.1000E2", &mut w).unwrap(),
        ratio(10, 1)
    );
    let decimal = ExactRational::decimal("0.1", &mut w).unwrap();
    let ieee = ExactRational::binary64(0.1, &mut w).unwrap();
    assert_eq!(decimal, ratio(1, 10));
    assert_eq!(ieee, ratio(3_602_879_701_896_397, 36_028_797_018_963_968));
    assert_ne!(decimal, ieee);
    for value in [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
    ] {
        let rational = ExactRational::binary64(value, &mut w).unwrap();
        let bytes = rational.to_bytes(&mut w).unwrap();
        assert_eq!(ExactRational::from_bytes(&bytes, &mut w).unwrap(), rational);
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            ExactRational::binary64(value, &mut w),
            Err(Error::InvalidRational)
        );
    }
    for value in ["", ".1", "1.", "1e", "1e2e3", "--1", "1/2", " 1", "1.2.3"] {
        assert_eq!(
            ExactRational::decimal(value, &mut w),
            Err(Error::InvalidRational),
            "{value}"
        );
    }
}

#[test]
fn rational_wire_is_canonical_bounded_and_rejects_malleable_forms() {
    let mut w = work();
    for n in -16..=16 {
        for d in 1..=16 {
            let value = ratio(n, d);
            let bytes = value.to_bytes(&mut w).unwrap();
            assert_eq!(ratio(n * 2, d * 2).to_bytes(&mut w).unwrap(), bytes);
            assert_eq!(ExactRational::from_bytes(&bytes, &mut w).unwrap(), value);
            for end in 0..bytes.len() {
                assert!(ExactRational::from_bytes(&bytes[..end], &mut w).is_err());
            }
            let mut extra = bytes.clone();
            extra.push(0);
            assert_eq!(
                ExactRational::from_bytes(&extra, &mut w),
                Err(Error::InvalidRational)
            );
        }
    }
    let mut one = ratio(1, 1).to_bytes(&mut w).unwrap();
    one[14] = 2;
    one[23] = 2; // unreduced 2/2
    assert_eq!(
        ExactRational::from_bytes(&one, &mut w),
        Err(Error::InvalidRational)
    );
    let mut zero = ExactRational::zero().to_bytes(&mut w).unwrap();
    zero[5] = 1;
    assert_eq!(
        ExactRational::from_bytes(&zero, &mut w),
        Err(Error::InvalidRational)
    );
    let mut one = ratio(1, 1).to_bytes(&mut w).unwrap();
    one[23] = 0;
    assert_eq!(
        ExactRational::from_bytes(&one, &mut w),
        Err(Error::InvalidRational)
    );
}

struct Stop(Cell<usize>);
impl Control for Stop {
    fn checkpoint(&self) -> crate::Result<()> {
        let n = self.0.get();
        self.0.set(n + 1);
        if n >= 1 {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[test]
fn limits_refuse_without_rounding_and_poll_after_bigint_work() {
    let mut tiny = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 1,
            operations: 100,
        },
        &(),
    );
    let one = ExactRational::binary64(1.0, &mut tiny).unwrap();
    assert_eq!(one, ExactRational::one());
    let bytes = one.to_bytes(&mut tiny).unwrap();
    assert_eq!(ExactRational::from_bytes(&bytes, &mut tiny).unwrap(), one);
    assert_eq!(
        ExactRational::binary64(2.0, &mut tiny),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    );
    assert_eq!(
        ExactRational::decimal("1e1000000000", &mut tiny),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    );
    assert_eq!(
        one.add(&one, &mut tiny),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    );
    let mut no_work = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 100,
            operations: 0,
        },
        &(),
    );
    assert_eq!(
        one.to_bytes(&mut no_work),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    );
    let stop = Stop(Cell::new(0));
    assert_eq!(
        ExactRational::binary64(
            1.0,
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &stop)
        ),
        Err(Error::Cancelled)
    );
}
