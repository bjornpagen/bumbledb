//! Observations remain source-owned values across relational stage boundaries.
#![allow(clippy::too_many_lines)]
use bumbledb::{
    AnswerValue, BindValue, Db, Error, ExpectationValue, ProbabilityValue,
    event::{
        ArithmeticLimits, DensityPiece, ExactArithmetic, ExactRational, LawLimits, Space, SpaceId,
    },
    query,
};
mod common;

bumbledb::schema! {
    pub ObservationStages;
    relation Trial { id: u64, label: str, region: event, given: event }
    relation Patch { id: u64, group: u64, value: i64, region: event, given: event }
    relation Allowed { id: u64 }
    Trial(id) -> Trial;
    Patch(id) -> Patch;
    Allowed(id) -> Allowed;
}
fn arithmetic() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn q(n: i64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut arithmetic()).unwrap()
}
fn source() -> Space {
    let raw = Space::new(SpaceId([244; 32]), 2, &()).unwrap();
    // World 3 remains possible with zero mass.
    raw.with_density(
        &[DensityPiece {
            region: raw.table(3, &[7], &()).unwrap(),
            density: q(1, 3),
        }],
        LawLimits::default(),
        &mut arithmetic(),
    )
    .unwrap()
}

#[test]
fn probability_stages_match_all_four_world_pairs_and_preserve_identity() {
    let dir = common::TempDir::new("observation-stages-oracle");
    let db = Db::create(dir.path(), ObservationStages, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    db.write(common::work(), |tx| {
        for event in 0..16 {
            for given in 0..16 {
                tx.insert([&Trial {
                    id: event * 16 + given,
                    label: "owned",
                    region: source.table(3, &[event], &())?,
                    given: source.table(3, &[given], &())?,
                }])?;
            }
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(db);
    let db = Db::open(dir.path(), ObservationStages, common::work()).unwrap();
    let staged = query!(ObservationStages {
        interior observed(id, label, region, given, chance: Probability(region, given)) | Trial(id, label, region, given);
        interior copied(id, label, region, given, chance) | observed(id, label, region, given, chance);
        interior repeated(id, chance: Probability(region & Full(region), given)) | Trial(id, region, given);
        (id, label, region, chance, fresh: Probability(region, given)) |
            copied(id, label, region, given, chance), repeated(id, chance);
    });
    let mut retained = Vec::new();
    for fallback in [false, true] {
        let mut prepared = db.prepare(&staged, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        for _ in 0..2 {
            retained.push(
                db.read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap(),
            );
        }
    }
    drop((db, source, staged));
    for answers in retained {
        assert_eq!(answers.len(), 256);
        for row in 0..answers.len() {
            let AnswerValue::U64(id) = answers.get(row, 0) else {
                panic!("id")
            };
            let (a, e) = (id / 16, id % 16);
            assert_eq!(answers.get(row, 1), AnswerValue::String("owned"));
            let AnswerValue::Probability(observation) = answers.get(row, 3) else {
                panic!("probability")
            };
            assert_eq!(answers.get(row, 3), answers.get(row, 4));
            assert_eq!(answers.get(row, 2), AnswerValue::Event(observation.event()));
            let ProbabilityValue::Fixed { value, .. } = observation.value() else {
                panic!("fixed")
            };
            let mass = (e & 7).count_ones();
            assert_eq!(
                *value,
                if mass == 0 {
                    None
                } else {
                    Some(q(i64::from((a & e & 7).count_ones()), u64::from(mass)))
                }
            );
            for world in 0..4 {
                assert_eq!(
                    observation.given().contains(world).unwrap(),
                    e & (1 << world) != 0
                );
            }
        }
    }
}

#[test]
fn equality_joins_antijoins_and_count_use_observations_not_equal_numbers() {
    let dir = common::TempDir::new("observation-identity");
    let db = Db::create(dir.path(), ObservationStages, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    db.write(common::work(), |tx| {
        for (id, event) in [1, 2, 1, 8].into_iter().enumerate() {
            tx.insert([&Trial {
                id: id as u64,
                label: "same number, different evidence",
                region: source.table(3, &[event], &())?,
                given: source.full(),
            }])?;
        }
        tx.insert([&Allowed { id: 0 }])
    })
    .unwrap()
    .unwrap();
    let queries = [
        query!(ObservationStages {
            interior observed(id, p: Probability(region, given)) | Trial(id, region, given);
            (left, right, p) | observed(left, p), observed(right, p);
        })
        .into_query(),
        query!(ObservationStages {
            interior observed(id, p: Probability(region, given)) | Trial(id, region, given);
            (left, right, p) | observed(left, p), observed(right, other), p == other;
        })
        .into_query(),
    ];
    for template in queries {
        for fallback in [false, true] {
            let mut prepared = db.prepare(&template, common::work()).unwrap();
            prepared.force_cursor_fallback(fallback);
            let answers = db
                .read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            assert_eq!(answers.len(), 6);
            for row in 0..answers.len() {
                let (AnswerValue::U64(a), AnswerValue::U64(b)) =
                    (answers.get(row, 0), answers.get(row, 1))
                else {
                    panic!("ids")
                };
                assert!(a == b || matches!((a, b), (0, 2) | (2, 0)));
            }
        }
    }
    let anti = query!(ObservationStages {
        interior observed(id, p: Probability(region, given)) | Trial(id, region, given);
        interior selected(p) | observed(id, p), Allowed(id);
        (id, p) | observed(id, p), !selected(p);
    });
    let count = query!(ObservationStages {
        interior observed(id, p: Probability(region, given)) | Trial(id, region, given);
        interior distinct(p) | observed(1: p);
        (p, n: Count) | distinct(p), observed(1: p);
    });
    for fallback in [false, true] {
        for (template, expected) in [(&*anti, 2), (&*count, 3)] {
            let mut prepared = db.prepare(template, common::work()).unwrap();
            prepared.force_cursor_fallback(fallback);
            let answers = db
                .read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            assert_eq!(answers.len(), expected);
        }
    }
}

#[test]
fn projected_expectations_are_values_and_canonicalize_before_stage_deduplication() {
    let dir = common::TempDir::new("expectation-staging");
    let db = Db::create(dir.path(), ObservationStages, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    db.write(common::work(), |tx| {
        for (group, values) in [[0, 3, 6, 0], [0, 3, 6, 0], [6, 3, 0, 0]]
            .into_iter()
            .enumerate()
        {
            for (world, value) in values.into_iter().enumerate() {
                tx.insert([&Patch {
                    id: (group * 4 + world) as u64,
                    group: group as u64,
                    value,
                    region: source.table(3, &[1 << world], &())?,
                    given: source.full(),
                }])?;
            }
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(ObservationStages {
        interior means(group, mean: Expectation(value, region, given)) | Patch(group, value, region, given);
        interior values(mean) | means(1: mean);
        (mean, groups: Count) | values(mean), means(group, mean);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 2);
        let mut counts = Vec::new();
        for row in 0..2 {
            let AnswerValue::Expectation(mean) = answers.get(row, 0) else {
                panic!("expectation")
            };
            let ExpectationValue::Fixed { value, .. } = mean.value() else {
                panic!("fixed")
            };
            assert_eq!(*value, Some(q(3, 1)));
            let AnswerValue::U64(count) = answers.get(row, 1) else {
                panic!("count")
            };
            counts.push(count);
        }
        counts.sort_unstable();
        assert_eq!(counts, [1, 2]);
        assert_ne!(answers.get(0, 0), answers.get(1, 0));
    }
    let mixed = query!(ObservationStages {
        interior means(group, expected: Expectation(value, region, given)) | Patch(group, value, region, given);
        (group, expected, fresh: Expectation(value, region, given)) | means(group, expected), Patch(group, value, region, given);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&mixed, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 3);
        for row in 0..3 {
            assert_eq!(answers.get(row, 1), answers.get(row, 2));
        }
    }
}

#[test]
fn producer_admission_cannot_be_hidden_by_a_consumer_filter() {
    let dir = common::TempDir::new("observation-producer-errors");
    let db = Db::create(dir.path(), ObservationStages, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    db.write(common::work(), |tx| {
        tx.insert([&Patch {
            id: 0,
            group: 0,
            value: 0,
            region: source.table(3, &[7], &())?,
            given: source.full(),
        }])
    })
    .unwrap()
    .unwrap();
    // Missing world 3 has zero mass but is structurally possible.
    let template = query!(ObservationStages {
        interior observed(group, mean: Expectation(value, region, given)) | Patch(group, value, region, given);
        (mean) | observed(group, mean), group == 999;
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        assert!(
            db.read(common::work(), |snapshot| snapshot
                .execute_collect(&mut prepared, &[] as &[BindValue]))
                .is_err()
        );
    }
}

#[test]
fn observation_domain_does_not_admit_scalar_event_or_parameter_coercion() {
    let dir = common::TempDir::new("observation-type-wall");
    let db = Db::create(dir.path(), ObservationStages, common::work())
        .unwrap()
        .unwrap();
    let templates = [
        query!(ObservationStages { interior obs(p: Probability(region, given)) | Trial(region, given); (p) | obs(p), Allowed(id: p); }).into_query(),
        query!(ObservationStages { interior obs(p: Probability(region, given)) | Trial(region, given); (p) | obs(p), p > p; }).into_query(),
        query!(ObservationStages { interior obs(p: Probability(region, given)) | Trial(region, given); (p) | obs(p), p == ?number; }).into_query(),
        query!(ObservationStages { interior obs(p: Probability(region, given)) | Trial(region, given); (n: Sum(p)) | obs(p); }).into_query(),
        query!(ObservationStages { interior obs(p: Probability(region, given)) | Trial(region, given); (e: Event(!p)) | obs(p); }).into_query(),
        query!(ObservationStages { interior obs(p: Probability(region, given)) | Trial(region, given); (id) | Trial(id), obs(0 == ?number); }).into_query(),
    ];
    for template in templates {
        assert!(matches!(
            db.prepare(&template, common::work()),
            Err(Error::Validation(_))
        ));
    }
}

#[test]
fn family_observations_keep_their_parameter_holes_after_staging_and_owner_release() {
    use bumbledb::{
        ImportedPayoff, PayoffImport,
        event::{
            BoolOp4, ExactPolynomial as Poly, FamilyFunction, GuardedRationalFunction,
            ParameterDensityPiece, ParameterDomain, ParameterId, ParameterRegion,
            ParameterSourceLimits, PolynomialSigns, SourceDescriptorLimits,
        },
    };
    let limits = ParameterSourceLimits::default();
    let name = ParameterId([245; 32]);
    let p = Poly::parameter(name);
    let tail = Poly::one()
        .sub(&p, limits.parameters.region.polynomial, &mut arithmetic())
        .unwrap();
    let region = |poly: &Poly| {
        ParameterRegion::from_polynomial(
            name,
            poly,
            PolynomialSigns::NON_NEGATIVE,
            limits.parameters.region,
            &mut arithmetic(),
        )
        .unwrap()
    };
    let domain = ParameterDomain::new(
        region(&p)
            .apply(
                BoolOp4::AND,
                &region(&tail),
                limits.parameters.region,
                &mut arithmetic(),
            )
            .unwrap(),
    )
    .unwrap();
    let function = |poly: Poly| {
        GuardedRationalFunction::new(
            domain.clone(),
            poly,
            Poly::one(),
            limits.parameters.region,
            &mut arithmetic(),
        )
        .unwrap()
    };
    let raw = Space::new(SpaceId([245; 32]), 1, &())
        .unwrap()
        .with_parameters(domain.clone(), &[], limits, &mut arithmetic())
        .unwrap();
    let bit = raw.coordinate(0, &()).unwrap();
    let source = raw
        .with_parameter_density(
            &[
                ParameterDensityPiece {
                    region: bit.clone(),
                    density: function(p.clone()),
                },
                ParameterDensityPiece {
                    region: bit.complement(),
                    density: function(tail),
                },
            ],
            limits,
            &mut arithmetic(),
        )
        .unwrap();
    let utility = PayoffImport::capture(
        ImportedPayoff::Family(
            FamilyFunction::constant(&source, function(p), limits, &mut arithmetic()).unwrap(),
        ),
        SourceDescriptorLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let dir = common::TempDir::new("family-observation-stages");
    let db = Db::create(dir.path(), ObservationStages, common::work())
        .unwrap()
        .unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Trial {
            id: 0,
            label: "family",
            region: source.coordinate(0, &())?,
            given: source.coordinate(0, &())?,
        }])
    })
    .unwrap()
    .unwrap();
    let template = query!(ObservationStages {
        use payoff utility = &utility;
        interior observed(id, chance: Probability(region, given), expected: Expectation(Payoff(utility), region, given)) | Trial(id, region, given);
        interior copied(chance, expected) | observed(1: chance, 2: expected);
        (chance, expected) | copied(chance, expected);
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
    drop((db, source, raw, template, utility));
    for answers in retained {
        assert_eq!(answers.len(), 1);
        let AnswerValue::Probability(chance) = answers.get(0, 0) else {
            panic!("probability")
        };
        let evidence = chance.given().clone();
        let ProbabilityValue::Parameter(chance) = chance.value() else {
            panic!("parameter")
        };
        let AnswerValue::Expectation(mean) = answers.get(0, 1) else {
            panic!("expectation")
        };
        let ExpectationValue::Family(mean) = mean.value() else {
            panic!("family")
        };
        assert_eq!(
            chance
                .value_at(&q(0, 1), limits, &mut arithmetic())
                .unwrap(),
            None
        );
        assert_eq!(
            mean.value_at(&q(0, 1), limits, &mut arithmetic()).unwrap(),
            None
        );
        assert_eq!(
            chance
                .value_at(&q(1, 3), limits, &mut arithmetic())
                .unwrap(),
            Some(q(1, 1))
        );
        assert_eq!(
            mean.value_at(&q(1, 3), limits, &mut arithmetic()).unwrap(),
            Some(q(1, 3))
        );
        assert_eq!(&evidence, mean.evidence());
    }
}

#[test]
fn reexecution_rebinds_observation_handles_without_changing_old_answers() {
    let dir = common::TempDir::new("observation-rebind");
    let db = Db::create(dir.path(), ObservationStages, common::work())
        .unwrap()
        .unwrap();
    let source = source();
    db.write(common::work(), |tx| {
        for id in 0..2 {
            tx.insert([&Trial {
                id,
                label: "rebind",
                region: source.table(3, &[1 << id], &())?,
                given: source.full(),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(ObservationStages {
        interior observed(p: Probability(region, given)) | Trial(id, region, given), id == ?selected;
        (p) | observed(p);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let retained = [0, 1, 0].map(|id| {
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[BindValue::U64(id)])
            })
            .unwrap()
        });
        prepared.release_memory();
        drop(prepared);
        assert_eq!(retained[0].get(0, 0), retained[2].get(0, 0));
        assert_ne!(retained[0].get(0, 0), retained[1].get(0, 0));
        for (answer, id) in retained.iter().zip([0, 1, 0]) {
            let AnswerValue::Probability(p) = answer.get(0, 0) else {
                panic!("probability")
            };
            assert!(p.event().contains(id).unwrap());
        }
    }
}
