#![allow(clippy::too_many_lines)]
use super::*;
use crate::event::{
    ArithmeticLimits, Control, DensityPiece, ExactPolynomial as Poly, ExactRational as Rat,
    GuardedRationalFunction, LawLimits, Limits, NumberOp, NumberPredicateView, ParameterCell,
    ParameterDensityPiece, ParameterId, ParameterLimits, ParameterRegion, ParameterSourceLimits,
    ParameterWorld, RealWitness, Space, SpaceId,
};
use crate::{ObservationComponent, ObservationNumberCodecLimits, ProbabilityAnswer};
use std::hash::{Hash, Hasher};

type N = ObservationNumber;
type P = ObservationPredicate;
type I = ObservationPredicateImport;
fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn limits() -> ObservationNumberLimits {
    ObservationNumberLimits::default()
}
fn codec() -> ObservationNumberCodecLimits {
    ObservationNumberCodecLimits::default()
}
fn q(n: i64, d: u64) -> Rat {
    Rat::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn number(n: i64) -> N {
    N::literal(q(n, 1), limits(), &mut work()).unwrap()
}
fn truth(value: Option<bool>) -> P {
    let number = match value {
        Some(value) => number(i64::from(value)),
        None => number(1)
            .apply(NumberOp::Divide, &number(0), limits(), &mut work())
            .unwrap(),
    };
    number
        .where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
        .unwrap()
}
fn fixed(value: &P) -> Option<bool> {
    let NumberPredicateView::Fixed(value) = value.predicate().view() else {
        panic!("fixed")
    };
    value
}
fn roundtrip(value: &P) -> I {
    let import = I::capture(value, codec(), &mut work()).unwrap();
    let decoded = I::from_bytes(import.bytes(), codec(), &mut work()).unwrap();
    assert_eq!(import, decoded);
    assert!(
        value
            .equivalent(decoded.value(), limits(), &mut work())
            .unwrap()
    );
    let hash = |value: &I| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut h);
        h.finish()
    };
    assert_eq!(hash(&import), hash(&decoded));
    decoded
}
fn domain(parameter: ParameterId) -> ParameterDomain {
    let p = Poly::parameter(parameter);
    let bound = Poly::one()
        .sub(&p, ParameterLimits::default().polynomial, &mut work())
        .unwrap();
    let a = ParameterRegion::from_polynomial(
        parameter,
        &p,
        PolynomialSigns::NON_NEGATIVE,
        ParameterLimits::default(),
        &mut work(),
    )
    .unwrap();
    let b = ParameterRegion::from_polynomial(
        parameter,
        &bound,
        PolynomialSigns::NON_NEGATIVE,
        ParameterLimits::default(),
        &mut work(),
    )
    .unwrap();
    ParameterDomain::new(
        a.apply(BoolOp4::AND, &b, ParameterLimits::default(), &mut work())
            .unwrap(),
    )
    .unwrap()
}
fn source() -> Space {
    let parameter = ParameterId([212; 32]);
    let p = Poly::parameter(parameter);
    let limits = ParameterSourceLimits::default();
    let domain = domain(parameter);
    let tail = Poly::one()
        .sub(&p, limits.parameters.region.polynomial, &mut work())
        .unwrap();
    let raw = Space::new(SpaceId([213; 32]), 1, &())
        .unwrap()
        .with_parameters(domain.clone(), &[], limits, &mut work())
        .unwrap();
    let bit = raw.coordinate(0, &()).unwrap();
    let density = |n| {
        GuardedRationalFunction::new(
            domain.clone(),
            n,
            Poly::one(),
            limits.parameters.region,
            &mut work(),
        )
        .unwrap()
    };
    raw.with_parameter_density(
        &[
            ParameterDensityPiece {
                region: bit.clone(),
                density: density(p),
            },
            ParameterDensityPiece {
                region: bit.complement(),
                density: density(tail),
            },
        ],
        limits,
        &mut work(),
    )
    .unwrap()
}
fn measured(source: &Space) -> N {
    N::probability(
        ProbabilityAnswer::new(
            source.coordinate(0, &()).unwrap(),
            source.full(),
            &mut work(),
        )
        .unwrap(),
        ObservationComponent::Value,
        limits(),
        &mut work(),
    )
    .unwrap()
}

#[test]
fn common_predicate_refinement_preserves_roster_origins_and_one_joint_presentation() {
    let source = source();
    let p = measured(&source);
    let half = N::literal(q(1, 2), limits(), &mut work()).unwrap();
    let low = p
        .compare(&half, PolynomialSigns::NEGATIVE, limits(), &mut work())
        .unwrap();
    let high = p
        .compare(&half, PolynomialSigns::POSITIVE, limits(), &mut work())
        .unwrap();
    let partial = p
        .apply(NumberOp::Divide, &p, limits(), &mut work())
        .unwrap()
        .where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
        .unwrap();
    let roster = [low.clone(), high.clone(), partial.clone()];
    let id = SpaceId([230; 32]);
    let source_limits = ParameterSourceLimits::default();
    let common = |roster: &[P], limits, work: &mut ExactArithmetic<'_>| {
        PredicateRefinement::common(id, &source, roster, Limits::default(), limits, work)
    };
    let first = common(&roster, source_limits, &mut work()).unwrap();
    let reordered = common(
        &[partial, high, low.clone(), low],
        source_limits,
        &mut work(),
    )
    .unwrap();
    let bytes = first[0]
        .refinement()
        .refined()
        .full()
        .to_bytes(&())
        .unwrap();
    for value in first.iter().chain(&reordered) {
        assert_eq!(
            value.refinement().refined().full().to_bytes(&()).unwrap(),
            bytes
        );
    }
    for (input, result) in roster.iter().zip(&first) {
        assert_eq!(roundtrip(input), roundtrip(result.events().predicate()));
    }
    let cases: Vec<_> = first.iter().map(PredicateRefinement::events).collect();
    assert!(
        cases[0]
            .holds()
            .apply(BoolOp4::AND, cases[1].holds(), &())
            .unwrap()
            .is_empty()
    );
    assert!(!cases[2].undefined().is_empty());
    for n in 0..=4 {
        for outcomes in 0..2 {
            let world = ParameterWorld {
                parameter: RealWitness::Rational(q(n, 4)),
                outcomes,
            };
            for (index, case) in cases.iter().enumerate() {
                let expected = [
                    Some(n < 2),
                    Some(n > 2),
                    if n == 0 { None } else { Some(true) },
                ][index];
                for (event, truth) in [
                    (case.holds(), Some(true)),
                    (case.fails(), Some(false)),
                    (case.undefined(), None),
                ] {
                    assert_eq!(
                        event
                            .contains_parameter(&world, source_limits, &mut work())
                            .unwrap(),
                        expected == truth
                    );
                }
            }
        }
    }
    let claim = source.coordinate(0, &()).unwrap();
    let lifted = first[0].refinement().lift(&claim, &()).unwrap();
    assert_eq!(first[0].refinement().descend(&lifted, &()).unwrap(), claim);
    assert!(
        first[0]
            .refinement()
            .descend(cases[0].holds(), &())
            .is_err()
    );
    assert!(common(&[], source_limits, &mut work()).is_err());
    assert!(
        common(
            &roster,
            ParameterSourceLimits {
                steps: 1,
                ..source_limits
            },
            &mut work()
        )
        .is_err()
    );
    let mut small = source_limits;
    small.parameters.bytes = 1;
    assert!(common(&roster, small, &mut work()).is_err());
    let foreign = truth(Some(true))
        .on_domain(&domain(ParameterId([231; 32])), limits(), &mut work())
        .unwrap();
    assert!(common(&[truth(Some(true)), foreign], source_limits, &mut work()).is_err());
    let control = crate::WorkContext::new();
    control.cancel();
    assert!(
        common(
            &roster,
            source_limits,
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &control)
        )
        .is_err()
    );
}
#[test]
fn common_source_rosters_share_bytes_before_deduplication_and_exact_work() {
    let source = source();
    let source_limits = ParameterSourceLimits::default();
    let predicate = measured(&source)
        .where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
        .unwrap();
    let peer = predicate
        .refine(
            SpaceId([233; 32]),
            &source,
            Limits::default(),
            source_limits,
            &mut work(),
        )
        .unwrap();
    let peers = vec![peer.events().source().clone(); 100];
    let roster = [truth(Some(true))];
    let run = |peers: &[Space], limits, work: &mut ExactArithmetic<'_>| {
        PredicateRefinement::common_sources(
            SpaceId([234; 32]),
            &source,
            peers,
            &roster,
            Limits::default(),
            limits,
            work,
        )
    };
    let once = run(&peers[..1], source_limits, &mut work()).unwrap();
    let repeated = run(&peers, source_limits, &mut work()).unwrap();
    assert_eq!(
        once[0].events().holds().to_bytes(&()).unwrap(),
        repeated[0].events().holds().to_bytes(&()).unwrap()
    );
    let mut small = source_limits;
    small.parameters.bytes = 1024;
    assert!(matches!(
        run(&peers, small, &mut work()),
        Err(crate::Error::Event(Error::Capacity(
            Capacity::DescriptorBytes
        )))
    ));
    assert!(
        run(
            &peers[..1],
            source_limits,
            &mut ExactArithmetic::new(
                ArithmeticLimits {
                    operations: 0,
                    ..ArithmeticLimits::default()
                },
                &()
            )
        )
        .is_err()
    );
    let control = crate::WorkContext::new();
    control.cancel();
    assert!(
        run(
            &peers[..1],
            source_limits,
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &control)
        )
        .is_err()
    );
}

fn value_at(value: &P, point: &Rat) -> Option<bool> {
    let NumberPredicateView::Parameter {
        holds,
        fails,
        undefined,
        ..
    } = value.predicate().view()
    else {
        return fixed(value);
    };
    let at = |region: &ParameterRegion| {
        region
            .contains(
                &RealWitness::Rational(point.clone()),
                ParameterLimits::default(),
                &mut work(),
            )
            .unwrap()
    };
    let members = [at(holds), at(fails), at(undefined)];
    assert_eq!(members.iter().filter(|&&b| b).count(), 1);
    if members[0] {
        Some(true)
    } else if members[1] {
        Some(false)
    } else {
        None
    }
}

#[test]
fn all_partial_truth_tables_replay_strictly_and_retain_written_operands() {
    for bits in 0..16 {
        let op = BoolOp4::new(bits).unwrap();
        for left in [None, Some(false), Some(true)] {
            for right in [None, Some(false), Some(true)] {
                let value = truth(left)
                    .apply(op, &truth(right), limits(), &mut work())
                    .unwrap();
                let import = roundtrip(&value);
                assert_eq!(
                    fixed(import.value()),
                    left.zip(right).map(|(a, b)| op.evaluate(a, b))
                );
                let ObservationPredicateExpr::Binary {
                    op: actual,
                    left: a,
                    right: b,
                } = import.value().expression()
                else {
                    panic!("binary")
                };
                assert_eq!(*actual, op);
                assert_eq!((fixed(a), fixed(b)), (left, right));
                let opposite = roundtrip(&value.negate(limits(), &mut work()).unwrap());
                assert_eq!(fixed(opposite.value()), fixed(&value).map(|b| !b));
            }
        }
    }
    for n in [-1, 0, 1] {
        for bits in 0..8 {
            let signs = PolynomialSigns::new(bits).unwrap();
            let p = number(n).where_sign(signs, limits(), &mut work()).unwrap();
            assert_eq!(
                fixed(roundtrip(&p).value()),
                Some(signs.contains(n.cmp(&0)))
            );
        }
    }
    let a = roundtrip(&truth(Some(true)));
    let b = roundtrip(
        &truth(Some(true))
            .negate(limits(), &mut work())
            .unwrap()
            .negate(limits(), &mut work())
            .unwrap(),
    );
    assert_ne!(a, b, "written derivations stay distinct");
    assert!(
        a.value()
            .equivalent(b.value(), limits(), &mut work())
            .unwrap()
    );
    let shared = truth(None);
    let pair = shared
        .apply(BoolOp4::OR, &shared, limits(), &mut work())
        .unwrap();
    assert_eq!(
        roundtrip(&pair),
        roundtrip(
            &truth(None)
                .apply(BoolOp4::OR, &truth(None), limits(), &mut work())
                .unwrap()
        )
    );
}

#[test]
fn explicit_domains_retain_holes_and_reject_aliasing_or_extension() {
    let source = source();
    let p = measured(&source);
    let first = p
        .apply(NumberOp::Divide, &p, limits(), &mut work())
        .unwrap()
        .where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
        .unwrap();
    let tail = number(1)
        .apply(NumberOp::Subtract, &p, limits(), &mut work())
        .unwrap();
    let second = tail
        .apply(NumberOp::Divide, &tail, limits(), &mut work())
        .unwrap()
        .where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
        .unwrap();
    for bits in 0..16 {
        let op = BoolOp4::new(bits).unwrap();
        let result = roundtrip(&first.apply(op, &second, limits(), &mut work()).unwrap());
        assert_eq!(value_at(result.value(), &q(0, 1)), None);
        assert_eq!(value_at(result.value(), &q(1, 1)), None);
        assert_eq!(
            value_at(result.value(), &q(1, 2)),
            Some(op.evaluate(true, true))
        );
        assert!(!result.value().predicate().always());
    }
    let domain = source.parameter_domain().unwrap();
    let total = truth(Some(true))
        .on_domain(domain, limits(), &mut work())
        .unwrap();
    assert!(
        total
            .equivalent(&truth(Some(true)), limits(), &mut work())
            .unwrap()
    );
    assert!(!first.equivalent(&total, limits(), &mut work()).unwrap());
    let NumberPredicateView::Parameter { holds, .. } = first.predicate().view() else {
        panic!("parameter")
    };
    let narrower = ParameterDomain::new(holds.clone()).unwrap();
    let restricted = roundtrip(&first.on_domain(&narrower, limits(), &mut work()).unwrap());
    assert!(restricted.value().predicate().always());
    assert!(
        restricted
            .value()
            .on_domain(domain, limits(), &mut work())
            .is_err()
    );
    assert!(
        first
            .apply(BoolOp4::TRUE, restricted.value(), limits(), &mut work())
            .is_err()
    );
    let foreign = truth(Some(true))
        .on_domain(
            &super::tests::domain(ParameterId([214; 32])),
            limits(),
            &mut work(),
        )
        .unwrap();
    assert!(
        total
            .apply(BoolOp4::FALSE, &foreign, limits(), &mut work())
            .is_err()
    );
    assert!(total.equivalent(&foreign, limits(), &mut work()).is_err());
    let nowhere = roundtrip(
        &truth(None)
            .on_domain(domain, limits(), &mut work())
            .unwrap(),
    );
    assert!(
        nowhere
            .value()
            .equivalent(&truth(None), limits(), &mut work())
            .unwrap()
    );
    assert!(!nowhere.value().predicate().possibly());
    assert!(!nowhere.value().predicate().always());
    assert!(!nowhere.value().predicate().is_total());
}

#[test]
fn explicit_guards_preserve_actual_worlds_law_and_all_three_truth_cases() {
    let source = source();
    let p = measured(&source);
    // (p^2 - 1/2) / p: a genuine pole, an irrational boundary, and both signs.
    let delta = p
        .pow(2, limits(), &mut work())
        .unwrap()
        .apply(
            NumberOp::Subtract,
            &N::literal(q(1, 2), limits(), &mut work()).unwrap(),
            limits(),
            &mut work(),
        )
        .unwrap()
        .apply(NumberOp::Divide, &p, limits(), &mut work())
        .unwrap();
    let predicate = roundtrip(
        &delta
            .where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
            .unwrap(),
    );
    let limits = ParameterSourceLimits::default();
    assert!(matches!(
        predicate.value().events(&source, limits, &mut work()),
        Err(crate::Error::Event(Error::ParameterRefinementRequired))
    ));
    let refined = predicate
        .value()
        .refine(
            SpaceId([215; 32]),
            &source,
            Limits::default(),
            limits,
            &mut work(),
        )
        .unwrap();
    let cases = refined.events();
    let union = cases
        .holds()
        .apply(BoolOp4::OR, cases.fails(), &())
        .unwrap()
        .apply(BoolOp4::OR, cases.undefined(), &())
        .unwrap();
    assert_eq!(union, cases.source().full());
    for (a, b) in [
        (cases.holds(), cases.fails()),
        (cases.holds(), cases.undefined()),
        (cases.fails(), cases.undefined()),
    ] {
        assert!(a.apply(BoolOp4::AND, b, &()).unwrap().is_empty());
    }
    assert_eq!(
        cases.source().outcome_coordinates(),
        source.outcome_coordinates()
    );
    let bit = source.coordinate(0, &()).unwrap();
    let lifted = refined.refinement().lift(&bit, &()).unwrap();
    assert!(
        bit.parameter_mass(limits, &mut work())
            .unwrap()
            .equivalent(
                &lifted.parameter_mass(limits, &mut work()).unwrap(),
                limits.parameters.region,
                limits.functions,
                &mut work()
            )
            .unwrap()
    );
    for n in 0..=10 {
        let expected = if n == 0 { None } else { Some(2 * n * n > 100) };
        assert_eq!(value_at(predicate.value(), &q(n, 10)), expected);
        for outcome in 0..2 {
            let world = ParameterWorld {
                parameter: RealWitness::Rational(q(n, 10)),
                outcomes: outcome,
            };
            for (event, truth) in [
                (cases.holds(), Some(true)),
                (cases.fails(), Some(false)),
                (cases.undefined(), None),
            ] {
                assert_eq!(
                    event
                        .contains_parameter(&world, limits, &mut work())
                        .unwrap(),
                    truth == expected
                );
            }
            assert_eq!(
                lifted
                    .contains_parameter(&world, limits, &mut work())
                    .unwrap(),
                outcome == 1
            );
        }
    }
    // Check the exact irrational root, not a floating-point approximation.
    let NumberPredicateView::Parameter { holds, .. } = predicate.value().predicate().view() else {
        panic!("parameter")
    };
    let root = holds
        .cells()
        .find_map(|(cell, _)| match cell {
            ParameterCell::Point(root)
                if root
                    .compare_rational(&q(1, 2), limits.parameters.region.roots, &mut work())
                    .unwrap()
                    .is_gt()
                    && root
                        .compare_rational(&q(1, 1), limits.parameters.region.roots, &mut work())
                        .unwrap()
                        .is_lt() =>
            {
                Some(root.clone())
            }
            _ => None,
        })
        .expect("sqrt(1/2) boundary");
    for outcomes in 0..2 {
        let world = ParameterWorld {
            parameter: RealWitness::Algebraic(root.clone()),
            outcomes,
        };
        assert!(
            cases
                .fails()
                .contains_parameter(&world, limits, &mut work())
                .unwrap()
        );
        assert!(
            !cases
                .holds()
                .contains_parameter(&world, limits, &mut work())
                .unwrap()
        );
        assert!(
            !cases
                .undefined()
                .contains_parameter(&world, limits, &mut work())
                .unwrap()
        );
    }
    // The p=0 heads outcome has zero mass, yet is still a possible undefined world.
    assert!(
        !cases
            .undefined()
            .apply(BoolOp4::AND, &lifted, &())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        refined.refinement().descend(cases.holds(), &()),
        Err(Error::ParameterGuardEssential)
    ));
    let total = p
        .compare(
            &N::literal(q(1, 2), super::tests::limits(), &mut work()).unwrap(),
            PolynomialSigns::POSITIVE,
            super::tests::limits(),
            &mut work(),
        )
        .unwrap();
    let total_refinement = total
        .refine(
            SpaceId([219; 32]),
            &source,
            Limits::default(),
            limits,
            &mut work(),
        )
        .unwrap();
    assert_eq!(
        total_refinement.events().source().dimensions(),
        source.dimensions() + 1
    );
    assert!(total_refinement.events().undefined().is_empty());
    let again = predicate
        .value()
        .refine(
            SpaceId([216; 32]),
            cases.source(),
            Limits::default(),
            limits,
            &mut work(),
        )
        .unwrap();
    assert_eq!(
        again.events().source().dimensions(),
        cases.source().dimensions()
    );
    let NumberPredicateView::Parameter { ambient, fails, .. } =
        predicate.value().predicate().view()
    else {
        panic!("parameter")
    };
    let narrowed = ParameterDomain::new(fails.clone()).unwrap();
    let restricted = predicate
        .value()
        .on_domain(&narrowed, super::tests::limits(), &mut work())
        .unwrap();
    assert!(restricted.events(&source, limits, &mut work()).is_err());
    assert_eq!(
        ambient.parameter(),
        source.parameter_domain().unwrap().parameter()
    );
}

#[test]
fn replay_preserves_distinct_sources_and_rechecks_hidden_numeric_operands() {
    let raw = Space::new(SpaceId([217; 32]), 1, &()).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.coordinate(0, &()).unwrap(),
                density: q(1, 1),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let observed = |event| {
        N::probability(
            ProbabilityAnswer::new(event, source.full(), &mut work()).unwrap(),
            ObservationComponent::Value,
            limits(),
            &mut work(),
        )
        .unwrap()
        .where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
        .unwrap()
    };
    let first = roundtrip(&observed(source.full()));
    let second = roundtrip(&observed(source.coordinate(0, &()).unwrap()));
    assert_ne!(first, second);
    assert!(
        first
            .value()
            .equivalent(second.value(), limits(), &mut work())
            .unwrap()
    );
    let large = N::literal(
        Rat::fraction("18446744073709551616", "1", &mut work()).unwrap(),
        limits(),
        &mut work(),
    )
    .unwrap();
    let masked = large
        .apply(NumberOp::Subtract, &large, limits(), &mut work())
        .unwrap()
        .where_sign(PolynomialSigns::NONE, limits(), &mut work())
        .unwrap();
    let encoded = roundtrip(&masked);
    let mut tiny = ExactArithmetic::new(
        ArithmeticLimits {
            bits: 32,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    assert!(I::from_bytes(encoded.bytes(), codec(), &mut tiny).is_err());
    drop(source);
    let partition = second
        .value()
        .events(
            &Space::new(SpaceId([218; 32]), 0, &()).unwrap(),
            ParameterSourceLimits::default(),
            &mut work(),
        )
        .unwrap();
    assert!(partition.holds().is_full());
    assert!(partition.fails().is_empty());
    assert!(partition.undefined().is_empty());
}

#[test]
fn malformed_nested_streams_and_combined_bounds_refuse_without_panics() {
    let simple = roundtrip(&truth(Some(true)));
    for n in 0..simple.bytes().len() {
        assert!(I::from_bytes(&simple.bytes()[..n], codec(), &mut work()).is_err());
    }
    for (offset, byte) in [(0, b'X'), (4, 2), (5, 9), (6, 8), (7, 99)] {
        let mut bytes = simple.bytes().to_vec();
        bytes[offset] = byte;
        assert!(I::from_bytes(&bytes, codec(), &mut work()).is_err());
    }
    let mut extra = simple.bytes().to_vec();
    extra.push(0);
    assert!(I::from_bytes(&extra, codec(), &mut work()).is_err());
    for bits in [16, 255] {
        let mut bytes = b"BENP\x01\x02".to_vec();
        bytes.push(bits);
        bytes.extend_from_slice(&simple.bytes()[5..]);
        bytes.extend_from_slice(&simple.bytes()[5..]);
        assert!(I::from_bytes(&bytes, codec(), &mut work()).is_err());
    }
    let mut deep = truth(Some(true));
    for _ in 2..256 {
        deep = deep.negate(limits(), &mut work()).unwrap();
    }
    let import = roundtrip(&deep);
    assert!(deep.negate(limits(), &mut work()).is_err());
    let mut excessive = b"BENP\x01\x01".to_vec();
    excessive.extend_from_slice(&import.bytes()[5..]);
    assert!(I::from_bytes(&excessive, codec(), &mut work()).is_err());
    let mut small = codec();
    small.numbers.nodes = 255;
    assert!(I::from_bytes(import.bytes(), small, &mut work()).is_err());
    small = codec();
    small.sources.descriptors.bytes = simple.bytes().len() - 1;
    assert!(I::from_bytes(simple.bytes(), small, &mut work()).is_err());
    small = codec();
    small.sources.descriptors.items = 2; // sign, numeric literal, rational blob
    assert!(I::from_bytes(simple.bytes(), small, &mut work()).is_err());
    let pair = roundtrip(
        &truth(Some(true))
            .apply(BoolOp4::OR, &truth(Some(false)), limits(), &mut work())
            .unwrap(),
    );
    small = codec();
    small.numbers.nodes = 4; // one binary plus two (sign + number) trees
    assert!(I::from_bytes(pair.bytes(), small, &mut work()).is_err());
}

#[test]
fn expectation_components_replay_the_complete_indexed_payoff() {
    use crate::event::{EventPartition, PartitionLimits};
    use crate::{ExpectationAnswer, ExpectationPayoff, ObservationNumberExpr};
    let raw = Space::new(SpaceId([220; 32]), 1, &()).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.coordinate(0, &()).unwrap(),
                density: q(1, 1),
            }],
            LawLimits::default(),
            &mut work(),
        )
        .unwrap();
    let bit = source.coordinate(0, &()).unwrap();
    let partition = EventPartition::on(
        &source.full(),
        &[bit.clone(), bit.complement(), source.empty()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap();
    let answer = ExpectationAnswer::new(
        ExpectationPayoff::Scalar {
            partition,
            values: vec![q(3, 1), q(-7, 1), q(19, 1)],
        },
        &mut work(),
    )
    .unwrap();
    for component in [
        ObservationComponent::Value,
        ObservationComponent::Numerator,
        ObservationComponent::EvidenceMass,
    ] {
        let n = N::expectation(answer.clone(), component, limits(), &mut work()).unwrap();
        let encoded = roundtrip(
            &n.where_sign(PolynomialSigns::POSITIVE, limits(), &mut work())
                .unwrap(),
        );
        assert_eq!(fixed(encoded.value()), Some(true));
        let ObservationPredicateExpr::Sign { number, .. } = encoded.value().expression() else {
            panic!("sign")
        };
        let ObservationNumberExpr::Expectation {
            observation,
            component: selected,
        } = number.expression()
        else {
            panic!("expectation")
        };
        assert_eq!(component, *selected);
        assert_eq!(observation.values().unwrap(), [q(3, 1), q(-7, 1), q(19, 1)]);
        let cells = observation.partition().unwrap().cells();
        assert_eq!(cells.len(), 3);
        assert!(!cells[1].is_empty());
        assert!(cells[2].is_empty());
    }
}

#[test]
fn replay_and_guard_construction_share_work_and_obey_cancellation() {
    struct Stop;
    impl Control for Stop {
        fn checkpoint(&self) -> crate::event::Result<()> {
            Err(Error::Cancelled)
        }
    }
    let value = roundtrip(
        &truth(Some(true))
            .apply(BoolOp4::AND, &truth(None), limits(), &mut work())
            .unwrap(),
    );
    assert!(
        I::from_bytes(
            value.bytes(),
            codec(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Stop)
        )
        .is_err()
    );
    let mut measured = work();
    I::from_bytes(value.bytes(), codec(), &mut measured).unwrap();
    let operations = measured.operations();
    assert!(operations > 0);
    let mut bounded = ExactArithmetic::new(
        ArithmeticLimits {
            operations,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    I::from_bytes(value.bytes(), codec(), &mut bounded).unwrap();
    assert!(I::from_bytes(value.bytes(), codec(), &mut bounded).is_err());
    let source = source();
    let mut cancelled = ExactArithmetic::new(ArithmeticLimits::default(), &Stop);
    assert!(
        value
            .value()
            .events(&source, ParameterSourceLimits::default(), &mut cancelled)
            .is_err()
    );
    assert!(
        value
            .value()
            .refine(
                SpaceId([218; 32]),
                &source,
                Limits::default(),
                ParameterSourceLimits::default(),
                &mut cancelled
            )
            .is_err()
    );
}

#[test]
fn region_derivations_retain_unclipped_origins_and_versioned_replay() {
    let domain = domain(ParameterId([212; 32]));
    let full = ParameterRegion::full(domain.parameter());
    let clipped = P::region(&domain, domain.region(), limits(), &mut work()).unwrap();
    let global = P::region(&domain, &full, limits(), &mut work()).unwrap();
    assert!(clipped.predicate().always() && global.predicate().is_total());
    assert!(clipped.equivalent(&global, limits(), &mut work()).unwrap());
    let a = roundtrip(&clipped);
    let b = roundtrip(&global);
    assert_ne!(
        a, b,
        "authored region survives clipping in the truth partition"
    );
    assert_eq!(&a.bytes()[..5], b"BENP\x02");
    let ObservationPredicateExpr::Region { region, .. } = b.value().expression() else {
        panic!("region origin")
    };
    assert!(region.is_full());
    let hole = truth(None);
    let mixed = global
        .apply(BoolOp4::OR, &hole, limits(), &mut work())
        .unwrap();
    let mixed = roundtrip(&mixed);
    assert_eq!(mixed.bytes()[4], 2);
    assert!(!mixed.value().predicate().possibly() && !mixed.value().predicate().is_total());
    assert_eq!(
        roundtrip(&truth(Some(true))).bytes()[4],
        1,
        "v1 bytes stay v1"
    );
    for version in [0, 1, 3, 255] {
        let mut bytes = a.bytes().to_vec();
        bytes[4] = version;
        assert!(I::from_bytes(&bytes, codec(), &mut work()).is_err());
    }
    for n in 0..a.bytes().len() {
        assert!(I::from_bytes(&a.bytes()[..n], codec(), &mut work()).is_err());
    }
    let mut small = codec();
    small.sources.descriptors.items = 2;
    assert!(I::from_bytes(a.bytes(), small, &mut work()).is_err());
    small = codec();
    small.sources.descriptors.bytes = a.bytes().len() - 1;
    assert!(I::from_bytes(a.bytes(), small, &mut work()).is_err());
    let source = source();
    assert!(
        global
            .events(&source, ParameterSourceLimits::default(), &mut work())
            .unwrap()
            .holds()
            .is_full()
    );
    let foreign = ParameterRegion::empty(ParameterId([250; 32]));
    assert!(P::region(&domain, &foreign, limits(), &mut work()).is_err());
    let mut measured = work();
    I::from_bytes(a.bytes(), codec(), &mut measured).unwrap();
    let spent = measured.operations();
    assert!(spent > 0);
    let mut shared = ExactArithmetic::new(
        ArithmeticLimits {
            operations: spent,
            ..ArithmeticLimits::default()
        },
        &(),
    );
    I::from_bytes(a.bytes(), codec(), &mut shared).unwrap();
    assert!(I::from_bytes(a.bytes(), codec(), &mut shared).is_err());
    // Pinned BENP v2: Region(full domain, empty set), named parameter all zero.
    // Two independent 46-byte BEPR blobs use little-endian u32 lengths.
    let golden = "42454e5002042e000000424550520100000000000000000000000000000000000000000000000000000000000000000000000000000000012e00000042455052010000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    let bytes: Vec<u8> = golden
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let pinned = I::from_bytes(&bytes, codec(), &mut work()).unwrap();
    assert!(pinned.value().predicate().is_total() && !pinned.value().predicate().possibly());
    assert_eq!(pinned.bytes(), bytes);
    let context = crate::WorkContext::new();
    context.cancel();
    assert!(
        I::from_bytes(
            a.bytes(),
            codec(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &context)
        )
        .is_err()
    );
}
