use std::cell::Cell;
use std::fmt::Write as _;

use crate::{
    ArithmeticLimits, Capacity, Control, Error, ExactArithmetic, ExactPolynomial as Poly,
    ExactRational as Rat, ParameterId, PolynomialLimits, PolynomialTerm,
};

const P: ParameterId = ParameterId([1; 32]);
const Q: ParameterId = ParameterId([2; 32]);
const R: ParameterId = ParameterId([3; 32]);

fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn limits() -> PolynomialLimits {
    PolynomialLimits::default()
}
fn rat(n: i64, d: i64) -> Rat {
    Rat::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn term(n: i64, d: i64, powers: &[(ParameterId, u32)]) -> PolynomialTerm {
    PolynomialTerm {
        coefficient: rat(n, d),
        powers: powers.into(),
    }
}
fn poly(terms: &[PolynomialTerm]) -> Poly {
    Poly::from_terms(terms, limits(), &mut work()).unwrap()
}
fn add(a: &Poly, b: &Poly) -> Poly {
    a.add(b, limits(), &mut work()).unwrap()
}
fn sub(a: &Poly, b: &Poly) -> Poly {
    a.sub(b, limits(), &mut work()).unwrap()
}
fn mul(a: &Poly, b: &Poly) -> Poly {
    a.mul(b, limits(), &mut work()).unwrap()
}
fn pow(a: &Poly, n: u32) -> Poly {
    a.pow(n, limits(), &mut work()).unwrap()
}
fn eval(a: &Poly, p: &Rat, q: &Rat) -> Rat {
    a.evaluate(&[(P, p.clone()), (Q, q.clone())], limits(), &mut work())
        .unwrap()
}
fn beta(a: &Poly, p: ParameterId, alpha: &Rat, beta: &Rat) -> Poly {
    a.integrate_beta(p, alpha, beta, limits(), &mut work())
        .unwrap()
}

#[test]
fn sparse_normal_form_is_independent_of_input_order_and_expansion() {
    let presentation = [
        term(3, 2, &[(Q, 0), (P, 1), (P, 2)]),
        term(-1, 2, &[(P, 3)]),
        term(0, 1, &[(Q, 3)]),
        term(2, 3, &[]),
        term(-2, 3, &[]),
        term(1, 2, &[(Q, 1)]),
    ];
    let expected = poly(&[term(1, 1, &[(P, 3)]), term(1, 2, &[(Q, 1)])]);
    for shift in 0..presentation.len() {
        let mut input = presentation.to_vec();
        input.rotate_left(shift);
        if shift % 2 != 0 {
            input.reverse();
        }
        let actual = poly(&input);
        assert_eq!(actual, expected);
        assert_eq!(
            actual.to_bytes(limits(), &mut work()).unwrap(),
            expected.to_bytes(limits(), &mut work()).unwrap()
        );
    }
    let p = Poly::parameter(P);
    let q = Poly::parameter(Q);
    assert_eq!(
        mul(&add(&p, &q), &sub(&p, &q)),
        sub(&pow(&p, 2), &pow(&q, 2))
    );
    assert_ne!(pow(&p, 2), p); // No unproved endpoint-domain equality.
    assert_ne!(p, q);
    assert_eq!(sub(&expected, &expected), Poly::zero());
}

#[test]
fn polynomial_operations_match_independent_integer_evaluation() {
    // Signed, mixed-source coefficients; the oracle uses direct integer cross
    // products, not the polynomial evaluator or arithmetic implementation.
    for a in -2..=2 {
        for b in -2..=2 {
            let x = poly(&[
                term(a, 3, &[(P, 2)]),
                term(b, 2, &[(Q, 1)]),
                term(1, 6, &[]),
            ]);
            let y = poly(&[term(b, 3, &[(P, 1), (Q, 1)]), term(a, 6, &[])]);
            let sum = add(&x, &y);
            let difference = sub(&x, &y);
            let product = mul(&x, &y);
            for pn in -2..=2 {
                for qn in -2..=2 {
                    let p = rat(pn, 2);
                    let q = rat(qn, 3);
                    // x=(3*a*pn²+6*b*qn+6)/36, y=(b*pn*qn+3*a)/18.
                    let xn = 3 * a * pn * pn + 6 * b * qn + 6;
                    let yn = b * pn * qn + 3 * a;
                    assert_eq!(eval(&x, &p, &q), rat(xn, 36));
                    assert_eq!(eval(&y, &p, &q), rat(yn, 18));
                    assert_eq!(eval(&sum, &p, &q), rat(xn + 2 * yn, 36));
                    assert_eq!(eval(&difference, &p, &q), rat(xn - 2 * yn, 36));
                    assert_eq!(eval(&product, &p, &q), rat(xn * yn, 648));
                }
            }
        }
    }
}

#[test]
fn repeated_shared_bias_retains_the_evidence_polynomial_and_endpoints() {
    let p = Poly::parameter(P);
    let tail = sub(&Poly::one(), &p);
    let hh = mul(&p, &p);
    let ht = mul(&p, &tail);
    let th = mul(&tail, &p);
    let tt = mul(&tail, &tail);
    assert_eq!(add(&add(&hh, &ht), &add(&th, &tt)), Poly::one());
    assert_eq!(ht, th);
    let evidence = add(&ht, &th);
    assert_eq!(
        evidence,
        poly(&[term(2, 1, &[(P, 1)]), term(-2, 1, &[(P, 2)])])
    );
    assert_eq!(mul(&Poly::constant(rat(1, 2)), &evidence), ht);
    // The fair conditional ratio only exists away from endpoints. The exact
    // numerator identity does not erase its zero-evidence domain boundary.
    for n in 0..=8 {
        let assignment = rat(n, 8);
        let z = eval(&evidence, &assignment, &Rat::zero());
        assert_eq!(z, rat(2 * n * (8 - n), 64));
        if n == 0 || n == 8 {
            assert!(z.is_zero());
        } else {
            assert_eq!(
                eval(&ht, &assignment, &Rat::zero())
                    .div(&z, &mut work())
                    .unwrap(),
                rat(1, 2)
            );
        }
    }
    let unrelated = mul(&p, &sub(&Poly::one(), &Poly::parameter(Q)));
    assert_ne!(unrelated, mul(&tail, &Poly::parameter(Q)));
    // Discarding a fresh draw sums alternatives and returns the old marginal.
    assert_eq!(add(&hh, &ht), p);
}

#[test]
fn binomial_partition_and_bernstein_degree_elevation_are_exact() {
    fn choose(n: u32, k: u32) -> u64 {
        (0..k).fold(1, |v, i| v * u64::from(n - i) / u64::from(i + 1))
    }
    let p = Poly::parameter(P);
    let t = sub(&Poly::one(), &p);
    let bernstein = |n: u32, k: u32| {
        mul(
            &Poly::constant(Rat::from(choose(n, k))),
            &mul(&pow(&p, k), &pow(&t, n - k)),
        )
    };
    for n in 0..=9 {
        let mut total = Poly::zero();
        for k in 0..=n {
            let term = bernstein(n, k);
            total = add(&total, &term);
            let left = mul(
                &Poly::constant(rat(i64::from(n + 1 - k), i64::from(n + 1))),
                &bernstein(n + 1, k),
            );
            let right = mul(
                &Poly::constant(rat(i64::from(k + 1), i64::from(n + 1))),
                &bernstein(n + 1, k + 1),
            );
            assert_eq!(term, add(&left, &right));
        }
        assert_eq!(total, Poly::one());
    }
}

#[test]
fn substitution_is_simultaneous_and_preserves_shared_names() {
    let p = Poly::parameter(P);
    let q = Poly::parameter(Q);
    let expression = add(&pow(&p, 2), &mul(&p, &q));
    let swapped = expression
        .substitute(&[(P, q.clone()), (Q, p.clone())], limits(), &mut work())
        .unwrap();
    assert_eq!(swapped, add(&pow(&q, 2), &mul(&q, &p)));
    let diagonal = expression
        .substitute(&[(Q, p.clone())], limits(), &mut work())
        .unwrap();
    assert_eq!(diagonal, mul(&Poly::constant(rat(2, 1)), &pow(&p, 2)));
    let collapsed = expression
        .substitute(&[(P, Poly::zero()), (Q, p.clone())], limits(), &mut work())
        .unwrap();
    assert_eq!(collapsed, Poly::zero());
    let fresh = expression
        .substitute(&[(P, Poly::parameter(R))], limits(), &mut work())
        .unwrap();
    assert_ne!(fresh, expression);
    assert_eq!(
        expression.substitute(&[], limits(), &mut work()).unwrap(),
        expression
    );
    assert_eq!(
        expression.substitute(&[(P, p.clone()), (P, q)], limits(), &mut work()),
        Err(Error::ParameterBinding)
    );
    assert_eq!(
        p.evaluate(&[], limits(), &mut work()),
        Err(Error::ParameterBinding)
    );
    assert_eq!(
        Poly::zero().evaluate(&[(P, Rat::one()), (P, Rat::one())], limits(), &mut work()),
        Err(Error::ParameterBinding)
    );
}

#[test]
fn explicit_beta_prior_distinguishes_shared_allocation_and_independent_allocations() {
    let p = Poly::parameter(P);
    let q = Poly::parameter(Q);
    let one = Rat::one();
    assert_eq!(beta(&pow(&p, 2), P, &one, &one), Poly::constant(rat(1, 3)));
    assert_eq!(
        beta(&beta(&mul(&p, &q), P, &one, &one), Q, &one, &one),
        Poly::constant(rat(1, 4))
    );
    assert_eq!(
        beta(&mul(&p, &q), P, &rat(2, 1), &rat(3, 1)),
        mul(&Poly::constant(rat(2, 5)), &q)
    );
    let t = sub(&Poly::one(), &p);
    for h in 0..=5 {
        for tails in 0..=5 {
            let evidence = mul(&pow(&p, h), &pow(&t, tails));
            let z = beta(&evidence, P, &rat(2, 1), &rat(3, 1));
            let next_head = beta(&mul(&p, &evidence), P, &rat(2, 1), &rat(3, 1));
            assert_eq!(
                next_head,
                mul(
                    &Poly::constant(rat(i64::from(2 + h), i64::from(5 + h + tails))),
                    &z
                )
            );
        }
    }
    // Rational, nonintegral shapes are supported; no implicit integer counts.
    assert_eq!(
        beta(&pow(&p, 2), P, &rat(1, 2), &rat(1, 2)),
        Poly::constant(rat(3, 8))
    );
    assert_eq!(beta(&q, P, &one, &one), q);
    assert_eq!(beta(&Poly::zero(), P, &one, &one), Poly::zero());
    for shape in [Rat::zero(), rat(-1, 1)] {
        assert_eq!(
            Poly::zero().integrate_beta(P, &shape, &one, limits(), &mut work()),
            Err(Error::InvalidBetaPrior)
        );
        assert_eq!(
            Poly::one().integrate_beta(P, &one, &shape, limits(), &mut work()),
            Err(Error::InvalidBetaPrior)
        );
    }
}

#[test]
fn polynomial_wire_refuses_noncanonical_and_malformed_values() {
    // Independent grammar writer for malleable normal forms.
    fn packet(terms: &[(Rat, Vec<(ParameterId, u32)>)]) -> Vec<u8> {
        let mut bytes = b"BEPL\x01".to_vec();
        bytes.extend_from_slice(&(terms.len() as u64).to_le_bytes());
        for (coefficient, powers) in terms {
            let scalar = coefficient.to_bytes(&mut work()).unwrap();
            bytes.extend_from_slice(&(scalar.len() as u64).to_le_bytes());
            bytes.extend(scalar);
            bytes.extend_from_slice(&(powers.len() as u64).to_le_bytes());
            for (name, power) in powers {
                bytes.extend(name.0);
                bytes.extend(power.to_le_bytes());
            }
        }
        bytes
    }
    let values = [
        Poly::zero(),
        Poly::one(),
        Poly::parameter(P),
        poly(&[term(-2, 3, &[]), term(5, 7, &[(P, 2), (Q, 1)])]),
    ];
    for value in values {
        let bytes = value.to_bytes(limits(), &mut work()).unwrap();
        assert_eq!(
            Poly::from_bytes(&bytes, limits(), &mut work()).unwrap(),
            value
        );
        for end in 0..bytes.len() {
            assert!(Poly::from_bytes(&bytes[..end], limits(), &mut work()).is_err());
        }
        let mut extra = bytes.clone();
        extra.push(0);
        assert_eq!(
            Poly::from_bytes(&extra, limits(), &mut work()),
            Err(Error::InvalidPolynomial)
        );
        let mut version = bytes.clone();
        version[4] = 2;
        assert_eq!(
            Poly::from_bytes(&version, limits(), &mut work()),
            Err(Error::UnsupportedVersion(2))
        );
    }
    let p = Poly::parameter(P).to_bytes(limits(), &mut work()).unwrap();
    assert_eq!(packet(&[(Rat::one(), vec![(P, 1)])]), p);
    for terms in [
        vec![(Rat::zero(), vec![])],
        vec![(Rat::one(), vec![(P, 0)])],
        vec![(Rat::one(), vec![(P, 1), (P, 1)])],
        vec![(Rat::one(), vec![(Q, 1), (P, 1)])],
        vec![(Rat::one(), vec![(P, 1)]), (Rat::one(), vec![(P, 1)])],
        vec![(Rat::one(), vec![(Q, 1)]), (Rat::one(), vec![(P, 1)])],
    ] {
        assert_eq!(
            Poly::from_bytes(&packet(&terms), limits(), &mut work()),
            Err(Error::InvalidPolynomial)
        );
    }
    // Canonical positive constant is a stable independent format fixture.
    let one = Poly::one().to_bytes(limits(), &mut work()).unwrap();
    let fixture = include_str!("../tests/fixtures/polynomial-v1-one.hex").trim();
    assert_eq!(
        one.iter().fold(String::new(), |mut text, b| {
            write!(&mut text, "{b:02x}").unwrap();
            text
        }),
        fixture
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
fn limits_and_cancellation_never_become_a_mathematical_zero() {
    let p = Poly::parameter(P);
    let zero = Poly::zero();
    let no_terms = PolynomialLimits {
        terms: 0,
        ..limits()
    };
    assert_eq!(
        zero.mul(&p, no_terms, &mut work()),
        Err(Error::Capacity(Capacity::PolynomialTerms))
    );
    assert_eq!(
        p.pow(0, no_terms, &mut work()),
        Err(Error::Capacity(Capacity::PolynomialTerms))
    );
    let no_factors = PolynomialLimits {
        factors: 0,
        ..limits()
    };
    assert_eq!(
        zero.substitute(&[(P, p.clone())], no_factors, &mut work()),
        Err(Error::Capacity(Capacity::PolynomialFactors))
    );
    assert_eq!(
        Poly::from_terms(&[term(0, 1, &[(P, 1)])], no_factors, &mut work()),
        Err(Error::Capacity(Capacity::PolynomialFactors))
    );
    let degree = PolynomialLimits {
        degree: 1,
        ..limits()
    };
    assert_eq!(
        p.pow(2, degree, &mut work()),
        Err(Error::Capacity(Capacity::PolynomialDegree))
    );
    assert_eq!(
        Poly::from_terms(
            &[term(0, 1, &[(P, u32::MAX), (Q, 1)])],
            PolynomialLimits {
                degree: u32::MAX,
                ..limits()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::PolynomialDegree))
    );
    let no_steps = PolynomialLimits {
        steps: 0,
        ..limits()
    };
    assert_eq!(
        zero.add(&zero, no_steps, &mut work()),
        Err(Error::Capacity(Capacity::PolynomialSteps))
    );
    let p_bytes = p.to_bytes(limits(), &mut work()).unwrap();
    let bytes = PolynomialLimits {
        bytes: p_bytes.len() - 1,
        ..limits()
    };
    assert_eq!(
        p.to_bytes(bytes, &mut work()),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    );
    assert_eq!(
        Poly::from_bytes(&p_bytes, bytes, &mut work()),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    );
    let huge = Rat::from(1024u64);
    let mut tiny = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 2,
            operations: 1000,
        },
        &(),
    );
    assert_eq!(
        zero.evaluate(&[(P, huge.clone())], limits(), &mut tiny),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    );
    assert_eq!(
        huge.pow(0, &mut tiny),
        Err(Error::Capacity(Capacity::ArithmeticBits))
    );
}

#[test]
fn cancellation_and_high_degree_moments_obey_work_limits() {
    let p = Poly::parameter(P);
    for remaining in 0..20 {
        let stop = Stop(Cell::new(remaining));
        let mut w = ExactArithmetic::new(ArithmeticLimits::default(), &stop);
        assert_eq!(
            add(&p, &Poly::one()).pow(10, limits(), &mut w),
            Err(Error::Cancelled)
        );
    }
    // High-degree moments honor structural work budgets before a long loop.
    let high = poly(&[term(1, 1, &[(P, 50_000)])]);
    assert_eq!(
        high.integrate_beta(
            P,
            &Rat::one(),
            &Rat::one(),
            PolynomialLimits {
                steps: 100,
                ..limits()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::PolynomialSteps))
    );
}
