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

#[test]
fn query_guards_refine_lift_descend_compose_and_retain_exact_partition() {
    use bumbledb::PredicateGuardPlan;
    let path = common::TempDir::new("query-guard-algebra");
    let db = Db::create(path.path(), PredicateGuards, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    let bit = source.coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Trial {
            game: 1,
            claim: bit.clone(),
            given: source.full(),
        }])
    })
    .unwrap()
    .unwrap();
    let plan = PredicateGuardPlan::capture(
        &source,
        Some(SpaceId([213; 32])),
        ObservationNumberCodecLimits::default(),
        &mut work(),
    )
    .unwrap();
    let query = query!(PredicateGuards {
        use guard g = &plan;
        interior observed(game,claim,p: Probability(claim,given)) | Trial(game,claim,given);
        interior truth(game,claim,p: Predicate((3*Value(p)-1)/Value(p)>0)) | observed(game,claim,p);
        interior cases(game,p,yes: Guard(Holds(p,g)),no: Guard(Fails(p,g)),unknown: Guard(Undefined(p,g)),held: Guard(Lift(p,g,claim))) | truth(game,claim,p);
        (game,yes,no,unknown,held,original: Guard(Descend(p,g,held)),win: Event(yes & held),missing: Event(unknown & held),whole: Event(yes | no | unknown)) | cases(game,p,yes,no,unknown,held);
    });
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        for _ in 0..2 {
            let answers = db
                .read(common::work(), |tx| {
                    tx.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            assert_eq!(answers.len(), 1);
            let AnswerValue::Event(original) = answers.get(0, 5) else {
                panic!("event")
            };
            assert_eq!(original.to_bytes(&()).unwrap(), bit.to_bytes(&()).unwrap());
            retained.push(answers);
            prepared.release_memory();
        }
    }
    drop((query, plan, db));
    let event = |value| {
        let AnswerValue::Event(value) = value else {
            panic!("event")
        };
        value
    };
    for answers in &retained {
        let yes = event(answers.get(0, 1));
        let no = event(answers.get(0, 2));
        let unknown = event(answers.get(0, 3));
        assert!(
            yes.apply(BoolOp4::AND, no, &()).unwrap().is_empty()
                && yes.apply(BoolOp4::AND, unknown, &()).unwrap().is_empty()
                && no.apply(BoolOp4::AND, unknown, &()).unwrap().is_empty()
        );
        assert!(event(answers.get(0, 8)).is_full());
        for n in 0..=6 {
            for outcomes in 0..2 {
                let world = ParameterWorld {
                    parameter: RealWitness::Rational(q(n, 6)),
                    outcomes,
                };
                let expected = [
                    n > 2,
                    n > 0 && n <= 2,
                    n == 0,
                    outcomes == 1,
                    outcomes == 1,
                    n > 2 && outcomes == 1,
                    n == 0 && outcomes == 1,
                    true,
                ];
                for (i, expect) in expected.into_iter().enumerate() {
                    assert_eq!(
                        event(answers.get(0, i + 1))
                            .contains_parameter(
                                &world,
                                ParameterSourceLimits::default(),
                                &mut work()
                            )
                            .unwrap(),
                        expect
                    );
                }
            }
        }
        let missing = event(answers.get(0, 7));
        assert!(!missing.is_empty());
        assert!(
            missing
                .parameter_mass(ParameterSourceLimits::default(), &mut work())
                .unwrap()
                .where_sign(
                    Signs::NON_ZERO,
                    ParameterSourceLimits::default().parameters.region,
                    ParameterSourceLimits::default().functions,
                    &mut work()
                )
                .unwrap()
                .is_empty()
        );
    }
    let db = Db::open(path.path(), PredicateGuards, common::work()).unwrap();
    let answers = &retained[0];
    db.write(common::work(), |tx| {
        tx.insert([&Decision { game: 1 }])?;
        for case in 1..=3 {
            tx.insert([&Truth {
                game: 1,
                case,
                when: event(answers.get(0, usize::try_from(case).unwrap())).clone(),
            }])?;
        }
        tx.insert([&Claim {
            game: 1,
            when: event(answers.get(0, 4)).clone(),
        }])
    })
    .unwrap()
    .unwrap();
    drop((db, retained));
    let db = Db::open(path.path(), PredicateGuards, common::work()).unwrap();
    let query = query!(PredicateGuards { (case,when: Event(region & claim)) | Truth(game,case,when:region),Claim(game:game,when:claim); });
    let mut prepared = db.prepare(&query, common::work()).unwrap();
    assert_eq!(
        db.read(common::work(), |tx| tx
            .execute_collect(&mut prepared, &[] as &[BindValue]))
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn guard_query_refusals_and_context_faults_survive_later_filtering() {
    use bumbledb::PredicateGuardPlan;
    let path = common::TempDir::new("query-guard-refusals");
    let db = Db::create(path.path(), PredicateGuards, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    let bit = source.coordinate(0, &()).unwrap();
    let other = Space::new(SpaceId([214; 32]), 1, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([
            &Trial {
                game: 1,
                claim: bit.clone(),
                given: source.full(),
            },
            &Trial {
                game: 2,
                claim: other.empty(),
                given: other.full(),
            },
        ])
    })
    .unwrap()
    .unwrap();
    let existing = PredicateGuardPlan::capture(
        &source,
        None,
        ObservationNumberCodecLimits::default(),
        &mut work(),
    )
    .unwrap();
    let refined = PredicateGuardPlan::capture(
        &source,
        Some(SpaceId([215; 32])),
        ObservationNumberCodecLimits::default(),
        &mut work(),
    )
    .unwrap();
    let unresolved = query!(PredicateGuards {
        use guard g = &existing;
        interior p(game,p: Probability(claim,given)) | Trial(game,claim,given),game==1;
        interior bad(game,when: Guard(Holds(Value(p)>1/2,g))) | p(game,p);
        (when) | bad(game,when),game==999;
    });
    let essential = query!(PredicateGuards {
        use guard g = &refined;
        interior p(game,p: Probability(claim,given)) | Trial(game,claim,given),game==1;
        interior truth(game,p: Predicate(Value(p)>1/2)) | p(game,p);
        interior region(game,p,when: Guard(Holds(p,g))) | truth(game,p);
        (when: Guard(Descend(p,g,when))) | region(game,p,when);
    });
    for (query, error) in [
        (
            unresolved.into_query(),
            bumbledb::event::Error::ParameterRefinementRequired,
        ),
        (
            essential.into_query(),
            bumbledb::event::Error::ParameterGuardEssential,
        ),
    ] {
        for cursor in [false, true] {
            let mut prepared = db.prepare(&query, common::work()).unwrap();
            prepared.force_cursor_fallback(cursor);
            let out = db.read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            });
            assert!(matches!(out,Err(bumbledb::Error::Event(e)) if e==error));
        }
    }
    let fault = query!(PredicateGuards {
        use guard g = &refined;
        interior bad(game,a: Guard(Lift(1>0,g,claim)),b: Guard(Lift(1<0,g,given))) | Trial(game,claim,given),game==?game;
        (a,b) | bad(game,a,b),game==999;
    });
    let mut baseline = None;
    for cursor in [false, true] {
        let mut prepared = db.prepare(&fault, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        for _ in 0..2 {
            let out = db.read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[BindValue::U64(2)])
            });
            let Err(bumbledb::Error::EventFaults(faults)) = out else {
                panic!("context faults")
            };
            assert_eq!(faults.len(), 2);
            assert!(faults.iter().all(|f| f.stage == Some(0) && f.operand == 0));
            if let Some(expected) = &baseline {
                assert_eq!(&faults, expected);
            } else {
                baseline = Some(faults);
            }
            assert!(
                db.read(common::work(), |tx| tx
                    .execute_collect(&mut prepared, &[BindValue::U64(1)]))
                    .unwrap()
                    .is_empty()
            );
            prepared.release_memory();
        }
    }
    assert!(
        PredicateGuardPlan::from_parts(
            &bit.to_bytes(&()).unwrap(),
            None,
            ObservationNumberCodecLimits::default(),
            &mut work()
        )
        .is_err()
    );
    assert!(
        PredicateGuardPlan::capture(
            &other,
            Some(SpaceId([216; 32])),
            ObservationNumberCodecLimits::default(),
            &mut work()
        )
        .is_err()
    );
}

#[test]
fn existing_guard_plans_accept_constants_and_resolved_regions_but_enforce_input_types() {
    use bumbledb::PredicateGuardPlan;
    let path = common::TempDir::new("existing-query-guards");
    let db = Db::create(path.path(), PredicateGuards, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    let bit = source.coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Trial {
            game: 1,
            claim: bit.clone(),
            given: source.full(),
        }])
    })
    .unwrap()
    .unwrap();
    let limits = ObservationNumberCodecLimits::default();
    let observe = query!(PredicateGuards {
        interior observed(p: Probability(claim,given)) | Trial(game,claim,given);
        (p: Predicate(Value(p)>0)) | observed(p);
    });
    let mut prepared = db.prepare(&observe, common::work()).unwrap();
    let answers = db
        .read(common::work(), |tx| {
            tx.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    let AnswerValue::Predicate(truth) = answers.get(0, 0) else {
        panic!("predicate")
    };
    let refined = truth
        .value()
        .refine(
            SpaceId([227; 32]),
            &source,
            Limits::default(),
            ParameterSourceLimits::default(),
            &mut work(),
        )
        .unwrap();
    let existing =
        PredicateGuardPlan::capture(refined.events().source(), None, limits, &mut work()).unwrap();
    let fixed = PredicateGuardPlan::capture(
        &Space::new(SpaceId([228; 32]), 1, &()).unwrap(),
        None,
        limits,
        &mut work(),
    )
    .unwrap();
    let query = query!(PredicateGuards {
        use guard g = &existing;
        use guard f = &fixed;
        interior observed(p: Probability(claim,given)) | Trial(game,claim,given);
        (yes: Guard(Holds(Value(p)>0,g)),constant: Guard(Holds(Sign(1,4),f)),hole: Guard(Undefined(Sign(1/0,7),f))) | observed(p);
    });
    let mut prepared = db.prepare(&query, common::work()).unwrap();
    let answers = db
        .read(common::work(), |tx| {
            tx.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert_eq!(answers.len(), 1);
    for column in 0..3 {
        let AnswerValue::Event(event) = answers.get(0, column) else {
            panic!("Event")
        };
        if column == 0 {
            assert_eq!(
                event.to_bytes(&()).unwrap(),
                refined.events().holds().to_bytes(&()).unwrap()
            );
        } else {
            assert!(event.is_full());
        }
    }
    let bad = query!(PredicateGuards {use guard g = &fixed; (bad: Guard(Holds(claim,g))) | Trial(game,claim,given);});
    assert!(db.prepare(&bad, common::work()).is_err());
    let bad = query!(PredicateGuards {use guard g = &fixed; (bad: Guard(Lift(Sign(1,4),g,game))) | Trial(game,claim,given);});
    assert!(db.prepare(&bad, common::work()).is_err());
}

#[test]
fn common_query_guards_compose_on_one_parameter_and_retain_holes_and_origins() {
    use bumbledb::PredicateGuardPlan;
    let path = common::TempDir::new("common-query-guards");
    let db = Db::create(path.path(), PredicateGuards, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    let claim = source.coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Trial {
            game: 1,
            claim: claim.clone(),
            given: source.full(),
        }])
    })
    .unwrap()
    .unwrap();
    let plan = PredicateGuardPlan::capture(
        &source,
        Some(SpaceId([232; 32])),
        ObservationNumberCodecLimits::default(),
        &mut work(),
    )
    .unwrap();
    let program = query!(PredicateGuards {
        use guard g = &plan;
        interior observed(game,claim,p: Probability(claim,given)) | Trial(game,claim,given);
        interior truth(game,claim,low: Predicate(Value(p)<1/2),high: Predicate(Value(p)>1/2),known: Predicate(Value(p)/Value(p)>0)) | observed(game,claim,p);
        interior cases(game,low,high,known,
            a: Guard(Holds(low,Common(g,high,known))),
            b: Guard(Holds(high,Common(g,known,low,low))),
            hole: Guard(Undefined(known,Common(g,low,high))),
            held: Guard(Lift(low,Common(g,high,known),claim))) | truth(game,claim,low,high,known);
        (game,a,b,hole,both: Event(a & b),middle: Event(!(a | b)),
            original: Guard(Descend(known,Common(g,high,low),held)),
            zero: Event(hole & held)) | cases(game,low,high,known,a,b,hole,held);
    });
    let mut retained = Vec::new();
    for fallback in [false, true] {
        let mut prepared = db.prepare(&program, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        retained.push(
            db.read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
        prepared.release_memory();
    }
    drop((db, program, plan, source));
    for answers in retained {
        assert_eq!(answers.len(), 1);
        for column in 1..8 {
            let AnswerValue::Event(event) = answers.get(0, column) else {
                panic!("event")
            };
            if column == 6 {
                assert_eq!(event.to_bytes(&()).unwrap(), claim.to_bytes(&()).unwrap());
                continue;
            }
            for n in 0..=4 {
                for outcomes in 0..2 {
                    let world = ParameterWorld {
                        parameter: RealWitness::Rational(q(n, 4)),
                        outcomes,
                    };
                    let expected = match column {
                        1 => n < 2,
                        2 => n > 2,
                        3 => n == 0,
                        4 => false,
                        5 => n == 2,
                        7 => n == 0 && outcomes == 1,
                        _ => unreachable!(),
                    };
                    assert_eq!(
                        event
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
            if column == 7 {
                assert!(!event.is_empty());
            }
        }
    }
}
