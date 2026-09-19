#![allow(clippy::too_many_lines)]
use crate::parameter_source_tests::{
    and, c, domain, limits, mul, p, ratio, shared_bias, sign, space, sub, work,
};
use crate::*;

fn interior() -> ParameterRegion {
    domain()
        .region()
        .apply(
            BoolOp4::AND,
            &sign(&mul(&p(), &sub(&c(1), &p())), PolynomialSigns::POSITIVE),
            limits().parameters.region,
            &mut work(),
        )
        .unwrap()
}

#[test]
fn conditioning_at_an_irrational_parameter_keeps_exact_worlds_and_law() {
    let prior = shared_bias();
    let roots = sign(
        &sub(&mul(&c(2), &mul(&p(), &p())), &c(1)),
        PolynomialSigns::ZERO,
    );
    let restriction =
        ParameterRestriction::new(SpaceId([162; 32]), &prior, &roots, limits(), &mut work())
            .unwrap();
    assert_eq!(restriction.space().full().count(&()).unwrap(), 4);
    let head = restriction
        .pullback(&prior.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let revision = restriction
        .space()
        .parameter_condition(SpaceId([163; 32]), &head, limits(), &mut work())
        .unwrap();
    let posterior = revision.revised().unwrap();
    assert_eq!(posterior.space().full().count(&()).unwrap(), 4);
    let observed = posterior.pullback(&head, &()).unwrap();
    assert_eq!(observed.count(&()).unwrap(), 2);
    assert_eq!(observed.complement().count(&()).unwrap(), 2);
    let mass = observed.parameter_mass(limits(), &mut work()).unwrap();
    let domain = posterior.space().parameter_domain().unwrap();
    let one = GuardedRationalFunction::new(
        domain.clone(),
        c(1),
        c(1),
        limits().parameters.region,
        &mut work(),
    )
    .unwrap();
    let constant = ParameterFunction::new(
        domain.clone(),
        &[one],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    assert!(
        mass.equivalent(
            &constant,
            limits().parameters.region,
            limits().functions,
            &mut work()
        )
        .unwrap()
    );
    let witness = observed
        .parameter_witness(limits(), &mut work())
        .unwrap()
        .unwrap();
    assert!(matches!(&witness.parameter, RealWitness::Algebraic(_)));
    assert!(
        observed
            .contains_parameter(&witness, limits(), &mut work())
            .unwrap()
    );
}

#[test]
fn parameter_inclusion_keeps_missing_domains_unreachable_in_images_and_sums() {
    let prior = shared_bias();
    let restriction = ParameterRestriction::new(
        SpaceId([150; 32]),
        &prior,
        &interior(),
        limits(),
        &mut work(),
    )
    .unwrap();
    let narrow = restriction.space();
    let full = restriction.refined_prior();
    let present = full
        .parameter_event(&interior(), limits(), &mut work())
        .unwrap();
    let map = restriction.inclusion();
    assert_eq!(map.support_image(&()).unwrap(), present);
    assert_eq!(map.image(&narrow.empty(), &()).unwrap(), full.empty());
    assert_eq!(
        map.universal_image(&narrow.empty(), &()).unwrap(),
        present.complement()
    );
    assert!(matches!(
        map.clone().certify_surjective(&()),
        Err(Error::IncompleteImage)
    ));
    let coordinates: Vec<_> = (0..narrow.dimensions()).collect();
    assert!(matches!(
        CoordinateMap::coordinates(full, narrow, &coordinates, &()),
        Err(Error::ParameterDomainMismatch)
    ));
    let original = prior.coordinate(0, &()).unwrap();
    let copied = restriction.pullback(&original, &()).unwrap();
    assert!(!copied.complement().is_empty());
    assert!(
        copied
            .parameter_mass(limits(), &mut work())
            .unwrap()
            .equivalent(
                &original
                    .parameter_mass(limits(), &mut work())
                    .unwrap()
                    .on_domain(
                        narrow.parameter_domain().unwrap(),
                        limits().parameters.region,
                        limits().functions,
                        &mut work()
                    )
                    .unwrap(),
                limits().parameters.region,
                limits().functions,
                &mut work(),
            )
            .unwrap()
    );
    let ones =
        FiniteFunction::constant(narrow, 1u64.into(), limits().functions, &mut work()).unwrap();
    let image = ones
        .pushforward(map, limits().functions, &mut work())
        .unwrap();
    let expected = FiniteFunction::new(
        full,
        &[FunctionPiece {
            region: present,
            value: 1u64.into(),
        }],
        limits().functions,
        &mut work(),
    )
    .unwrap();
    assert!(image.equivalent(&expected, &()).unwrap());
    let portable = Descriptor::capture(
        &AdmittedDescriptor::Map(map.clone()),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap();
    let AdmittedDescriptor::Map(decoded) =
        portable.admit(DescriptorLimits::default(), &()).unwrap()
    else {
        panic!("map")
    };
    assert_eq!(
        decoded.source().full().to_bytes(&()).unwrap(),
        narrow.full().to_bytes(&()).unwrap()
    );
    assert_eq!(
        decoded.support_image(&()).unwrap().to_bytes(&()).unwrap(),
        map.support_image(&()).unwrap().to_bytes(&()).unwrap()
    );
}

#[test]
fn domain_inclusion_requires_a_target_presentation_resolving_the_actual_subset() {
    let prior = space(151, 1, &[]);
    let narrow = Space::new(SpaceId([152; 32]), 1, &())
        .unwrap()
        .with_parameters(
            ParameterDomain::new(interior()).unwrap(),
            &[],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert!(matches!(
        CoordinateMap::coordinates(&narrow, &prior, &[0], &()),
        Err(Error::ParameterRefinementRequired)
    ));
    let restriction = ParameterRestriction::new(
        SpaceId([153; 32]),
        &prior,
        &interior(),
        limits(),
        &mut work(),
    )
    .unwrap();
    assert_eq!(restriction.space().full().atom_count(&()).unwrap(), 2);
    assert!(!restriction.space().is_measured());
    assert!(matches!(
        ParameterRestriction::new(
            SpaceId([154; 32]),
            &prior,
            &ParameterRegion::empty(prior.parameter_domain().unwrap().parameter()),
            limits(),
            &mut work()
        ),
        Err(Error::EmptyParameterDomain)
    ));
    let outside = sign(&sub(&p(), &c(2)), PolynomialSigns::POSITIVE);
    assert!(matches!(
        ParameterRestriction::new(SpaceId([154; 32]), &prior, &outside, limits(), &mut work()),
        Err(Error::EmptyParameterDomain)
    ));
    let foreign = space(155, 1, &[]);
    assert!(matches!(
        restriction.pullback(&foreign.empty(), &()),
        Err(Error::SpaceMismatch)
    ));
}

#[test]
fn ambient_domain_change_preserves_undefined_points_and_refuses_extension() {
    let d = domain();
    let hole =
        GuardedRationalFunction::new(d.clone(), p(), p(), limits().parameters.region, &mut work())
            .unwrap();
    let lower = ParameterDomain::new(
        d.region()
            .apply(
                BoolOp4::AND,
                &sign(
                    &sub(&mul(&c(2), &p()), &c(1)),
                    PolynomialSigns::NON_POSITIVE,
                ),
                limits().parameters.region,
                &mut work(),
            )
            .unwrap(),
    )
    .unwrap();
    let reduced = hole
        .on_domain(&lower, limits().parameters.region, &mut work())
        .unwrap();
    assert_eq!(
        reduced
            .value_at(&0u64.into(), limits().parameters.region, &mut work())
            .unwrap(),
        None
    );
    assert_eq!(
        reduced
            .value_at(&ratio("1", "4"), limits().parameters.region, &mut work())
            .unwrap(),
        Some(1u64.into())
    );
    assert_eq!(
        reduced
            .value_at(&ratio("3", "4"), limits().parameters.region, &mut work())
            .unwrap(),
        None
    );
    assert!(matches!(
        reduced.on_domain(&d, limits().parameters.region, &mut work()),
        Err(Error::ParameterDomainMismatch)
    ));
    let piecewise = ParameterFunction::new(
        d.clone(),
        &[hole],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap();
    let reduced = piecewise
        .on_domain(
            &lower,
            limits().parameters.region,
            limits().functions,
            &mut work(),
        )
        .unwrap();
    assert_eq!(
        reduced
            .value_at(
                &0u64.into(),
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap(),
        None
    );
    assert!(matches!(
        reduced.on_domain(
            &d,
            limits().parameters.region,
            limits().functions,
            &mut work()
        ),
        Err(Error::ParameterDomainMismatch)
    ));
}

#[test]
fn family_conditioning_retains_endpoint_failures_and_zero_posterior_possibilities() {
    let prior = shared_bias();
    let first = prior.coordinate(0, &()).unwrap();
    let second = prior.coordinate(1, &()).unwrap();
    let evidence = first.apply(BoolOp4::XOR, &second, &()).unwrap();
    let revision = prior
        .parameter_condition(SpaceId([156; 32]), &evidence, limits(), &mut work())
        .unwrap();
    assert!(!revision.is_impossible());
    assert!(
        revision
            .defined_on()
            .equivalent(&interior(), limits().parameters.region, &mut work())
            .unwrap()
    );
    assert_eq!(
        revision.evidence().to_bytes(&()).unwrap(),
        evidence.to_bytes(&()).unwrap()
    );
    let posterior = revision.revised().unwrap();
    let head = posterior.pullback(&first, &()).unwrap();
    let observed = posterior.pullback(&evidence, &()).unwrap();
    assert!(!observed.is_full());
    assert!(!observed.complement().is_empty());
    assert_eq!(posterior.space().full().atom_count(&()).unwrap(), 4);
    assert!(matches!(
        posterior.translation().clone().certify_surjective(&()),
        Err(Error::IncompleteImage)
    ));
    let head_mass = head.parameter_mass(limits(), &mut work()).unwrap();
    let zero = observed
        .complement()
        .parameter_mass(limits(), &mut work())
        .unwrap();
    let original_observation = first
        .parameter_probability(&evidence, limits(), &mut work())
        .unwrap();
    assert_eq!(original_observation.event(), &first);
    assert_eq!(original_observation.given(), &evidence);
    let expected = original_observation
        .conditional()
        .on_domain(
            posterior.space().parameter_domain().unwrap(),
            limits().parameters.region,
            limits().functions,
            &mut work(),
        )
        .unwrap();
    assert!(
        head_mass
            .equivalent(
                &expected,
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap()
    );
    for n in 0..=8 {
        let at = ratio(&n.to_string(), "8");
        let defined = n != 0 && n != 8;
        assert_eq!(
            head_mass
                .value_at(
                    &at,
                    limits().parameters.region,
                    limits().functions,
                    &mut work()
                )
                .unwrap(),
            defined.then(|| ratio("1", "2"))
        );
        assert_eq!(
            zero.value_at(
                &at,
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap(),
            defined.then(|| 0u64.into())
        );
        assert_eq!(
            revision
                .evidence_mass()
                .value_at(
                    &at,
                    limits().parameters.region,
                    limits().functions,
                    &mut work()
                )
                .unwrap(),
            Some(ratio(&(2 * n * (8 - n)).to_string(), "64"))
        );
    }
    let bytes = head.to_bytes(&()).unwrap();
    let decoded = Event::from_bytes(&bytes, &()).unwrap();
    assert_eq!(decoded.to_bytes(&()).unwrap(), bytes);
    drop((prior, first, second, evidence, revision, head, observed));
    assert_eq!(
        decoded
            .parameter_probability(&decoded.space().full(), limits(), &mut work())
            .unwrap()
            .value_at(&ratio("1", "3"), limits(), &mut work())
            .unwrap(),
        Some(ratio("1", "2"))
    );
}

#[test]
fn repeated_family_evidence_is_idempotent_and_possible_zero_mass_is_impossible_conditioning() {
    let prior = shared_bias();
    let a = prior.coordinate(0, &()).unwrap();
    let b = prior.coordinate(1, &()).unwrap();
    let first = prior
        .parameter_condition(SpaceId([157; 32]), &a, limits(), &mut work())
        .unwrap();
    let p1 = first.revised().unwrap();
    let repeated = p1
        .space()
        .parameter_condition(
            SpaceId([158; 32]),
            &p1.pullback(&a, &()).unwrap(),
            limits(),
            &mut work(),
        )
        .unwrap();
    let old_b = p1.pullback(&b, &()).unwrap();
    let new_b = repeated.revised().unwrap().pullback(&old_b, &()).unwrap();
    assert!(
        old_b
            .parameter_mass(limits(), &mut work())
            .unwrap()
            .equivalent(
                &new_b.parameter_mass(limits(), &mut work()).unwrap(),
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap()
    );
    let at_zero = prior.coordinate(2, &()).unwrap();
    let impossible = and(&a, &at_zero);
    assert!(!impossible.is_empty());
    let receipt = prior
        .parameter_condition(SpaceId([159; 32]), &impossible, limits(), &mut work())
        .unwrap();
    assert!(receipt.is_impossible());
    assert!(receipt.defined_on().is_empty());
    assert!(!receipt.evidence().is_empty());
    assert_eq!(receipt.evidence().count(&()).unwrap(), 2);
    let empty = prior
        .parameter_condition(SpaceId([159; 32]), &prior.empty(), limits(), &mut work())
        .unwrap();
    assert!(empty.is_impossible());
    let foreign = space(160, 1, &[]);
    assert!(matches!(
        prior.parameter_condition(SpaceId([159; 32]), &foreign.empty(), limits(), &mut work()),
        Err(Error::SpaceMismatch)
    ));
    assert!(matches!(
        foreign.parameter_condition(SpaceId([159; 32]), &foreign.full(), limits(), &mut work()),
        Err(Error::MissingLaw)
    ));
}

#[test]
fn family_conditioning_keeps_shared_arithmetic_and_explicit_operational_failures() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let prior = shared_bias();
    let evidence = prior.coordinate(0, &()).unwrap();
    let zero_work = ArithmeticLimits {
        operations: 0,
        ..ArithmeticLimits::default()
    };
    assert!(matches!(
        prior.parameter_condition(
            SpaceId([161; 32]),
            &evidence,
            limits(),
            &mut ExactArithmetic::new(zero_work, &())
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    assert!(matches!(
        prior.parameter_condition(
            SpaceId([161; 32]),
            &evidence,
            limits(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    let small = ParameterSourceLimits {
        steps: 0,
        ..limits()
    };
    let readouts: Vec<_> = (0..prior.dimensions())
        .map(|bit| prior.coordinate(bit, &()).unwrap())
        .collect();
    assert!(matches!(
        CoordinateMap::new_with_parameters(&prior, &prior, &readouts, small, &mut work()),
        Err(Error::Capacity(Capacity::ParameterSourceSteps))
    ));
    assert!(matches!(
        prior.parameter_condition(SpaceId([161; 32]), &evidence, small, &mut work()),
        Err(Error::Capacity(Capacity::ParameterSourceSteps))
    ));
    assert_eq!(prior.full().atom_count(&()).unwrap(), 12);
}
