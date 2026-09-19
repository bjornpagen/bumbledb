use std::cell::Cell;
use std::cmp::Ordering;

use crate::{
    AlgebraicRoot, ArithmeticLimits, BoolOp4, Capacity, Control, Error, ExactArithmetic,
    ExactPolynomial as Poly, ExactRational as Rat, GuardedRationalFunction as Function,
    ParameterCell, ParameterDomain, ParameterId, ParameterLimits, ParameterRegion as Region,
    PolynomialSigns as Signs, PolynomialTerm, RealWitness, RootLimits,
};

const P: ParameterId = ParameterId([31; 32]);
const Q: ParameterId = ParameterId([32; 32]);
fn limits() -> ParameterLimits {
    ParameterLimits::default()
}
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
                vec![(P, u32::try_from(i).unwrap())].into_boxed_slice()
            },
        })
        .collect();
    Poly::from_terms(&terms, limits().polynomial, &mut work()).unwrap()
}
fn region(p: &Poly, signs: Signs) -> Region {
    Region::from_polynomial(P, p, signs, limits(), &mut work()).unwrap()
}
fn apply(a: &Region, op: BoolOp4, b: &Region) -> Region {
    a.apply(op, b, limits(), &mut work()).unwrap()
}
fn same(a: &Region, b: &Region) -> bool {
    a.equivalent(b, limits(), &mut work()).unwrap()
}
fn member(a: &Region, n: i64, d: i64) -> bool {
    a.contains(&RealWitness::Rational(rat(n, d)), limits(), &mut work())
        .unwrap()
}
fn unit() -> ParameterDomain {
    ParameterDomain::new(apply(
        &region(&poly(&[0, 1]), Signs::NON_NEGATIVE),
        BoolOp4::AND,
        &region(&poly(&[1, -1]), Signs::NON_NEGATIVE),
    ))
    .unwrap()
}
fn function(domain: ParameterDomain, n: &Poly, d: &Poly) -> Function {
    Function::new(domain, n.clone(), d.clone(), limits(), &mut work()).unwrap()
}
fn value(f: &Function, n: i64, d: i64) -> Option<Rat> {
    f.value_at(&rat(n, d), limits(), &mut work()).unwrap()
}

#[test]
fn exact_sign_domains_keep_open_closed_and_isolated_boundary_membership() {
    let p = poly(&[0, 1]);
    let nonnegative = region(&p, Signs::NON_NEGATIVE);
    let positive = region(&p, Signs::POSITIVE);
    let zero = region(&p, Signs::ZERO);
    assert!(member(&nonnegative, 0, 1));
    assert!(!member(&positive, 0, 1));
    assert!(member(&zero, 0, 1));
    assert!(same(
        &positive.complement(),
        &region(&p, Signs::NON_POSITIVE)
    ));
    assert!(same(&zero.complement(), &region(&p, Signs::NON_ZERO)));
    assert!(apply(&nonnegative, BoolOp4::OR, &positive.complement()).is_full());
    assert!(apply(&zero, BoolOp4::AND, &positive).is_empty());
    assert!(region(&Poly::zero(), Signs::ZERO).is_full());
    assert!(region(&Poly::zero(), Signs::NON_ZERO).is_empty());
    assert!(region(&poly(&[3]), Signs::POSITIVE).is_full());
    assert!(region(&poly(&[-3]), Signs::POSITIVE).is_empty());
    let domain = unit();
    for n in -2..=6 {
        assert_eq!(member(domain.region(), n, 4), (0..=4).contains(&n));
    }
    let cells: Vec<_> = domain.region().cells().collect();
    assert_eq!(cells.len(), 5);
    assert!(matches!(cells[1], (ParameterCell::Point(_), true)));
    assert!(matches!(cells[2], (ParameterCell::Open { .. }, true)));
    assert!(matches!(cells[3], (ParameterCell::Point(_), true)));
    let joined = apply(domain.region(), BoolOp4::OR, &domain.region().complement());
    assert!(joined.is_full());
    assert_eq!(joined.cells().len(), 1);
}

#[test]
fn every_boolean_operation_aligns_algebraic_boundaries_semantically() {
    let a = region(&poly(&[-1, 0, 1]), Signs::NON_NEGATIVE);
    let b = region(&poly(&[-2, 0, 1]), Signs::NEGATIVE);
    for bits in 0..16 {
        let result = apply(&a, BoolOp4::new(bits).unwrap(), &b);
        for n in -12i64..=12 {
            let in_a = n * n >= 16;
            let in_b = n * n < 32;
            let expected = bits >> (u8::from(in_a) * 2 + u8::from(in_b)) & 1 != 0;
            assert_eq!(member(&result, n, 4), expected, "op {bits}, p={n}/4");
        }
        for (equation, in_a, in_b) in [
            (poly(&[-1, 0, 1]), true, true),
            (poly(&[-2, 0, 1]), true, false),
        ] {
            for root in equation
                .isolate_roots(P, RootLimits::default(), &mut work())
                .unwrap()
            {
                let expected = bits >> (u8::from(in_a) * 2 + u8::from(in_b)) & 1 != 0;
                assert_eq!(
                    result
                        .contains(&RealWitness::Algebraic(root), limits(), &mut work())
                        .unwrap(),
                    expected
                );
            }
        }
    }
    // Same set with different, repeated polynomial factors and root enclosures.
    let equivalent = region(
        &poly(&[-2, 0, 1])
            .pow(2, limits().polynomial, &mut work())
            .unwrap(),
        Signs::ZERO,
    );
    assert!(same(&equivalent, &region(&poly(&[-2, 0, 1]), Signs::ZERO)));
}

#[test]
fn irrational_singleton_witnesses_and_source_inhabitation_are_exact() {
    let equality = region(&poly(&[-1, 0, 2]), Signs::ZERO);
    let positive = region(&poly(&[0, 1]), Signs::POSITIVE);
    let singleton = apply(&equality, BoolOp4::AND, &positive);
    for n in 0..=100 {
        assert!(!member(&singleton, n, 100));
    }
    let witness = singleton.witness(limits(), &mut work()).unwrap().unwrap();
    assert!(matches!(witness, RealWitness::Algebraic(_)));
    assert!(singleton.contains(&witness, limits(), &mut work()).unwrap());
    let domain = ParameterDomain::new(singleton).unwrap();
    drop((equality, positive));
    assert!(
        domain
            .region()
            .contains(&witness, limits(), &mut work())
            .unwrap()
    );
    assert!(matches!(
        ParameterDomain::new(Region::empty(P)),
        Err(Error::EmptyParameterDomain)
    ));
    assert!(
        Region::empty(P)
            .witness(limits(), &mut work())
            .unwrap()
            .is_none()
    );
    assert!(
        Region::full(P)
            .witness(limits(), &mut work())
            .unwrap()
            .is_some()
    );
    // Same numerical algebraic point described in another formal indeterminate.
    let RealWitness::Algebraic(root) = witness else {
        unreachable!()
    };
    let renamed = root
        .polynomial()
        .substitute(&[(P, Poly::parameter(Q))], limits().polynomial, &mut work())
        .unwrap();
    let other = AlgebraicRoot::from_interval(
        &renamed,
        Q,
        root.interval().0.clone(),
        root.interval().1.clone(),
        limits().roots,
        &mut work(),
    )
    .unwrap();
    assert!(
        domain
            .region()
            .contains(&RealWitness::Algebraic(other), limits(), &mut work())
            .unwrap()
    );
}

#[test]
fn rational_cell_separators_are_strict_and_checked() {
    let roots = poly(&[-2, 0, 1])
        .isolate_roots(P, limits().roots, &mut work())
        .unwrap();
    let separator = roots[0]
        .rational_between(&roots[1], limits().roots, &mut work())
        .unwrap();
    assert_eq!(
        roots[0]
            .compare_rational(&separator, limits().roots, &mut work())
            .unwrap(),
        Ordering::Less
    );
    assert_eq!(
        roots[1]
            .compare_rational(&separator, limits().roots, &mut work())
            .unwrap(),
        Ordering::Greater
    );
    for (a, b) in [(&roots[0], &roots[0]), (&roots[1], &roots[0])] {
        assert_eq!(
            a.rational_between(b, limits().roots, &mut work()),
            Err(Error::InvalidRootOrder)
        );
    }
    let rational = AlgebraicRoot::from_interval(
        &poly(&[-3, 2]),
        P,
        rat(3, 2),
        rat(3, 2),
        limits().roots,
        &mut work(),
    )
    .unwrap();
    assert_eq!(
        rational
            .compare_rational(&rat(3, 2), limits().roots, &mut work())
            .unwrap(),
        Ordering::Equal
    );
    assert_eq!(
        rational
            .compare_rational(&rat(1, 1), limits().roots, &mut work())
            .unwrap(),
        Ordering::Greater
    );
    let between = roots[1]
        .rational_between(&rational, limits().roots, &mut work())
        .unwrap();
    assert_eq!(
        rational
            .compare_rational(&between, limits().roots, &mut work())
            .unwrap(),
        Ordering::Greater
    );
}

#[test]
fn shared_bias_fairness_retains_its_evidence_and_endpoint_holes() {
    let domain = unit();
    let n = poly(&[0, 1, -1]);
    let z = poly(&[0, 2, -2]);
    let fair = function(domain.clone(), &n, &z);
    assert_eq!(fair.numerator(), &n);
    assert_eq!(fair.denominator(), &z);
    assert_eq!(value(&fair, 0, 1), None);
    assert_eq!(value(&fair, 1, 1), None);
    for n in 1..8 {
        assert_eq!(value(&fair, n, 8), Some(rat(1, 2)));
    }
    let endpoints = apply(domain.region(), BoolOp4::DIFFERENCE, fair.defined_on());
    assert!(member(&endpoints, 0, 1));
    assert!(member(&endpoints, 1, 1));
    assert!(!member(&endpoints, 1, 2));
    let unconditional = function(domain.clone(), &poly(&[1]), &poly(&[2]));
    assert!(
        !fair
            .equivalent(&unconditional, limits(), &mut work())
            .unwrap()
    );
    let matching = unconditional
        .restrict(fair.defined_on(), limits(), &mut work())
        .unwrap();
    assert!(fair.equivalent(&matching, limits(), &mut work()).unwrap());
    assert_ne!(fair.denominator(), matching.denominator());
    let zero = function(domain, &Poly::zero(), &Poly::one());
    let product = fair.mul(&zero, limits(), &mut work()).unwrap();
    assert_eq!(value(&product, 0, 1), None);
    assert_eq!(value(&product, 1, 2), Some(Rat::zero()));
    assert!(same(product.defined_on(), fair.defined_on()));
    assert!(
        zero.reciprocal(limits(), &mut work())
            .unwrap()
            .is_nowhere_defined()
    );
    assert!(!zero.ambient().region().is_empty());
}

#[test]
fn guarded_arithmetic_and_sign_regions_match_independent_rational_values() {
    let domain = ParameterDomain::new(Region::full(P)).unwrap();
    let a = function(domain.clone(), &poly(&[1, 1]), &poly(&[-1, 1]));
    let b = function(domain, &poly(&[0, 1]), &poly(&[2]));
    let results = [
        a.add(&b, limits(), &mut work()).unwrap(),
        a.sub(&b, limits(), &mut work()).unwrap(),
        a.mul(&b, limits(), &mut work()).unwrap(),
        a.div(&b, limits(), &mut work()).unwrap(),
    ];
    for (kind, result) in results.iter().enumerate() {
        let signs: Vec<_> = (0..8)
            .map(|bits| {
                result
                    .where_sign(Signs::new(bits).unwrap(), limits(), &mut work())
                    .unwrap()
            })
            .collect();
        for n in -8i64..=8 {
            if n == 4 || (kind == 3 && n == 0) {
                assert_eq!(value(result, n, 4), None);
                continue;
            }
            let (num, den) = match kind {
                0 => (8 * (4 + n) + n * (n - 4), 8 * (n - 4)),
                1 => (8 * (4 + n) - n * (n - 4), 8 * (n - 4)),
                2 => (n * (4 + n), 8 * (n - 4)),
                3 => (8 * (4 + n), n * (n - 4)),
                _ => unreachable!(),
            };
            assert_eq!(value(result, n, 4), Some(rat(num, den)));
            let sign = if num == 0 {
                1
            } else if (num < 0) == (den < 0) {
                2
            } else {
                0
            };
            for (bits, region) in signs.iter().enumerate() {
                assert_eq!(member(region, n, 4), bits >> sign & 1 != 0);
            }
        }
        assert!(!member(&signs[7], 1, 1));
    }
}

#[test]
fn scopes_undefined_functions_and_limits_never_become_false_domains() {
    let zero = Region::empty(P);
    let foreign = Region::empty(Q);
    assert!(matches!(
        zero.apply(BoolOp4::FALSE, &foreign, limits(), &mut work()),
        Err(Error::ParameterScopeMismatch)
    ));
    assert!(matches!(
        Region::from_polynomial(P, &Poly::parameter(Q), Signs::ANY, limits(), &mut work()),
        Err(Error::NotUnivariate)
    ));
    let no_cells = ParameterLimits {
        cells: 0,
        ..limits()
    };
    assert!(matches!(
        zero.apply(BoolOp4::FALSE, &zero, no_cells, &mut work()),
        Err(Error::Capacity(Capacity::ParameterCells))
    ));
    let limited = ParameterLimits {
        cells: 2,
        ..limits()
    };
    assert!(matches!(
        Region::from_polynomial(P, &poly(&[0, 1]), Signs::ZERO, limited, &mut work()),
        Err(Error::Capacity(Capacity::ParameterCells))
    ));
    let singleton = region(&poly(&[-2, 0, 1]), Signs::ZERO);
    let degree = ParameterLimits {
        roots: RootLimits {
            degree: 1,
            ..limits().roots
        },
        ..limits()
    };
    assert!(matches!(
        zero.apply(BoolOp4::FALSE, &singleton, degree, &mut work()),
        Err(Error::Capacity(Capacity::AlgebraicDegree))
    ));
    let everywhere = ParameterDomain::new(Region::full(P)).unwrap();
    let nowhere = function(everywhere.clone(), &Poly::one(), &Poly::zero());
    assert!(nowhere.is_nowhere_defined());
    assert_eq!(value(&nowhere, 1, 2), None);
    let small = function(unit(), &Poly::one(), &Poly::one());
    let large = function(everywhere, &Poly::one(), &Poly::one());
    assert!(matches!(
        small.mul(&large, limits(), &mut work()),
        Err(Error::ParameterDomainMismatch)
    ));
    assert!(!small.equivalent(&large, limits(), &mut work()).unwrap());
    let mut tiny = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 1,
            operations: 1000,
        },
        &(),
    );
    assert!(matches!(
        nowhere.value_at(&rat(1024, 1), limits(), &mut tiny),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    ));
}

struct Stop(Cell<usize>);
impl Control for Stop {
    fn checkpoint(&self) -> crate::Result<()> {
        let n = self.0.get();
        if n == 0 {
            Err(Error::Cancelled)
        } else {
            self.0.set(n - 1);
            Ok(())
        }
    }
}
#[test]
fn solver_cancellation_does_not_publish_empty_or_undefined_answers() {
    let domain = unit();
    let f = function(domain.clone(), &poly(&[0, 1]), &poly(&[1, -1]));
    for budget in 0..25 {
        let stop = Stop(Cell::new(budget));
        let mut w = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
        assert!(matches!(
            Region::from_polynomial(P, &poly(&[-2, 0, 1]), Signs::POSITIVE, limits(), &mut w),
            Err(Error::Cancelled)
        ));
        let stop = Stop(Cell::new(budget));
        let mut w = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
        assert!(matches!(
            domain.region().witness(limits(), &mut w),
            Err(Error::Cancelled)
        ));
        let stop = Stop(Cell::new(budget));
        let mut w = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
        assert!(matches!(
            f.value_at(&rat(1, 2), limits(), &mut w),
            Err(Error::Cancelled)
        ));
    }
}
