#![allow(clippy::too_many_lines)]
use super::*;
use crate::event::{
    ArithmeticLimits, BoolOp4, DensityPiece, ExactPolynomial as Poly, FunctionLimits,
    GuardedRationalFunction, LawLimits, ParameterDensityPiece, ParameterId, ParameterRegion,
    ParameterSourceLimits, PartialNumber, PartitionLimits, PolynomialSigns, Space, SpaceId,
};
use std::hash::{Hash, Hasher};

type Number = ObservationNumber;
type Import = ObservationNumberImport;
type Limits = ObservationNumberCodecLimits;
type Component = ObservationComponent;
fn arithmetic() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn q(n: i64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut arithmetic()).unwrap()
}
fn literal(n: i64) -> Number {
    Number::literal(
        q(n, 1),
        ObservationNumberLimits::default(),
        &mut arithmetic(),
    )
    .unwrap()
}
fn fixed_source() -> Space {
    let space = Space::new(SpaceId([219; 32]), 1, &()).unwrap();
    space
        .with_density(
            &[DensityPiece {
                region: space.coordinate(0, &()).unwrap(),
                density: q(1, 1),
            }],
            LawLimits::default(),
            &mut arithmetic(),
        )
        .unwrap()
}
fn probability(event: Event, given: Event, component: Component) -> Number {
    Number::probability(
        ProbabilityAnswer::new(event, given, &mut arithmetic()).unwrap(),
        component,
        ObservationNumberLimits::default(),
        &mut arithmetic(),
    )
    .unwrap()
}
fn expectation(payoff: ExpectationPayoff, component: Component) -> Number {
    Number::expectation(
        ExpectationAnswer::new(payoff, &mut arithmetic()).unwrap(),
        component,
        ObservationNumberLimits::default(),
        &mut arithmetic(),
    )
    .unwrap()
}
fn round_trip(value: &Number) -> Import {
    let encoded = Import::capture(value, Limits::default(), &mut arithmetic()).unwrap();
    let decoded =
        Import::from_bytes(encoded.bytes(), Limits::default(), &mut arithmetic()).unwrap();
    assert_eq!(encoded, decoded);
    assert!(
        value
            .value()
            .equivalent(
                decoded.value().value(),
                ObservationNumberLimits::default().numbers,
                &mut arithmetic()
            )
            .unwrap()
    );
    let hash = |v: &Import| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    };
    assert_eq!(hash(&encoded), hash(&decoded));
    decoded
}
fn raw(build: impl FnOnce(&mut Writer)) -> Vec<u8> {
    let mut output = Writer {
        bytes: Vec::new(),
        budget: Budget::new(Limits::default()),
    };
    output.put(b"BENO\x01", &()).unwrap();
    build(&mut output);
    output.bytes
}
fn event_error(result: Result<Import>, expected: Error) {
    let actual = result.unwrap_err();
    if expected == Error::Cancelled {
        let crate::Error::Store(error) = actual else {
            panic!("{actual:?}")
        };
        assert!(matches!(
            *error,
            crate::storage::store::StoreError::Work(crate::WorkError::Cancelled)
        ));
    } else {
        assert!(
            matches!(actual, crate::Error::Event(error) if error == expected),
            "expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn encoded_identity_retains_sources_components_and_written_operations() {
    let space = fixed_source();
    let bit = space.coordinate(0, &()).unwrap();
    // Both measure one, but their original Events differ.
    let a = probability(bit.clone(), space.full(), Component::Value);
    let b = probability(space.full(), space.full(), Component::Value);
    let ai = round_trip(&a);
    let bi = round_trip(&b);
    assert_ne!(ai, bi);
    let impossible = probability(space.full(), bit.complement(), Component::Value);
    assert!(matches!(
        round_trip(&impossible).value().value(),
        PartialNumber::Fixed(None)
    ));
    for component in [
        Component::Value,
        Component::Numerator,
        Component::EvidenceMass,
    ] {
        let number = probability(bit.clone(), space.full(), component);
        let output = round_trip(&number);
        let ObservationNumberExpr::Probability {
            observation,
            component: selected,
        } = output.value().expression()
        else {
            panic!("probability")
        };
        assert_eq!(*selected, component);
        assert_eq!(
            observation.event().to_bytes(&()).unwrap(),
            bit.to_bytes(&()).unwrap()
        );
        assert_eq!(
            observation.given().to_bytes(&()).unwrap(),
            space.full().to_bytes(&()).unwrap()
        );
        if component != Component::Value {
            assert_ne!(ai, output);
        }
    }
    for op in [
        NumberOp::Add,
        NumberOp::Subtract,
        NumberOp::Multiply,
        NumberOp::Divide,
        NumberOp::Min,
        NumberOp::Max,
    ] {
        for (left, right) in [
            (&a, &b),
            (&a, &impossible),
            (&impossible, &a),
            (&a, &literal(0)),
        ] {
            let value = left
                .apply(
                    op,
                    right,
                    ObservationNumberLimits::default(),
                    &mut arithmetic(),
                )
                .unwrap();
            let import = round_trip(&value);
            assert!(
                matches!(import.value().expression(), ObservationNumberExpr::Binary { op: actual, .. } if *actual == op)
            );
        }
    }
    for value in [&a, &impossible, &literal(-2)] {
        round_trip(
            &value
                .negate(ObservationNumberLimits::default(), &mut arithmetic())
                .unwrap(),
        );
        round_trip(
            &value
                .abs(ObservationNumberLimits::default(), &mut arithmetic())
                .unwrap(),
        );
        for n in [0, 1, 3] {
            round_trip(
                &value
                    .pow(n, ObservationNumberLimits::default(), &mut arithmetic())
                    .unwrap(),
            );
        }
    }
    let doubled = a
        .apply(
            NumberOp::Add,
            &a,
            ObservationNumberLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let separate = a
        .apply(
            NumberOp::Add,
            ai.value(),
            ObservationNumberLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    assert_eq!(
        round_trip(&doubled),
        round_trip(&separate),
        "Arc sharing is not portable identity"
    );
    assert_ne!(
        round_trip(&doubled),
        round_trip(&literal(2)),
        "algebraic simplification must not erase provenance"
    );
    drop((space, a, b, ai, doubled, separate));
    assert!(
        bi.value()
            .where_sign(
                PolynomialSigns::POSITIVE,
                ObservationNumberLimits::default(),
                &mut arithmetic()
            )
            .unwrap()
            .predicate()
            .always()
    );

    // The transport recomputes, rather than trusting even a deliberately
    // inconsistent private cached value. Public numbers cannot construct this.
    let forged = Number::new(
        ObservationNumberExpr::Literal(q(7, 1)),
        PartialNumber::Fixed(Some(q(0, 1))),
        (1, 1),
    );
    let replayed = Import::capture(&forged, Limits::default(), &mut arithmetic()).unwrap();
    assert!(
        matches!(replayed.value().value(), PartialNumber::Fixed(Some(value)) if *value == q(7, 1))
    );
}

#[test]
fn every_payoff_leaf_retains_the_indexed_roster_and_full_patch_source() {
    let space = fixed_source();
    let bit = space.coordinate(0, &()).unwrap();
    let cells = [bit.clone(), bit.complement(), space.empty()];
    let partition =
        EventPartition::on(&space.full(), &cells, PartitionLimits::default(), &()).unwrap();
    let payoff = ExpectationPayoff::Scalar {
        partition,
        values: vec![q(3, 1), q(-7, 1), q(19, 1)],
    };
    for component in [
        Component::Value,
        Component::Numerator,
        Component::EvidenceMass,
    ] {
        let result = round_trip(&expectation(payoff.clone(), component));
        let ObservationNumberExpr::Expectation {
            observation,
            component: selected,
        } = result.value().expression()
        else {
            panic!("expectation")
        };
        assert_eq!(component, *selected);
        let roster = observation.partition().unwrap();
        assert_eq!(roster.cells().len(), 3);
        assert!(
            !roster.cells()[1].is_empty(),
            "zero-mass outcome remains possible"
        );
        assert!(roster.cells()[2].is_empty());
        assert_eq!(observation.values().unwrap(), [q(3, 1), q(-7, 1), q(19, 1)]);
    }
    let function = FiniteFunction::constant(
        &space,
        q(3, 1),
        FunctionLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let cover = FiniteFunction::glue(
        &bit,
        &[
            FunctionPatch {
                region: space.full(),
                function: function.clone(),
            },
            FunctionPatch {
                region: space.empty(),
                function,
            },
        ],
        FunctionLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let result = round_trip(&expectation(
        ExpectationPayoff::Finite(cover),
        Component::Value,
    ));
    let ObservationNumberExpr::Expectation { observation, .. } = result.value().expression() else {
        panic!("expectation")
    };
    let ExpectationPayoff::Finite(cover) = observation.payoff() else {
        panic!("finite")
    };
    assert_eq!(cover.patches().len(), 2);
    assert!(
        cover.patches()[0].region.is_full(),
        "retain patch outside evidence"
    );
    assert!(cover.patches()[1].region.is_empty());
    assert_eq!(
        cover.parent().to_bytes(&()).unwrap(),
        bit.to_bytes(&()).unwrap()
    );
}

#[test]
fn shared_parameter_leaves_and_domain_restrictions_replay_without_a_prior() {
    let limits = ParameterSourceLimits::default();
    let id = ParameterId([220; 32]);
    let p = Poly::parameter(id);
    let tail = Poly::one()
        .sub(&p, limits.parameters.region.polynomial, &mut arithmetic())
        .unwrap();
    let lower = ParameterRegion::from_polynomial(
        id,
        &p,
        PolynomialSigns::NON_NEGATIVE,
        limits.parameters.region,
        &mut arithmetic(),
    )
    .unwrap();
    let upper = ParameterRegion::from_polynomial(
        id,
        &tail,
        PolynomialSigns::NON_NEGATIVE,
        limits.parameters.region,
        &mut arithmetic(),
    )
    .unwrap();
    let domain = ParameterDomain::new(
        lower
            .apply(
                BoolOp4::AND,
                &upper,
                limits.parameters.region,
                &mut arithmetic(),
            )
            .unwrap(),
    )
    .unwrap();
    let raw = Space::new(SpaceId([220; 32]), 1, &())
        .unwrap()
        .with_parameters(domain.clone(), &[], limits, &mut arithmetic())
        .unwrap();
    let value = |polynomial: Poly| {
        GuardedRationalFunction::new(
            domain.clone(),
            polynomial,
            Poly::one(),
            limits.parameters.region,
            &mut arithmetic(),
        )
        .unwrap()
    };
    let bit = raw.coordinate(0, &()).unwrap();
    let source = raw
        .with_parameter_density(
            &[
                ParameterDensityPiece {
                    region: bit.clone(),
                    density: value(p.clone()),
                },
                ParameterDensityPiece {
                    region: bit.complement(),
                    density: value(tail),
                },
            ],
            limits,
            &mut arithmetic(),
        )
        .unwrap();
    let bit = source.coordinate(0, &()).unwrap();
    let chance = probability(bit.clone(), bit.clone(), Component::Value);
    let function = FamilyFunction::constant(&source, value(p), limits, &mut arithmetic()).unwrap();
    let payoff = ExpectationPayoff::Family(
        FamilyFunction::glue(
            &bit,
            &[
                FunctionPatch {
                    region: source.full(),
                    function: function.clone(),
                },
                FunctionPatch {
                    region: source.empty(),
                    function,
                },
            ],
            limits,
            &mut arithmetic(),
        )
        .unwrap(),
    );
    for component in [
        Component::Value,
        Component::Numerator,
        Component::EvidenceMass,
    ] {
        round_trip(&expectation(payoff.clone(), component));
        round_trip(&probability(bit.clone(), bit.clone(), component));
    }
    let mean = expectation(payoff, Component::Value);
    let product = chance
        .apply(
            NumberOp::Multiply,
            &mean,
            ObservationNumberLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let result = round_trip(&product);
    let PartialNumber::Parameter(value) = result.value().value() else {
        panic!("family")
    };
    assert_eq!(value.ambient().parameter(), id);
    assert_eq!(
        value
            .value_at(
                &q(0, 1),
                limits.parameters.region,
                limits.functions,
                &mut arithmetic()
            )
            .unwrap(),
        None
    );
    assert_eq!(
        value
            .value_at(
                &q(1, 2),
                limits.parameters.region,
                limits.functions,
                &mut arithmetic()
            )
            .unwrap(),
        Some(q(1, 2))
    );
    let restricted = product
        .on_domain(
            &ParameterDomain::new(value.defined_on().clone()).unwrap(),
            ObservationNumberLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let result = round_trip(&restricted);
    let ObservationNumberExpr::OnDomain { value, .. } = result.value().expression() else {
        panic!("domain")
    };
    let ObservationNumberExpr::Binary { right, .. } = value.expression() else {
        panic!("product")
    };
    let ObservationNumberExpr::Expectation { observation, .. } = right.expression() else {
        panic!("mean")
    };
    let ExpectationPayoff::Family(cover) = observation.payoff() else {
        panic!("family")
    };
    assert_eq!(cover.patches().len(), 2);
    assert_eq!(
        observation.given().to_bytes(&()).unwrap(),
        bit.to_bytes(&()).unwrap()
    );
    assert_eq!(
        observation
            .given()
            .space()
            .parameter_domain()
            .unwrap()
            .region()
            .to_bytes(limits.parameters, &mut arithmetic())
            .unwrap(),
        domain
            .region()
            .to_bytes(limits.parameters, &mut arithmetic())
            .unwrap()
    );
    round_trip(
        &literal(9)
            .on_domain(
                &domain,
                ObservationNumberLimits::default(),
                &mut arithmetic(),
            )
            .unwrap(),
    );
}

#[test]
fn invalid_contexts_coverage_roles_and_noncanonical_rosters_are_refused() {
    let source = fixed_source();
    let bit = source.coordinate(0, &()).unwrap();
    let bare = Space::new(SpaceId([219; 32]), 1, &()).unwrap();
    let missing_law = raw(|out| {
        out.put(&[1, 0], &()).unwrap();
        out.event(&bare.full(), &mut arithmetic()).unwrap();
        out.event(&bare.full(), &mut arithmetic()).unwrap();
    });
    event_error(
        Import::from_bytes(&missing_law, Limits::default(), &mut arithmetic()),
        Error::MissingLaw,
    );
    let foreign = raw(|out| {
        out.put(&[1, 0], &()).unwrap();
        out.event(&bit, &mut arithmetic()).unwrap();
        out.event(&bare.full(), &mut arithmetic()).unwrap();
    });
    event_error(
        Import::from_bytes(&foreign, Limits::default(), &mut arithmetic()),
        Error::SpaceMismatch,
    );
    let roster = |given: &Event, regions: &[Event]| {
        raw(|out| {
            out.put(&[2, 0], &()).unwrap();
            out.event(given, &mut arithmetic()).unwrap();
            out.put(&[0], &()).unwrap();
            out.count(regions.len(), &()).unwrap();
            for region in regions {
                out.event(region, &mut arithmetic()).unwrap();
                out.blob(&q(1, 1).to_bytes(&mut arithmetic()).unwrap(), &())
                    .unwrap();
            }
        })
    };
    let gap = roster(&source.full(), std::slice::from_ref(&bit));
    event_error(
        Import::from_bytes(&gap, Limits::default(), &mut arithmetic()),
        Error::PartitionGap,
    );
    let overlap = roster(&source.full(), &[source.full(), bit.clone()]);
    event_error(
        Import::from_bytes(&overlap, Limits::default(), &mut arithmetic()),
        Error::PartitionOverlap,
    );
    // Native partition construction clips to evidence. Wire identity requires
    // already admitted cells, so the outside-evidence alias is rejected.
    let alias = roster(&bit, &[source.full()]);
    event_error(
        Import::from_bytes(&alias, Limits::default(), &mut arithmetic()),
        Error::InvalidEncoding,
    );
    let hidden_foreign = roster(&source.full(), &[source.full(), bare.empty()]);
    event_error(
        Import::from_bytes(&hidden_foreign, Limits::default(), &mut arithmetic()),
        Error::SpaceMismatch,
    );
    let wrong_role = raw(|out| {
        out.put(&[2, 0], &()).unwrap();
        out.event(&source.full(), &mut arithmetic()).unwrap();
        out.put(&[1], &()).unwrap();
        out.count(1, &()).unwrap();
        out.event(&source.full(), &mut arithmetic()).unwrap();
        out.blob(&q(1, 1).to_bytes(&mut arithmetic()).unwrap(), &())
            .unwrap();
    });
    assert!(Import::from_bytes(&wrong_role, Limits::default(), &mut arithmetic()).is_err());
}

#[test]
fn malformed_and_excessive_envelopes_refuse_before_publication() {
    let number = literal(-2)
        .apply(
            NumberOp::Divide,
            &literal(7),
            ObservationNumberLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let import = round_trip(&number);
    for end in 0..import.bytes().len() {
        assert!(
            Import::from_bytes(&import.bytes()[..end], Limits::default(), &mut arithmetic())
                .is_err(),
            "prefix {end}"
        );
    }
    let mut bytes = import.bytes().to_vec();
    bytes.push(0);
    event_error(
        Import::from_bytes(&bytes, Limits::default(), &mut arithmetic()),
        Error::InvalidEncoding,
    );
    for (offset, byte, expected) in [
        (0, 0, Error::InvalidEncoding),
        (4, 255, Error::UnsupportedVersion(255)),
        (5, 255, Error::InvalidEncoding),
        (6, 255, Error::InvalidEncoding),
    ] {
        let mut bytes = import.bytes().to_vec();
        bytes[offset] = byte;
        event_error(
            Import::from_bytes(&bytes, Limits::default(), &mut arithmetic()),
            expected,
        );
    }
    for (limits, error) in [
        (
            Limits {
                sources: SourceDescriptorLimits {
                    descriptors: crate::event::DescriptorLimits {
                        bytes: import.bytes().len() - 1,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            Capacity::DescriptorBytes,
        ),
        (
            Limits {
                sources: SourceDescriptorLimits {
                    descriptors: crate::event::DescriptorLimits {
                        items: 4,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            Capacity::DescriptorItems,
        ),
        (
            Limits {
                numbers: ObservationNumberLimits {
                    nodes: 2,
                    ..Default::default()
                },
                ..Default::default()
            },
            Capacity::ProgramNodes,
        ),
        (
            Limits {
                numbers: ObservationNumberLimits {
                    depth: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
            Capacity::ProgramNodes,
        ),
    ] {
        event_error(
            Import::from_bytes(import.bytes(), limits, &mut arithmetic()),
            Error::Capacity(error),
        );
    }
    let mut deep = b"BENO\x01".to_vec();
    deep.extend(std::iter::repeat_n(4, 257));
    deep.extend_from_slice(
        &encode(&literal(1), Limits::default(), &mut arithmetic()).unwrap()[5..],
    );
    let limits = Limits {
        numbers: ObservationNumberLimits {
            depth: usize::MAX,
            ..Default::default()
        },
        ..Default::default()
    };
    event_error(
        Import::from_bytes(&deep, limits, &mut arithmetic()),
        Error::Capacity(Capacity::ProgramNodes),
    );
    let mut deepest = literal(1);
    for _ in 1..256 {
        deepest = deepest
            .negate(ObservationNumberLimits::default(), &mut arithmetic())
            .unwrap();
    }
    round_trip(&deepest);
    let mut wrong_extent = encode(&literal(1), Limits::default(), &mut arithmetic()).unwrap();
    wrong_extent[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
    event_error(
        Import::from_bytes(&wrong_extent, Limits::default(), &mut arithmetic()),
        Error::InvalidEncoding,
    );
    let mut first = arithmetic();
    Import::from_bytes(import.bytes(), Limits::default(), &mut first).unwrap();
    let mut shared = ExactArithmetic::new(
        ArithmeticLimits {
            operations: first.operations(),
            ..Default::default()
        },
        &(),
    );
    Import::from_bytes(import.bytes(), Limits::default(), &mut shared).unwrap();
    event_error(
        Import::from_bytes(import.bytes(), Limits::default(), &mut shared),
        Error::Capacity(Capacity::ArithmeticSteps),
    );
    let control = crate::WorkContext::new();
    control.cancel();
    event_error(
        Import::from_bytes(
            import.bytes(),
            Limits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &control),
        ),
        Error::Cancelled,
    );
}

#[test]
fn replay_rechecks_original_leaves_even_when_arithmetic_erases_their_values() {
    let source = fixed_source();
    let huge = ExactRational::fraction(
        "123456789012345678901234567890123456789",
        "1",
        &mut arithmetic(),
    )
    .unwrap();
    let partition = EventPartition::on(
        &source.full(),
        &[source.full(), source.empty()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap();
    let mean = expectation(
        ExpectationPayoff::Scalar {
            partition,
            values: vec![q(1, 1), huge],
        },
        Component::Value,
    );
    let cancelled = mean
        .apply(
            NumberOp::Subtract,
            &mean,
            ObservationNumberLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let encoded = Import::capture(&cancelled, Limits::default(), &mut arithmetic()).unwrap();
    assert!(
        matches!(encoded.value().value(), PartialNumber::Fixed(Some(value)) if value.is_zero())
    );
    let mut small = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 32,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    event_error(
        Import::from_bytes(encoded.bytes(), Limits::default(), &mut small),
        Error::Capacity(Capacity::ArithmeticBits),
    );
    let limits = Limits {
        sources: SourceDescriptorLimits {
            partitions: PartitionLimits { cells: 1 },
            ..SourceDescriptorLimits::default()
        },
        ..Limits::default()
    };
    event_error(
        Import::from_bytes(encoded.bytes(), limits, &mut arithmetic()),
        Error::Capacity(Capacity::PartitionCells),
    );
    let limits = Limits {
        sources: SourceDescriptorLimits {
            laws: LawLimits {
                cells: 0,
                ..LawLimits::default()
            },
            ..SourceDescriptorLimits::default()
        },
        ..Limits::default()
    };
    event_error(
        Import::from_bytes(encoded.bytes(), limits, &mut arithmetic()),
        Error::Capacity(Capacity::LawCells),
    );
    let malicious = raw(|out| {
        out.put(&[2, 0], &()).unwrap();
        out.event(&source.full(), &mut arithmetic()).unwrap();
        out.put(&[0], &()).unwrap();
        out.count(u32::MAX as usize, &()).unwrap();
    });
    event_error(
        Import::from_bytes(&malicious, Limits::default(), &mut arithmetic()),
        Error::Capacity(Capacity::DescriptorItems),
    );
}
