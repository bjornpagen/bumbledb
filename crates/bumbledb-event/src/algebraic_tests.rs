use std::cell::Cell;
use std::cmp::Ordering;

use crate::{
    AlgebraicRoot, ArithmeticLimits, Capacity, Control, Error, ExactArithmetic,
    ExactPolynomial as Poly, ExactRational as Rat, ParameterId, PolynomialLimits, PolynomialTerm,
    RootLimits,
};

const X: ParameterId = ParameterId([8; 32]);
const Y: ParameterId = ParameterId([9; 32]);
fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn rat(n: i64, d: i64) -> Rat {
    Rat::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn poly(coefficients: &[i64]) -> Poly {
    let terms: Vec<_> = coefficients
        .iter()
        .enumerate()
        .map(|(i, &n)| PolynomialTerm {
            coefficient: Rat::from(n),
            powers: if i == 0 {
                Box::new([])
            } else {
                vec![(X, u32::try_from(i).unwrap())].into_boxed_slice()
            },
        })
        .collect();
    Poly::from_terms(&terms, PolynomialLimits::default(), &mut work()).unwrap()
}
fn mul(a: &Poly, b: &Poly) -> Poly {
    a.mul(b, PolynomialLimits::default(), &mut work()).unwrap()
}
fn roots(p: &Poly) -> Vec<AlgebraicRoot> {
    p.isolate_roots(X, RootLimits::default(), &mut work())
        .unwrap()
}
fn root(p: &Poly, lower: Rat, upper: Rat) -> AlgebraicRoot {
    AlgebraicRoot::from_interval(p, X, lower, upper, RootLimits::default(), &mut work()).unwrap()
}
fn sign(at: &AlgebraicRoot, p: &Poly) -> Ordering {
    at.sign(p, RootLimits::default(), &mut work()).unwrap()
}
fn compare(a: &AlgebraicRoot, b: &AlgebraicRoot) -> Ordering {
    a.compare(b, RootLimits::default(), &mut work()).unwrap()
}

#[test]
fn irrational_witnesses_and_strict_parameter_guards_are_exact() {
    let p = poly(&[-1, 0, 2]); // p²=1/2, a domain with no rational witness.
    let alpha = root(&p, rat(0, 1), rat(1, 1));
    assert_eq!(sign(&alpha, &p), Ordering::Equal);
    assert_eq!(sign(&alpha, &poly(&[0, 1])), Ordering::Greater);
    assert_eq!(sign(&alpha, &poly(&[1, -1])), Ordering::Greater);
    assert_eq!(sign(&alpha, &poly(&[-7, 10])), Ordering::Greater);
    assert_eq!(sign(&alpha, &poly(&[-71, 100])), Ordering::Less);
    // Vanishing factors are detected algebraically, including repeated roots.
    assert_eq!(sign(&alpha, &mul(&p, &poly(&[3, 1]))), Ordering::Equal);
    assert_eq!(sign(&alpha, &mul(&p, &p)), Ordering::Equal);
    let both = roots(&p);
    assert_eq!(both.len(), 2);
    assert_eq!(compare(&both[0], &both[1]), Ordering::Less);
    assert_eq!(compare(&both[1], &alpha), Ordering::Equal);
    let portable = root(
        alpha.polynomial(),
        alpha.interval().0.clone(),
        alpha.interval().1.clone(),
    );
    drop((p, both, alpha));
    assert_eq!(sign(&portable, &poly(&[-1, 0, 2])), Ordering::Equal);
}

#[test]
fn sturm_isolation_handles_multiplicity_no_roots_and_zero_roots() {
    for a in -3..=3 {
        for b in -3..=3 {
            for c in -2..=2 {
                let polynomial = mul(&mul(&poly(&[-a, 1]), &poly(&[-b, 1])), &poly(&[-c, 1]));
                let isolated = roots(&polynomial);
                let mut expected = vec![a, b, c];
                expected.sort_unstable();
                expected.dedup();
                assert_eq!(isolated.len(), expected.len());
                for (actual, n) in isolated.iter().zip(expected) {
                    let exact = root(&poly(&[-n, 1]), rat(n, 1), rat(n, 1));
                    assert_eq!(compare(actual, &exact), Ordering::Equal);
                    assert_eq!(sign(actual, &polynomial), Ordering::Equal);
                }
            }
        }
    }
    for polynomial in [
        poly(&[1]),
        poly(&[-2]),
        poly(&[1, 0, 1]),
        poly(&[1, 0, 2, 0, 1]),
    ] {
        assert!(roots(&polynomial).is_empty());
    }
    assert!(matches!(
        Poly::zero().isolate_roots(X, RootLimits::default(), &mut work()),
        Err(Error::IndeterminateRoots)
    ));
    assert!(matches!(
        Poly::parameter(Y).isolate_roots(X, RootLimits::default(), &mut work()),
        Err(Error::NotUnivariate)
    ));
}

#[test]
fn real_equality_ignores_reducible_presentations_and_isolation_choices() {
    let sqrt2 = root(&poly(&[-2, 0, 1]), rat(1, 1), rat(2, 1));
    let alternative = root(&poly(&[6, 2, -3, -1]), rat(7, 5), rat(3, 2)); // -(x+3)(x²-2).
    assert_eq!(compare(&sqrt2, &alternative), Ordering::Equal);
    let cubic = roots(&poly(&[-2, 0, 0, 1])).pop().unwrap();
    assert_eq!(compare(&cubic, &sqrt2), Ordering::Less);
    assert_eq!(compare(&sqrt2, &cubic), Ordering::Greater);
    let other_name = AlgebraicRoot::from_interval(
        &poly(&[-2, 0, 1])
            .substitute(
                &[(X, Poly::parameter(Y))],
                PolynomialLimits::default(),
                &mut work(),
            )
            .unwrap(),
        Y,
        rat(1, 1),
        rat(2, 1),
        RootLimits::default(),
        &mut work(),
    )
    .unwrap();
    assert_eq!(compare(&sqrt2, &other_name), Ordering::Equal);
    assert!(matches!(
        sqrt2.sign(&Poly::parameter(Y), RootLimits::default(), &mut work()),
        Err(Error::NotUnivariate)
    ));
    // Overlapping isolating intervals for roots separated by a tiny rational
    // distance must be refined; overlap never establishes equality.
    let delta = Rat::fraction("1", "1000000000000000000000000000000", &mut work()).unwrap();
    let shifted = poly(&[-2, 0, 1])
        .substitute(
            &[(
                X,
                Poly::parameter(X)
                    .sub(
                        &Poly::constant(delta),
                        PolynomialLimits::default(),
                        &mut work(),
                    )
                    .unwrap(),
            )],
            PolynomialLimits::default(),
            &mut work(),
        )
        .unwrap();
    let near = root(&shifted, rat(1, 1), rat(2, 1));
    assert_eq!(compare(&sqrt2, &near), Ordering::Less);
    assert_eq!(compare(&near, &sqrt2), Ordering::Greater);
}

#[test]
fn sign_is_not_inferred_from_endpoint_signs_or_approximate_zero() {
    let alpha = root(&poly(&[-2, 0, 1]), rat(1, 1), rat(2, 1));
    // Positive at both endpoints, negative at sqrt(2): two interior zeros.
    assert_eq!(sign(&alpha, &poly(&[48, -70, 25])), Ordering::Less);
    let tiny = Rat::fraction("1", "1000000000000000000000000000000000000000", &mut work()).unwrap();
    assert_eq!(sign(&alpha, &Poly::constant(tiny)), Ordering::Greater);
    // A tested polynomial can vanish at an isolating endpoint without vanishing
    // at the root. Sturm's half-open count must not confuse the two.
    assert_eq!(sign(&alpha, &poly(&[-1, 1])), Ordering::Greater);
    assert_eq!(sign(&alpha, &poly(&[-2, 1])), Ordering::Less);
    let rational = root(&poly(&[-3, 2]), rat(3, 2), rat(3, 2));
    assert_eq!(sign(&rational, &poly(&[-3, 2])), Ordering::Equal);
    assert_eq!(compare(&alpha, &rational), Ordering::Less);
    assert_eq!(compare(&rational, &alpha), Ordering::Greater);
}

#[test]
fn isolation_claims_are_recomputed_and_capacities_refuse_explicitly() {
    let polynomial = poly(&[-2, 0, 1]);
    for (lower, upper) in [
        (rat(2, 1), rat(1, 1)),
        (rat(-2, 1), rat(2, 1)),
        (rat(0, 1), rat(1, 1)),
        (rat(1, 1), rat(1, 1)),
    ] {
        assert!(matches!(
            AlgebraicRoot::from_interval(
                &polynomial,
                X,
                lower,
                upper,
                RootLimits::default(),
                &mut work()
            ),
            Err(Error::InvalidRootInterval)
        ));
    }
    assert!(matches!(
        AlgebraicRoot::from_interval(
            &poly(&[0, -1, 1]),
            X,
            rat(0, 1),
            rat(2, 1),
            RootLimits::default(),
            &mut work()
        ),
        Err(Error::InvalidRootInterval)
    ));
    assert!(matches!(
        polynomial.isolate_roots(
            X,
            RootLimits {
                roots: 1,
                ..RootLimits::default()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::AlgebraicRoots))
    ));
    let alpha = root(&polynomial, rat(1, 1), rat(2, 1));
    let degree = RootLimits {
        degree: 1,
        ..RootLimits::default()
    };
    assert_eq!(
        alpha.sign(&Poly::zero(), degree, &mut work()),
        Err(Error::Capacity(Capacity::AlgebraicDegree))
    );
    assert!(matches!(
        polynomial.isolate_roots(X, degree, &mut work()),
        Err(Error::Capacity(Capacity::AlgebraicDegree))
    ));
    assert_eq!(
        alpha.compare(
            &alpha,
            RootLimits {
                steps: 1,
                ..RootLimits::default()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::AlgebraicSteps))
    );
    let mut tiny = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 0,
            operations: 100,
        },
        &(),
    );
    assert_eq!(
        alpha.sign(&Poly::zero(), RootLimits::default(), &mut tiny),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    );
}

struct Stop(Cell<usize>);
impl Control for Stop {
    fn checkpoint(&self) -> crate::Result<()> {
        let left = self.0.get();
        if left == 0 {
            Err(Error::Cancelled)
        } else {
            self.0.set(left - 1);
            Ok(())
        }
    }
}

#[test]
fn root_operations_poll_cancellation_without_publishing_partial_rosters() {
    let polynomial = poly(&[-2, 0, 1]);
    let alpha = root(&polynomial, rat(1, 1), rat(2, 1));
    for remaining in 0..25 {
        let stop = Stop(Cell::new(remaining));
        let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
        assert!(matches!(
            polynomial.isolate_roots(X, RootLimits::default(), &mut work),
            Err(Error::Cancelled)
        ));
        let stop = Stop(Cell::new(remaining));
        let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
        assert_eq!(
            alpha.sign(&polynomial, RootLimits::default(), &mut work),
            Err(Error::Cancelled)
        );
        let stop = Stop(Cell::new(remaining));
        let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
        assert_eq!(
            alpha.compare(&alpha, RootLimits::default(), &mut work),
            Err(Error::Cancelled)
        );
    }
}

#[test]
fn exact_sympy_oracle_matches_all_real_roots_and_polynomial_signs() {
    fn fraction(text: &str) -> Rat {
        let (n, d) = text.split_once('/').unwrap();
        Rat::fraction(n, d, &mut work()).unwrap()
    }
    let probes = [
        poly(&[-1, 1]),
        poly(&[-2, 0, 1]),
        poly(&[1, 1, -1]),
        poly(&[3, -2, 1]),
    ];
    let fixture = include_str!("../tests/fixtures/algebraic-roots-sympy.txt");
    let mut cases = 0;
    for line in fixture.lines().filter(|line| !line.starts_with('#')) {
        cases += 1;
        let (coefficients, expected) = line.split_once('|').unwrap();
        let coefficients: Vec<_> = coefficients
            .split(',')
            .map(|v| v.parse::<i64>().unwrap())
            .collect();
        let polynomial = poly(&coefficients);
        if expected == "indeterminate" {
            assert!(matches!(
                polynomial.isolate_roots(X, RootLimits::default(), &mut work()),
                Err(Error::IndeterminateRoots)
            ));
            continue;
        }
        let expected: Vec<_> = expected.split('|').filter(|s| !s.is_empty()).collect();
        let actual = roots(&polynomial);
        assert_eq!(actual.len(), expected.len(), "{line}");
        for (actual, description) in actual.iter().zip(expected) {
            let fields: Vec<_> = description.split(',').collect();
            assert_eq!(fields.len(), 6);
            let independent = root(&polynomial, fraction(fields[0]), fraction(fields[1]));
            assert_eq!(compare(actual, &independent), Ordering::Equal, "{line}");
            for (probe, expected) in probes.iter().zip(&fields[2..]) {
                let expected = match *expected {
                    "-1" => Ordering::Less,
                    "0" => Ordering::Equal,
                    "1" => Ordering::Greater,
                    _ => panic!("sign"),
                };
                assert_eq!(sign(actual, probe), expected, "{line}");
            }
        }
    }
    assert_eq!(cases, 105);
}

#[test]
fn sparse_high_degree_is_bounded_before_dense_allocation() {
    let polynomial = Poly::from_terms(
        &[PolynomialTerm {
            coefficient: Rat::one(),
            powers: Box::new([(X, u32::MAX)]),
        }],
        PolynomialLimits {
            degree: u32::MAX,
            ..PolynomialLimits::default()
        },
        &mut work(),
    )
    .unwrap();
    assert!(matches!(
        polynomial.isolate_roots(
            X,
            RootLimits {
                degree: u32::MAX,
                steps: 100,
                ..RootLimits::default()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::AlgebraicSteps))
    ));
}
