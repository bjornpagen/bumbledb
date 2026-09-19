use std::cell::Cell;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;

use crate::{
    ArithmeticLimits, BoolOp4, Capacity, Control, Error, ExactArithmetic, ExactPolynomial as Poly,
    ExactRational as Rat, GuardedRationalFunction, NumberLimits, NumberPredicate,
    NumberPredicateView, ParameterDomain, ParameterFunction, ParameterId, ParameterRegion,
    PartialNumber as Number, PolynomialSigns as Signs, RealWitness,
};

struct Stop(Cell<usize>);
impl Control for Stop {
    fn checkpoint(&self) -> crate::Result<()> {
        let n = self.0.get();
        self.0.set(n + 1);
        if n >= 2 {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

const P: ParameterId = ParameterId([249; 32]);
fn limits() -> NumberLimits {
    NumberLimits::default()
}
fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn rat(n: i64, d: i64) -> Rat {
    Rat::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn number(n: i64, d: i64) -> Number {
    Number::Fixed(Some(rat(n, d)))
}
fn unit(parameter: ParameterId) -> ParameterDomain {
    let p = Poly::parameter(parameter);
    let nonnegative = ParameterRegion::from_polynomial(
        parameter,
        &p,
        Signs::NON_NEGATIVE,
        limits().parameters,
        &mut work(),
    )
    .unwrap();
    let tail = Poly::one()
        .sub(&p, limits().parameters.polynomial, &mut work())
        .unwrap();
    let bounded = ParameterRegion::from_polynomial(
        parameter,
        &tail,
        Signs::NON_NEGATIVE,
        limits().parameters,
        &mut work(),
    )
    .unwrap();
    ParameterDomain::new(
        nonnegative
            .apply(BoolOp4::AND, &bounded, limits().parameters, &mut work())
            .unwrap(),
    )
    .unwrap()
}
fn function(domain: &ParameterDomain, numerator: Poly, denominator: Poly) -> Number {
    Number::Parameter(
        ParameterFunction::new(
            domain.clone(),
            &[GuardedRationalFunction::new(
                domain.clone(),
                numerator,
                denominator,
                limits().parameters,
                &mut work(),
            )
            .unwrap()],
            limits().parameters,
            limits().functions,
            &mut work(),
        )
        .unwrap(),
    )
}
fn at(value: &Number, n: i64, d: i64) -> Option<String> {
    match value {
        Number::Fixed(value) => value.as_ref().map(ToString::to_string),
        Number::Parameter(value) => value
            .value_at(
                &rat(n, d),
                limits().parameters,
                limits().functions,
                &mut work(),
            )
            .unwrap()
            .map(|v| v.to_string()),
    }
}
fn truth(value: &NumberPredicate, n: i64, d: i64) -> Option<bool> {
    match value.view() {
        NumberPredicateView::Fixed(value) => value,
        NumberPredicateView::Parameter {
            ambient,
            holds,
            fails,
            undefined,
        } => {
            let contains = |region: &ParameterRegion| {
                region
                    .contains(
                        &RealWitness::Rational(rat(n, d)),
                        limits().parameters,
                        &mut work(),
                    )
                    .unwrap()
            };
            let cells = [contains(holds), contains(fails), contains(undefined)];
            assert_eq!(
                cells.iter().filter(|v| **v).count(),
                usize::from(contains(ambient.region()))
            );
            if cells[0] {
                Some(true)
            } else if cells[1] {
                Some(false)
            } else {
                None
            }
        }
    }
}
fn oracle(n: i64, d: i64) -> BigRational {
    BigRational::new(BigInt::from(n), BigInt::from(d))
}

#[test]
fn fixed_arithmetic_matches_rational_oracle_and_keeps_undefined_values() {
    let values = [
        None,
        Some((-3, 2)),
        Some((-1, 1)),
        Some((0, 1)),
        Some((1, 3)),
        Some((2, 1)),
    ];
    for a in values {
        for b in values {
            let left = a.map_or(Number::Fixed(None), |(n, d)| number(n, d));
            let right = b.map_or(Number::Fixed(None), |(n, d)| number(n, d));
            let av = a.map(|(n, d)| oracle(n, d));
            let bv = b.map(|(n, d)| oracle(n, d));
            let pair = av.clone().zip(bv.clone());
            let expected = [
                pair.clone().map(|(a, b)| a + b),
                pair.clone().map(|(a, b)| a - b),
                pair.clone().map(|(a, b)| a * b),
                pair.clone()
                    .and_then(|(a, b)| (!b.is_zero()).then(|| a / b)),
                pair.clone().map(|(a, b)| a.min(b)),
                pair.map(|(a, b)| a.max(b)),
            ];
            let actual = [
                left.add(&right, limits(), &mut work()).unwrap(),
                left.sub(&right, limits(), &mut work()).unwrap(),
                left.mul(&right, limits(), &mut work()).unwrap(),
                left.div(&right, limits(), &mut work()).unwrap(),
                left.min(&right, limits(), &mut work()).unwrap(),
                left.max(&right, limits(), &mut work()).unwrap(),
            ];
            for (answer, expected) in actual.iter().zip(expected) {
                assert_eq!(at(answer, 0, 1), expected.map(|v| v.to_string()));
            }
            for mask in 0..8 {
                let signs = Signs::new(mask).unwrap();
                let compared = left.compare(&right, signs, limits(), &mut work()).unwrap();
                assert_eq!(
                    truth(&compared, 0, 1),
                    av.clone()
                        .zip(bv.clone())
                        .map(|(a, b)| signs.contains(a.cmp(&b)))
                );
            }
        }
    }
    assert_eq!(
        at(
            &Number::Fixed(None).pow(0, limits(), &mut work()).unwrap(),
            0,
            1
        ),
        None
    );
    assert_eq!(
        at(&number(0, 1).pow(0, limits(), &mut work()).unwrap(), 0, 1),
        Some("1".into())
    );
    assert_eq!(
        at(&number(-2, 3).pow(5, limits(), &mut work()).unwrap(), 0, 1),
        Some("-32/243".into())
    );
    assert_eq!(
        at(&number(-2, 3).abs(limits(), &mut work()).unwrap(), 0, 1),
        Some("2/3".into())
    );
}

#[test]
fn shared_parameter_arithmetic_preserves_both_poles_and_exact_branch_boundaries() {
    let domain = unit(P);
    let p = Poly::parameter(P);
    let half = p
        .sub(
            &Poly::constant(rat(1, 2)),
            limits().parameters.polynomial,
            &mut work(),
        )
        .unwrap();
    let quarter = p
        .sub(
            &Poly::constant(rat(1, 4)),
            limits().parameters.polynomial,
            &mut work(),
        )
        .unwrap();
    let tail = Poly::one()
        .sub(&p, limits().parameters.polynomial, &mut work())
        .unwrap();
    let f = function(&domain, p, half);
    let g = function(&domain, tail, quarter);
    let results = [
        f.add(&g, limits(), &mut work()).unwrap(),
        f.sub(&g, limits(), &mut work()).unwrap(),
        f.mul(&g, limits(), &mut work()).unwrap(),
        f.div(&g, limits(), &mut work()).unwrap(),
        f.min(&g, limits(), &mut work()).unwrap(),
        f.max(&g, limits(), &mut work()).unwrap(),
    ];
    for n in -2..=10 {
        let theta = oracle(n, 8);
        let values = if (0..=8).contains(&n) && n != 2 && n != 4 {
            Some((
                theta.clone() / (theta.clone() - oracle(1, 2)),
                (oracle(1, 1) - theta.clone()) / (theta - oracle(1, 4)),
            ))
        } else {
            None
        };
        let expected = [
            values.clone().map(|(a, b)| a + b),
            values.clone().map(|(a, b)| a - b),
            values.clone().map(|(a, b)| a * b),
            values
                .clone()
                .and_then(|(a, b)| (!b.is_zero()).then(|| a / b)),
            values.clone().map(|(a, b)| a.min(b)),
            values.map(|(a, b)| a.max(b)),
        ];
        for (actual, expected) in results.iter().zip(expected) {
            assert_eq!(
                at(actual, n, 8),
                expected.map(|v| v.to_string()),
                "theta={n}/8"
            );
        }
    }
    let self_ratio = f.div(&f, limits(), &mut work()).unwrap();
    assert_eq!(at(&self_ratio, 0, 1), None);
    assert_eq!(at(&self_ratio, 1, 2), None);
    assert_eq!(at(&self_ratio, 3, 4), Some("1".into()));
    let zero = f.mul(&number(0, 1), limits(), &mut work()).unwrap();
    assert_eq!(at(&zero, 1, 2), None);
    assert_eq!(at(&zero, 0, 1), Some("0".into()));
    let power0 = f.pow(0, limits(), &mut work()).unwrap();
    assert_eq!(at(&power0, 1, 2), None);
    assert_eq!(at(&power0, 0, 1), Some("1".into()));
    assert!(
        !zero
            .equivalent(&number(0, 1), limits(), &mut work())
            .unwrap()
    );
    assert!(f.equivalent(&f, limits(), &mut work()).unwrap());
    assert!(
        !f.compare(&f, Signs::ZERO, limits(), &mut work())
            .unwrap()
            .always()
    );
    let never = Number::Fixed(None).add(&f, limits(), &mut work()).unwrap();
    assert!(
        never
            .equivalent(&Number::Fixed(None), limits(), &mut work())
            .unwrap()
    );
}

#[test]
fn predicate_negation_and_all_boolean_lifts_preserve_the_three_way_partition() {
    let fixed = [number(-1, 1), number(1, 1), Number::Fixed(None)].map(|x| {
        x.where_sign(Signs::POSITIVE, limits(), &mut work())
            .unwrap()
    });
    for mask in 0..16 {
        let op = BoolOp4::new(mask).unwrap();
        for a in &fixed {
            for b in &fixed {
                let result = a.apply(op, b, limits(), &mut work()).unwrap();
                assert_eq!(
                    truth(&result, 0, 1),
                    truth(a, 0, 1)
                        .zip(truth(b, 0, 1))
                        .map(|(a, b)| op.evaluate(a, b))
                );
            }
        }
    }
    let domain = unit(P);
    let p = Poly::parameter(P);
    let f = function(
        &domain,
        p.clone(),
        p.sub(
            &Poly::constant(rat(1, 2)),
            limits().parameters.polynomial,
            &mut work(),
        )
        .unwrap(),
    );
    let a = f
        .where_sign(Signs::POSITIVE, limits(), &mut work())
        .unwrap();
    let b = function(&domain, p, Poly::one())
        .compare(&number(3, 4), Signs::NON_NEGATIVE, limits(), &mut work())
        .unwrap();
    assert!(a.possibly());
    assert!(!a.always());
    assert!(!a.is_total());
    assert_eq!(truth(&a.negate(), 1, 2), None);
    for mask in 0..16 {
        let op = BoolOp4::new(mask).unwrap();
        let combined = a.apply(op, &b, limits(), &mut work()).unwrap();
        for n in 0..=8 {
            assert_eq!(
                truth(&combined, n, 8),
                truth(&a, n, 8)
                    .zip(truth(&b, n, 8))
                    .map(|(a, b)| op.evaluate(a, b))
            );
            assert_eq!(truth(&a.negate(), n, 8), truth(&a, n, 8).map(|v| !v));
        }
    }
    let total = function(&domain, Poly::one(), Poly::one())
        .where_sign(Signs::POSITIVE, limits(), &mut work())
        .unwrap();
    assert!(total.always());
    assert!(total.is_total());
    assert!(!total.negate().possibly());
    assert!(
        total
            .apply(BoolOp4::AND, &fixed[1], limits(), &mut work())
            .unwrap()
            .always()
    );
    assert!(
        !total
            .apply(BoolOp4::TRUE, &fixed[2], limits(), &mut work())
            .unwrap()
            .is_total()
    );
}

#[test]
fn comparisons_keep_irrational_boundary_points_and_undefined_endpoints() {
    let domain = unit(P);
    let p = Poly::parameter(P);
    let squared = p
        .mul(&p, limits().parameters.polynomial, &mut work())
        .unwrap();
    let n = squared
        .sub(
            &Poly::constant(rat(1, 2)),
            limits().parameters.polynomial,
            &mut work(),
        )
        .unwrap();
    let f = function(&domain, n.clone(), p);
    let predicate = f.where_sign(Signs::ZERO, limits(), &mut work()).unwrap();
    let NumberPredicateView::Parameter {
        holds,
        fails,
        undefined,
        ..
    } = predicate.view()
    else {
        panic!("parameter")
    };
    for root in n
        .isolate_roots(P, limits().parameters.roots, &mut work())
        .unwrap()
    {
        let witness = RealWitness::Algebraic(root);
        let inside = domain
            .region()
            .contains(&witness, limits().parameters, &mut work())
            .unwrap();
        assert_eq!(
            holds
                .contains(&witness, limits().parameters, &mut work())
                .unwrap(),
            inside
        );
        assert!(
            !fails
                .contains(&witness, limits().parameters, &mut work())
                .unwrap()
        );
        assert!(
            !undefined
                .contains(&witness, limits().parameters, &mut work())
                .unwrap()
        );
    }
    assert_eq!(truth(&predicate, 0, 1), None);
    assert!(!predicate.always());
    assert!(predicate.possibly());
}

#[test]
fn foreign_domains_and_resource_errors_cannot_hide_behind_undefined_or_constant_results() {
    let domain = unit(P);
    let other = unit(ParameterId([250; 32]));
    let empty = Number::Fixed(None)
        .on_domain(&domain, limits(), &mut work())
        .unwrap();
    let foreign = number(0, 1)
        .on_domain(&other, limits(), &mut work())
        .unwrap();
    assert!(matches!(
        empty.mul(&foreign, limits(), &mut work()),
        Err(Error::ParameterScopeMismatch)
    ));
    let positive = ParameterRegion::from_polynomial(
        P,
        &Poly::parameter(P),
        Signs::POSITIVE,
        limits().parameters,
        &mut work(),
    )
    .unwrap();
    let smaller = ParameterDomain::new(
        domain
            .region()
            .apply(BoolOp4::AND, &positive, limits().parameters, &mut work())
            .unwrap(),
    )
    .unwrap();
    let restricted = empty.on_domain(&smaller, limits(), &mut work()).unwrap();
    assert!(matches!(
        restricted.add(&empty, limits(), &mut work()),
        Err(Error::ParameterDomainMismatch)
    ));
    assert!(matches!(
        restricted.on_domain(&domain, limits(), &mut work()),
        Err(Error::ParameterDomainMismatch)
    ));
    let huge = number(65536, 1);
    let mut narrow = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 8,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert!(matches!(
        Number::Fixed(None).mul(&huge, limits(), &mut narrow),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    ));
    let mut counter = work();
    number(1, 2)
        .add(&number(1, 3), limits(), &mut counter)
        .unwrap();
    let mut bounded = ExactArithmetic::new(
        ArithmeticLimits {
            operations: counter.operations(),
            ..ArithmeticLimits::default()
        },
        &(),
    );
    number(1, 2)
        .add(&number(1, 3), limits(), &mut bounded)
        .unwrap();
    assert!(matches!(
        number(1, 2).add(&number(1, 3), limits(), &mut bounded),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let p = Poly::parameter(P);
    let f = function(&domain, p.clone(), Poly::one());
    let g = function(
        &domain,
        Poly::one()
            .sub(&p, limits().parameters.polynomial, &mut work())
            .unwrap(),
        Poly::one(),
    );
    let mut low = limits();
    low.functions.cells = 1;
    assert!(matches!(
        f.min(&g, low, &mut work()),
        Err(Error::Capacity(Capacity::FunctionCells))
    ));
    let stop = Stop(Cell::new(0));
    let mut cancelled = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
    assert!(matches!(
        f.add(&g, limits(), &mut cancelled),
        Err(Error::Cancelled)
    ));
}
