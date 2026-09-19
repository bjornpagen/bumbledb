#![allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
use crate::*;
use std::cmp::Ordering;

const P: ParameterId = ParameterId([44; 32]);
fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn limits() -> ParameterSourceLimits {
    ParameterSourceLimits::default()
}
fn p() -> ExactPolynomial {
    ExactPolynomial::parameter(P)
}
fn c(n: u64) -> ExactPolynomial {
    ExactPolynomial::constant(ExactRational::from(n))
}
fn sub(a: &ExactPolynomial, b: &ExactPolynomial) -> ExactPolynomial {
    a.sub(b, limits().parameters.region.polynomial, &mut work())
        .unwrap()
}
fn mul(a: &ExactPolynomial, b: &ExactPolynomial) -> ExactPolynomial {
    a.mul(b, limits().parameters.region.polynomial, &mut work())
        .unwrap()
}
fn sign(polynomial: &ExactPolynomial, signs: PolynomialSigns) -> ParameterRegion {
    ParameterRegion::from_polynomial(
        P,
        polynomial,
        signs,
        limits().parameters.region,
        &mut work(),
    )
    .unwrap()
}
fn domain() -> ParameterDomain {
    ParameterDomain::new(
        sign(&p(), PolynomialSigns::NON_NEGATIVE)
            .apply(
                BoolOp4::AND,
                &sign(&sub(&c(1), &p()), PolynomialSigns::NON_NEGATIVE),
                limits().parameters.region,
                &mut work(),
            )
            .unwrap(),
    )
    .unwrap()
}
fn function(
    domain: &ParameterDomain,
    n: ExactPolynomial,
    d: ExactPolynomial,
) -> GuardedRationalFunction {
    GuardedRationalFunction::new(
        domain.clone(),
        n,
        d,
        limits().parameters.region,
        &mut work(),
    )
    .unwrap()
}
fn space(id: u8, bits: u8, guards: &[ParameterGuard]) -> Space {
    Space::new(SpaceId([id; 32]), bits, &())
        .unwrap()
        .with_parameters(domain(), guards, limits(), &mut work())
        .unwrap()
}
fn endpoint_guards() -> Vec<ParameterGuard> {
    vec![
        ParameterGuard {
            coordinate: 2,
            region: sign(&p(), PolynomialSigns::ZERO),
        },
        ParameterGuard {
            coordinate: 3,
            region: sign(&sub(&p(), &c(1)), PolynomialSigns::ZERO),
        },
    ]
}
fn and(a: &Event, b: &Event) -> Event {
    a.apply(BoolOp4::AND, b, &()).unwrap()
}
fn ratio(n: &str, d: &str) -> ExactRational {
    ExactRational::fraction(n, d, &mut work()).unwrap()
}
fn value(f: &ParameterFunction, p: ExactRational) -> Option<ExactRational> {
    f.value_at(
        &p,
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}
fn shared_bias() -> Space {
    let raw = space(201, 4, &endpoint_guards());
    let a = raw.coordinate(0, &()).unwrap();
    let b = raw.coordinate(1, &()).unwrap();
    let q = sub(&c(1), &p());
    let pieces = [
        (and(&a.complement(), &b.complement()), mul(&q, &q)),
        (and(&a, &b.complement()), mul(&p(), &q)),
        (and(&a.complement(), &b), mul(&p(), &q)),
        (and(&a, &b), mul(&p(), &p())),
    ]
    .into_iter()
    .map(|(region, n)| ParameterDensityPiece {
        region,
        density: function(raw.parameter_domain().unwrap(), n, c(1)),
    })
    .collect::<Vec<_>>();
    raw.with_parameter_density(&pieces, limits(), &mut work())
        .unwrap()
}

#[test]
fn sealed_guards_remove_impossible_codes_without_assigning_mass() {
    let s = space(200, 4, &endpoint_guards());
    assert_eq!(s.guard_coordinates(), 12);
    assert_eq!(s.outcome_coordinates(), 3);
    assert_eq!(s.full().atom_count(&()).unwrap(), 12);
    assert_eq!(
        s.full().world_cardinality(&()).unwrap(),
        WorldCardinality::Continuum
    );
    assert_eq!(s.full().count(&()), Err(Error::InfiniteWorlds));
    let zero = s.coordinate(2, &()).unwrap();
    let one = s.coordinate(3, &()).unwrap();
    assert!(and(&zero, &one).is_empty());
    assert_eq!(zero.count(&()).unwrap(), 4);
    assert_eq!(one.count(&()).unwrap(), 4);
    assert_eq!(s.empty().count(&()).unwrap(), 0);
    for n in 0..=4 {
        let w = ParameterWorld {
            parameter: RealWitness::Rational(ratio(&n.to_string(), "4")),
            outcomes: 1,
        };
        assert!(
            s.full()
                .contains_parameter(&w, limits(), &mut work())
                .unwrap()
        );
        assert_eq!(
            zero.contains_parameter(&w, limits(), &mut work()).unwrap(),
            n == 0
        );
        assert_eq!(
            one.contains_parameter(&w, limits(), &mut work()).unwrap(),
            n == 4
        );
    }
    assert!(matches!(
        s.full().contains_parameter(
            &ParameterWorld {
                parameter: RealWitness::Rational(ExactRational::from(2u64)),
                outcomes: 0
            },
            limits(),
            &mut work()
        ),
        Err(Error::IllegalParameter)
    ));
    assert_eq!(zero.saturate(4, &()), Err(Error::ParameterGuardElimination));
    assert_eq!(zero.saturate(3, &()).unwrap(), zero);
    let witness = zero
        .parameter_witness(limits(), &mut work())
        .unwrap()
        .unwrap();
    assert!(
        zero.contains_parameter(&witness, limits(), &mut work())
            .unwrap()
    );
    let raw = Space::new(SpaceId([202; 32]), 2, &()).unwrap();
    let keep = raw.coordinate(1, &()).unwrap().complement();
    let incomplete = raw.restrict(&keep, &()).unwrap();
    let guard = ParameterGuard {
        coordinate: 1,
        region: sign(&p(), PolynomialSigns::ZERO),
    };
    assert!(matches!(
        incomplete.with_parameters(domain(), &[guard], limits(), &mut work()),
        Err(Error::EmptyParameterFibre)
    ));
}

#[test]
fn shared_bias_measurement_retains_evidence_endpoints_and_zero_mass_possibility() {
    let s = shared_bias();
    assert!(s.is_measured());
    let a = s.coordinate(0, &()).unwrap();
    let b = s.coordinate(1, &()).unwrap();
    let different = a.apply(BoolOp4::XOR, &b, &()).unwrap();
    let observation = a
        .parameter_probability(&different, limits(), &mut work())
        .unwrap();
    assert!(!observation.is_impossible());
    assert_eq!(
        observation
            .value_at(&ExactRational::zero(), limits(), &mut work())
            .unwrap(),
        None
    );
    assert_eq!(
        observation
            .value_at(&ExactRational::one(), limits(), &mut work())
            .unwrap(),
        None
    );
    for n in 1..8 {
        assert_eq!(
            observation
                .value_at(&ratio(&n.to_string(), "8"), limits(), &mut work())
                .unwrap(),
            Some(ratio("1", "2"))
        );
    }
    let full_mass = s.full().parameter_mass(limits(), &mut work()).unwrap();
    let a_mass = a.parameter_mass(limits(), &mut work()).unwrap();
    let not_mass = a
        .complement()
        .parameter_mass(limits(), &mut work())
        .unwrap();
    for n in 0..=8 {
        let at = ratio(&n.to_string(), "8");
        assert_eq!(value(&full_mass, at.clone()), Some(ExactRational::one()));
        assert_eq!(value(&a_mass, at.clone()), Some(at.clone()));
        assert_eq!(value(&not_mass, at), Some(ratio(&(8 - n).to_string(), "8")));
    }
    let zero_mass = and(&a, &s.coordinate(2, &()).unwrap());
    assert!(!zero_mass.is_empty());
    assert_eq!(zero_mass.count(&()).unwrap(), 2);
    let impossible = a
        .parameter_probability(&zero_mass, limits(), &mut work())
        .unwrap();
    assert!(impossible.is_impossible());
    assert_eq!(
        s.full().mass(&mut work()),
        Err(Error::ParameterizedMeasurement)
    );
    assert!(!s.unmeasured(&()).unwrap().is_measured());
    drop((s, a, b, different));
    assert_eq!(
        observation
            .value_at(&ratio("1", "3"), limits(), &mut work())
            .unwrap(),
        Some(ratio("1", "2"))
    );
}

#[test]
fn law_admission_checks_every_parameter_fibre_and_skipped_outcome() {
    let s = space(
        203,
        3,
        &[ParameterGuard {
            coordinate: 2,
            region: sign(&p(), PolynomialSigns::ZERO),
        }],
    );
    let half = function(s.parameter_domain().unwrap(), c(1), c(2));
    assert!(matches!(
        s.with_parameter_density(
            &[ParameterDensityPiece {
                region: s.full(),
                density: half
            }],
            limits(),
            &mut work()
        ),
        Err(Error::LawNotNormalized)
    ));
    // Two skipped outcome coordinates need density 1/4, never 1/8 for guards.
    let quarter = function(s.parameter_domain().unwrap(), c(1), c(4));
    let measured = s
        .with_parameter_density(
            &[ParameterDensityPiece {
                region: s.full(),
                density: quarter.clone(),
            }],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert_eq!(
        value(
            &measured
                .full()
                .parameter_mass(limits(), &mut work())
                .unwrap(),
            ratio("1", "2")
        ),
        Some(ExactRational::one())
    );
    let rational = s
        .with_density(
            &[DensityPiece {
                region: s.full(),
                density: ratio("1", "4"),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    assert!(rational.is_measured());
    let hole = function(s.parameter_domain().unwrap(), p(), mul(&c(4), &p()));
    assert!(matches!(
        s.with_parameter_density(
            &[ParameterDensityPiece {
                region: s.full(),
                density: hole.clone()
            }],
            limits(),
            &mut work()
        ),
        Err(Error::UndefinedDensity)
    ));
    let at_zero = s.coordinate(2, &()).unwrap();
    let combined = s
        .with_parameter_density(
            &[
                ParameterDensityPiece {
                    region: at_zero.clone(),
                    density: quarter,
                },
                ParameterDensityPiece {
                    region: at_zero.complement(),
                    density: hole,
                },
            ],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert_eq!(
        value(
            &combined
                .full()
                .parameter_mass(limits(), &mut work())
                .unwrap(),
            ExactRational::zero()
        ),
        Some(ExactRational::one())
    );
    let negative = function(s.parameter_domain().unwrap(), sub(&p(), &c(1)), c(4));
    assert!(matches!(
        s.with_parameter_density(
            &[ParameterDensityPiece {
                region: s.full(),
                density: negative
            }],
            limits(),
            &mut work()
        ),
        Err(Error::NegativeMass)
    ));
    // Normalization must hold at endpoints too, not just generic interior points.
    let bad_zero = function(s.parameter_domain().unwrap(), c(1), c(2));
    let good = function(s.parameter_domain().unwrap(), c(1), c(4));
    assert!(matches!(
        s.with_parameter_density(
            &[
                ParameterDensityPiece {
                    region: at_zero.clone(),
                    density: bad_zero
                },
                ParameterDensityPiece {
                    region: at_zero.complement(),
                    density: good
                },
            ],
            limits(),
            &mut work()
        ),
        Err(Error::LawNotNormalized)
    ));
}

#[test]
fn structural_restriction_captures_the_feasible_parameter_domain() {
    let s = shared_bias();
    let at_zero = s.coordinate(2, &()).unwrap();
    let restricted = s.restrict(&at_zero, &()).unwrap();
    assert!(!restricted.is_measured());
    assert_eq!(restricted.full().count(&()).unwrap(), 4);
    assert!(
        restricted
            .parameter_domain()
            .unwrap()
            .region()
            .equivalent(
                &sign(&p(), PolynomialSigns::ZERO),
                limits().parameters.region,
                &mut work()
            )
            .unwrap()
    );
    assert!(at_zero.in_space(&restricted, &()).unwrap().is_full());
    assert!(restricted.full().in_space(&s, &()).is_err());
    assert!(
        s.full()
            .in_space(&Space::new(s.identity(), s.dimensions(), &()).unwrap(), &())
            .is_err()
    );
    let bytes = restricted.full().to_bytes(&()).unwrap();
    let reopened = Event::from_bytes(&bytes, &()).unwrap();
    assert_eq!(reopened.count(&()).unwrap(), 4);
    assert_eq!(reopened.to_bytes(&()).unwrap(), bytes);
}

#[test]
fn parameterized_events_roundtrip_align_and_retain_owners() {
    let s = shared_bias();
    let event = s.coordinate(0, &()).unwrap();
    let bytes = event.to_bytes(&()).unwrap();
    assert_eq!(&bytes[..5], b"BEVT\x03");
    let hex = include_str!("../tests/fixtures/event-v3-parameter-source.hex").trim();
    let fixture: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(bytes, fixture);
    let restored =
        Event::from_bytes_with_order(&bytes, Some(&[3, 2, 1, 0]), Limits::default(), &()).unwrap();
    assert_eq!(restored.to_bytes(&()).unwrap(), bytes);
    assert_eq!(restored.align_to(&s, &()).unwrap(), event);
    assert_eq!(
        event
            .diagram(&())
            .unwrap()
            .rebuild(&restored.space(), &())
            .unwrap(),
        restored
    );
    let mut registry = Registry::default();
    let first = registry.intern(&event, &()).unwrap();
    let second = registry.decode(&bytes, &()).unwrap();
    assert_eq!(first, second);
    let observed = second
        .parameter_probability(&second.space().full(), limits(), &mut work())
        .unwrap();
    drop((s, event, restored, first, second, registry));
    assert_eq!(
        observed
            .value_at(&ratio("2", "3"), limits(), &mut work())
            .unwrap(),
        Some(ratio("2", "3"))
    );
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(Event::from_bytes(&bad, &()).is_err());
    let narrow = ParameterSourceLimits {
        fibres: 2,
        ..limits()
    };
    assert!(matches!(
        Event::from_bytes_with_parameter_limits(
            &bytes,
            None,
            Limits::default(),
            narrow,
            &mut work()
        ),
        Err(Error::Capacity(Capacity::ParameterSourceCells))
    ));
}

#[test]
fn parameter_preserving_maps_and_products_keep_the_actual_shared_value() {
    let guard = sign(&p(), PolynomialSigns::ZERO);
    let a = space(
        204,
        2,
        &[ParameterGuard {
            coordinate: 1,
            region: guard.clone(),
        }],
    );
    let env = space(
        205,
        1,
        &[ParameterGuard {
            coordinate: 0,
            region: guard.clone(),
        }],
    );
    let map = CoordinateMap::coordinates(&a, &env, &[1], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    assert!(matches!(
        CoordinateMap::new(&a, &env, &[a.coordinate(1, &()).unwrap().complement()], &()),
        Err(Error::ParameterGuardMismatch)
    ));
    let opposite = space(
        206,
        1,
        &[ParameterGuard {
            coordinate: 0,
            region: guard.complement(),
        }],
    );
    let reverse = CoordinateMap::new(
        &a,
        &opposite,
        &[a.coordinate(1, &()).unwrap().complement()],
        &(),
    )
    .unwrap();
    assert_eq!(
        reverse.image(&a.coordinate(0, &()).unwrap(), &()).unwrap(),
        opposite.full()
    );
    let changed = space(
        207,
        1,
        &[ParameterGuard {
            coordinate: 0,
            region: sign(&sub(&p(), &c(1)), PolynomialSigns::ZERO),
        }],
    );
    assert!(matches!(
        CoordinateMap::coordinates(&a, &changed, &[1], &()),
        Err(Error::ParameterRefinementRequired)
    ));
    let finite = Space::new(SpaceId([208; 32]), 1, &()).unwrap();
    assert!(matches!(
        CoordinateMap::coordinates(&a, &finite, &[1], &()),
        Err(Error::ParameterScopeMismatch)
    ));
    let pair = FibreProduct::new(SpaceId([209; 32]), &map, &map, &()).unwrap();
    assert!(pair.space().parameter_domain().is_some());
    assert!(!pair.space().is_measured());
    assert_eq!(pair.space().full().atom_count(&()).unwrap(), 8);
    assert_eq!(
        pair.space().full().world_cardinality(&()).unwrap(),
        WorldCardinality::Continuum
    );
    let at_zero = pair
        .left()
        .map()
        .pullback(&a.coordinate(1, &()).unwrap(), &())
        .unwrap();
    assert_eq!(at_zero.count(&()).unwrap(), 4);
    let a_head = pair
        .left()
        .map()
        .pullback(&a.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let b_head = pair
        .right()
        .map()
        .pullback(&a.coordinate(0, &()).unwrap(), &())
        .unwrap();
    assert_eq!(and(&and(&a_head, &b_head), &at_zero).count(&()).unwrap(), 1);
    // Weighted images count each outcome once; redundant guards add no mass.
    let one = FiniteFunction::constant(
        &a,
        ExactRational::one(),
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap();
    let rows = one
        .pushforward(map.map(), FunctionLimits::default(), &mut work())
        .unwrap();
    assert_eq!(rows.at(0, &()).unwrap(), ExactRational::from(2u64));
    assert_eq!(rows.at(1, &()).unwrap(), ExactRational::from(2u64));
    let bytes = Descriptor::capture(
        &AdmittedDescriptor::Fibre(pair.clone()),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap()
    .to_bytes(DescriptorLimits::default(), &())
    .unwrap();
    let AdmittedDescriptor::Fibre(restored) =
        Descriptor::import(&bytes, DescriptorLimits::default(), &()).unwrap()
    else {
        panic!("fibre descriptor")
    };
    assert_eq!(
        restored.space().full().to_bytes(&()).unwrap(),
        pair.space().full().to_bytes(&()).unwrap()
    );
}

#[test]
fn irrational_finite_domains_and_continuous_fixed_points_use_distinct_counts() {
    let equation = sub(&mul(&c(2), &mul(&p(), &p())), &c(1));
    let singleton = sign(&equation, PolynomialSigns::ZERO)
        .apply(
            BoolOp4::AND,
            &sign(&p(), PolynomialSigns::POSITIVE),
            limits().parameters.region,
            &mut work(),
        )
        .unwrap();
    let source = Space::new(SpaceId([210; 32]), 1, &())
        .unwrap()
        .with_parameters(
            ParameterDomain::new(singleton).unwrap(),
            &[],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert_eq!(source.full().count(&()).unwrap(), 2);
    let witness = source
        .full()
        .parameter_witness(limits(), &mut work())
        .unwrap()
        .unwrap();
    let RealWitness::Algebraic(root) = &witness.parameter else {
        panic!("irrational witness required")
    };
    let renamed = equation
        .substitute(
            &[(P, ExactPolynomial::parameter(root.parameter()))],
            limits().parameters.region.polynomial,
            &mut work(),
        )
        .unwrap();
    assert_eq!(
        root.sign(&renamed, RootLimits::default(), &mut work())
            .unwrap(),
        Ordering::Equal
    );
    assert!(
        source
            .full()
            .contains_parameter(&witness, limits(), &mut work())
            .unwrap()
    );
    let continuous = space(211, 1, &[]);
    let mut builder = EventProgramBuilder::new(&continuous, &()).unwrap();
    let input = builder.input();
    let goal = builder
        .constant(&continuous.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let body = builder.apply(BoolOp4::OR, &input, &goal, &()).unwrap();
    let fixed = builder
        .finish(&body, &())
        .unwrap()
        .fixed_points(&())
        .unwrap();
    assert_eq!(fixed.carrier().atoms(), 2);
    assert_eq!(
        fixed
            .least(FixedPointLimits::default(), &())
            .unwrap()
            .event(),
        &continuous.coordinate(0, &()).unwrap()
    );
}

#[test]
fn piecewise_functions_compare_values_and_keep_all_inherited_holes() {
    let d = domain();
    let middle = sub(&mul(&c(2), &p()), &c(1));
    let left = sign(&middle, PolynomialSigns::NON_POSITIVE);
    let right = left.complement();
    let rising = function(&d, p(), c(1));
    let falling = function(&d, sub(&c(1), &p()), c(1));
    let triangle = ParameterFunction::new(
        d.clone(),
        &[
            rising
                .restrict(&left, limits().parameters.region, &mut work())
                .unwrap(),
            falling
                .restrict(&right, limits().parameters.region, &mut work())
                .unwrap(),
        ],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    let quarter = sign(&sub(&mul(&c(4), &p()), &c(1)), PolynomialSigns::NEGATIVE);
    let split_left = left
        .apply(
            BoolOp4::AND,
            &quarter.complement(),
            limits().parameters.region,
            &mut work(),
        )
        .unwrap();
    let refined = ParameterFunction::new(
        d.clone(),
        &[
            rising
                .restrict(&quarter, limits().parameters.region, &mut work())
                .unwrap(),
            rising
                .restrict(&split_left, limits().parameters.region, &mut work())
                .unwrap(),
            falling
                .restrict(&right, limits().parameters.region, &mut work())
                .unwrap(),
        ],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    assert!(
        triangle
            .equivalent(
                &refined,
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap()
    );
    let doubled = triangle
        .add(
            &refined,
            limits().parameters.region,
            limits().functions,
            &mut work(),
        )
        .unwrap();
    for n in 0..=8 {
        let numerator = n.min(8 - n);
        assert_eq!(
            value(&triangle, ratio(&n.to_string(), "8")),
            Some(ratio(&numerator.to_string(), "8"))
        );
        assert_eq!(
            value(&doubled, ratio(&n.to_string(), "8")),
            Some(ratio(&numerator.to_string(), "4"))
        );
    }
    let partial = ParameterFunction::new(
        d.clone(),
        &[function(&d, c(1), middle)],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    let zero = ParameterFunction::new(
        d.clone(),
        &[function(&d, c(0), c(1))],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    let product = partial
        .mul(
            &zero,
            limits().parameters.region,
            limits().functions,
            &mut work(),
        )
        .unwrap();
    assert_eq!(value(&product, ratio("1", "2")), None);
    assert_eq!(
        value(&product, ratio("1", "3")),
        Some(ExactRational::zero())
    );
    assert!(
        !product
            .equivalent(
                &zero,
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap()
    );
    assert!(matches!(
        ParameterFunction::new(
            d,
            &[rising.clone(), rising],
            limits().parameters.region,
            limits().functions,
            &mut work()
        ),
        Err(Error::FunctionOverlap)
    ));
}

#[test]
fn semantic_cardinality_does_not_overflow_a_continuum_and_source_refusal_is_explicit() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }

    let mut region = sign(&p(), PolynomialSigns::NON_NEGATIVE);
    for n in 1..=4 {
        let singleton = sign(
            &p().add(&c(n), limits().parameters.region.polynomial, &mut work())
                .unwrap(),
            PolynomialSigns::ZERO,
        );
        region = region
            .apply(
                BoolOp4::OR,
                &singleton,
                limits().parameters.region,
                &mut work(),
            )
            .unwrap();
    }
    let wide = Space::new(SpaceId([213; 32]), 62, &())
        .unwrap()
        .with_parameters(
            ParameterDomain::new(region).unwrap(),
            &[],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert_eq!(
        wide.full().world_cardinality(&()).unwrap(),
        WorldCardinality::Continuum
    );
    assert_eq!(wide.full().atom_count(&()).unwrap(), 1u64 << 62);
    let raw = Space::new(SpaceId([214; 32]), 2, &()).unwrap();
    let duplicate = ParameterGuard {
        coordinate: 1,
        region: sign(&p(), PolynomialSigns::ZERO),
    };
    assert!(matches!(
        raw.with_parameters(
            domain(),
            &[duplicate.clone(), duplicate],
            limits(),
            &mut work()
        ),
        Err(Error::InvalidOrder)
    ));
    let foreign = ParameterGuard {
        coordinate: 1,
        region: ParameterRegion::empty(ParameterId([99; 32])),
    };
    assert!(matches!(
        raw.with_parameters(domain(), &[foreign], limits(), &mut work()),
        Err(Error::ParameterScopeMismatch)
    ));
    assert!(matches!(
        raw.with_parameters(
            domain(),
            &[],
            limits(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
}

#[test]
fn source_wire_recomputes_guard_feasibility_and_law_normalization() {
    fn blob(bytes: &[u8], at: usize) -> (std::ops::Range<usize>, usize) {
        let n = usize::try_from(u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())).unwrap();
        (at + 8..at + 8 + n, at + 8 + n)
    }
    fn push(bytes: &mut Vec<u8>, part: &[u8]) {
        bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
        bytes.extend_from_slice(part);
    }
    let s = shared_bias();
    let original = s.full().to_bytes(&()).unwrap();
    let (_, after_event) = blob(&original, 5);
    let (context, after_context) = blob(&original, after_event);
    let (law, _) = blob(&original, after_context);
    let unconstrained = Space::new(s.identity(), 4, &())
        .unwrap()
        .full()
        .to_bytes(&())
        .unwrap();
    let mut false_support = b"BEVT\x03".to_vec();
    push(&mut false_support, &unconstrained);
    push(&mut false_support, &original[context.clone()]);
    push(&mut false_support, &[]);
    assert!(matches!(
        Event::from_bytes(&false_support, &()),
        Err(Error::InvalidEncoding)
    ));
    let (_, after_domain) = blob(&original, context.start + 5);
    let first_coordinate = after_domain + 8;
    let (_, after_first_guard) = blob(&original, first_coordinate + 1);
    let mut duplicate = original.clone();
    duplicate[after_first_guard] = duplicate[first_coordinate];
    assert!(matches!(
        Event::from_bytes(&duplicate, &()),
        Err(Error::InvalidOrder)
    ));
    let (function, _) = blob(&original, law.start + 13);
    let (_, after_numerator) = blob(&original, function.start + 5);
    let (denominator, _) = blob(&original, after_numerator);
    let replacement = c(2)
        .to_bytes(limits().parameters.region.polynomial, &mut work())
        .unwrap();
    assert_eq!(replacement.len(), denominator.len());
    let mut unnormalized = original.clone();
    unnormalized[denominator].copy_from_slice(&replacement);
    assert!(matches!(
        Event::from_bytes(&unnormalized, &()),
        Err(Error::LawNotNormalized)
    ));
    for n in [
        0,
        4,
        5,
        12,
        after_event - 1,
        after_context - 1,
        original.len() - 1,
    ] {
        assert!(Event::from_bytes(&original[..n], &()).is_err());
    }
}
