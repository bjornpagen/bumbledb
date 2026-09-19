//! Query numbers feed an explicit, owned truth partition. Its Event cases use
//! ordinary dependencies and Free Join after persistence/reopen.
#![allow(clippy::too_many_lines)]
use bumbledb::{
    AnswerValue, BindValue, Db, ObservationNumberCodecLimits, ObservationNumberLimits,
    ObservationPredicateImport,
    event::{
        ArithmeticLimits, BoolOp4, ExactArithmetic, ExactPolynomial as Poly, ExactRational as Rat,
        GuardedRationalFunction, Limits, ParameterDensityPiece, ParameterDomain, ParameterId,
        ParameterRegion, ParameterSourceLimits, ParameterWorld, PolynomialSigns as Signs,
        RealWitness, Space, SpaceId,
    },
    query,
};
mod common;

bumbledb::schema! {
    pub PredicateGuards;
    relation Trial { game: u64, claim: event, given: event }
    relation Decision { game: u64 }
    relation Truth { game: u64, case: u64, when: event }
    relation Claim { game: u64, when: event }
    Trial(game) -> Trial;
    Decision(game) -> Decision;
    Decision(game, true) -> Decision;
    Truth(game, case) -> Truth;
    Truth(game, when) -> Truth;
    Truth(game) <= Decision(game);
    Decision(game, true) == Truth(game, when);
    Claim(game) -> Claim;
    Claim(game) <= Decision(game);
}

fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn q(n: i64, d: u64) -> Rat {
    Rat::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn source() -> Space {
    let limits = ParameterSourceLimits::default();
    let parameter = ParameterId([210; 32]);
    let p = Poly::parameter(parameter);
    let tail = Poly::one()
        .sub(&p, limits.parameters.region.polynomial, &mut work())
        .unwrap();
    let positive = |poly: &Poly| {
        ParameterRegion::from_polynomial(
            parameter,
            poly,
            Signs::NON_NEGATIVE,
            limits.parameters.region,
            &mut work(),
        )
        .unwrap()
    };
    let domain = ParameterDomain::new(
        positive(&p)
            .apply(
                BoolOp4::AND,
                &positive(&tail),
                limits.parameters.region,
                &mut work(),
            )
            .unwrap(),
    )
    .unwrap();
    let raw = Space::new(SpaceId([211; 32]), 1, &())
        .unwrap()
        .with_parameters(domain.clone(), &[], limits, &mut work())
        .unwrap();
    let bit = raw.coordinate(0, &()).unwrap();
    let density = |poly| {
        GuardedRationalFunction::new(
            domain.clone(),
            poly,
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

#[test]
fn three_truth_regions_flow_from_query_arithmetic_into_dependencies_and_free_join() {
    let path = common::TempDir::new("predicate-guards");
    let db = Db::create(path.path(), PredicateGuards, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    db.write(common::work(), |tx| {
        tx.insert([&Trial {
            game: 1,
            claim: source.coordinate(0, &()).unwrap(),
            given: source.full(),
        }])
    })
    .unwrap()
    .unwrap();
    let score = query!(PredicateGuards {
        interior chances(game,p: Probability(claim,given)) | Trial(game,claim,given);
        (game,advantage: Number((3 * Value(p) - 1) / Value(p))) | chances(game,p);
    });
    let mut predicates = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&score, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let result = db
            .read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        let AnswerValue::Number(number) = result.get(0, 1) else {
            panic!("number")
        };
        let predicate = number
            .value()
            .where_sign(
                Signs::POSITIVE,
                ObservationNumberLimits::default(),
                &mut work(),
            )
            .unwrap();
        predicates.push(
            ObservationPredicateImport::capture(
                &predicate,
                ObservationNumberCodecLimits::default(),
                &mut work(),
            )
            .unwrap(),
        );
    }
    assert_eq!(predicates[0], predicates[1]);
    drop((db, score));
    let predicate = ObservationPredicateImport::from_bytes(
        predicates[0].bytes(),
        ObservationNumberCodecLimits::default(),
        &mut work(),
    )
    .unwrap();
    let refinement = predicate
        .value()
        .refine(
            SpaceId([212; 32]),
            &source,
            Limits::default(),
            ParameterSourceLimits::default(),
            &mut work(),
        )
        .unwrap();
    let cases = refinement.events();
    let claim = refinement
        .refinement()
        .lift(&source.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let db = Db::open(path.path(), PredicateGuards, common::work()).unwrap();
    // Undefined assignments are part of the required full partition.
    let gaps = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Decision { game: 1 }])?;
        tx.insert([
            &Truth {
                game: 1,
                case: 1,
                when: cases.holds().clone(),
            },
            &Truth {
                game: 1,
                case: 2,
                when: cases.fails().clone(),
            },
        ])?;
        Ok(())
    }));
    assert!(!gaps.is_empty());
    db.write(common::work(), |tx| {
        tx.insert([&Decision { game: 1 }])?;
        tx.insert([
            &Truth {
                game: 1,
                case: 1,
                when: cases.holds().clone(),
            },
            &Truth {
                game: 1,
                case: 2,
                when: cases.fails().clone(),
            },
            &Truth {
                game: 1,
                case: 3,
                when: cases.undefined().clone(),
            },
        ])?;
        tx.insert([&Claim {
            game: 1,
            when: claim.clone(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let overlaps = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Truth {
            game: 1,
            case: 4,
            when: cases.holds().clone(),
        }])
    }));
    assert!(!overlaps.is_empty());
    drop((db, source, refinement, predicates, predicate));
    let db = Db::open(path.path(), PredicateGuards, common::work()).unwrap();
    let consumers = query!(PredicateGuards {
        (case,when: Event(region & held)) | Truth(game,case,when:region),Claim(game:game,when:held);
    });
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&consumers, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        retained.push(
            db.read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop(db);
    for answer in retained {
        assert_eq!(answer.len(), 3);
        for row in 0..answer.len() {
            let AnswerValue::U64(case) = answer.get(row, 0) else {
                panic!("case")
            };
            let AnswerValue::Event(region) = answer.get(row, 1) else {
                panic!("event")
            };
            for n in 0..=6 {
                for outcomes in 0..2 {
                    let world = ParameterWorld {
                        parameter: RealWitness::Rational(q(n, 6)),
                        outcomes,
                    };
                    let expected = outcomes == 1
                        && match case {
                            1 => n > 2,
                            2 => n > 0 && n <= 2,
                            3 => n == 0,
                            _ => panic!("case"),
                        };
                    assert_eq!(
                        region
                            .contains_parameter(
                                &world,
                                ParameterSourceLimits::default(),
                                &mut work()
                            )
                            .unwrap(),
                        expected
                    );
                }
            }
            if case == 3 {
                assert!(!region.is_empty(), "zero-mass worlds remain possible");
                let mass = region
                    .parameter_mass(ParameterSourceLimits::default(), &mut work())
                    .unwrap();
                assert!(
                    mass.where_sign(
                        Signs::NON_ZERO,
                        ParameterSourceLimits::default().parameters.region,
                        ParameterSourceLimits::default().functions,
                        &mut work()
                    )
                    .unwrap()
                    .is_empty()
                );
            }
        }
    }
}
