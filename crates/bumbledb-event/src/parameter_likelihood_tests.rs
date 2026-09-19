#![allow(clippy::too_many_lines)]
use crate::parameter_source_tests::{c, limits, p, ratio, shared_bias, sub, work};
use crate::*;

fn factor(source: &Space, yes: ExactPolynomial, no: ExactPolynomial) -> FamilyFunction {
    let head = source.coordinate(0, &()).unwrap();
    let pieces = [(head.clone(), yes), (head.complement(), no)]
        .into_iter()
        .map(|(region, numerator)| FamilyFunctionPiece {
            region,
            value: GuardedRationalFunction::new(
                source.parameter_domain().unwrap().clone(),
                numerator,
                c(1),
                limits().parameters.region,
                &mut work(),
            )
            .unwrap(),
        })
        .collect::<Vec<_>>();
    FamilyFunction::new(source, &pieces, limits(), &mut work()).unwrap()
}
fn translated(f: &FamilyFunction, posterior: &ParameterRevisedSource) -> FamilyFunction {
    f.refine(posterior.restriction().refinement(), limits(), &mut work())
        .unwrap()
        .pullback(posterior.translation(), limits(), &mut work())
        .unwrap()
}
fn at(f: &ParameterFunction, n: u64, d: u64) -> Option<ExactRational> {
    f.value_at(
        &ratio(&n.to_string(), &d.to_string()),
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}

#[test]
fn family_likelihood_preserves_scale_receipts_and_multiplies_repeated_factors() {
    let source = shared_bias();
    let head = source.coordinate(0, &()).unwrap();
    let likelihood = factor(&source, c(2), c(1));
    let update = source
        .parameter_likelihood(SpaceId([194; 32]), &likelihood, limits(), &mut work())
        .unwrap();
    let scaled = source
        .parameter_likelihood(
            SpaceId([195; 32]),
            &factor(&source, c(6), c(3)),
            limits(),
            &mut work(),
        )
        .unwrap();
    let posterior = update.revised().unwrap();
    let first_head = posterior.pullback(&head, &()).unwrap();
    let mass = first_head.parameter_mass(limits(), &mut work()).unwrap();
    let scaled_mass = scaled
        .revised()
        .unwrap()
        .pullback(&head, &())
        .unwrap()
        .parameter_mass(limits(), &mut work())
        .unwrap();
    assert!(
        mass.equivalent(
            &scaled_mass,
            limits().parameters.region,
            limits().functions,
            &mut work()
        )
        .unwrap()
    );
    let repeated = posterior
        .space()
        .parameter_likelihood(
            SpaceId([196; 32]),
            &translated(&likelihood, posterior),
            limits(),
            &mut work(),
        )
        .unwrap();
    let repeated_head = repeated
        .revised()
        .unwrap()
        .pullback(&first_head, &())
        .unwrap();
    let repeat_mass = repeated_head.parameter_mass(limits(), &mut work()).unwrap();
    let joint = source
        .parameter_likelihood(
            SpaceId([197; 32]),
            &likelihood
                .multiply(&likelihood, limits(), &mut work())
                .unwrap(),
            limits(),
            &mut work(),
        )
        .unwrap();
    let joint_mass = joint
        .revised()
        .unwrap()
        .pullback(&head, &())
        .unwrap()
        .parameter_mass(limits(), &mut work())
        .unwrap();
    assert!(
        repeat_mass
            .equivalent(
                &joint_mass,
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap()
    );
    for n in 0..=8 {
        assert_eq!(
            at(&mass, n, 8),
            Some(ratio(&(2 * n).to_string(), &(8 + n).to_string()))
        );
        assert_eq!(
            at(&repeat_mass, n, 8),
            Some(ratio(&(4 * n).to_string(), &(8 + 3 * n).to_string()))
        );
        assert_eq!(
            at(update.normalizer(), n, 8),
            Some(ratio(&(8 + n).to_string(), "8"))
        );
        assert_eq!(
            at(scaled.normalizer(), n, 8),
            Some(ratio(&(3 * (8 + n)).to_string(), "8"))
        );
    }
    assert!(!first_head.complement().is_empty());
    assert!(
        update
            .likelihood()
            .equivalent(&likelihood, limits(), &mut work())
            .unwrap()
    );
    let bytes = first_head.to_bytes(&()).unwrap();
    let decoded = Event::from_bytes(&bytes, &()).unwrap();
    drop((
        source, head, likelihood, update, scaled, repeated, joint, first_head,
    ));
    assert_eq!(
        at(
            &decoded.parameter_mass(limits(), &mut work()).unwrap(),
            1,
            2
        ),
        Some(ratio("2", "3"))
    );
}

#[test]
fn likelihood_values_can_depend_on_the_same_unknown_parameter() {
    let source = shared_bias();
    let head = source.coordinate(0, &()).unwrap();
    let likelihood = factor(&source, p(), sub(&c(1), &p()));
    let update = source
        .parameter_likelihood(SpaceId([198; 32]), &likelihood, limits(), &mut work())
        .unwrap();
    let mass = update
        .revised()
        .unwrap()
        .pullback(&head, &())
        .unwrap()
        .parameter_mass(limits(), &mut work())
        .unwrap();
    for n in 0..=8 {
        let denominator = n * n + (8 - n) * (8 - n);
        assert_eq!(
            at(&mass, n, 8),
            Some(ratio(&(n * n).to_string(), &denominator.to_string()))
        );
        assert_eq!(
            at(update.normalizer(), n, 8),
            Some(ratio(&denominator.to_string(), "64"))
        );
    }
    assert!(
        update
            .defined_on()
            .equivalent(
                source.parameter_domain().unwrap().region(),
                limits().parameters.region,
                &mut work()
            )
            .unwrap()
    );
}

#[test]
fn likelihood_indicator_matches_conditioning_and_retains_undefined_endpoints() {
    let source = shared_bias();
    let a = source.coordinate(0, &()).unwrap();
    let evidence = a
        .apply(BoolOp4::XOR, &source.coordinate(1, &()).unwrap(), &())
        .unwrap();
    let finite = FiniteFunction::new(
        &source,
        &[FunctionPiece {
            region: evidence.clone(),
            value: 1u64.into(),
        }],
        limits().functions,
        &mut work(),
    )
    .unwrap();
    let likelihood = FamilyFunction::from_finite(&finite, limits(), &mut work()).unwrap();
    let revised = source
        .parameter_likelihood(SpaceId([199; 32]), &likelihood, limits(), &mut work())
        .unwrap();
    let conditioned = source
        .parameter_condition(SpaceId([200; 32]), &evidence, limits(), &mut work())
        .unwrap();
    let left = revised
        .revised()
        .unwrap()
        .pullback(&a, &())
        .unwrap()
        .parameter_mass(limits(), &mut work())
        .unwrap();
    let right = conditioned
        .revised()
        .unwrap()
        .pullback(&a, &())
        .unwrap()
        .parameter_mass(limits(), &mut work())
        .unwrap();
    assert!(
        left.equivalent(
            &right,
            limits().parameters.region,
            limits().functions,
            &mut work()
        )
        .unwrap()
    );
    assert!(
        revised
            .normalizer()
            .equivalent(
                conditioned.evidence_mass(),
                limits().parameters.region,
                limits().functions,
                &mut work()
            )
            .unwrap()
    );
    assert_eq!(at(&left, 0, 1), None);
    assert_eq!(at(&left, 1, 1), None);
    assert!(
        !revised
            .revised()
            .unwrap()
            .pullback(&evidence, &())
            .unwrap()
            .complement()
            .is_empty()
    );
    assert!(matches!(
        revised
            .revised()
            .unwrap()
            .translation()
            .clone()
            .certify_surjective(&()),
        Err(Error::IncompleteImage)
    ));
}

#[test]
fn family_likelihood_refuses_negative_zero_prior_factors_and_owns_impossible_requests() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let source = shared_bias();
    let impossible = source
        .coordinate(0, &())
        .unwrap()
        .apply(BoolOp4::AND, &source.coordinate(2, &()).unwrap(), &())
        .unwrap();
    assert!(!impossible.is_empty());
    let negative = FamilyFunction::new(
        &source,
        &[FamilyFunctionPiece {
            region: impossible,
            value: GuardedRationalFunction::new(
                source.parameter_domain().unwrap().clone(),
                ExactPolynomial::constant((-1i64).into()),
                c(1),
                limits().parameters.region,
                &mut work(),
            )
            .unwrap(),
        }],
        limits(),
        &mut work(),
    )
    .unwrap();
    assert!(matches!(
        source.parameter_likelihood(SpaceId([202; 32]), &negative, limits(), &mut work()),
        Err(Error::NegativeMass)
    ));
    let zero = FamilyFunction::new(&source, &[], limits(), &mut work()).unwrap();
    assert!(matches!(
        source.parameter_likelihood(
            SpaceId([203; 32]),
            &zero,
            limits(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        source.parameter_likelihood(
            SpaceId([203; 32]),
            &zero,
            limits(),
            &mut ExactArithmetic::new(
                ArithmeticLimits {
                    operations: 0,
                    ..ArithmeticLimits::default()
                },
                &()
            )
        ),
        Err(Error::Capacity(Capacity::ArithmeticSteps))
    ));
    let unmeasured = source.unmeasured(&()).unwrap();
    let missing = FamilyFunction::new(&unmeasured, &[], limits(), &mut work()).unwrap();
    assert!(matches!(
        unmeasured.parameter_likelihood(SpaceId([203; 32]), &missing, limits(), &mut work()),
        Err(Error::MissingLaw)
    ));
    assert!(matches!(
        source.parameter_likelihood(SpaceId([203; 32]), &missing, limits(), &mut work()),
        Err(Error::SpaceMismatch)
    ));
    let receipt = source
        .parameter_likelihood(SpaceId([203; 32]), &zero, limits(), &mut work())
        .unwrap();
    assert!(receipt.is_impossible());
    assert!(receipt.defined_on().is_empty());
    drop((source, zero, negative));
    assert_eq!(at(receipt.normalizer(), 1, 2), Some(0u64.into()));
    assert_eq!(receipt.prior().full().atom_count(&()).unwrap(), 12);
    assert_eq!(
        receipt
            .likelihood()
            .value_at(&ratio("1", "2"), 1, limits(), &mut work())
            .unwrap(),
        Some(0u64.into())
    );
}
