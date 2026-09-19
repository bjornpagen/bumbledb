#![allow(clippy::too_many_lines)]
use crate::parameter_source_tests::{
    c, domain, limits, mul, p, ratio, shared_bias, sign, space, sub, work,
};
use crate::*;

fn value(
    source: &Space,
    numerator: ExactPolynomial,
    denominator: ExactPolynomial,
) -> GuardedRationalFunction {
    GuardedRationalFunction::new(
        source.parameter_domain().unwrap().clone(),
        numerator,
        denominator,
        limits().parameters.region,
        &mut work(),
    )
    .unwrap()
}
fn constant_function(source: &Space, numerator: ExactPolynomial) -> FamilyFunction {
    FamilyFunction::constant(
        source,
        value(source, numerator, c(1)),
        limits(),
        &mut work(),
    )
    .unwrap()
}
fn table(source: &Space, values: &[ExactPolynomial; 4]) -> FamilyFunction {
    let pieces: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(i, v)| FamilyFunctionPiece {
            region: source.table(3, &[1 << i], &()).unwrap(),
            value: value(source, v.clone(), c(1)),
        })
        .collect();
    FamilyFunction::new(source, &pieces, limits(), &mut work()).unwrap()
}
fn function_value(f: &ParameterFunction, n: u64, d: u64) -> Option<ExactRational> {
    f.value_at(
        &ratio(&n.to_string(), &d.to_string()),
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}
fn functions_equal(a: &ParameterFunction, b: &ParameterFunction) -> bool {
    a.equivalent(
        b,
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}

#[test]
fn family_totality_keeps_active_holes_zero_defaults_and_checked_division() {
    let positive = sign(&p(), PolynomialSigns::POSITIVE);
    let source = space(
        180,
        2,
        &[ParameterGuard {
            coordinate: 1,
            region: positive.clone(),
        }],
    );
    let guard = source.coordinate(1, &()).unwrap();
    let reciprocal = value(&source, c(1), p());
    assert!(matches!(
        FamilyFunction::constant(&source, reciprocal.clone(), limits(), &mut work()),
        Err(Error::UndefinedFunction)
    ));
    let f = FamilyFunction::new(
        &source,
        &[FamilyFunctionPiece {
            region: guard.clone(),
            value: reciprocal,
        }],
        limits(),
        &mut work(),
    )
    .unwrap();
    assert_eq!(
        f.value_at(&0u64.into(), 0, limits(), &mut work()).unwrap(),
        Some(0u64.into())
    );
    assert_eq!(
        f.value_at(&ratio("1", "2"), 1, limits(), &mut work())
            .unwrap(),
        Some(2u64.into())
    );
    assert_eq!(
        f.value_at(&2u64.into(), 0, limits(), &mut work()).unwrap(),
        None
    );
    let zero = FamilyFunction::new(&source, &[], limits(), &mut work()).unwrap();
    assert!(
        f.multiply(&zero, limits(), &mut work())
            .unwrap()
            .equivalent(&zero, limits(), &mut work())
            .unwrap()
    );
    assert!(matches!(
        zero.divide(&f, limits(), &mut work()),
        Err(Error::UndefinedFunction)
    ));
    let all_positive = f
        .add(&constant_function(&source, c(1)), limits(), &mut work())
        .unwrap();
    let quotient = all_positive
        .divide(&all_positive, limits(), &mut work())
        .unwrap();
    assert!(
        quotient
            .equivalent(&constant_function(&source, c(1)), limits(), &mut work())
            .unwrap()
    );
    assert!(matches!(
        FamilyFunction::new(
            &source,
            &[
                FamilyFunctionPiece {
                    region: guard.clone(),
                    value: value(&source, c(0), c(1))
                },
                FamilyFunctionPiece {
                    region: guard,
                    value: value(&source, p(), c(1))
                }
            ],
            limits(),
            &mut work()
        ),
        Err(Error::FunctionOverlap)
    ));
    let partial = ParameterFunction::new(
        domain(),
        &[value(&source, c(1), p())],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    assert!(matches!(
        FamilyFunction::from_parameter(&source, &partial, limits(), &mut work()),
        Err(Error::UndefinedFunction)
    ));
}

#[test]
fn family_arithmetic_signs_refine_and_compare_actual_parameter_values() {
    let source = space(181, 1, &[]);
    let f = constant_function(&source, sub(&mul(&c(2), &p()), &c(1)));
    assert!(!f.is_nonnegative(limits(), &mut work()).unwrap());
    assert!(matches!(
        f.where_sign(PolynomialSigns::POSITIVE, limits(), &mut work()),
        Err(Error::ParameterRefinementRequired)
    ));
    let positive = sign(&sub(&mul(&c(2), &p()), &c(1)), PolynomialSigns::POSITIVE);
    let refinement = ParameterRefinement::new(
        SpaceId([182; 32]),
        &source,
        std::slice::from_ref(&positive),
        limits(),
        &mut work(),
    )
    .unwrap();
    let lifted = f.refine(&refinement, limits(), &mut work()).unwrap();
    assert_eq!(
        lifted
            .where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
            .unwrap(),
        refinement
            .refined()
            .parameter_event(&positive, limits(), &mut work())
            .unwrap()
    );
    let a = constant_function(&source, p());
    let b = FamilyFunction::constant(
        &source,
        value(&source, mul(&c(2), &p()), c(2)),
        limits(),
        &mut work(),
    )
    .unwrap();
    assert!(a.equivalent(&b, limits(), &mut work()).unwrap());
    assert!(
        a.sub(&b, limits(), &mut work())
            .unwrap()
            .where_sign(PolynomialSigns::ZERO, limits(), &mut work())
            .unwrap()
            .is_full()
    );
    let foreign = space(183, 1, &[]);
    assert!(matches!(
        a.multiply(
            &FamilyFunction::new(&foreign, &[], limits(), &mut work()).unwrap(),
            limits(),
            &mut work()
        ),
        Err(Error::SpaceMismatch)
    ));
}

#[test]
fn family_density_and_parameter_payoffs_retain_law_and_conditional_holes() {
    let source = shared_bias();
    let density = FamilyFunction::density(&source, limits(), &mut work()).unwrap();
    assert_eq!(
        density
            .designate(limits(), &mut work())
            .unwrap()
            .full()
            .to_bytes(&())
            .unwrap(),
        source.full().to_bytes(&()).unwrap()
    );
    let heads = source.coordinate(0, &()).unwrap();
    let payoff = FamilyFunction::new(
        &source,
        &[
            FamilyFunctionPiece {
                region: heads.clone(),
                value: value(&source, p(), c(1)),
            },
            FamilyFunctionPiece {
                region: heads.complement(),
                value: value(&source, sub(&c(1), &p()), c(1)),
            },
        ],
        limits(),
        &mut work(),
    )
    .unwrap();
    let differ = heads
        .apply(BoolOp4::XOR, &source.coordinate(1, &()).unwrap(), &())
        .unwrap();
    let observation = payoff.expectation(&differ, limits(), &mut work()).unwrap();
    assert_eq!(observation.evidence(), &differ);
    assert!(
        observation
            .function()
            .equivalent(&payoff, limits(), &mut work())
            .unwrap()
    );
    for n in 0..=8 {
        assert_eq!(
            observation
                .value_at(&ratio(&n.to_string(), "8"), limits(), &mut work())
                .unwrap(),
            (n != 0 && n != 8).then(|| ratio("1", "2"))
        );
    }
    let unconditioned = payoff
        .expectation(&source.full(), limits(), &mut work())
        .unwrap();
    assert_eq!(
        function_value(unconditioned.numerator(), 1, 4),
        Some(ratio("5", "8"))
    );
    let scalar = FiniteFunction::new(
        &source,
        &[FunctionPiece {
            region: heads,
            value: (-3i64).into(),
        }],
        limits().functions,
        &mut work(),
    )
    .unwrap();
    let old = scalar
        .parameter_expectation(&differ, limits(), &mut work())
        .unwrap();
    let generalized = FamilyFunction::from_finite(&scalar, limits(), &mut work())
        .unwrap()
        .expectation(&differ, limits(), &mut work())
        .unwrap();
    assert!(functions_equal(
        old.conditional(),
        generalized.conditional()
    ));
    drop((source, density, payoff, differ, scalar, old, generalized));
    assert_eq!(
        observation
            .value_at(&ratio("1", "3"), limits(), &mut work())
            .unwrap(),
        Some(ratio("1", "2"))
    );
}

#[test]
fn family_weighted_images_match_every_four_world_map_and_restricted_controls() {
    let raw = space(184, 2, &[]);
    let target = space(185, 2, &[]);
    let values = [
        p(),
        sub(&c(1), &p()),
        sub(&c(0), &mul(&c(2), &p())),
        sub(&mul(&c(3), &p()), &sub(&c(0), &c(1))),
    ];
    for support in 1..16 {
        let source = raw
            .restrict(&raw.table(3, &[support], &()).unwrap(), &())
            .unwrap();
        let input = table(&source, &values);
        for code in 0..256 {
            if support != 15 && ![0, 27, 228, 255].contains(&code) {
                continue;
            }
            let destinations: Vec<_> = (0..4).map(|i| (code >> (i * 2)) & 3).collect();
            let bits: Vec<_> = (0..2)
                .map(|bit| {
                    let mask = destinations
                        .iter()
                        .enumerate()
                        .fold(0, |m, (i, d)| m | (((d >> bit) & 1) << i));
                    source.table(3, &[mask], &()).unwrap()
                })
                .collect();
            let map = CoordinateMap::new(&source, &target, &bits, &()).unwrap();
            let actual = input.pushforward(&map, limits(), &mut work()).unwrap();
            let mut expected = [c(0), c(0), c(0), c(0)];
            for (i, &destination) in destinations.iter().enumerate() {
                if support & (1 << i) != 0 {
                    let at = usize::try_from(destination).unwrap();
                    expected[at] = expected[at]
                        .add(
                            &values[i],
                            limits().parameters.region.polynomial,
                            &mut work(),
                        )
                        .unwrap();
                }
            }
            assert!(
                actual
                    .equivalent(&table(&target, &expected), limits(), &mut work())
                    .unwrap(),
                "support={support},map={code}"
            );
            assert!(functions_equal(
                &actual
                    .outcome_sum(&target.full(), limits(), &mut work())
                    .unwrap(),
                &input
                    .outcome_sum(&source.full(), limits(), &mut work())
                    .unwrap()
            ));
        }
    }
}

#[test]
fn family_weighted_pairing_keeps_copied_readouts_and_missing_parameter_domains() {
    let source = space(186, 2, &[]);
    let target = space(187, 2, &[]);
    let map = CoordinateMap::coordinates(&source, &target, &[0, 0], &()).unwrap();
    let w = table(&source, &[p(), sub(&c(1), &p()), c(2), sub(&c(0), &p())]);
    let v = table(&target, &[c(3), p(), c(4), sub(&c(0), &c(2))]);
    let left = w
        .multiply(
            &v.pullback(&map, limits(), &mut work()).unwrap(),
            limits(),
            &mut work(),
        )
        .unwrap()
        .outcome_sum(&source.full(), limits(), &mut work())
        .unwrap();
    let right = w
        .pushforward(&map, limits(), &mut work())
        .unwrap()
        .multiply(&v, limits(), &mut work())
        .unwrap()
        .outcome_sum(&target.full(), limits(), &mut work())
        .unwrap();
    assert!(functions_equal(&left, &right));
    let prior = shared_bias();
    let positive = sign(&mul(&p(), &sub(&c(1), &p())), PolynomialSigns::POSITIVE);
    let narrow =
        ParameterRestriction::new(SpaceId([188; 32]), &prior, &positive, limits(), &mut work())
            .unwrap();
    let density = FamilyFunction::density(narrow.space(), limits(), &mut work()).unwrap();
    let image = density
        .pushforward(narrow.inclusion(), limits(), &mut work())
        .unwrap();
    let sum = image
        .outcome_sum(&narrow.refined_prior().full(), limits(), &mut work())
        .unwrap();
    assert_eq!(function_value(&sum, 0, 1), Some(0u64.into()));
    assert_eq!(function_value(&sum, 1, 2), Some(1u64.into()));
    assert_eq!(function_value(&sum, 1, 1), Some(0u64.into()));
    assert!(matches!(
        image.designate(limits(), &mut work()),
        Err(Error::LawNotNormalized)
    ));
    assert!(
        image
            .pullback(narrow.inclusion(), limits(), &mut work())
            .unwrap()
            .equivalent(&density, limits(), &mut work())
            .unwrap()
    );
}

#[test]
fn family_weighted_image_stays_symbolic_for_sixty_two_outcomes() {
    let source = space(189, 62, &[]);
    let target = space(190, 0, &[]);
    let map = CoordinateMap::new(&source, &target, &[], &()).unwrap();
    let input = constant_function(&source, p());
    let image = input.pushforward(&map, limits(), &mut work()).unwrap();
    let expected = constant_function(&target, mul(&c(1u64 << 62), &p()));
    assert!(image.equivalent(&expected, limits(), &mut work()).unwrap());
    assert_eq!(
        image
            .value_at(&ratio("1", "2"), 0, limits(), &mut work())
            .unwrap(),
        Some((1u64 << 61).into())
    );
}

#[test]
fn family_functions_keep_capacity_cancellation_and_foreign_empty_inputs_explicit() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let source = space(191, 1, &[]);
    let f = constant_function(&source, p());
    let restricted = source
        .restrict(&source.coordinate(0, &()).unwrap(), &())
        .unwrap();
    assert!(matches!(
        constant_function(&restricted, p()).value_at(&ratio("1", "2"), 0, limits(), &mut work()),
        Err(Error::IllegalWorld(_))
    ));
    let mut small = limits();
    small.functions.cells = 0;
    assert!(matches!(
        f.add(&f, small, &mut work()),
        Err(Error::Capacity(Capacity::FunctionCells))
    ));
    small = limits();
    small.steps = 0;
    assert!(matches!(
        f.align_to(&source, small, &mut work()),
        Err(Error::Capacity(Capacity::ParameterSourceSteps))
    ));
    assert!(matches!(
        f.outcome_sum(
            &source.full(),
            limits(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        f.expectation(&source.full(), limits(), &mut work()),
        Err(Error::MissingLaw)
    ));
    let finite = Space::new(SpaceId([192; 32]), 1, &()).unwrap();
    assert!(matches!(
        FamilyFunction::new(&finite, &[], limits(), &mut work()),
        Err(Error::MissingParameter)
    ));
    let foreign = space(193, 1, &[]);
    assert!(matches!(
        FamilyFunction::new(
            &source,
            &[FamilyFunctionPiece {
                region: foreign.empty(),
                value: value(&source, p(), c(1))
            }],
            limits(),
            &mut work()
        ),
        Err(Error::SpaceMismatch)
    ));
    small = limits();
    small.parameters.region.polynomial.degree = 0;
    assert!(matches!(
        FamilyFunction::new(
            &source,
            &[FamilyFunctionPiece {
                region: source.empty(),
                value: value(&source, p(), c(1))
            }],
            small,
            &mut work()
        ),
        Err(Error::Capacity(Capacity::PolynomialDegree))
    ));
}
