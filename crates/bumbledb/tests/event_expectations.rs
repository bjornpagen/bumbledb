use bumbledb::{
    AnswerValue, BindValue, Db, Error, Event, ExpectationValue,
    event::{
        ArithmeticLimits, DensityPiece, ExactArithmetic, ExactRational, LawLimits, Space, SpaceId,
    },
    query,
};

mod common;

bumbledb::schema! {
    pub Payoffs;
    closed relation Utility as UtilityId { value: i64 } = { Low { value: -3 }, High { value: 7 } };
    relation CategoricalPayoff { utility: u64 as UtilityId, region: event, evidence: event }
    relation Payoff { id: u64, group: u64, value: i64, second: u64, region: event, evidence: event }
    relation FloatingPayoff { value: f64, region: event }
    relation UnsignedPayoff { id: u64, group: u64, value: u64, region: event, evidence: event }
    CategoricalPayoff(utility) <= Utility(id);
    Payoff(id) -> Payoff;
    UnsignedPayoff(id) -> UnsignedPayoff;
}

fn arithmetic() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn ratio(n: i64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut arithmetic()).unwrap()
}
fn source(zero_half: bool) -> Space {
    let raw = Space::new(SpaceId([204; 32]), 1, &()).unwrap();
    let region = if zero_half {
        raw.coordinate(0, &()).unwrap()
    } else {
        raw.full()
    };
    raw.with_density(
        &[DensityPiece {
            region,
            density: if zero_half { ratio(1, 1) } else { ratio(1, 2) },
        }],
        LawLimits::default(),
        &mut arithmetic(),
    )
    .unwrap()
}
fn row(id: u64, group: u64, value: i64, second: u64, region: Event, evidence: Event) -> Payoff {
    Payoff {
        id,
        group,
        value,
        second,
        region,
        evidence,
    }
}
fn fixed(value: AnswerValue<'_>) -> Option<ExactRational> {
    let AnswerValue::Expectation(answer) = value else {
        panic!("expectation");
    };
    let ExpectationValue::Fixed { value, .. } = answer.value() else {
        panic!("fixed");
    };
    value.clone()
}

#[test]
fn expectation_groups_union_duplicates_and_compose_with_scalar_aggregates() {
    let dir = common::TempDir::new("expectation-composition");
    let db = Db::create(dir.path(), Payoffs, common::work())
        .unwrap()
        .unwrap();
    let source = source(false);
    let x = source.coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([
            &row(0, 1, -4, 2, x.clone(), source.full()),
            &row(1, 1, -4, 2, x.clone(), source.full()),
            &row(2, 1, 10, 8, x.complement(), source.full()),
            &row(3, 2, 0, u64::MAX, source.full(), source.full()),
        ])
    })
    .unwrap()
    .unwrap();
    drop((source, db));
    let db = Db::open(dir.path(), Payoffs, common::work()).unwrap();
    let template = query!(Payoffs {
        (group, expected: Expectation(value, region, evidence), second_mean: Expectation(second, region, evidence), paths: Count, total: Sum(value)) |
            Payoff(id: path, group, value, second, region, evidence);
    });
    let mut retained = Vec::new();
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop(db);
    for answers in retained {
        assert_eq!(answers.len(), 2);
        for n in 0..answers.len() {
            match answers.get(n, 0) {
                AnswerValue::U64(1) => {
                    assert_eq!(fixed(answers.get(n, 1)), Some(ratio(3, 1)));
                    assert_eq!(fixed(answers.get(n, 2)), Some(ratio(5, 1)));
                    assert_eq!(answers.get(n, 3), AnswerValue::U64(3));
                    assert_eq!(answers.get(n, 4), AnswerValue::I64(2));
                }
                AnswerValue::U64(2) => {
                    assert_eq!(fixed(answers.get(n, 1)), Some(ratio(0, 1)));
                    assert_eq!(
                        fixed(answers.get(n, 2)),
                        Some(ExactRational::from(u64::MAX))
                    );
                    assert_eq!(answers.get(n, 3), AnswerValue::U64(1));
                }
                _ => panic!("group"),
            }
            let AnswerValue::Expectation(answer) = answers.get(n, 1) else {
                unreachable!()
            };
            assert!(answer.partition().parent().is_full());
        }
    }
}

#[test]
fn expectation_clips_before_overlap_checks_and_retains_zero_mass_evidence() {
    let dir = common::TempDir::new("expectation-evidence");
    let db = Db::create(dir.path(), Payoffs, common::work())
        .unwrap()
        .unwrap();
    let source = source(true);
    let x = source.coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([
            // Distinct values overlap only outside the evidence.
            &row(0, 1, 7, 0, source.full(), x.clone()),
            &row(1, 1, -5, 0, x.complement(), x.clone()),
            &row(2, 2, i64::MIN, 0, x.complement(), x.complement()),
            &row(3, 3, 0, 0, source.empty(), source.empty()),
        ])
    })
    .unwrap()
    .unwrap();
    let template = query!(Payoffs {
        (group, expected: Expectation(value, region, evidence)) | Payoff(group, value, region, evidence);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let answers = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert_eq!(answers.len(), 3);
    for n in 0..answers.len() {
        let AnswerValue::Expectation(answer) = answers.get(n, 1) else {
            unreachable!()
        };
        if answers.get(n, 0) == AnswerValue::U64(1) {
            assert_eq!(fixed(answers.get(n, 1)), Some(ratio(7, 1)));
            assert_eq!(answer.partition().cells().len(), 2);
        } else {
            assert!(answer.is_impossible());
            assert_eq!(fixed(answers.get(n, 1)), None);
            if answers.get(n, 0) == AnswerValue::U64(2) {
                assert!(!answer.given().is_empty());
                assert_eq!(answer.values(), &[ExactRational::from(i64::MIN)]);
            }
        }
    }
}

#[test]
fn expectation_refuses_gaps_overlap_changing_evidence_and_missing_laws_atomically() {
    let source = source(true);
    let x = source.coordinate(0, &()).unwrap();
    let raw = Space::new(SpaceId([205; 32]), 1, &()).unwrap();
    for (case, rows) in [
        vec![row(0, 0, 1, 0, x.clone(), source.full())], // gap has zero mass
        vec![
            row(0, 0, 1, 0, source.full(), source.full()),
            row(1, 0, 2, 0, x.clone(), source.full()),
        ],
        vec![
            row(0, 0, 1, 0, source.full(), source.full()),
            row(1, 0, 1, 0, source.empty(), x.clone()),
        ],
        vec![row(0, 0, 0, 0, raw.full(), raw.full())],
    ]
    .into_iter()
    .enumerate()
    {
        let dir = common::TempDir::new(&format!("expectation-refusal-{case}"));
        let db = Db::create(dir.path(), Payoffs, common::work())
            .unwrap()
            .unwrap();
        db.write(common::work(), |tx| tx.insert(rows.iter()))
            .unwrap()
            .unwrap();
        let template = query!(Payoffs { (expected: Expectation(value, region, evidence)) | Payoff(value, region, evidence); });
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        for fallback in [false, true] {
            prepared.force_cursor_fallback(fallback);
            let error = db
                .read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap_err();
            match (case, error) {
                (0, Error::Event(bumbledb::event::Error::PartitionGap))
                | (1, Error::Event(bumbledb::event::Error::PartitionOverlap))
                | (2, Error::ExpectationEvidenceMismatch { .. })
                | (3, Error::Event(bumbledb::event::Error::MissingLaw)) => {}
                (_, error) => panic!("wrong refusal: {error:?}"),
            }
        }
    }
}

#[test]
fn expectation_union_arms_align_positive_integer_types_and_pack_outputs() {
    let dir = common::TempDir::new("expectation-union");
    let db = Db::create(dir.path(), Payoffs, common::work())
        .unwrap()
        .unwrap();
    let source = source(false);
    let x = source.coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&row(0, 1, 4, 0, x.clone(), source.full())])?;
        tx.insert([&UnsignedPayoff {
            id: 0,
            group: 1,
            value: 4,
            region: x.complement(),
            evidence: source.full(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(Payoffs {
        (group, expected: Expectation(value, region, evidence), covered: Pack(region)) | Payoff(group, value, region, evidence);
        (group, expected: Expectation(value, region, evidence), covered: Pack(region)) | UnsignedPayoff(group, value, region, evidence);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 1);
        assert_eq!(fixed(answers.get(0, 1)), Some(ratio(4, 1)));
        let AnswerValue::Event(covered) = answers.get(0, 2) else {
            panic!("packed");
        };
        assert!(covered.is_full());
        let AnswerValue::Expectation(answer) = answers.get(0, 1) else {
            unreachable!()
        };
        assert_eq!(answer.values().len(), 1);
    }
}

#[test]
fn expectation_empty_input_has_no_group() {
    let dir = common::TempDir::new("expectation-no-group");
    let db = Db::create(dir.path(), Payoffs, common::work())
        .unwrap()
        .unwrap();
    let template = query!(Payoffs { (expected: Expectation(value, region, evidence), paths: Count) | Payoff(value, region, evidence); });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let answers = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert!(answers.is_empty());
}

#[test]
fn expectation_matches_independent_four_world_oracle_for_all_regions_and_evidence() {
    let dir = common::TempDir::new("expectation-world-oracle");
    let db = Db::create(dir.path(), Payoffs, common::work())
        .unwrap()
        .unwrap();
    let raw = Space::new(SpaceId([206; 32]), 2, &()).unwrap();
    let source = raw
        .with_density(
            &[3, 2, 1]
                .into_iter()
                .enumerate()
                .map(|(world, weight)| DensityPiece {
                    region: raw.table(3, &[1 << world], &()).unwrap(),
                    density: ratio(weight, 6),
                })
                .collect::<Vec<_>>(),
            LawLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    db.write(common::work(), |tx| {
        for evidence in 0..16 {
            for region in 0..16 {
                let group = evidence * 16 + region;
                let given = source.table(3, &[evidence], &())?;
                let when = source.table(3, &[region], &())?;
                tx.insert([
                    &row(group * 3, group, -3, 0, when.clone(), given.clone()),
                    &row(group * 3 + 1, group, -3, 0, when.clone(), given.clone()),
                    &row(group * 3 + 2, group, 7, 0, when.complement(), given),
                ])?;
            }
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(Payoffs { (group, expected: Expectation(value, region, evidence)) | Payoff(group, value, region, evidence); });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 256);
        for n in 0..answers.len() {
            let AnswerValue::U64(group) = answers.get(n, 0) else {
                panic!("group");
            };
            let (evidence, region) = (group / 16, group % 16);
            let mut numerator = 0;
            let mut mass = 0;
            for (world, weight) in [3, 2, 1, 0].into_iter().enumerate() {
                if evidence & (1 << world) != 0 {
                    mass += weight;
                    numerator += weight * if region & (1 << world) != 0 { -3 } else { 7 };
                }
            }
            assert_eq!(
                fixed(answers.get(n, 1)),
                if mass == 0 {
                    None
                } else {
                    Some(ratio(numerator, u64::try_from(mass).unwrap()))
                }
            );
        }
    }
}

#[test]
fn expectation_checks_types_on_empty_inputs_and_refuses_observation_interiors() {
    let dir = common::TempDir::new("expectation-static-validation");
    let db = Db::create(dir.path(), Payoffs, common::work())
        .unwrap()
        .unwrap();
    let float = query!(Payoffs { (expected: Expectation(value, region, region)) | FloatingPayoff(value, region); });
    assert!(matches!(
        db.prepare(&float, common::work()),
        Err(Error::Validation(
            bumbledb::ValidationError::AggregateInputType { .. }
        ))
    ));
    let wrong_event =
        query!(Payoffs { (expected: Expectation(value, region, value)) | Payoff(value, region); });
    assert!(matches!(
        db.prepare(&wrong_event, common::work()),
        Err(Error::Validation(
            bumbledb::ValidationError::EventExpression { .. }
        ))
    ));
    let staged = query!(Payoffs {
        interior observed(expected: Expectation(value, region, evidence)) | Payoff(value, region, evidence);
        (expected) | observed(expected);
    });
    assert!(matches!(
        db.prepare(&staged, common::work()),
        Err(Error::Validation(
            bumbledb::ValidationError::ObservationInterior { .. }
        ))
    ));
}

#[test]
fn expectation_requires_explicit_utility_mapping_for_closed_references() {
    let dir = common::TempDir::new("expectation-utility-mapping");
    let db = Db::create(dir.path(), Payoffs, common::work())
        .unwrap()
        .unwrap();
    let source = source(false);
    db.write(common::work(), |tx| {
        tx.insert([&CategoricalPayoff {
            utility: Utility::High.id(),
            region: source.full(),
            evidence: source.full(),
        }])
    })
    .unwrap()
    .unwrap();
    let implicit = query!(Payoffs { (expected: Expectation(utility, region, evidence)) | CategoricalPayoff(utility, region, evidence); });
    assert!(matches!(
        db.prepare(&implicit, common::work()),
        Err(Error::Validation(
            bumbledb::ValidationError::AggregateOverClosedReference { .. }
        ))
    ));
    let mapped = query!(Payoffs { (expected: Expectation(value, region, evidence)) | CategoricalPayoff(utility, region, evidence), Utility(id: utility, value); });
    let mut prepared = db.prepare(&mapped, common::work()).unwrap();
    let answers = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert_eq!(fixed(answers.get(0, 0)), Some(ratio(7, 1)));
}
