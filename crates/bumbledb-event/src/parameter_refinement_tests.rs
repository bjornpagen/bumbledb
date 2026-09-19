#![allow(clippy::too_many_lines)]
use crate::parameter_source_tests::{
    and, c, domain, limits, mul, p, ratio, shared_bias, sign, space, sub, work,
};
use crate::*;

fn above(denominator: u64) -> ParameterRegion {
    sign(
        &sub(&mul(&c(denominator), &p()), &c(1)),
        PolynomialSigns::POSITIVE,
    )
}

#[test]
fn every_refined_event_descends_exactly_when_its_new_guards_are_inessential() {
    let source = space(241, 1, &[]);
    let third = above(3);
    let two_thirds = sign(&sub(&mul(&c(3), &p()), &c(2)), PolynomialSigns::POSITIVE);
    let refinement = ParameterRefinement::new(
        SpaceId([242; 32]),
        &source,
        &[third, two_thirds],
        limits(),
        &mut work(),
    )
    .unwrap();
    // Independent six-cell oracle: low/middle/high, each with both outcomes.
    let codes = [0, 1, 2, 3, 6, 7];
    for membership in 0..64u64 {
        let mut table = 0u64;
        for (index, code) in codes.iter().enumerate() {
            table |= ((membership >> index) & 1) << code;
        }
        let event = refinement.refined().table(7, &[table], &()).unwrap();
        let representable = (0..2).all(|outcome| {
            (0..3).all(|cell| {
                ((membership >> (2 * cell + outcome)) & 1) == ((membership >> outcome) & 1)
            })
        });
        match refinement.descend(&event, &()) {
            Ok(coarse) => {
                assert!(representable, "unexpected descent for {membership:06b}");
                assert_eq!(coarse, source.table(1, &[membership & 3], &()).unwrap());
                assert_eq!(refinement.lift(&coarse, &()).unwrap(), event);
            }
            Err(Error::ParameterGuardEssential) => {
                assert!(!representable, "missing descent for {membership:06b}");
            }
            other => panic!("unexpected result {other:?}"),
        }
    }
}

#[test]
fn refining_a_readout_preserves_both_images_and_pullbacks() {
    let original = shared_bias();
    let swap = CoordinateMap::coordinates(&original, &original, &[1, 0, 2, 3], &()).unwrap();
    let source = ParameterRefinement::new(
        SpaceId([238; 32]),
        &original,
        &[above(2)],
        limits(),
        &mut work(),
    )
    .unwrap();
    let target = ParameterRefinement::new(
        SpaceId([239; 32]),
        &original,
        &[above(2).complement()],
        limits(),
        &mut work(),
    )
    .unwrap();
    let map = swap
        .refine_parameters(&source, &target, limits(), &mut work())
        .unwrap();
    let a = original.coordinate(0, &()).unwrap();
    let b = original.coordinate(1, &()).unwrap();
    for bits in 0..16 {
        let event = a.apply(BoolOp4::new(bits).unwrap(), &b, &()).unwrap();
        assert_eq!(
            map.pullback(&target.lift(&event, &()).unwrap(), &())
                .unwrap(),
            source
                .lift(&swap.pullback(&event, &()).unwrap(), &())
                .unwrap()
        );
        assert_eq!(
            map.image(&source.lift(&event, &()).unwrap(), &()).unwrap(),
            target.lift(&swap.image(&event, &()).unwrap(), &()).unwrap()
        );
        assert_eq!(
            map.universal_image(&source.lift(&event, &()).unwrap(), &())
                .unwrap(),
            target
                .lift(&swap.universal_image(&event, &()).unwrap(), &())
                .unwrap()
        );
    }
    let new_target = target
        .refined()
        .parameter_event(&above(2), limits(), &mut work())
        .unwrap();
    assert_eq!(
        map.pullback(&new_target, &()).unwrap(),
        source
            .refined()
            .parameter_event(&above(2), limits(), &mut work())
            .unwrap()
    );
    let only_old =
        ParameterRefinement::new(SpaceId([240; 32]), &original, &[], limits(), &mut work())
            .unwrap();
    assert!(matches!(
        swap.refine_parameters(&only_old, &target, limits(), &mut work()),
        Err(Error::ParameterRefinementRequired)
    ));
}

#[test]
fn refinement_lifts_every_truth_function_without_changing_worlds_or_law() {
    let source = shared_bias();
    let half = above(2);
    assert!(matches!(
        source.parameter_event(&half, limits(), &mut work()),
        Err(Error::ParameterRefinementRequired)
    ));
    let refine = ParameterRefinement::new(
        SpaceId([220; 32]),
        &source,
        std::slice::from_ref(&half),
        limits(),
        &mut work(),
    )
    .unwrap();
    let target = refine.refined();
    assert_eq!(source.full().atom_count(&()).unwrap(), 12);
    assert_eq!(target.full().atom_count(&()).unwrap(), 16);
    assert_eq!(source.outcome_coordinates(), target.outcome_coordinates());
    assert_eq!(
        target.full().world_cardinality(&()).unwrap(),
        WorldCardinality::Continuum
    );
    let predicate = target
        .parameter_event(&half, limits(), &mut work())
        .unwrap();
    assert_eq!(predicate, target.coordinate(4, &()).unwrap());
    assert!(matches!(
        refine.descend(&predicate, &()),
        Err(Error::ParameterGuardEssential)
    ));
    let a = source.coordinate(0, &()).unwrap();
    let b = source.coordinate(1, &()).unwrap();
    let lifted_a = refine.lift(&a, &()).unwrap();
    let lifted_b = refine.lift(&b, &()).unwrap();
    for bits in 0..16 {
        let op = BoolOp4::new(bits).unwrap();
        let old = a.apply(op, &b, &()).unwrap();
        let new = lifted_a.apply(op, &lifted_b, &()).unwrap();
        assert_eq!(refine.lift(&old, &()).unwrap(), new);
        assert_eq!(refine.descend(&new, &()).unwrap(), old);
        assert!(
            old.parameter_mass(limits(), &mut work())
                .unwrap()
                .equivalent(
                    &new.parameter_mass(limits(), &mut work()).unwrap(),
                    limits().parameters.region,
                    limits().functions,
                    &mut work(),
                )
                .unwrap()
        );
        for numerator in 0..=8 {
            for outcomes in 0..4 {
                let world = ParameterWorld {
                    parameter: RealWitness::Rational(ratio(&numerator.to_string(), "8")),
                    outcomes,
                };
                assert_eq!(
                    old.contains_parameter(&world, limits(), &mut work())
                        .unwrap(),
                    new.contains_parameter(&world, limits(), &mut work())
                        .unwrap()
                );
            }
        }
    }
    let observation = lifted_a
        .parameter_probability(&predicate, limits(), &mut work())
        .unwrap();
    assert_eq!(
        observation
            .value_at(&ratio("1", "2"), limits(), &mut work())
            .unwrap(),
        None
    );
    assert_eq!(
        observation
            .value_at(&ratio("3", "4"), limits(), &mut work())
            .unwrap(),
        Some(ratio("3", "4"))
    );
    let endpoint = target
        .parameter_event(&sign(&p(), PolynomialSigns::ZERO), limits(), &mut work())
        .unwrap();
    assert!(!and(&lifted_a, &endpoint).is_empty());
    assert_eq!(endpoint.count(&()).unwrap(), 4);
    let bytes = lifted_a.to_bytes(&()).unwrap();
    let decoded =
        Event::from_bytes_with_order(&bytes, Some(&[4, 3, 2, 1, 0]), Limits::default(), &())
            .unwrap();
    assert_eq!(refine.descend(&decoded, &()).unwrap(), a);
    assert_eq!(decoded.to_bytes(&()).unwrap(), bytes);
    let diagram = decoded.diagram(&()).unwrap();
    assert_eq!(diagram.rebuild(&decoded.space(), &()).unwrap(), decoded);
}

#[test]
fn common_refinement_enables_exact_parameter_maps_and_products() {
    let left = space(
        221,
        2,
        &[ParameterGuard {
            coordinate: 1,
            region: above(2),
        }],
    );
    let right = space(
        222,
        2,
        &[ParameterGuard {
            coordinate: 1,
            region: above(3),
        }],
    );
    let environment = space(223, 0, &[]);
    assert!(matches!(
        CoordinateMap::coordinates(&left, &right, &[0, 1], &()),
        Err(Error::ParameterRefinementRequired)
    ));
    let refined = ParameterRefinement::common(
        &[
            (SpaceId([224; 32]), left.clone()),
            (SpaceId([225; 32]), right.clone()),
            (SpaceId([226; 32]), environment),
        ],
        Limits::default(),
        limits(),
        &mut work(),
    )
    .unwrap();
    let l = refined[0].refined();
    let r = refined[1].refined();
    let env = refined[2].refined();
    assert_eq!(l.full().atom_count(&()).unwrap(), 6);
    assert_eq!(r.full().atom_count(&()).unwrap(), 6);
    assert_eq!(env.full().atom_count(&()).unwrap(), 3);
    let readouts: Vec<_> = r
        .parameter_guards()
        .unwrap()
        .iter()
        .map(|g| {
            (
                g.coordinate,
                l.parameter_event(&g.region, limits(), &mut work()).unwrap(),
            )
        })
        .collect();
    let mut complete = vec![l.coordinate(0, &()).unwrap(); usize::from(r.dimensions())];
    for (coordinate, predicate) in readouts {
        complete[usize::from(coordinate)] = predicate;
    }
    let map = CoordinateMap::new(l, r, &complete, &()).unwrap();
    let head = refined[0]
        .lift(&left.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let image = map.image(&head, &()).unwrap();
    assert_eq!(
        refined[1].descend(&image, &()).unwrap(),
        right.coordinate(0, &()).unwrap()
    );
    for n in 0..=12 {
        for outcomes in 0..2 {
            let world = ParameterWorld {
                parameter: RealWitness::Rational(ratio(&n.to_string(), "12")),
                outcomes,
            };
            assert_eq!(
                head.contains_parameter(&world, limits(), &mut work())
                    .unwrap(),
                image
                    .contains_parameter(&world, limits(), &mut work())
                    .unwrap()
            );
        }
    }
    let faces: Vec<_> = [l, r]
        .into_iter()
        .map(|source| {
            let readouts: Vec<_> = env
                .parameter_guards()
                .unwrap()
                .iter()
                .map(|g| {
                    source
                        .parameter_event(&g.region, limits(), &mut work())
                        .unwrap()
                })
                .collect();
            CoordinateMap::new(source, env, &readouts, &())
                .unwrap()
                .certify_surjective(&())
                .unwrap()
        })
        .collect();
    let product = FaceProduct::new(SpaceId([227; 32]), &faces, &()).unwrap();
    assert_eq!(product.space().full().atom_count(&()).unwrap(), 12);
    assert!(!product.space().is_measured());
    let a = product.projections()[0]
        .map()
        .pullback(
            &l.parameter_event(&above(2), limits(), &mut work()).unwrap(),
            &(),
        )
        .unwrap();
    let b = product.projections()[1]
        .map()
        .pullback(
            &r.parameter_event(&above(3), limits(), &mut work()).unwrap(),
            &(),
        )
        .unwrap();
    assert!(a.signature(&b, &()).unwrap().included());
    let descriptor = Descriptor::capture(
        &AdmittedDescriptor::Faces(product),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap();
    let restored = descriptor.admit(DescriptorLimits::default(), &()).unwrap();
    let AdmittedDescriptor::Faces(restored) = restored else {
        panic!("faces")
    };
    assert_eq!(restored.space().full().atom_count(&()).unwrap(), 12);
}

#[test]
fn refinement_retains_irrational_singletons_and_checks_all_owners() {
    let roots = sign(&sub(&mul(&p(), &p()), &c(2)), PolynomialSigns::ZERO);
    let source = Space::new(SpaceId([228; 32]), 1, &())
        .unwrap()
        .with_parameters(
            ParameterDomain::new(roots).unwrap(),
            &[],
            limits(),
            &mut work(),
        )
        .unwrap();
    let source = source
        .with_density(
            &[DensityPiece {
                region: source.full(),
                density: ratio("1", "2"),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let positive = sign(&p(), PolynomialSigns::POSITIVE);
    let refinement = ParameterRefinement::new(
        SpaceId([229; 32]),
        &source,
        &[positive.clone(), positive.complement()],
        limits(),
        &mut work(),
    )
    .unwrap();
    assert_eq!(source.full().count(&()).unwrap(), 4);
    assert_eq!(refinement.refined().full().count(&()).unwrap(), 4);
    assert_eq!(refinement.refined().full().atom_count(&()).unwrap(), 4);
    assert_eq!(
        refinement
            .refined()
            .parameter_event(&positive, limits(), &mut work())
            .unwrap()
            .count(&())
            .unwrap(),
        2
    );
    assert!(
        refinement
            .refined()
            .coordinate(1, &())
            .unwrap()
            .equivalent(
                &refinement
                    .refined()
                    .coordinate(2, &())
                    .unwrap()
                    .complement()
            )
            .unwrap()
    );
    let source_full = refinement.lift(&source.full(), &()).unwrap();
    drop(source);
    drop(refinement);
    assert_eq!(source_full.count(&()).unwrap(), 4);
    let conditional = source_full.parameter_mass(limits(), &mut work()).unwrap();
    assert!(!conditional.is_nowhere_defined());
    let wrong = space(230, 1, &[]);
    let fresh =
        ParameterRefinement::new(SpaceId([231; 32]), &wrong, &[], limits(), &mut work()).unwrap();
    assert!(matches!(
        fresh.lift(&source_full.complement(), &()),
        Err(Error::SpaceMismatch)
    ));
    assert!(matches!(
        fresh.descend(&source_full.complement(), &()),
        Err(Error::SpaceMismatch)
    ));
}

#[test]
fn refinement_refuses_scope_capacity_and_cancellation_without_narrowing() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let source = space(232, 1, &[]);
    let foreign = ParameterRegion::empty(ParameterId([99; 32]));
    assert!(matches!(
        source.parameter_event(&foreign, limits(), &mut work()),
        Err(Error::ParameterScopeMismatch)
    ));
    assert!(matches!(
        ParameterRefinement::new(
            SpaceId([233; 32]),
            &source,
            &[foreign],
            limits(),
            &mut work()
        ),
        Err(Error::ParameterScopeMismatch)
    ));
    assert!(matches!(
        ParameterRefinement::common(&[], Limits::default(), limits(), &mut work()),
        Err(Error::NoParameterSources)
    ));
    let narrower = Space::new(SpaceId([234; 32]), 1, &())
        .unwrap()
        .with_parameters(
            ParameterDomain::new(
                domain()
                    .region()
                    .apply(
                        BoolOp4::AND,
                        &above(2),
                        limits().parameters.region,
                        &mut work(),
                    )
                    .unwrap(),
            )
            .unwrap(),
            &[],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert!(matches!(
        ParameterRefinement::common(
            &[
                (SpaceId([235; 32]), source.clone()),
                (SpaceId([236; 32]), narrower)
            ],
            Limits::default(),
            limits(),
            &mut work()
        ),
        Err(Error::ParameterDomainMismatch)
    ));
    let many = vec![above(2); 62];
    assert!(matches!(
        ParameterRefinement::new(SpaceId([237; 32]), &source, &many, limits(), &mut work()),
        Err(Error::Capacity(Capacity::Coordinates))
    ));
    let mut small = limits();
    small.fibres = 1;
    assert!(matches!(
        ParameterRefinement::new(SpaceId([237; 32]), &source, &[above(2)], small, &mut work()),
        Err(Error::Capacity(Capacity::ParameterSourceCells))
    ));
    let graph = Limits {
        records: 0,
        ..Limits::default()
    };
    assert!(matches!(
        ParameterRefinement::with_limits(
            SpaceId([237; 32]),
            &source,
            &[],
            graph,
            limits(),
            &mut work()
        ),
        Err(Error::Capacity(Capacity::Records))
    ));
    assert!(matches!(
        ParameterRefinement::new(
            SpaceId([237; 32]),
            &source,
            &[],
            limits(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    assert_eq!(source.full().atom_count(&()).unwrap(), 2);
}
