#![allow(clippy::too_many_lines)]
use bumbledb::{
    AnswerValue, BindValue, Db, ObservationComponent as Component, ObservationNumber as Number,
    ObservationNumberExpr as Expr, ObservationNumberLimits as Limits,
    event::{
        ArithmeticLimits, DensityPiece, ExactArithmetic, ExactPolynomial as Poly,
        ExactRational as Rat, GuardedRationalFunction, LawLimits, NumberOp, NumberPredicateView,
        ParameterDensityPiece, ParameterDomain, ParameterId, ParameterRegion,
        ParameterSourceLimits, PartialNumber, PolynomialSigns as Signs, Space, SpaceId,
    },
    query,
};
mod common;

bumbledb::schema! {
    pub NumberQueries;
    relation Trial { id: u64, value: i64, when: event, given: event }
    Trial(id) -> Trial;
}
fn arithmetic() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn q(n: i64, d: u64) -> Rat {
    Rat::fraction(&n.to_string(), &d.to_string(), &mut arithmetic()).unwrap()
}
fn fixed(value: &Number) -> Option<String> {
    let PartialNumber::Fixed(value) = value.value() else {
        panic!("fixed")
    };
    value.as_ref().map(ToString::to_string)
}

#[test]
fn completed_database_observations_keep_their_full_derivations_after_close() {
    let path = common::TempDir::new("observation-number-owned");
    let db = Db::create(path.path(), NumberQueries, common::work())
        .unwrap()
        .unwrap();
    let raw = Space::new(SpaceId([251; 32]), 2, &()).unwrap();
    let source = raw
        .with_density(
            &[DensityPiece {
                region: raw.table(3, &[6], &()).unwrap(),
                density: q(1, 2),
            }],
            LawLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let a = source.coordinate(0, &()).unwrap();
    let b = source.coordinate(1, &()).unwrap();
    let zero = source.table(3, &[1], &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([
            &Trial {
                id: 1,
                value: 7,
                when: a.clone(),
                given: source.full(),
            },
            &Trial {
                id: 2,
                value: 7,
                when: b.clone(),
                given: source.full(),
            },
            &Trial {
                id: 3,
                value: 0,
                when: source.full(),
                given: zero.clone(),
            },
        ])
    })
    .unwrap()
    .unwrap();
    let observed = query!(NumberQueries {
        interior stage(id,chance: Probability(when,given)) | Trial(id,when,given);
        (id,chance) | stage(id,chance);
    });
    let mean = query!(NumberQueries { (mean: Expectation(value,given,given)) | Trial(id == 1,value,given); });
    let mut values = Vec::new();
    let mut means = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&observed, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answer = db
            .read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        let mut rows = Vec::new();
        for i in 0..answer.len() {
            let AnswerValue::U64(id) = answer.get(i, 0) else {
                panic!("id")
            };
            let AnswerValue::Probability(chance) = answer.get(i, 1) else {
                panic!("chance")
            };
            rows.push((
                id,
                Number::probability(
                    chance.clone(),
                    Component::Value,
                    Limits::default(),
                    &mut arithmetic(),
                )
                .unwrap(),
            ));
        }
        values.push(rows);
        let mut prepared = db.prepare(&mean, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let result = db
            .read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        let AnswerValue::Expectation(answer) = result.get(0, 0) else {
            panic!("mean")
        };
        means.push(
            Number::expectation(
                answer.clone(),
                Component::Value,
                Limits::default(),
                &mut arithmetic(),
            )
            .unwrap(),
        );
    }
    drop((db, raw, source));
    for (mut values, mean) in values.into_iter().zip(means) {
        values.sort_by_key(|(id, _)| *id);
        let [(_, first), (_, second), (_, missing)]: [_; 3] = values.try_into().unwrap();
        assert_eq!(fixed(&first), Some("1/2".into()));
        assert_eq!(fixed(&missing), None);
        assert!(
            first
                .equivalent(&second, Limits::default(), &mut arithmetic())
                .unwrap()
        );
        let difference = first
            .apply(
                NumberOp::Subtract,
                &second,
                Limits::default(),
                &mut arithmetic(),
            )
            .unwrap();
        assert_eq!(fixed(&difference), Some("0".into()));
        let Expr::Binary { left, right, .. } = difference.expression() else {
            panic!("binary")
        };
        let Expr::Probability {
            observation: left, ..
        } = left.expression()
        else {
            panic!("left source")
        };
        let Expr::Probability {
            observation: right, ..
        } = right.expression()
        else {
            panic!("right source")
        };
        assert_eq!(left.event().align_to(&a.space(), &()).unwrap(), a);
        assert_eq!(right.event().align_to(&b.space(), &()).unwrap(), b);
        assert_ne!(
            left, right,
            "numerically equal results retain distinct events"
        );
        let numerical_product = first
            .apply(
                NumberOp::Multiply,
                &second,
                Limits::default(),
                &mut arithmetic(),
            )
            .unwrap();
        assert_eq!(fixed(&numerical_product), Some("1/4".into()));
        let joint = left
            .event()
            .apply(bumbledb::event::BoolOp4::AND, right.event(), &())
            .unwrap();
        assert_eq!(
            joint
                .probability(left.given(), &mut arithmetic())
                .unwrap()
                .value(&mut arithmetic())
                .unwrap(),
            Some(q(0, 1))
        );
        let payoff = first
            .apply(
                NumberOp::Multiply,
                &mean,
                Limits::default(),
                &mut arithmetic(),
            )
            .unwrap();
        assert_eq!(fixed(&payoff), Some("7/2".into()));
        let Expr::Binary { right, .. } = payoff.expression() else {
            panic!("binary")
        };
        assert!(matches!(right.expression(), Expr::Expectation { .. }));
        let impossible = missing
            .compare(&first, Signs::ZERO, Limits::default(), &mut arithmetic())
            .unwrap();
        assert!(!impossible.predicate().always());
        assert!(!impossible.predicate().possibly());
        assert!(!impossible.predicate().is_total());
        let Expr::Probability { observation, .. } = missing.expression() else {
            panic!("missing source")
        };
        assert_eq!(
            observation.given().align_to(&zero.space(), &()).unwrap(),
            zero
        );
        let mass = Number::probability(
            (**observation).clone(),
            Component::EvidenceMass,
            Limits::default(),
            &mut arithmetic(),
        )
        .unwrap();
        assert_eq!(fixed(&mass), Some("0".into()));
        let numerator = Number::probability(
            (**observation).clone(),
            Component::Numerator,
            Limits::default(),
            &mut arithmetic(),
        )
        .unwrap();
        assert_eq!(fixed(&numerator), Some("0".into()));
        assert_eq!(
            fixed(
                &numerator
                    .apply(
                        NumberOp::Divide,
                        &mass,
                        Limits::default(),
                        &mut arithmetic()
                    )
                    .unwrap()
            ),
            None
        );
    }
}

#[test]
fn parameter_observation_numbers_retain_holes_sources_and_explicit_domain_restriction() {
    let limits = ParameterSourceLimits::default();
    let parameter = ParameterId([251; 32]);
    let p = Poly::parameter(parameter);
    let tail = Poly::one()
        .sub(&p, limits.parameters.region.polynomial, &mut arithmetic())
        .unwrap();
    let nonnegative = ParameterRegion::from_polynomial(
        parameter,
        &p,
        Signs::NON_NEGATIVE,
        limits.parameters.region,
        &mut arithmetic(),
    )
    .unwrap();
    let bounded = ParameterRegion::from_polynomial(
        parameter,
        &tail,
        Signs::NON_NEGATIVE,
        limits.parameters.region,
        &mut arithmetic(),
    )
    .unwrap();
    let domain = ParameterDomain::new(
        nonnegative
            .apply(
                bumbledb::event::BoolOp4::AND,
                &bounded,
                limits.parameters.region,
                &mut arithmetic(),
            )
            .unwrap(),
    )
    .unwrap();
    let raw = Space::new(SpaceId([252; 32]), 1, &())
        .unwrap()
        .with_parameters(domain.clone(), &[], limits, &mut arithmetic())
        .unwrap();
    let bit = raw.coordinate(0, &()).unwrap();
    let density = |poly: Poly| {
        GuardedRationalFunction::new(
            domain.clone(),
            poly,
            Poly::one(),
            limits.parameters.region,
            &mut arithmetic(),
        )
        .unwrap()
    };
    let source = raw
        .with_parameter_density(
            &[
                ParameterDensityPiece {
                    region: bit.clone(),
                    density: density(p.clone()),
                },
                ParameterDensityPiece {
                    region: bit.complement(),
                    density: density(tail),
                },
            ],
            limits,
            &mut arithmetic(),
        )
        .unwrap();
    let bit = source.coordinate(0, &()).unwrap();
    let dir = common::TempDir::new("parameter-observation-number");
    let db = Db::create(dir.path(), NumberQueries, common::work())
        .unwrap()
        .unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Trial {
            id: 1,
            value: 0,
            when: bit.clone(),
            given: bit.clone(),
        }])
    })
    .unwrap()
    .unwrap();
    let query = query!(NumberQueries { (chance: Probability(when,given), mean: Expectation(value,when,given)) | Trial(when,given,value); });
    let mut prepared = db.prepare(&query, common::work()).unwrap();
    let answers = db
        .read(common::work(), |tx| {
            tx.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    let AnswerValue::Probability(chance) = answers.get(0, 0) else {
        panic!("chance")
    };
    let chance = Number::probability(
        chance.clone(),
        Component::Value,
        Limits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let AnswerValue::Expectation(mean) = answers.get(0, 1) else {
        panic!("mean")
    };
    let mean = Number::expectation(
        mean.clone(),
        Component::Value,
        Limits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    drop((answers, prepared, db, source, raw));
    let one = Number::literal(q(1, 1), Limits::default(), &mut arithmetic()).unwrap();
    let predicate = chance
        .compare(&one, Signs::ZERO, Limits::default(), &mut arithmetic())
        .unwrap();
    assert!(predicate.predicate().possibly());
    assert!(!predicate.predicate().always());
    let NumberPredicateView::Parameter {
        undefined, holds, ..
    } = predicate.predicate().view()
    else {
        panic!("parameter")
    };
    assert!(
        undefined
            .contains(
                &bumbledb::event::RealWitness::Rational(q(0, 1)),
                limits.parameters.region,
                &mut arithmetic()
            )
            .unwrap()
    );
    let smaller = ParameterDomain::new(holds.clone()).unwrap();
    let restricted = chance
        .on_domain(&smaller, Limits::default(), &mut arithmetic())
        .unwrap();
    assert!(
        restricted
            .compare(&one, Signs::ZERO, Limits::default(), &mut arithmetic())
            .unwrap()
            .predicate()
            .always()
    );
    let Expr::OnDomain { value, .. } = restricted.expression() else {
        panic!("restriction")
    };
    let Expr::Probability { observation, .. } = value.expression() else {
        panic!("provenance")
    };
    assert_eq!(
        observation.given().align_to(&bit.space(), &()).unwrap(),
        bit
    );
    let powered = mean.pow(0, Limits::default(), &mut arithmetic()).unwrap();
    let PartialNumber::Parameter(function) = powered.value() else {
        panic!("parameter")
    };
    assert_eq!(
        function
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
        function
            .value_at(
                &q(1, 2),
                limits.parameters.region,
                limits.functions,
                &mut arithmetic()
            )
            .unwrap(),
        Some(q(1, 1))
    );
}

#[test]
fn owned_derivation_shape_is_bounded_before_publication() {
    let mut value = Number::literal(q(1, 1), Limits::default(), &mut arithmetic()).unwrap();
    let narrow = Limits {
        nodes: 3,
        depth: 2,
        ..Limits::default()
    };
    value = value
        .apply(NumberOp::Add, &value, narrow, &mut arithmetic())
        .unwrap();
    assert!(
        value
            .apply(NumberOp::Add, &value, narrow, &mut arithmetic())
            .is_err()
    );
    assert_eq!(fixed(&value), Some("2".into()));
    let deep = Limits {
        nodes: usize::MAX,
        depth: usize::MAX,
        ..Limits::default()
    };
    let mut value = Number::literal(q(1, 1), deep, &mut arithmetic()).unwrap();
    for _ in 1..256 {
        value = value.negate(deep, &mut arithmetic()).unwrap();
    }
    assert!(value.negate(deep, &mut arithmetic()).is_err());
}
