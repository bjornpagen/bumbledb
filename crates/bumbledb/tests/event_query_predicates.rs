#![allow(clippy::too_many_lines)]
use bumbledb::{
    AnswerValue, BindValue, Db, ObservationNumberCodecLimits, ObservationPredicateImport,
    event::{
        ArithmeticLimits, DensityPiece, ExactArithmetic, ExactRational, LawLimits,
        NumberPredicateView, Space, SpaceId,
    },
    query,
};
mod common;

bumbledb::schema! {
    pub Predicates;
    relation Trial { id: u64, value: i64, when: event, given: event }
    relation Selected { id: u64 }
    Trial(id) -> Trial;
    Selected(id) -> Selected;
}
fn arithmetic() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn q(n: i64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut arithmetic()).unwrap()
}
fn populate(db: &Db<Predicates>) {
    let raw = Space::new(SpaceId([217; 32]), 2, &()).unwrap();
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
    db.write(common::work(), |tx| {
        for (id, when) in [a.clone(), a, b].into_iter().enumerate() {
            tx.insert([&Trial {
                id: id as u64,
                value: 7,
                when,
                given: source.full(),
            }])?;
        }
        tx.insert([&Trial {
            id: 3,
            value: 0,
            when: source.full(),
            given: source.table(3, &[1], &()).unwrap(),
        }])?;
        tx.insert([&Selected { id: 0 }])
    })
    .unwrap()
    .unwrap();
}
fn predicate(value: AnswerValue<'_>) -> &ObservationPredicateImport {
    let AnswerValue::Predicate(value) = value else {
        panic!("predicate")
    };
    value
}
fn fixed(value: &ObservationPredicateImport) -> Option<bool> {
    let NumberPredicateView::Fixed(value) = value.value().predicate().view() else {
        panic!("fixed")
    };
    value
}

#[test]
fn completed_predicates_group_before_folding_and_keep_distinct_sources() {
    let dir = common::TempDir::new("query-predicates-groups");
    let db = Db::create(dir.path(), Predicates, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let direct = query!(Predicates {
        interior observed(id, value, p: Probability(when, given)) | Trial(id, value, when, given);
        (score: Predicate(Value(p) > 0), count: Count) | observed(id, value, p);
    });
    let staged = query!(Predicates {
        interior observed(id, value, p: Probability(when, given)) | Trial(id, value, when, given);
        interior scores(id, score: Predicate(Value(p) > 0)) | observed(id, value, p);
        (score, count: Count) | scores(id, score);
    });
    let mut retained = Vec::new();
    for template in [direct.into_query(), staged.into_query()] {
        for fallback in [false, true] {
            let mut prepared = db.prepare(&template, common::work()).unwrap();
            prepared.force_cursor_fallback(fallback);
            for _ in 0..2 {
                let out = db
                    .read(common::work(), |tx| {
                        tx.execute_collect(&mut prepared, &[] as &[BindValue])
                    })
                    .unwrap();
                assert_eq!(out.len(), 3);
                let mut counts = Vec::new();
                for row in 0..out.len() {
                    let value = predicate(out.get(row, 0));
                    let AnswerValue::U64(count) = out.get(row, 1) else {
                        panic!("count")
                    };
                    counts.push((fixed(value), count));
                }
                counts.sort();
                assert_eq!(counts, vec![(None, 1), (Some(true), 1), (Some(true), 2)]);
                retained.push(out);
            }
        }
    }
    drop(db);
    let mut reference = None;
    for out in retained {
        let mut rows = (0..out.len())
            .map(|row| {
                let value = predicate(out.get(row, 0));
                let replayed = ObservationPredicateImport::from_bytes(
                    value.bytes(),
                    ObservationNumberCodecLimits::default(),
                    &mut arithmetic(),
                )
                .unwrap();
                assert_eq!(*value, replayed);
                let AnswerValue::U64(count) = out.get(row, 1) else {
                    panic!("count")
                };
                (value.bytes().to_vec(), count)
            })
            .collect::<Vec<_>>();
        rows.sort();
        if let Some(reference) = &reference {
            assert_eq!(&rows, reference);
        } else {
            reference = Some(rows);
        }
    }
}

#[test]
fn predicate_identity_joins_and_antijoins_use_full_derivations() {
    let dir = common::TempDir::new("query-predicate-identity");
    let db = Db::create(dir.path(), Predicates, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let joined = query!(Predicates {
        interior p(id, p: Probability(when,given)) | Trial(id,when,given);
        interior n(id, n: Predicate(Value(p) > 0)) | p(id,p);
        (left,right,n) | n(left,n), n(right,other), n == other;
    });
    let anti = query!(Predicates {
        interior p(id, p: Probability(when,given)) | Trial(id,when,given);
        interior n(id, n: Predicate(Value(p) > 0)) | p(id,p);
        interior chosen(n) | n(id,n), Selected(id);
        (id,n) | n(id,n), !chosen(n);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&joined, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let out = db
            .read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(out.len(), 6);
        for row in 0..out.len() {
            let (AnswerValue::U64(a), AnswerValue::U64(b)) = (out.get(row, 0), out.get(row, 1))
            else {
                panic!("ids")
            };
            assert!(a == b || matches!((a, b), (0, 1) | (1, 0)));
        }
        let mut prepared = db.prepare(&anti, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let out = db
            .read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        let mut ids = (0..out.len())
            .map(|row| match out.get(row, 0) {
                AnswerValue::U64(id) => id,
                _ => panic!("id"),
            })
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, [2, 3]);
    }
}

#[test]
fn macro_comparisons_strict_boolean_algebra_imports_and_explicit_quantifiers() {
    let path = common::TempDir::new("query-predicate-operators");
    let db = Db::create(path.path(), Predicates, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let query = query!(Predicates {
        interior p(id, p: Probability(when,given)) | Trial(id,when,given);
        interior truth(id, v: Predicate(Value(p) > 0)) | p(id,p);
        (id, opposite: Predicate(!v), constant: Predicate(Bool4(15,v,!v)),
            anywhere: PredicateTest(Possibly(v)), everywhere: PredicateTest(Always(v)),
            total: PredicateTest(IsTotal(v))) | truth(id,v);
    });
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answers = db
            .read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 4);
        for row in 0..answers.len() {
            let AnswerValue::U64(id) = answers.get(row, 0) else {
                panic!("id")
            };
            assert_eq!(
                fixed(predicate(answers.get(row, 1))),
                if id == 3 { None } else { Some(false) }
            );
            assert_eq!(
                fixed(predicate(answers.get(row, 2))),
                if id == 3 { None } else { Some(true) }
            );
            for column in 3..6 {
                assert_eq!(answers.get(row, column), AnswerValue::Bool(id != 3));
            }
        }
    }
    let operators = query!(Predicates {
        (a: Predicate(2 > 1), b: Predicate(2 >= 2), c: Predicate(1 < 2), d: Predicate(2 <= 2),
         e: Predicate(2 == 2), f: Predicate(2 != 1),
         g: Predicate(((2 + 1) / 3 >= 1) & !(0 != 0) | (1/0 > 0)),
         h: Predicate(Sign(-1, 0b001)), i: Predicate((1==1) ^ (2==2))) | Selected(id);
    });
    let mut prepared = db.prepare(&operators, common::work()).unwrap();
    let answers = db
        .read(common::work(), |tx| {
            tx.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    for column in [0, 1, 2, 3, 4, 5, 7] {
        assert_eq!(fixed(predicate(answers.get(0, column))), Some(true));
    }
    assert_eq!(
        fixed(predicate(answers.get(0, 6))),
        None,
        "strict OR keeps the right-hand hole"
    );
    assert_eq!(fixed(predicate(answers.get(0, 8))), Some(false));
    let imported = predicate(answers.get(0, 6)).clone();
    let domain = bumbledb::NumberDomain::capture(
        bumbledb::event::ParameterDomain::new(bumbledb::event::ParameterRegion::full(
            bumbledb::event::ParameterId([188; 32]),
        ))
        .unwrap(),
        bumbledb::event::ParameterCodecLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let query = query!(Predicates {
        use predicate p = &imported;
        use number_domain d = &domain;
        (p: Predicate(OnDomain(Imported(p),d)), total: PredicateTest(IsTotal(Imported(p)))) | Selected(id);
    });
    let mut prepared = db.prepare(&query, common::work()).unwrap();
    let answers = db
        .read(common::work(), |tx| {
            tx.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert_eq!(answers.get(0, 1), AnswerValue::Bool(false));
    let NumberPredicateView::Parameter {
        holds,
        fails,
        undefined,
        ambient,
    } = predicate(answers.get(0, 0)).value().predicate().view()
    else {
        panic!("parameter")
    };
    assert!(holds.is_empty() && fails.is_empty());
    assert!(
        undefined
            .equivalent(
                ambient.region(),
                bumbledb::event::ParameterLimits::default(),
                &mut arithmetic()
            )
            .unwrap()
    );
}

#[test]
fn imported_stages_recursive_forwarding_and_predicate_type_walls() {
    let path = common::TempDir::new("query-predicate-stages");
    let db = Db::create(path.path(), Predicates, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let stage = query!(Predicates {
        interior p(id,p: Probability(when,given)) | Trial(id,when,given);
        (id,verdict: Predicate(Value(p)>0)) | p(id,p);
    });
    let forwarded = query!(Predicates {
        use observed = &stage;
        interior staged(id,verdict) | observed(id,verdict);
        rec propagated(id,verdict) | staged(id,verdict);
        rec propagated(id,verdict) | propagated(id,verdict), Selected(id);
        (id,verdict) | propagated(id,verdict);
    });
    for cursor in [false, true] {
        let mut prepared = db.prepare(&forwarded, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answers = db
            .read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 4);
    }
    let cases = [
        query!(Predicates { use observed=&stage; (n: Number(verdict)) | observed(id,verdict); })
            .into_query(),
        query!(Predicates { use observed=&stage; (p: Predicate(id)) | observed(id,verdict); })
            .into_query(),
    ];
    for query in cases {
        assert!(db.prepare(&query, common::work()).is_err());
    }
}

fn domain(id: u8) -> bumbledb::NumberDomain {
    bumbledb::NumberDomain::capture(
        bumbledb::event::ParameterDomain::new(bumbledb::event::ParameterRegion::full(
            bumbledb::event::ParameterId([id; 32]),
        ))
        .unwrap(),
        bumbledb::event::ParameterCodecLimits::default(),
        &mut arithmetic(),
    )
    .unwrap()
}

#[test]
fn predicate_producer_refusal_survives_filter_and_prepared_recovery() {
    let dir = common::TempDir::new("query-predicate-refusal");
    let db = Db::create(dir.path(), Predicates, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let a = domain(219);
    let b = domain(220);
    // Even a constant truth table must validate both parameter domains.
    let template = query!(Predicates {
        use number_domain a = &a;
        use number_domain b = &b;
        interior bad(id, n: Predicate(Bool4(15, OnDomain(0 == 0, a), OnDomain(1 == 1, b)))) | Trial(id), id == ?selected;
        (n) | bad(id,n), id == 999;
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        for _ in 0..2 {
            let result = db.read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[BindValue::U64(0)])
            });
            assert!(
                matches!(
                    &result,
                    Err(bumbledb::Error::Event(
                        bumbledb::event::Error::ParameterScopeMismatch
                    ))
                ),
                "{:?}",
                result.err()
            );
            let empty = db
                .read(common::work(), |tx| {
                    tx.execute_collect(&mut prepared, &[BindValue::U64(999)])
                })
                .unwrap();
            assert!(empty.is_empty());
            prepared.release_memory();
        }
    }
}

#[test]
fn mixed_producers_and_rebinding_keep_owned_predicate_answers() {
    let dir = common::TempDir::new("query-predicate-mixed");
    let db = Db::create(dir.path(), Predicates, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let template = query!(Predicates {
        interior p(id,value,when,given,p: Probability(when,given)) | Trial(id,value,when,given), id == ?selected;
        interior mixed(id,n: Number(Value(p)), verdict: Predicate(Value(p)>0), fresh: Probability(when,given), mean: Expectation(value,given,given)) | p(id,value,when,given,p);
        (id,verdict,score: Predicate((n * Value(mean) > 0) & verdict),n,fresh,mean) | mixed(id,n,verdict,fresh,mean);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let mut retained = Vec::new();
        for id in [0, 2, 3, 0] {
            let out = db
                .read(common::work(), |tx| {
                    tx.execute_collect(&mut prepared, &[BindValue::U64(id)])
                })
                .unwrap();
            assert_eq!(out.len(), 1);
            assert_eq!(
                fixed(predicate(out.get(0, 2))),
                if id == 3 { None } else { Some(true) }
            );
            retained.push(out);
            prepared.release_memory();
        }
        drop(prepared);
        assert_eq!(retained[0].get(0, 1), retained[3].get(0, 1));
        assert_ne!(retained[0].get(0, 1), retained[1].get(0, 1));
        for out in retained {
            for column in [1, 2] {
                let value = predicate(out.get(0, column));
                assert_eq!(
                    *value,
                    ObservationPredicateImport::from_bytes(
                        value.bytes(),
                        ObservationNumberCodecLimits::default(),
                        &mut arithmetic()
                    )
                    .unwrap()
                );
            }
        }
    }
}

#[test]
fn predicate_types_refuse_implicit_scalar_observation_event_and_fold_coercions() {
    let dir = common::TempDir::new("query-predicate-types");
    let db = Db::create(dir.path(), Predicates, common::work())
        .unwrap()
        .unwrap();
    let templates = [
        query!(Predicates { interior p(p: Predicate(1 > 0)) | Trial(id); (n: Number(p)) | p(p); }).into_query(),
        query!(Predicates { interior p(p: Predicate(1 > 0)) | Trial(id); (n: Number(Value(p))) | p(p); }).into_query(),
        query!(Predicates { interior p(p: Predicate(1 > 0)) | Trial(id); (n: Event(!p)) | p(p); }).into_query(),
        query!(Predicates { interior p(p: Predicate(1 > 0)) | Trial(id); (p) | p(p), Selected(id:p); }).into_query(),
        query!(Predicates { interior p(p: Predicate(1 > 0)) | Trial(id); (p) | p(p), p > p; }).into_query(),
        query!(Predicates { interior p(p: Predicate(1 > 0)) | Trial(id); (n: Sum(p)) | p(p); }).into_query(),
        query!(Predicates { interior p(p: Predicate(1 > 0)) | Trial(id); (p) | p(p), p == ?value; }).into_query(),
        query!(Predicates { interior p(p: Probability(when,given)) | Trial(when,given); (n: Predicate(p)) | p(p); }).into_query(),
    ];
    for template in templates {
        assert!(matches!(
            db.prepare(&template, common::work()),
            Err(bumbledb::Error::Validation(_))
        ));
    }
}
