#![allow(clippy::too_many_lines)]
use bumbledb::{
    AnswerValue, BindValue, Db, ObservationNumber, ObservationNumberCodecLimits,
    ObservationNumberImport, ObservationNumberLimits,
    event::{
        ArithmeticLimits, DensityPiece, ExactArithmetic, ExactRational, LawLimits, ParameterDomain,
        ParameterId, ParameterRegion, PartialNumber, Space, SpaceId,
    },
    query,
};
mod common;

bumbledb::schema! {
    pub Numbers;
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
fn populate(db: &Db<Numbers>) {
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
fn number(value: AnswerValue<'_>) -> &ObservationNumberImport {
    let AnswerValue::Number(value) = value else {
        panic!("number")
    };
    value
}
fn fixed(value: &ObservationNumberImport) -> Option<String> {
    let PartialNumber::Fixed(value) = value.value().value() else {
        panic!("fixed")
    };
    value.as_ref().map(ToString::to_string)
}

#[test]
fn completed_numbers_group_before_folding_and_keep_distinct_sources() {
    let dir = common::TempDir::new("query-numbers-groups");
    let db = Db::create(dir.path(), Numbers, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let direct = query!(Numbers {
        interior observed(id, value, p: Probability(when, given)) | Trial(id, value, when, given);
        (score: Number(Value(p) * Integer(value)), count: Count) | observed(id, value, p);
    });
    let staged = query!(Numbers {
        interior observed(id, value, p: Probability(when, given)) | Trial(id, value, when, given);
        interior scores(id, score: Number(Value(p) * Integer(value))) | observed(id, value, p);
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
                    let value = number(out.get(row, 0));
                    let AnswerValue::U64(count) = out.get(row, 1) else {
                        panic!("count")
                    };
                    counts.push((fixed(value), count));
                }
                counts.sort();
                assert_eq!(
                    counts,
                    vec![(None, 1), (Some("7/2".into()), 1), (Some("7/2".into()), 2)]
                );
                retained.push(out);
            }
        }
    }
    drop(db);
    let mut reference = None;
    for out in retained {
        let mut rows = (0..out.len())
            .map(|row| {
                let value = number(out.get(row, 0));
                let replayed = ObservationNumberImport::from_bytes(
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
fn numerical_identity_joins_and_antijoins_use_full_derivations() {
    let dir = common::TempDir::new("query-number-identity");
    let db = Db::create(dir.path(), Numbers, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let joined = query!(Numbers {
        interior p(id, p: Probability(when,given)) | Trial(id,when,given);
        interior n(id, n: Number(Value(p))) | p(id,p);
        (left,right,n) | n(left,n), n(right,other), n == other;
    });
    let anti = query!(Numbers {
        interior p(id, p: Probability(when,given)) | Trial(id,when,given);
        interior n(id, n: Number(Value(p))) | p(id,p);
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
fn exact_macro_operators_imports_domains_and_repeated_main_execution() {
    let dir = common::TempDir::new("query-number-macro");
    let db = Db::create(dir.path(), Numbers, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let literal = query!(Numbers { (a: Number((1 + 2 * 3) / 4 - 2), b: Number(Min(3, -4i64)), c: Number(Max(-2, 1u64)), d: Number(Abs(-7)), e: Number(Pow(2,0x3u64)), f: Number(-9223372036854775808i64), g: Number(1/0)) | Trial(id == 0); });
    let count = query!(Numbers { (n: Number(Integer(value)), count: Count) | Trial(id,value); });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&literal, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        for _ in 0..2 {
            let out = db
                .read(common::work(), |tx| {
                    tx.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            assert_eq!(out.len(), 1);
            let values = (0..out.arity())
                .map(|i| fixed(number(out.get(0, i))))
                .collect::<Vec<_>>();
            assert_eq!(
                values,
                [
                    Some("-1/4".into()),
                    Some("-4".into()),
                    Some("1".into()),
                    Some("7".into()),
                    Some("8".into()),
                    Some(i64::MIN.to_string()),
                    None
                ]
            );
        }
        let mut prepared = db.prepare(&count, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let out = db
            .read(common::work(), |tx| {
                tx.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(out.len(), 2);
        for row in 0..out.len() {
            assert_eq!(
                out.get(row, 1),
                AnswerValue::U64(if fixed(number(out.get(row, 0))) == Some("7".into()) {
                    3
                } else {
                    1
                })
            );
        }
    }
    let literal_value = ObservationNumber::literal(
        q(1, 3),
        ObservationNumberLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let import = ObservationNumberImport::capture(
        &literal_value,
        ObservationNumberCodecLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let domain = ParameterDomain::new(ParameterRegion::full(ParameterId([218; 32]))).unwrap();
    let domain = bumbledb::NumberDomain::capture(
        domain,
        bumbledb::event::ParameterCodecLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let imported = query!(Numbers {
        use number third = &import;
        use number_domain theta = &domain;
        (n: Number(OnDomain(Imported(third) + 1, theta))) | Trial(id == 0);
    });
    let mut prepared = db.prepare(&imported, common::work()).unwrap();
    let out = db
        .read(common::work(), |tx| {
            tx.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    let PartialNumber::Parameter(function) = number(out.get(0, 0)).value().value() else {
        panic!("parameter")
    };
    assert_eq!(
        function
            .value_at(
                &q(10, 1),
                bumbledb::event::ParameterLimits::default(),
                bumbledb::event::FunctionLimits::default(),
                &mut arithmetic()
            )
            .unwrap(),
        Some(q(4, 3))
    );
}

fn domain(id: u8) -> bumbledb::NumberDomain {
    bumbledb::NumberDomain::capture(
        ParameterDomain::new(ParameterRegion::full(ParameterId([id; 32]))).unwrap(),
        bumbledb::event::ParameterCodecLimits::default(),
        &mut arithmetic(),
    )
    .unwrap()
}

#[test]
fn numerical_producer_refusal_survives_filter_and_prepared_recovery() {
    let dir = common::TempDir::new("query-number-refusal");
    let db = Db::create(dir.path(), Numbers, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let a = domain(219);
    let b = domain(220);
    // Multiplication by zero cannot erase an incompatible parameter domain.
    let template = query!(Numbers {
        use number_domain a = &a;
        use number_domain b = &b;
        interior bad(id, n: Number(OnDomain(0, a) * OnDomain(1, b))) | Trial(id), id == ?selected;
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
fn number_types_refuse_implicit_scalar_observation_and_event_coercions() {
    let dir = common::TempDir::new("query-number-types");
    let db = Db::create(dir.path(), Numbers, common::work())
        .unwrap()
        .unwrap();
    let templates = [
        query!(Numbers { (n: Number(value)) | Trial(value); }).into_query(),
        query!(Numbers { (n: Number(Integer(when))) | Trial(when); }).into_query(),
        query!(Numbers { (n: Number(Value(value))) | Trial(value); }).into_query(),
        query!(Numbers { interior p(p: Probability(when,given)) | Trial(when,given); (n: Number(p)) | p(p); }).into_query(),
        query!(Numbers { interior p(p: Number(1)) | Trial(id); (n: Number(Value(p))) | p(p); }).into_query(),
        query!(Numbers { interior p(p: Number(1)) | Trial(id); (p) | p(p), Selected(id:p); }).into_query(),
        query!(Numbers { interior p(p: Number(1)) | Trial(id); (p) | p(p), p > p; }).into_query(),
        query!(Numbers { interior p(p: Number(1)) | Trial(id); (n: Sum(p)) | p(p); }).into_query(),
        query!(Numbers { interior p(p: Number(1)) | Trial(id); (p) | p(p), p == ?value; }).into_query(),
    ];
    for template in templates {
        assert!(matches!(
            db.prepare(&template, common::work()),
            Err(bumbledb::Error::Validation(_))
        ));
    }
}

#[test]
fn imported_number_stages_projection_unions_and_recursive_forwarding() {
    let dir = common::TempDir::new("query-number-composition");
    let db = Db::create(dir.path(), Numbers, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let base = query!(Numbers {
        interior p(id,p: Probability(when,given)) | Trial(id,when,given);
        (id,n: Number(Value(p))) | p(id,p);
    });
    let union = query!(Numbers {
        use base = &base;
        interior both(id,n) | base(id,n);
        interior both(id,n) | base(id,n), Selected(id);
        (id,n) | both(id,n);
    });
    let recursive = query!(Numbers {
        use base = &base;
        rec reach(id,n) | base(id,n);
        rec reach(id,n) | reach(id,n), Selected(id);
        (id,n) | reach(id,n);
    });
    for template in [union.into_query(), recursive.into_query()] {
        for fallback in [false, true] {
            let mut prepared = db.prepare(&template, common::work()).unwrap();
            prepared.force_cursor_fallback(fallback);
            let out = db
                .read(common::work(), |tx| {
                    tx.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            assert_eq!(out.len(), 4);
            for row in 0..out.len() {
                let AnswerValue::U64(id) = out.get(row, 0) else {
                    panic!("id")
                };
                assert_eq!(
                    fixed(number(out.get(row, 1))),
                    if id == 3 { None } else { Some("1/2".into()) }
                );
            }
        }
    }
}

#[test]
fn mixed_producers_and_rebinding_keep_owned_numerical_answers() {
    let dir = common::TempDir::new("query-number-mixed");
    let db = Db::create(dir.path(), Numbers, common::work())
        .unwrap()
        .unwrap();
    populate(&db);
    let template = query!(Numbers {
        interior p(id,value,when,given,p: Probability(when,given)) | Trial(id,value,when,given), id == ?selected;
        interior mixed(id,n: Number(Value(p)), fresh: Probability(when,given), mean: Expectation(value,given,given)) | p(id,value,when,given,p);
        (id,n,score: Number(n * Value(mean)),fresh,mean) | mixed(id,n,fresh,mean);
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
                fixed(number(out.get(0, 2))),
                if id == 3 { None } else { Some("7/2".into()) }
            );
            retained.push(out);
            prepared.release_memory();
        }
        drop(prepared);
        assert_eq!(retained[0].get(0, 1), retained[3].get(0, 1));
        assert_ne!(retained[0].get(0, 1), retained[1].get(0, 1));
        for out in retained {
            for column in [1, 2] {
                let value = number(out.get(0, column));
                assert_eq!(
                    *value,
                    ObservationNumberImport::from_bytes(
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
