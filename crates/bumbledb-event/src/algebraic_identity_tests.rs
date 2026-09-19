use std::cell::Cell;
use std::cmp::Ordering;

use crate::{
    AlgebraicLimits, AlgebraicRoot, ArithmeticLimits, Capacity, Control, Error, ExactArithmetic,
    ExactPolynomial as Poly, ExactRational as Rat, ParameterId, PolynomialLimits, PolynomialTerm,
    RootLimits,
};

const X: ParameterId = ParameterId([14; 32]);
const Y: ParameterId = ParameterId([15; 32]);
fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn poly(coefficients: &[i64]) -> Poly {
    let terms: Vec<_> = coefficients
        .iter()
        .enumerate()
        .map(|(i, &value)| PolynomialTerm {
            coefficient: Rat::from(value),
            powers: if i == 0 {
                Box::new([])
            } else {
                Box::new([(X, u32::try_from(i).unwrap())])
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
fn encode(root: &AlgebraicRoot) -> Vec<u8> {
    root.to_bytes(AlgebraicLimits::default(), &mut work())
        .unwrap()
}
fn bare(polynomial: &Poly, ordinal: u64) -> Vec<u8> {
    let polynomial = polynomial
        .substitute(
            &[(X, Poly::parameter(ParameterId([0; 32])))],
            PolynomialLimits::default(),
            &mut work(),
        )
        .unwrap()
        .to_bytes(PolynomialLimits::default(), &mut work())
        .unwrap();
    let mut out = b"BEAR\x01".to_vec();
    out.extend_from_slice(&(polynomial.len() as u64).to_le_bytes());
    out.extend_from_slice(&polynomial);
    out.extend_from_slice(&ordinal.to_le_bytes());
    out
}

#[test]
fn canonical_real_identity_discards_factors_multiplicity_scale_name_and_interval() {
    let minimal = poly(&[-2, 0, 1]);
    let expected = encode(&roots(&minimal)[1]);
    let mixed = mul(&minimal, &poly(&[-3, 0, 1]));
    for description in [
        minimal.clone(),
        mixed.clone(),
        mul(&mixed, &minimal),
        mul(&minimal, &poly(&[3, 1])),
        mul(&minimal, &poly(&[-3])),
    ] {
        let root = AlgebraicRoot::from_interval(
            &description,
            X,
            Rat::fraction("7", "5", &mut work()).unwrap(),
            Rat::fraction("3", "2", &mut work()).unwrap(),
            RootLimits::default(),
            &mut work(),
        )
        .unwrap();
        assert_eq!(
            root.minimal_polynomial(AlgebraicLimits::default(), &mut work())
                .unwrap(),
            minimal
        );
        assert_eq!(encode(&root), expected);
        let renamed = description
            .substitute(
                &[(X, Poly::parameter(Y))],
                PolynomialLimits::default(),
                &mut work(),
            )
            .unwrap();
        let root = AlgebraicRoot::from_interval(
            &renamed,
            Y,
            root.interval().0.clone(),
            root.interval().1.clone(),
            RootLimits::default(),
            &mut work(),
        )
        .unwrap();
        assert_eq!(encode(&root), expected);
    }
    assert_ne!(encode(&roots(&minimal)[0]), expected);
    assert_eq!(expected, bare(&minimal, 1));
    let decoded =
        AlgebraicRoot::from_bytes(&expected, AlgebraicLimits::default(), &mut work()).unwrap();
    assert_eq!(
        decoded
            .compare(&roots(&minimal)[1], RootLimits::default(), &mut work())
            .unwrap(),
        Ordering::Equal
    );
    assert_eq!(encode(&decoded), expected);
}

#[test]
fn rational_roots_and_higher_degree_irreducibles_have_unique_minimal_identity() {
    for n in -4..=4 {
        for d in 1..=3 {
            let linear = poly(&[-n, d]);
            let p = mul(&mul(&linear, &poly(&[-2, 0, 1])), &linear);
            let value = Rat::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap();
            let root = AlgebraicRoot::from_interval(
                &p,
                X,
                value.clone(),
                value,
                RootLimits::default(),
                &mut work(),
            )
            .unwrap();
            assert_eq!(encode(&root), encode(&roots(&linear)[0]));
        }
    }
    // Rational root theorem for the cubics; x^4-x-1 is irreducible over Q.
    for minimal in [
        poly(&[-2, 0, 0, 1]),
        poly(&[1, -3, 0, 1]),
        poly(&[-1, -1, 0, 0, 1]),
    ] {
        for root in roots(&minimal) {
            assert_eq!(
                root.minimal_polynomial(AlgebraicLimits::default(), &mut work())
                    .unwrap(),
                minimal
            );
            let bytes = encode(&root);
            let decoded =
                AlgebraicRoot::from_bytes(&bytes, AlgebraicLimits::default(), &mut work()).unwrap();
            assert_eq!(
                decoded
                    .compare(&root, RootLimits::default(), &mut work())
                    .unwrap(),
                Ordering::Equal
            );
        }
    }
}

#[test]
fn canonical_real_decoder_refuses_nonminimal_and_noncanonical_claims() {
    let minimal = poly(&[-2, 0, 1]);
    for p in [
        mul(&minimal, &poly(&[-3, 0, 1])),
        mul(&minimal, &minimal),
        mul(&minimal, &poly(&[2])),
    ] {
        assert!(matches!(
            AlgebraicRoot::from_bytes(&bare(&p, 0), AlgebraicLimits::default(), &mut work()),
            Err(Error::InvalidEncoding)
        ));
    }
    for ordinal in [2, u64::MAX] {
        assert!(
            AlgebraicRoot::from_bytes(
                &bare(&minimal, ordinal),
                AlgebraicLimits::default(),
                &mut work()
            )
            .is_err()
        );
    }
    let bytes = bare(&minimal, 1);
    for size in 0..bytes.len() {
        assert!(
            AlgebraicRoot::from_bytes(&bytes[..size], AlgebraicLimits::default(), &mut work())
                .is_err()
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(AlgebraicRoot::from_bytes(&trailing, AlgebraicLimits::default(), &mut work()).is_err());
    let mut version = bytes;
    version[4] = 99;
    assert!(matches!(
        AlgebraicRoot::from_bytes(&version, AlgebraicLimits::default(), &mut work()),
        Err(Error::UnsupportedVersion(99))
    ));
}

#[test]
fn unfinished_factor_search_is_never_an_irreducibility_certificate() {
    struct Cancel(Cell<usize>);
    impl Control for Cancel {
        fn checkpoint(&self) -> crate::Result<()> {
            let next = self.0.get() + 1;
            self.0.set(next);
            if next >= 60 {
                Err(Error::Cancelled)
            } else {
                Ok(())
            }
        }
    }

    let root = roots(&poly(&[-2, 0, 1])).pop().unwrap();
    for (limits, capacity) in [
        (
            AlgebraicLimits {
                candidates: 0,
                ..AlgebraicLimits::default()
            },
            Capacity::AlgebraicCandidates,
        ),
        (
            AlgebraicLimits {
                steps: 0,
                ..AlgebraicLimits::default()
            },
            Capacity::AlgebraicIdentitySteps,
        ),
        (
            AlgebraicLimits {
                bytes: 1,
                ..AlgebraicLimits::default()
            },
            Capacity::DescriptorBytes,
        ),
    ] {
        assert!(
            matches!(root.to_bytes(limits, &mut work()), Err(Error::Capacity(actual)) if actual == capacity)
        );
    }
    let large = roots(&poly(&[-1_000_000_007, 0, 1])).pop().unwrap();
    assert!(matches!(
        large.to_bytes(
            AlgebraicLimits {
                steps: 40,
                ..AlgebraicLimits::default()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::AlgebraicIdentitySteps))
    ));
    let cancel = Cancel(Cell::new(0));
    let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &cancel);
    assert!(matches!(
        root.to_bytes(AlgebraicLimits::default(), &mut work),
        Err(Error::Cancelled)
    ));
    assert!(cancel.0.get() >= 60);
}

#[test]
fn parameter_wire_uses_numeric_boundaries_and_keeps_named_scope() {
    use crate::{
        BoolOp4, ParameterCodecLimits, ParameterDomain, ParameterLimits, ParameterRegion,
        PolynomialSigns,
    };
    let limits = ParameterCodecLimits::default();
    let p = poly(&[-2, 0, 1]);
    // The positive factor changes the presentation, never its sign set.
    let alternative = mul(&p, &poly(&[1, 0, 1]));
    for bits in 0..8 {
        let signs = PolynomialSigns::new(bits).unwrap();
        let a = ParameterRegion::from_polynomial(X, &p, signs, limits.region, &mut work()).unwrap();
        let b =
            ParameterRegion::from_polynomial(X, &alternative, signs, limits.region, &mut work())
                .unwrap();
        let bytes = a.to_bytes(limits, &mut work()).unwrap();
        assert_eq!(b.to_bytes(limits, &mut work()).unwrap(), bytes);
        let decoded = ParameterRegion::from_bytes(&bytes, limits, &mut work()).unwrap();
        assert!(a.equivalent(&decoded, limits.region, &mut work()).unwrap());
        assert_eq!(decoded.to_bytes(limits, &mut work()).unwrap(), bytes);
        assert_eq!(
            a.complement().to_bytes(limits, &mut work()).unwrap(),
            decoded.complement().to_bytes(limits, &mut work()).unwrap()
        );
        if a.is_empty() {
            assert!(matches!(
                ParameterDomain::from_bytes(&bytes, limits, &mut work()),
                Err(Error::EmptyParameterDomain)
            ));
        } else {
            assert_eq!(
                ParameterDomain::from_bytes(&bytes, limits, &mut work())
                    .unwrap()
                    .to_bytes(limits, &mut work())
                    .unwrap(),
                bytes
            );
        }
    }
    let full = ParameterRegion::full(X);
    assert_ne!(
        full.to_bytes(limits, &mut work()).unwrap(),
        ParameterRegion::full(Y)
            .to_bytes(limits, &mut work())
            .unwrap()
    );
    let singleton =
        ParameterRegion::from_polynomial(X, &p, PolynomialSigns::ZERO, limits.region, &mut work())
            .unwrap();
    let nonnegative = ParameterRegion::from_polynomial(
        X,
        &poly(&[0, 1]),
        PolynomialSigns::NON_NEGATIVE,
        limits.region,
        &mut work(),
    )
    .unwrap();
    let positive_root = singleton
        .apply(BoolOp4::AND, &nonnegative, limits.region, &mut work())
        .unwrap();
    let bytes = positive_root.to_bytes(limits, &mut work()).unwrap();
    let decoded = ParameterRegion::from_bytes(&bytes, limits, &mut work()).unwrap();
    assert!(matches!(
        decoded
            .witness(ParameterLimits::default(), &mut work())
            .unwrap(),
        Some(crate::RealWitness::Algebraic(_))
    ));
    // Canonical full has no boundaries, regardless of arithmetic construction.
    assert_eq!(
        singleton
            .apply(
                BoolOp4::OR,
                &singleton.complement(),
                limits.region,
                &mut work()
            )
            .unwrap()
            .to_bytes(limits, &mut work())
            .unwrap(),
        full.to_bytes(limits, &mut work()).unwrap()
    );
}

#[test]
fn parameter_wire_checks_boundary_order_necessity_and_every_extent() {
    use crate::{ParameterCodecLimits, ParameterRegion, PolynomialSigns};
    let limits = ParameterCodecLimits::default();
    let set = ParameterRegion::from_polynomial(
        X,
        &poly(&[-2, 0, 1]),
        PolynomialSigns::POSITIVE,
        limits.region,
        &mut work(),
    )
    .unwrap();
    let bytes = set.to_bytes(limits, &mut work()).unwrap();
    for n in 0..bytes.len() {
        assert!(ParameterRegion::from_bytes(&bytes[..n], limits, &mut work()).is_err());
    }
    let first_length =
        usize::try_from(u64::from_le_bytes(bytes[45..53].try_into().unwrap())).unwrap();
    let second_start = 53 + first_length;
    let second_length = usize::try_from(u64::from_le_bytes(
        bytes[second_start..second_start + 8].try_into().unwrap(),
    ))
    .unwrap();
    let cells_start = second_start + 8 + second_length;
    let mut reversed = bytes[..45].to_vec();
    reversed.extend_from_slice(&bytes[second_start..cells_start]);
    reversed.extend_from_slice(&bytes[45..second_start]);
    reversed.extend_from_slice(&bytes[cells_start..]);
    let mut duplicate = bytes[..second_start].to_vec();
    duplicate.extend_from_slice(&bytes[45..second_start]);
    duplicate.extend_from_slice(&bytes[cells_start..]);
    let mut redundant = bytes.clone();
    redundant[cells_start..].fill(1);
    let mut bad_member = bytes.clone();
    bad_member[cells_start] = 2;
    let mut trailing = bytes.clone();
    trailing.push(0);
    let mut bad_length = bytes.clone();
    bad_length[45..53].copy_from_slice(&u64::MAX.to_le_bytes());
    for malformed in [
        reversed, duplicate, redundant, bad_member, trailing, bad_length,
    ] {
        assert!(matches!(
            ParameterRegion::from_bytes(&malformed, limits, &mut work()),
            Err(Error::InvalidEncoding)
        ));
    }
    assert!(matches!(
        ParameterRegion::from_bytes(
            &bytes,
            crate::ParameterCodecLimits { bytes: 1, ..limits },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::DescriptorBytes))
    ));
    let mut narrow = limits;
    narrow.region.cells = 4;
    assert!(matches!(
        ParameterRegion::from_bytes(&bytes, narrow, &mut work()),
        Err(Error::Capacity(Capacity::ParameterCells))
    ));
    narrow = limits;
    narrow.algebraic.candidates = 0;
    assert!(matches!(
        ParameterRegion::from_bytes(&bytes, narrow, &mut work()),
        Err(Error::Capacity(Capacity::AlgebraicCandidates))
    ));
}

#[test]
fn minimal_polynomial_and_root_ordinals_match_independent_exact_oracle() {
    fn rational(raw: &str) -> Rat {
        let (n, d) = raw.split_once('/').unwrap();
        Rat::fraction(n, d, &mut work()).unwrap()
    }
    fn polynomial(raw: &str) -> Poly {
        let terms: Vec<_> = raw
            .split(',')
            .enumerate()
            .map(|(i, c)| PolynomialTerm {
                coefficient: rational(c),
                powers: if i == 0 {
                    Box::new([])
                } else {
                    Box::new([(X, u32::try_from(i).unwrap())])
                },
            })
            .collect();
        Poly::from_terms(&terms, PolynomialLimits::default(), &mut work()).unwrap()
    }
    let mut count = 0;
    for line in include_str!("../tests/fixtures/algebraic-identity-sympy.txt")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let columns: Vec<_> = line.split('|').collect();
        let input = polynomial(columns[0]);
        let (lower, upper) = columns[1].split_once(',').unwrap();
        let root = AlgebraicRoot::from_interval(
            &input,
            X,
            rational(lower),
            rational(upper),
            RootLimits::default(),
            &mut work(),
        )
        .unwrap();
        let minimal = polynomial(columns[2]);
        assert_eq!(
            root.minimal_polynomial(AlgebraicLimits::default(), &mut work())
                .unwrap(),
            minimal,
            "{line}"
        );
        assert_eq!(
            encode(&root),
            bare(&minimal, columns[3].parse().unwrap()),
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 143);
}
