#![allow(clippy::too_many_lines)]
use crate::parameter_source_tests::{
    and, c, domain, limits, mul, p, ratio, shared_bias, sign, space, sub, work,
};
use crate::*;

fn function(
    d: &ParameterDomain,
    n: ExactPolynomial,
    denominator: ExactPolynomial,
) -> ParameterFunction {
    ParameterFunction::new(
        d.clone(),
        &[GuardedRationalFunction::new(
            d.clone(),
            n,
            denominator,
            limits().parameters.region,
            &mut work(),
        )
        .unwrap()],
        limits().parameters.region,
        limits().functions,
        &mut work(),
    )
    .unwrap()
}
fn scalar(d: &ParameterDomain, n: u64, denominator: u64) -> ParameterFunction {
    function(d, c(n), c(denominator))
}
fn partition(s: &Space, a: &Event) -> EventPartition {
    EventPartition::on(
        &s.full(),
        &[a.clone(), a.complement(), s.empty()],
        PartitionLimits::default(),
        &(),
    )
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
fn mass(a: &Event) -> ParameterFunction {
    a.parameter_mass(limits(), &mut work()).unwrap()
}

#[test]
fn family_jeffrey_preserves_conditionals_and_is_idempotent_with_zero_target_endpoints() {
    let source = shared_bias();
    let a = source.coordinate(0, &()).unwrap();
    let b = source.coordinate(1, &()).unwrap();
    let cells = partition(&source, &a);
    let d = source.parameter_domain().unwrap();
    let targets = [
        function(d, mul(&p(), &p()), c(1)),
        function(d, sub(&c(1), &mul(&p(), &p())), c(1)),
        scalar(d, 0, 1),
    ];
    let receipt = source
        .parameter_jeffrey(SpaceId([215; 32]), &cells, &targets, limits(), &mut work())
        .unwrap();
    assert!(
        receipt
            .defined_on()
            .equivalent(d.region(), limits().parameters.region, &mut work())
            .unwrap()
    );
    assert_eq!(receipt.partition().cells().len(), 3);
    assert!(receipt.partition().cells()[2].is_empty());
    assert!(
        receipt
            .unsupported_regions()
            .iter()
            .all(ParameterRegion::is_empty)
    );
    let revised = receipt.revised().unwrap();
    let ra = revised.pullback(&a, &()).unwrap();
    let rb = revised.pullback(&b, &()).unwrap();
    let again = revised
        .space()
        .parameter_jeffrey(
            SpaceId([216; 32]),
            &partition(revised.space(), &ra),
            &targets,
            limits(),
            &mut work(),
        )
        .unwrap();
    for code in 0..4u64 {
        let cell = source.table(3, &[1 << code], &()).unwrap();
        let first = revised.pullback(&cell, &()).unwrap();
        let second = again.revised().unwrap().pullback(&first, &()).unwrap();
        assert!(
            mass(&first)
                .equivalent(
                    &mass(&second),
                    limits().parameters.region,
                    limits().functions,
                    &mut work()
                )
                .unwrap()
        );
        for n in 0..=4u64 {
            // Independent oracle: replace P(A) by p², keep P(B|A) = P(B|!A) = p.
            let a_numerator = if code & 1 != 0 { n * n } else { 16 - n * n };
            let b_numerator = if code & 2 != 0 { n } else { 4 - n };
            assert_eq!(
                at(&mass(&first), n, 4),
                Some(ratio(&(a_numerator * b_numerator).to_string(), "64"))
            );
        }
    }
    assert!(!ra.complement().is_empty());
    assert_eq!(at(&mass(&and(&ra, &rb)), 1, 2), Some(ratio("1", "8")));
    assert_eq!(at(&mass(&rb), 1, 2), Some(ratio("1", "2")));
}

#[test]
fn family_jeffrey_refines_piecewise_targets_and_keeps_original_translation() {
    let source = shared_bias();
    let a = source.coordinate(0, &()).unwrap();
    let lower = sign(&sub(&mul(&c(2), &p()), &c(1)), PolynomialSigns::NEGATIVE);
    let d = source.parameter_domain().unwrap();
    let mut targets = Vec::new();
    for invert in [false, true] {
        let pieces: Vec<_> = [
            (lower.clone(), u64::from(invert)),
            (lower.complement(), u64::from(!invert)),
        ]
        .into_iter()
        .map(|(region, n)| {
            GuardedRationalFunction::new(
                d.clone(),
                c(n),
                c(1),
                limits().parameters.region,
                &mut work(),
            )
            .unwrap()
            .restrict(&region, limits().parameters.region, &mut work())
            .unwrap()
        })
        .collect();
        targets.push(
            ParameterFunction::new(
                d.clone(),
                &pieces,
                limits().parameters.region,
                limits().functions,
                &mut work(),
            )
            .unwrap(),
        );
    }
    targets.push(scalar(d, 0, 1));
    assert!(matches!(
        FamilyFunction::from_parameter(&source, &targets[0], limits(), &mut work()),
        Err(Error::ParameterRefinementRequired)
    ));
    let receipt = source
        .parameter_jeffrey(
            SpaceId([217; 32]),
            &partition(&source, &a),
            &targets,
            limits(),
            &mut work(),
        )
        .unwrap();
    let revised = receipt.revised().unwrap();
    let head = revised.pullback(&a, &()).unwrap();
    assert_eq!(
        revised.prior().full().to_bytes(&()).unwrap(),
        source.full().to_bytes(&()).unwrap()
    );
    for n in 0..=4 {
        assert_eq!(at(&mass(&head), n, 4), Some(u64::from(n >= 2).into()));
    }
    assert!(!head.complement().is_empty());
    let descriptor = Descriptor::capture(
        &AdmittedDescriptor::Map(revised.translation().clone()),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap();
    let encoded = head.to_bytes(&()).unwrap();
    drop((source, a, targets, receipt, head));
    let decoded = Event::from_bytes(&encoded, &()).unwrap();
    assert_eq!(at(&mass(&decoded), 1, 2), Some(1u64.into()));
    let AdmittedDescriptor::Map(decoded_map) =
        descriptor.admit(DescriptorLimits::default(), &()).unwrap()
    else {
        panic!("map");
    };
    assert_eq!(
        decoded_map.source().full().to_bytes(&()).unwrap(),
        decoded.space().full().to_bytes(&()).unwrap()
    );
}

#[test]
fn family_jeffrey_owns_each_unsupported_region_and_all_invalid_requests() {
    let source = shared_bias();
    let a = source.coordinate(0, &()).unwrap();
    let d = source.parameter_domain().unwrap();
    let receipt = source
        .parameter_jeffrey(
            SpaceId([218; 32]),
            &partition(&source, &a),
            &[scalar(d, 1, 2), scalar(d, 1, 2), scalar(d, 0, 1)],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert!(
        receipt.unsupported_regions()[0]
            .equivalent(
                &sign(&p(), PolynomialSigns::ZERO),
                limits().parameters.region,
                &mut work()
            )
            .unwrap()
    );
    assert!(
        receipt.unsupported_regions()[1]
            .equivalent(
                &sign(&sub(&p(), &c(1)), PolynomialSigns::ZERO),
                limits().parameters.region,
                &mut work()
            )
            .unwrap()
    );
    let revised = receipt.revised().unwrap();
    assert_eq!(at(&mass(&revised.pullback(&a, &()).unwrap()), 0, 1), None);
    assert_eq!(revised.space().full().atom_count(&()).unwrap(), 4);
    assert!(matches!(
        revised.translation().clone().certify_surjective(&()),
        Err(Error::IncompleteImage)
    ));
    let empty_cells = EventPartition::on(
        &source.full(),
        &[source.empty(), source.full()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap();
    let point_only = source
        .parameter_jeffrey(
            SpaceId([219; 32]),
            &empty_cells,
            &[function(d, p(), c(1)), function(d, sub(&c(1), &p()), c(1))],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert_eq!(
        point_only
            .revised()
            .unwrap()
            .space()
            .full()
            .world_cardinality(&())
            .unwrap(),
        WorldCardinality::Finite(4)
    );
    let impossible = source
        .parameter_jeffrey(
            SpaceId([220; 32]),
            &empty_cells,
            &[scalar(d, 1, 1), scalar(d, 0, 1)],
            limits(),
            &mut work(),
        )
        .unwrap();
    assert!(impossible.is_impossible());
    assert!(impossible.defined_on().is_empty());
    drop((source, a, empty_cells, receipt, point_only));
    assert_eq!(impossible.partition().cells().len(), 2);
    assert_eq!(at(&impossible.targets()[0], 1, 2), Some(1u64.into()));
    assert_eq!(at(&impossible.old_masses()[0], 1, 2), Some(0u64.into()));
    assert_eq!(impossible.prior().full().atom_count(&()).unwrap(), 12);
}

#[test]
fn family_jeffrey_replacements_on_overlapping_partitions_need_not_commute() {
    let source = shared_bias();
    let a = source.coordinate(0, &()).unwrap();
    let x = a
        .apply(BoolOp4::XOR, &source.coordinate(1, &()).unwrap(), &())
        .unwrap();
    let update = |s: &Space, event: &Event, id| {
        s.parameter_jeffrey(
            SpaceId([id; 32]),
            &partition(s, event),
            &[
                scalar(s.parameter_domain().unwrap(), 1, 3),
                scalar(s.parameter_domain().unwrap(), 2, 3),
                scalar(s.parameter_domain().unwrap(), 0, 1),
            ],
            limits(),
            &mut work(),
        )
        .unwrap()
    };
    let ax = update(&source, &a, 221);
    let first = ax.revised().unwrap();
    let ax = update(first.space(), &first.pullback(&x, &()).unwrap(), 222);
    let ax_a = ax
        .revised()
        .unwrap()
        .pullback(&first.pullback(&a, &()).unwrap(), &())
        .unwrap();
    let xa = update(&source, &x, 223);
    let first = xa.revised().unwrap();
    let xa = update(first.space(), &first.pullback(&a, &()).unwrap(), 224);
    let xa_a = xa
        .revised()
        .unwrap()
        .pullback(&first.pullback(&a, &()).unwrap(), &())
        .unwrap();
    assert_eq!(at(&mass(&ax_a), 1, 2), Some(ratio("1", "3")));
    assert_eq!(at(&mass(&xa_a), 1, 2), Some(ratio("1", "3")));
    assert_ne!(at(&mass(&ax_a), 1, 4), at(&mass(&xa_a), 1, 4));
}

#[test]
fn family_jeffrey_refuses_invalid_targets_contexts_and_operational_failures() {
    struct Cancel;
    impl Control for Cancel {
        fn checkpoint(&self) -> Result<()> {
            Err(Error::Cancelled)
        }
    }
    let source = shared_bias();
    let cells = partition(&source, &source.coordinate(0, &()).unwrap());
    let d = source.parameter_domain().unwrap();
    let good = [scalar(d, 1, 2), scalar(d, 1, 2), scalar(d, 0, 1)];
    let id = SpaceId([225; 32]);
    assert!(matches!(
        source.parameter_jeffrey(id, &cells, &good[..2], limits(), &mut work()),
        Err(Error::PartitionArity)
    ));
    let bad = [scalar(d, 1, 1), scalar(d, 1, 1), scalar(d, 0, 1)];
    assert!(matches!(
        source.parameter_jeffrey(id, &cells, &bad, limits(), &mut work()),
        Err(Error::LawNotNormalized)
    ));
    let mut bad = good.clone();
    bad[0] = function(d, p(), p());
    assert!(matches!(
        source.parameter_jeffrey(id, &cells, &bad, limits(), &mut work()),
        Err(Error::UndefinedFunction)
    ));
    bad[0] = function(d, ExactPolynomial::constant((-1i64).into()), c(1));
    assert!(matches!(
        source.parameter_jeffrey(id, &cells, &bad, limits(), &mut work()),
        Err(Error::NegativeMass)
    ));
    let foreign = space(226, 1, &[]);
    let foreign_cells = partition(&foreign, &foreign.coordinate(0, &()).unwrap());
    assert!(matches!(
        source.parameter_jeffrey(id, &foreign_cells, &good, limits(), &mut work()),
        Err(Error::SpaceMismatch)
    ));
    assert!(matches!(
        foreign.parameter_jeffrey(id, &foreign_cells, &good, limits(), &mut work()),
        Err(Error::MissingLaw)
    ));
    let partial = EventPartition::on(
        &source.coordinate(0, &()).unwrap(),
        &[source.full()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap();
    assert!(matches!(
        source.parameter_jeffrey(
            id,
            &partial,
            &[scalar(&domain(), 1, 1)],
            limits(),
            &mut work()
        ),
        Err(Error::PartitionGap)
    ));
    assert!(matches!(
        source.parameter_jeffrey(
            id,
            &cells,
            &good,
            limits(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Cancel)
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        source.parameter_jeffrey(
            id,
            &cells,
            &good,
            ParameterSourceLimits {
                steps: 0,
                ..limits()
            },
            &mut work()
        ),
        Err(Error::Capacity(Capacity::ParameterSourceSteps))
    ));
    assert!(matches!(
        source.parameter_jeffrey(
            id,
            &cells,
            &good,
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
}
