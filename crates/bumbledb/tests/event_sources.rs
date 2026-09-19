use bumbledb::{
    Admission, AnswerValue, BindValue, Db, Fact,
    event::{
        ArithmeticLimits, DensityPiece, Error as EventError, ExactArithmetic, ExactRational,
        LawLimits, Limits, Space, SpaceId,
    },
    ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId},
    schema::FieldId,
};

mod common;

bumbledb::schema! {
    pub SourceSchema;
    relation Region { id: u64, condition: event }
    relation Observation { id: u64, condition: event }
    Region(id) -> Region;
    Observation(id) -> Observation;
}
bumbledb::schema! {
    pub PartitionSchema;
    relation Parent { group: u64, condition: event }
    relation Child { group: u64, branch: u64, condition: event }
    Parent(group) -> Parent;
    Parent(group, condition) -> Parent;
    Child(group, condition) -> Child;
    Parent(group, condition) == Child(group, condition);
}
fn arithmetic() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}

fn integrated_two_draws(shared: bool) -> Space {
    use bumbledb::event::{ExactPolynomial, ParameterId, PolynomialLimits};
    let limits = PolynomialLimits::default();
    let mut work = arithmetic();
    let first = ParameterId([1; 32]);
    let second = if shared { first } else { ParameterId([2; 32]) };
    let p = ExactPolynomial::parameter(first);
    let q = ExactPolynomial::parameter(second);
    let one = ExactPolynomial::one();
    let p_tail = one.sub(&p, limits, &mut work).unwrap();
    let q_tail = one.sub(&q, limits, &mut work).unwrap();
    let base = Space::new(SpaceId([93; 32]), 2, &()).unwrap();
    let mut pieces = Vec::new();
    for (world, (a, b)) in [(&p_tail, &q_tail), (&p, &q_tail), (&p_tail, &q), (&p, &q)]
        .into_iter()
        .enumerate()
    {
        let density = a.mul(b, limits, &mut work).unwrap();
        let density = density
            .integrate_beta(first, &ratio(1, 1), &ratio(1, 1), limits, &mut work)
            .unwrap();
        let density = if shared {
            density
        } else {
            density
                .integrate_beta(second, &ratio(1, 1), &ratio(1, 1), limits, &mut work)
                .unwrap()
        };
        // This fixture explicitly supplies the full, unconstrained prior cube.
        // Once every parameter is integrated, the existing finite constructor
        // still checks the complete joint distribution rather than trusting it.
        pieces.push(DensityPiece {
            region: base.table(3, &[1 << world], &()).unwrap(),
            density: density.evaluate(&[], limits, &mut work).unwrap(),
        });
    }
    base.with_density(&pieces, LawLimits::default(), &mut work)
        .unwrap()
}

#[test]
fn explicit_shared_prior_survives_stored_free_join_and_owner_release() {
    use bumbledb::query;
    let dir = common::TempDir::new("event-polynomial-prior");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    db.write(common::work(), |tx| {
        for (id, shared) in [(1, true), (2, false)] {
            let source = integrated_two_draws(shared);
            tx.insert([&Region {
                id,
                condition: source.coordinate(0, &())?,
            }])?;
            tx.insert([&Observation {
                id,
                condition: source.coordinate(1, &())?,
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(db);
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let query = query!(SourceSchema {
        (source_id, both: Event(a & b), first: Event(a), differ: Event(a ^ b)) |
            Region(id: source_id, condition: a), Observation(id: source_id, condition: b);
    });
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((db, query));
    for answers in retained {
        assert_eq!(answers.len(), 2);
        for row in 0..answers.len() {
            let AnswerValue::U64(id) = answers.get(row, 0) else {
                panic!("id")
            };
            let (AnswerValue::Event(both), AnswerValue::Event(first), AnswerValue::Event(differ)) = (
                answers.get(row, 1),
                answers.get(row, 2),
                answers.get(row, 3),
            ) else {
                panic!("owned Events")
            };
            assert_eq!(
                both.mass(&mut arithmetic()).unwrap(),
                ratio(1, if id == 1 { 3 } else { 4 })
            );
            assert_eq!(first.mass(&mut arithmetic()).unwrap(), ratio(1, 2));
            assert_eq!(
                differ.mass(&mut arithmetic()).unwrap(),
                ratio(1, if id == 1 { 3 } else { 2 })
            );
            assert_eq!(
                first
                    .probability(differ, &mut arithmetic())
                    .unwrap()
                    .value(&mut arithmetic())
                    .unwrap(),
                Some(ratio(1, 2))
            );
        }
    }
}
fn ratio(n: u64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut arithmetic()).unwrap()
}
fn source(reverse: bool, correlated: bool) -> Space {
    let base = Space::with_order(
        SpaceId([78; 32]),
        if reverse { &[1, 0] } else { &[0, 1] },
        Limits::default(),
        &(),
    )
    .unwrap();
    let region = if correlated {
        base.table(3, &[0b1001], &()).unwrap()
    } else {
        base.full()
    };
    base.with_density(
        &[DensityPiece {
            region,
            density: ratio(1, if correlated { 2 } else { 4 }),
        }],
        LawLimits::default(),
        &mut arithmetic(),
    )
    .unwrap()
}
fn scan() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(Region::RELATION),
            bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn measured_rows_reopen_deduplicate_and_keep_their_law_after_owner_drop() {
    let dir = common::TempDir::new("event-source-storage");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let first = source(false, true).coordinate(0, &()).unwrap();
    let same = source(true, true).coordinate(0, &()).unwrap();
    let canonical = first.to_bytes(&()).unwrap();
    let fixture = include_str!("fixtures/event-v2-source.hex").trim();
    let fixture: Vec<_> = (0..fixture.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&fixture[index..index + 2], 16).unwrap())
        .collect();
    assert_eq!(canonical, fixture);
    db.write(common::work(), |tx| {
        assert_eq!(
            tx.insert([&Region {
                id: 1,
                condition: first.clone()
            }])?
            .changed(),
            1
        );
        assert_eq!(
            tx.insert([&Region {
                id: 1,
                condition: same.clone()
            }])?
            .changed(),
            0
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(first);
    drop(same);
    drop(db);
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let mut prepared = db.prepare(&scan(), common::work()).unwrap();
    let answers = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    drop(prepared);
    drop(db);
    let AnswerValue::Event(event) = answers.get(0, 0) else {
        panic!("owned measured Event")
    };
    assert_eq!(event.to_bytes(&()).unwrap(), canonical);
    assert_eq!(event.mass(&mut arithmetic()).unwrap(), ratio(1, 2));
    let observation = event.probability(event, &mut arithmetic()).unwrap();
    assert_eq!(observation.evidence_mass(), &ratio(1, 2));
    assert_eq!(
        observation.value(&mut arithmetic()).unwrap(),
        Some(ExactRational::one())
    );
}

#[test]
fn free_join_distinguishes_laws_with_equal_marginals_on_both_execution_paths() {
    let dir = common::TempDir::new("event-source-free-join");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let correlated = source(false, true);
    let independent = source(false, false);
    let restored = source(true, true);
    db.write(common::work(), |tx| {
        for (id, space) in [(1, &correlated), (2, &independent)] {
            tx.insert([&Region {
                id,
                condition: space.coordinate(0, &())?,
            }])?;
            tx.insert([&Region {
                id: id + 2,
                condition: space.empty(),
            }])?;
        }
        tx.insert([
            &Observation {
                id: 1,
                condition: restored.coordinate(0, &())?,
            },
            &Observation {
                id: 2,
                condition: restored.empty(),
            },
        ])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let mut query = scan();
    query.rules[0].finds.insert(0, FindTerm::Var(VarId(1)));
    query.rules[0].atoms[0]
        .bindings
        .push((FieldId(0), Term::Var(VarId(1))));
    query.rules[0].atoms.push(Atom {
        source: AtomSource::Edb(Observation::RELATION),
        bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
    });
    let query = Query::single(query.rules.remove(0));
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        let mut ids = Vec::new();
        for row in 0..answers.len() {
            let AnswerValue::U64(id) = answers.get(row, 0) else {
                panic!("id")
            };
            let AnswerValue::Event(event) = answers.get(row, 1) else {
                panic!("Event")
            };
            assert_eq!(
                event.mass(&mut arithmetic()).unwrap(),
                if id == 1 {
                    ratio(1, 2)
                } else {
                    ExactRational::zero()
                }
            );
            ids.push(id);
        }
        ids.sort_unstable();
        assert_eq!(ids, [1, 3]);
        db.clear_cache();
    }
}

#[test]
fn zero_mass_branches_still_obey_pointwise_keys_coverage_and_rollback() {
    let dir = common::TempDir::new("event-source-admission");
    let db = Db::create(dir.path(), PartitionSchema, common::work())
        .unwrap()
        .unwrap();
    let space = source(false, true);
    let positive = Child {
        group: 1,
        branch: 1,
        condition: space.table(3, &[0b1001], &()).unwrap(),
    };
    let zero = Child {
        group: 1,
        branch: 2,
        condition: space.table(3, &[0b0110], &()).unwrap(),
    };
    assert_eq!(
        zero.condition.mass(&mut arithmetic()).unwrap(),
        ExactRational::zero()
    );
    assert!(!zero.condition.is_empty());
    let missing = db
        .write(common::work(), |tx| {
            tx.insert([&Parent {
                group: 1,
                condition: space.full(),
            }])?;
            tx.insert([&positive])?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(missing, Admission::Rejected(_)));
    db.write(common::work(), |tx| {
        tx.insert([&Parent {
            group: 1,
            condition: space.full(),
        }])?;
        tx.insert([&positive, &zero])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let duplicate = Child {
        branch: 3,
        ..zero.clone()
    };
    assert!(matches!(
        db.write(common::work(), |tx| tx.insert([&duplicate]))
            .unwrap(),
        Admission::Rejected(_)
    ));
    assert!(matches!(
        db.write(common::work(), |tx| tx.delete([&zero])).unwrap(),
        Admission::Rejected(_)
    ));
    let revised = source(false, false);
    let foreign = db.write(common::work(), |tx| {
        tx.delete([&zero])?;
        tx.insert([&Child {
            condition: revised.table(3, &[0b0110], &())?,
            ..zero.clone()
        }])?;
        Ok(())
    });
    assert!(matches!(
        foreign,
        Err(bumbledb::Error::Event(EventError::SpaceMismatch))
    ));
    let rows: Vec<Child> = db
        .read(common::work(), |snapshot| snapshot.scan_facts()?.collect())
        .unwrap();
    assert_eq!(rows.len(), 2);
    let retained = rows
        .iter()
        .find(|row| row.branch == 2)
        .unwrap()
        .condition
        .clone();
    drop(db);
    assert_eq!(retained.count(&()).unwrap(), 2);
    assert_eq!(
        retained.mass(&mut arithmetic()).unwrap(),
        ExactRational::zero()
    );
}

#[test]
fn conditional_source_maps_flow_through_persisted_coup_queries() {
    use bumbledb::{
        EventImport,
        event::{AdmittedDescriptor, DescriptorLimits},
        query,
    };
    let (prior, extension) = tax_extension();
    let import = EventImport::capture(
        &AdmittedDescriptor::Map(extension.parent().clone()),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap();
    let dir = common::TempDir::new("event-source-coup-query");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    db.write(common::work(), |tx| {
        tx.insert([
            &Region {
                id: 1,
                condition: prior.coordinate(0, &())?,
            },
            &Region {
                id: 2,
                condition: prior.coordinate(1, &())?,
            },
        ])?;
        tx.insert([&Observation {
            id: 1,
            condition: extension.space().coordinate(2, &())?,
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop((prior, extension, db));
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let query = query!(SourceSchema {
        use map parent = &import;
        (player, taxed: Event(Pullback(holding, parent) & tax), given: Event(tax)) |
            Region(id: player, condition: holding), Observation(condition: tax);
    });
    drop(import);
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((db, query));
    for answers in retained {
        assert_eq!(answers.len(), 2);
        for row in 0..answers.len() {
            let AnswerValue::U64(player) = answers.get(row, 0) else {
                panic!("player")
            };
            let (AnswerValue::Event(numerator), AnswerValue::Event(evidence)) =
                (answers.get(row, 1), answers.get(row, 2))
            else {
                panic!("Event")
            };
            let result = numerator.probability(evidence, &mut arithmetic()).unwrap();
            assert_eq!(
                result.value(&mut arithmetic()).unwrap(),
                Some(if player == 1 {
                    ratio(92, 147)
                } else {
                    ratio(5, 21)
                })
            );
            assert_eq!(result.evidence_mass(), &ratio(49, 130));
        }
    }
}

fn portable_source(
    value: bumbledb::event::AdmittedSourceDescriptor,
) -> bumbledb::event::AdmittedSourceDescriptor {
    use bumbledb::event::{SourceDescriptor, SourceDescriptorLimits};
    let limits = SourceDescriptorLimits::default();
    let bytes = SourceDescriptor::capture(&value, limits, &mut arithmetic())
        .unwrap()
        .to_bytes(limits, &())
        .unwrap();
    drop(value);
    SourceDescriptor::import(&bytes, limits, &mut arithmetic()).unwrap()
}

fn tax_extension() -> (Space, bumbledb::event::SourceExtension) {
    use bumbledb::event::{
        BoolOp4, CoordinateMap, FiniteFunction, FiniteKernel, FunctionLimits, FunctionPiece,
    };
    let raw = Space::new(SpaceId([101; 32]), 2, &()).unwrap();
    let pieces: Vec<_> = [36, 19, 19, 4]
        .into_iter()
        .enumerate()
        .map(|(world, numerator)| DensityPiece {
            region: raw.table(3, &[1 << world], &()).unwrap(),
            density: ratio(numerator, 78),
        })
        .collect();
    let prior = raw
        .with_density(&pieces, LawLimits::default(), &mut arithmetic())
        .unwrap();
    let joint = Space::new(SpaceId([102; 32]), 3, &()).unwrap();
    let parent = CoordinateMap::coordinates(&joint, &prior, &[0, 1], &()).unwrap();
    let same = joint
        .coordinate(0, &())
        .unwrap()
        .apply(
            BoolOp4::EQUIVALENCE,
            &joint.coordinate(2, &()).unwrap(),
            &(),
        )
        .unwrap();
    let likelihood = FiniteFunction::new(
        &joint,
        &[
            FunctionPiece {
                region: same.clone(),
                value: ratio(4, 5),
            },
            FunctionPiece {
                region: same.complement(),
                value: ratio(1, 5),
            },
        ],
        FunctionLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let kernel = FiniteKernel::new(
        &parent,
        &likelihood,
        FunctionLimits::default(),
        &mut arithmetic(),
    )
    .unwrap();
    let bumbledb::event::AdmittedSourceDescriptor::Kernel(kernel) =
        portable_source(bumbledb::event::AdmittedSourceDescriptor::Kernel(kernel))
    else {
        panic!("imported channel")
    };
    let extension = kernel
        .close(
            &prior,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    (prior, extension)
}

#[test]
fn conditioned_sources_reopen_translate_queries_and_retain_signed_expectations() {
    use bumbledb::{
        EventImport,
        event::{
            AdmittedDescriptor, AdmittedSourceDescriptor, DescriptorLimits, FunctionLimits,
            RevisionReceipt, SourceDescriptor, SourceDescriptorLimits,
        },
        query,
    };
    let (prior, extension) = tax_extension();
    let tax = extension.space().coordinate(2, &()).unwrap();
    let revision = extension
        .space()
        .condition(
            &tax,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let revised = revision.revised().unwrap();
    let limits = SourceDescriptorLimits::default();
    let receipt = SourceDescriptor::capture(
        &AdmittedSourceDescriptor::Revision(revision.clone()),
        limits,
        &mut arithmetic(),
    )
    .unwrap()
    .to_bytes(limits, &())
    .unwrap();
    let dir = common::TempDir::new("event-source-revision-query");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    db.write(common::work(), |tx| {
        for id in 1..=2 {
            tx.insert([&Region {
                id,
                condition: extension
                    .parent()
                    .pullback(&prior.coordinate(u8::try_from(id - 1).unwrap(), &())?, &())?,
            }])?;
        }
        tx.insert([&Observation {
            id: 1,
            condition: revised.translation().pullback(&tax, &())?,
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    // The envelope is an application-owned sidecar, not a new database field.
    let receipt_path = dir.path().join("revision.besc");
    std::fs::write(&receipt_path, receipt).unwrap();
    drop((prior, extension, tax, db, revision));
    let bytes = std::fs::read(&receipt_path).unwrap();
    let AdmittedSourceDescriptor::Revision(revision) =
        SourceDescriptor::import(&bytes, limits, &mut arithmetic()).unwrap()
    else {
        panic!("replayed revision")
    };
    let import = EventImport::capture(
        &AdmittedDescriptor::Map(revision.revised().unwrap().translation().clone()),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap();
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let query = query!(SourceSchema {
        use map revision = &import;
        (player, posterior: Event(Pullback(holding, revision)), observed: Event(tax)) |
            Region(id: player, condition: holding), Observation(condition: tax);
    });
    drop(import);
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((db, query));
    let RevisionReceipt::Condition { evidence, mass } = revision.receipt() else {
        panic!("owned conditioning receipt")
    };
    assert_eq!(mass, &ratio(49, 130));
    assert_eq!(evidence.mass(&mut arithmetic()).unwrap(), *mass);
    drop(revision);
    for answers in retained {
        check_revised_coup_answers(&answers);
    }
}

fn check_revised_coup_answers(answers: &bumbledb::Answers) {
    use bumbledb::event::{FiniteFunction, FunctionLimits, FunctionPiece};
    assert_eq!(answers.len(), 2);
    for row in 0..answers.len() {
        let AnswerValue::U64(player) = answers.get(row, 0) else {
            panic!("player")
        };
        let (AnswerValue::Event(duke), AnswerValue::Event(tax)) =
            (answers.get(row, 1), answers.get(row, 2))
        else {
            panic!("Events")
        };
        assert_eq!(
            duke.mass(&mut arithmetic()).unwrap(),
            if player == 1 {
                ratio(92, 147)
            } else {
                ratio(5, 21)
            }
        );
        assert_eq!(tax.mass(&mut arithmetic()).unwrap(), ratio(1, 1));
        assert!(!tax.is_full());
        assert!(!tax.complement().is_empty());
        assert_eq!(
            tax.complement().mass(&mut arithmetic()).unwrap(),
            ratio(0, 1)
        );
        if player == 1 {
            // Explicit toy utility: gain one on a bluff, lose one on a Duke.
            let payoff = FiniteFunction::new(
                &duke.space(),
                &[
                    FunctionPiece {
                        region: duke.clone(),
                        value: (-1i64).into(),
                    },
                    FunctionPiece {
                        region: duke.complement(),
                        value: 1u64.into(),
                    },
                ],
                FunctionLimits::default(),
                &mut arithmetic(),
            )
            .unwrap();
            let bumbledb::event::AdmittedSourceDescriptor::Function(payoff) =
                portable_source(bumbledb::event::AdmittedSourceDescriptor::Function(payoff))
            else {
                panic!("imported payoff")
            };
            let result = payoff.expectation(tax, &mut arithmetic()).unwrap();
            drop(payoff);
            assert_eq!(
                result.value(&mut arithmetic()).unwrap(),
                Some(ExactRational::fraction("-37", "147", &mut arithmetic()).unwrap())
            );
        }
    }
}

fn parameterized_draws() -> Space {
    use bumbledb::event::{
        BoolOp4, ExactPolynomial as Poly, GuardedRationalFunction, ParameterDensityPiece,
        ParameterDomain, ParameterGuard, ParameterId, ParameterRegion, ParameterSourceLimits,
        PolynomialSigns,
    };
    let limits = ParameterSourceLimits::default();
    let mut work = arithmetic();
    let name = ParameterId([97; 32]);
    let p = Poly::parameter(name);
    let q = Poly::one()
        .sub(&p, limits.parameters.region.polynomial, &mut work)
        .unwrap();
    let lower = ParameterRegion::from_polynomial(
        name,
        &p,
        PolynomialSigns::NON_NEGATIVE,
        limits.parameters.region,
        &mut work,
    )
    .unwrap();
    let upper = ParameterRegion::from_polynomial(
        name,
        &q,
        PolynomialSigns::NON_NEGATIVE,
        limits.parameters.region,
        &mut work,
    )
    .unwrap();
    let domain = ParameterDomain::new(
        lower
            .apply(BoolOp4::AND, &upper, limits.parameters.region, &mut work)
            .unwrap(),
    )
    .unwrap();
    let zero = ParameterRegion::from_polynomial(
        name,
        &p,
        PolynomialSigns::ZERO,
        limits.parameters.region,
        &mut work,
    )
    .unwrap();
    let base = Space::new(SpaceId([97; 32]), 3, &())
        .unwrap()
        .with_parameters(
            domain.clone(),
            &[ParameterGuard {
                coordinate: 2,
                region: zero,
            }],
            limits,
            &mut work,
        )
        .unwrap();
    let mut pieces = Vec::new();
    for (world, (a, b)) in [(&q, &q), (&p, &q), (&q, &p), (&p, &p)]
        .into_iter()
        .enumerate()
    {
        pieces.push(ParameterDensityPiece {
            region: base.table(3, &[1 << world], &()).unwrap(),
            density: GuardedRationalFunction::new(
                domain.clone(),
                a.mul(b, limits.parameters.region.polynomial, &mut work)
                    .unwrap(),
                Poly::one(),
                limits.parameters.region,
                &mut work,
            )
            .unwrap(),
        });
    }
    base.with_parameter_density(&pieces, limits, &mut work)
        .unwrap()
}

#[test]
fn shared_unknown_parameter_survives_reopen_both_free_join_paths_and_owner_release() {
    use bumbledb::{
        event::{ParameterSourceLimits, WorldCardinality},
        query,
    };
    let dir = common::TempDir::new("event-parameter-source");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let source = parameterized_draws();
    let first = source.coordinate(0, &()).unwrap();
    let second = source.coordinate(1, &()).unwrap();
    let bytes = first.to_bytes(&()).unwrap();
    let decoded = bumbledb::event::Event::from_bytes(&bytes, &()).unwrap();
    db.write(common::work(), |tx| {
        assert_eq!(
            tx.insert([&Region {
                id: 1,
                condition: first.clone()
            }])?
            .changed(),
            1
        );
        assert_eq!(
            tx.insert([&Region {
                id: 1,
                condition: decoded.clone()
            }])?
            .changed(),
            0
        );
        tx.insert([&Observation {
            id: 1,
            condition: second.clone(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop((source, first, second, decoded, db));
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let query = query!(SourceSchema {
        (both: Event(a & b), first: Event(a), differ: Event(a ^ b)) |
            Region(id: source_id, condition: a), Observation(id: source_id, condition: b);
    });
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((query, db));
    let limits = ParameterSourceLimits::default();
    for answers in retained {
        assert_eq!(answers.len(), 1);
        let (AnswerValue::Event(both), AnswerValue::Event(first), AnswerValue::Event(differ)) =
            (answers.get(0, 0), answers.get(0, 1), answers.get(0, 2))
        else {
            panic!("owned Events")
        };
        assert_eq!(first.to_bytes(&()).unwrap(), bytes);
        assert_eq!(
            first.world_cardinality(&()).unwrap(),
            WorldCardinality::Continuum
        );
        let mass = both.parameter_mass(limits, &mut arithmetic()).unwrap();
        let observation = first
            .parameter_probability(differ, limits, &mut arithmetic())
            .unwrap();
        for n in 0..=8 {
            let at = ratio(n, 8);
            assert_eq!(
                mass.value_at(
                    &at,
                    limits.parameters.region,
                    limits.functions,
                    &mut arithmetic()
                )
                .unwrap(),
                Some(ratio(n * n, 64))
            );
            assert_eq!(
                observation
                    .value_at(&at, limits, &mut arithmetic())
                    .unwrap(),
                if n == 0 || n == 8 {
                    None
                } else {
                    Some(ratio(1, 2))
                }
            );
        }
    }
}

#[test]
fn parameter_guarded_zero_mass_worlds_still_participate_in_fd_and_ind_admission() {
    use bumbledb::event::{BoolOp4, ParameterSourceLimits};
    let dir = common::TempDir::new("event-parameter-admission");
    let db = Db::create(dir.path(), PartitionSchema, common::work())
        .unwrap()
        .unwrap();
    let source = parameterized_draws();
    let zero_region = source
        .coordinate(0, &())
        .unwrap()
        .apply(BoolOp4::AND, &source.coordinate(2, &()).unwrap(), &())
        .unwrap();
    assert!(!zero_region.is_empty());
    let zero = Child {
        group: 1,
        branch: 0,
        condition: zero_region.clone(),
    };
    let rest = Child {
        group: 1,
        branch: 1,
        condition: zero_region.complement(),
    };
    let missing = db
        .write(common::work(), |tx| {
            tx.insert([&Parent {
                group: 1,
                condition: source.full(),
            }])?;
            tx.insert([&rest])?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(missing, Admission::Rejected(_)));
    db.write(common::work(), |tx| {
        tx.insert([&Parent {
            group: 1,
            condition: source.full(),
        }])?;
        tx.insert([&rest, &zero])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert!(matches!(
        db.write(common::work(), |tx| tx.insert([&Child {
            branch: 2,
            ..zero.clone()
        }]))
        .unwrap(),
        Admission::Rejected(_)
    ));
    assert!(matches!(
        db.write(common::work(), |tx| tx.delete([&zero])).unwrap(),
        Admission::Rejected(_)
    ));
    drop(db);
    let db = Db::open(dir.path(), PartitionSchema, common::work()).unwrap();
    let rows: Vec<Child> = db
        .read(common::work(), |snapshot| snapshot.scan_facts()?.collect())
        .unwrap();
    assert_eq!(rows.len(), 2);
    let retained = rows
        .iter()
        .find(|r| r.branch == 0)
        .unwrap()
        .condition
        .clone();
    drop((db, rows, source));
    let observation = retained
        .parameter_probability(
            &retained,
            ParameterSourceLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    assert!(observation.is_impossible());
    assert_eq!(retained.count(&()).unwrap(), 2);
}

#[test]
#[allow(clippy::too_many_lines)]
fn guard_refinement_splits_a_stored_partition_and_preserves_its_family_through_free_join() {
    use bumbledb::{
        event::{BoolOp4, ParameterRefinement, ParameterSourceLimits},
        query,
    };
    let dir = common::TempDir::new("event-parameter-refinement");
    let db = Db::create(dir.path(), PartitionSchema, common::work())
        .unwrap()
        .unwrap();
    let source = parameterized_draws();
    let heads = source.coordinate(0, &()).unwrap();
    let parent = Parent {
        group: 1,
        condition: source.full(),
    };
    let yes = Child {
        group: 1,
        branch: 1,
        condition: heads.clone(),
    };
    let no = Child {
        group: 1,
        branch: 2,
        condition: heads.complement(),
    };
    db.write(common::work(), |tx| {
        tx.insert([&parent])?;
        tx.insert([&yes, &no])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let limits = ParameterSourceLimits::default();
    let above_half = parameter_above_half(&source);
    let refinement = ParameterRefinement::new(
        SpaceId([98; 32]),
        &source,
        std::slice::from_ref(&above_half),
        limits,
        &mut arithmetic(),
    )
    .unwrap();
    let refined = refinement.refined();
    let guard = refined
        .parameter_event(&above_half, limits, &mut arithmetic())
        .unwrap();
    let lifted = refinement.lift(&heads, &()).unwrap();
    let high = Child {
        group: 1,
        branch: 1,
        condition: lifted.apply(BoolOp4::AND, &guard, &()).unwrap(),
    };
    let low = Child {
        group: 1,
        branch: 2,
        condition: lifted
            .apply(BoolOp4::AND, &guard.complement(), &())
            .unwrap(),
    };
    let tails = Child {
        group: 1,
        branch: 3,
        condition: lifted.complement(),
    };
    db.write(common::work(), |tx| {
        tx.delete([&parent])?;
        tx.delete([&yes, &no])?;
        tx.insert([&Parent {
            group: 1,
            condition: refined.full(),
        }])?;
        tx.insert([&high, &low, &tails])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert!(matches!(
        db.write(common::work(), |tx| tx.delete([&low])).unwrap(),
        Admission::Rejected(_)
    ));
    drop((
        source, heads, parent, yes, no, refinement, guard, lifted, high, low, tails, db,
    ));
    let db = Db::open(dir.path(), PartitionSchema, common::work()).unwrap();
    let query = query!(PartitionSchema {
        (branch, region: Event(all & part)) | Parent(group: group, condition: all), Child(group: group, branch: branch, condition: part);
    });
    let mut results = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        results.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((db, query));
    for answers in results {
        check_refined_partition(&answers);
    }
}

fn parameter_above_half(source: &Space) -> bumbledb::event::ParameterRegion {
    use bumbledb::event::{
        ExactPolynomial, ParameterRegion, ParameterSourceLimits, PolynomialSigns,
    };
    let limits = ParameterSourceLimits::default();
    let name = source.parameter_domain().unwrap().parameter();
    let p = ExactPolynomial::parameter(name);
    let polynomial = p
        .mul(
            &ExactPolynomial::constant(2u64.into()),
            limits.parameters.region.polynomial,
            &mut arithmetic(),
        )
        .unwrap()
        .sub(
            &ExactPolynomial::one(),
            limits.parameters.region.polynomial,
            &mut arithmetic(),
        )
        .unwrap();
    ParameterRegion::from_polynomial(
        name,
        &polynomial,
        PolynomialSigns::POSITIVE,
        limits.parameters.region,
        &mut arithmetic(),
    )
    .unwrap()
}

fn check_refined_partition(answers: &bumbledb::Answers) {
    let limits = bumbledb::event::ParameterSourceLimits::default();
    assert_eq!(answers.len(), 3);
    for row in 0..answers.len() {
        let (AnswerValue::U64(branch), AnswerValue::Event(event)) =
            (answers.get(row, 0), answers.get(row, 1))
        else {
            panic!("refined branch")
        };
        let mass = event.parameter_mass(limits, &mut arithmetic()).unwrap();
        for numerator in 0..=8 {
            let expected = match branch {
                1 if numerator > 4 => ratio(numerator, 8),
                2 if numerator <= 4 => ratio(numerator, 8),
                3 => ratio(8 - numerator, 8),
                _ => ratio(0, 1),
            };
            assert_eq!(
                mass.value_at(
                    &ratio(numerator, 8),
                    limits.parameters.region,
                    limits.functions,
                    &mut arithmetic()
                )
                .unwrap(),
                Some(expected)
            );
        }
    }
}

#[test]
fn family_posterior_translation_persists_and_runs_on_both_free_join_paths() {
    use bumbledb::{
        EventImport,
        event::{AdmittedDescriptor, BoolOp4, DescriptorLimits, ParameterSourceLimits},
        query,
    };
    let dir = common::TempDir::new("event-family-conditioning");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let prior = parameterized_draws();
    let first = prior.coordinate(0, &()).unwrap();
    let evidence = first
        .apply(BoolOp4::XOR, &prior.coordinate(1, &()).unwrap(), &())
        .unwrap();
    let receipt = prior
        .parameter_condition(
            SpaceId([99; 32]),
            &evidence,
            ParameterSourceLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let posterior = receipt.revised().unwrap();
    let import = EventImport::capture(
        &AdmittedDescriptor::Map(posterior.translation().clone()),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            condition: posterior.restriction().refinement().lift(&first, &())?,
        }])?;
        tx.insert([&Observation {
            id: 1,
            condition: posterior.pullback(&evidence, &())?,
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop((prior, first, evidence, receipt, db));
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let query = query!(SourceSchema {
        use map revision = &import;
        (posterior: Event(Pullback(holding, revision)), observed: Event(evidence)) |
            Region(id: id, condition: holding), Observation(id: id, condition: evidence);
    });
    drop(import);
    let mut results = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        results.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((query, db));
    for answers in results {
        check_family_posterior(&answers);
    }
}

fn check_family_posterior(answers: &bumbledb::Answers) {
    use bumbledb::event::ParameterSourceLimits;
    let limits = ParameterSourceLimits::default();
    assert_eq!(answers.len(), 1);
    let (AnswerValue::Event(head), AnswerValue::Event(observed)) =
        (answers.get(0, 0), answers.get(0, 1))
    else {
        panic!("family posterior")
    };
    check_family_query_payoff(head, observed);
    let probability = head
        .parameter_probability(observed, limits, &mut arithmetic())
        .unwrap();
    for n in 0..=8 {
        assert_eq!(
            probability
                .value_at(&ratio(n, 8), limits, &mut arithmetic())
                .unwrap(),
            if n == 0 || n == 8 {
                None
            } else {
                Some(ratio(1, 2))
            }
        );
    }
    assert!(!observed.complement().is_empty());
    assert_eq!(
        observed
            .complement()
            .parameter_mass(limits, &mut arithmetic())
            .unwrap()
            .value_at(
                &ratio(1, 2),
                limits.parameters.region,
                limits.functions,
                &mut arithmetic()
            )
            .unwrap(),
        Some(ratio(0, 1))
    );
}

fn check_family_query_payoff(head: &bumbledb::Event, observed: &bumbledb::Event) {
    use bumbledb::event::{
        ExactPolynomial, FamilyFunction, FiniteFunction, FunctionPiece, GuardedRationalFunction,
        ParameterSourceLimits,
    };
    let limits = ParameterSourceLimits::default();
    let function = FiniteFunction::new(
        &head.space(),
        &[FunctionPiece {
            region: head.clone(),
            value: 4u64.into(),
        }],
        limits.functions,
        &mut arithmetic(),
    )
    .unwrap();
    let observation = function
        .parameter_expectation(observed, limits, &mut arithmetic())
        .unwrap();
    let family = FamilyFunction::from_finite(&function, limits, &mut arithmetic()).unwrap();
    let domain = head.space().parameter_domain().unwrap().clone();
    let p = FamilyFunction::constant(
        &head.space(),
        GuardedRationalFunction::new(
            domain.clone(),
            ExactPolynomial::parameter(domain.parameter()),
            ExactPolynomial::one(),
            limits.parameters.region,
            &mut arithmetic(),
        )
        .unwrap(),
        limits,
        &mut arithmetic(),
    )
    .unwrap();
    let parameter_payoff = family
        .multiply(&p, limits, &mut arithmetic())
        .unwrap()
        .expectation(observed, limits, &mut arithmetic())
        .unwrap();
    drop((family, p));
    drop(function);
    for n in 0..=8 {
        assert_eq!(
            parameter_payoff
                .value_at(&ratio(n, 8), limits, &mut arithmetic())
                .unwrap(),
            if n == 0 || n == 8 {
                None
            } else {
                Some(ratio(n, 4))
            }
        );
        assert_eq!(
            observation
                .value_at(&ratio(n, 8), limits, &mut arithmetic())
                .unwrap(),
            if n == 0 || n == 8 {
                None
            } else {
                Some(2u64.into())
            }
        );
    }
}

#[test]
fn family_posterior_zero_mass_outcomes_still_require_relational_coverage() {
    use bumbledb::event::ParameterSourceLimits;
    let dir = common::TempDir::new("event-family-conditioning-coverage");
    let db = Db::create(dir.path(), PartitionSchema, common::work())
        .unwrap()
        .unwrap();
    let prior = parameterized_draws();
    let evidence = prior.coordinate(0, &()).unwrap();
    let receipt = prior
        .parameter_condition(
            SpaceId([100; 32]),
            &evidence,
            ParameterSourceLimits::default(),
            &mut arithmetic(),
        )
        .unwrap();
    let posterior = receipt.revised().unwrap();
    let parent = Parent {
        group: 1,
        condition: posterior.space().full(),
    };
    let yes = Child {
        group: 1,
        branch: 1,
        condition: posterior.pullback(&evidence, &()).unwrap(),
    };
    let no = Child {
        group: 1,
        branch: 2,
        condition: yes.condition.complement(),
    };
    assert!(!no.condition.is_empty());
    assert!(matches!(
        db.write(common::work(), |tx| {
            tx.insert([&parent])?;
            tx.insert([&yes])?;
            Ok(())
        })
        .unwrap(),
        Admission::Rejected(_)
    ));
    db.write(common::work(), |tx| {
        tx.insert([&parent])?;
        tx.insert([&yes, &no])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert!(matches!(
        db.write(common::work(), |tx| tx.insert([&Child {
            branch: 3,
            ..no.clone()
        }]))
        .unwrap(),
        Admission::Rejected(_)
    ));
    assert!(matches!(
        db.write(common::work(), |tx| tx.delete([&no])).unwrap(),
        Admission::Rejected(_)
    ));
}

#[test]
#[allow(clippy::too_many_lines)]
fn family_channel_and_jeffrey_maps_keep_owned_probabilities_through_free_join() {
    use bumbledb::event::{
        AdmittedFamilyDescriptor, CoordinateMap, EventPartition, ExactPolynomial, FamilyFunction,
        FamilyFunctionPiece, FamilyKernel, GuardedRationalFunction, ParameterFunction,
        ParameterSourceLimits, PartitionLimits,
    };
    let limits = ParameterSourceLimits::default();
    let prior = parameterized_draws();
    let raw = Space::new(SpaceId([104; 32]), 4, &())
        .unwrap()
        .with_parameters(
            prior.parameter_domain().unwrap().clone(),
            prior.parameter_guards().unwrap(),
            limits,
            &mut arithmetic(),
        )
        .unwrap();
    let domain = raw.parameter_domain().unwrap();
    let p = ExactPolynomial::parameter(domain.parameter());
    let q = ExactPolynomial::one()
        .sub(&p, limits.parameters.region.polynomial, &mut arithmetic())
        .unwrap();
    let value = |n, d| {
        GuardedRationalFunction::new(
            domain.clone(),
            n,
            d,
            limits.parameters.region,
            &mut arithmetic(),
        )
        .unwrap()
    };
    let next = raw.coordinate(3, &()).unwrap();
    let density = FamilyFunction::new(
        &raw,
        &[
            FamilyFunctionPiece {
                region: next.clone(),
                value: value(p, ExactPolynomial::one()),
            },
            FamilyFunctionPiece {
                region: next.complement(),
                value: value(q, ExactPolynomial::one()),
            },
        ],
        limits,
        &mut arithmetic(),
    )
    .unwrap();
    let parent = CoordinateMap::coordinates(&raw, &prior, &[0, 1, 2], &()).unwrap();
    let kernel = FamilyKernel::new(&parent, &density, limits, &mut arithmetic()).unwrap();
    let AdmittedFamilyDescriptor::Kernel(kernel) =
        restore_family_source(AdmittedFamilyDescriptor::Kernel(kernel))
    else {
        panic!("imported family channel");
    };
    let joint = kernel.close(&prior, limits, &mut arithmetic()).unwrap();
    let first = joint.space().coordinate(0, &()).unwrap();
    let next = joint.space().coordinate(3, &()).unwrap();
    let partition = EventPartition::on(
        &joint.space().full(),
        &[first.clone(), first.complement()],
        PartitionLimits::default(),
        &(),
    )
    .unwrap();
    let half = ParameterFunction::new(
        domain.clone(),
        &[value(
            ExactPolynomial::one(),
            ExactPolynomial::constant(2u64.into()),
        )],
        limits.parameters.region,
        limits.functions,
        &mut arithmetic(),
    )
    .unwrap();
    let receipt = joint
        .space()
        .parameter_jeffrey(
            SpaceId([105; 32]),
            &partition,
            &[half.clone(), half],
            limits,
            &mut arithmetic(),
        )
        .unwrap();
    let AdmittedFamilyDescriptor::Jeffrey(receipt) =
        restore_family_source(AdmittedFamilyDescriptor::Jeffrey(receipt))
    else {
        panic!("imported family revision");
    };
    let revised = receipt.revised().unwrap();
    let cases = [
        (
            joint.parent().clone(),
            prior.coordinate(0, &()).unwrap(),
            next.clone(),
            false,
        ),
        (
            revised.translation().clone(),
            revised
                .restriction()
                .refinement()
                .lift(&first, &())
                .unwrap(),
            revised.pullback(&next, &()).unwrap(),
            true,
        ),
    ];
    drop((
        prior, raw, density, parent, kernel, joint, first, next, partition, receipt,
    ));
    for (map, old, next, replaced) in cases {
        check_family_dynamics_map(map, old, next, replaced);
    }
}

fn restore_family_source(
    source: bumbledb::event::AdmittedFamilyDescriptor,
) -> bumbledb::event::AdmittedFamilyDescriptor {
    use bumbledb::event::{AdmittedSourceDescriptor, SourceDescriptor, SourceDescriptorLimits};
    let limits = SourceDescriptorLimits::default();
    let source = AdmittedSourceDescriptor::Family(Box::new(source));
    let data = SourceDescriptor::capture(&source, limits, &mut arithmetic()).unwrap();
    let bytes = data.to_bytes(limits, &()).unwrap();
    drop((source, data));
    let AdmittedSourceDescriptor::Family(source) =
        SourceDescriptor::import(&bytes, limits, &mut arithmetic()).unwrap()
    else {
        panic!("imported family source");
    };
    *source
}

fn check_family_dynamics_map(
    map: bumbledb::event::CoordinateMap,
    old: bumbledb::Event,
    next: bumbledb::Event,
    replaced: bool,
) {
    use bumbledb::{
        EventImport,
        event::{AdmittedDescriptor, DescriptorLimits, ParameterSourceLimits},
        query,
    };
    let limits = ParameterSourceLimits::default();
    let dir = common::TempDir::new("event-family-dynamics");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let import = EventImport::capture(
        &AdmittedDescriptor::Map(map),
        DescriptorLimits::default(),
        &(),
    )
    .unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            condition: old.clone(),
        }])?;
        tx.insert([&Observation {
            id: 1,
            condition: next.clone(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop((old, next, db));
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let query = query!(SourceSchema {
        use map transition = &import;
        (old: Event(Pullback(old, transition)), both: Event(Pullback(old, transition) & next)) |
            Region(id: id, condition: old), Observation(id: id, condition: next);
    });
    drop(import);
    let mut results = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        results.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((query, db));
    for answers in results {
        assert_eq!(answers.len(), 1);
        let (AnswerValue::Event(old), AnswerValue::Event(both)) =
            (answers.get(0, 0), answers.get(0, 1))
        else {
            panic!("owned dynamics");
        };
        assert!(!old.complement().is_empty());
        for n in 0..=4 {
            let expected = if replaced {
                if n == 0 || n == 4 {
                    None
                } else {
                    Some((ratio(1, 2), ratio(n, 8)))
                }
            } else {
                Some((ratio(n, 4), ratio(n * n, 16)))
            };
            for (event, value) in [
                (old, expected.as_ref().map(|v| v.0.clone())),
                (both, expected.as_ref().map(|v| v.1.clone())),
            ] {
                assert_eq!(
                    event
                        .parameter_mass(limits, &mut arithmetic())
                        .unwrap()
                        .value_at(
                            &ratio(n, 4),
                            limits.parameters.region,
                            limits.functions,
                            &mut arithmetic()
                        )
                        .unwrap(),
                    value
                );
            }
        }
    }
}

#[test]
fn probability_heads_retain_exact_family_functions_across_reopen_and_both_join_paths() {
    use bumbledb::{ProbabilityValue, query};
    let dir = common::TempDir::new("probability-family-query");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let source = parameterized_draws();
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            condition: source.coordinate(0, &())?,
        }])?;
        tx.insert([&Observation {
            id: 1,
            condition: source.coordinate(1, &())?,
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop((source, db));
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let template = query!(SourceSchema {
        interior regions(source_id, joint: Event(a & b), evidence: Event(a)) |
            Region(id: source_id, condition: a), Observation(id: source_id, condition: b);
        (chance: Probability(joint, evidence), self_chance: Probability(evidence, evidence)) |
            regions(source_id, joint, evidence);
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
    drop((template, db));
    for answers in retained {
        assert_eq!(answers.len(), 1);
        for column in 0..2 {
            let AnswerValue::Probability(answer) = answers.get(0, column) else {
                panic!("owned probability")
            };
            let ProbabilityValue::Parameter(observation) = answer.value() else {
                panic!("exact function")
            };
            assert!(!answer.is_impossible());
            let limits = bumbledb::event::ParameterSourceLimits::default();
            assert!(
                observation
                    .value_at(&ratio(0, 1), limits, &mut arithmetic())
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                observation
                    .value_at(&ratio(1, 3), limits, &mut arithmetic())
                    .unwrap()
                    .unwrap(),
                if column == 0 {
                    ratio(1, 3)
                } else {
                    ratio(1, 1)
                }
            );
            assert_eq!(
                observation
                    .evidence_mass()
                    .value_at(
                        &ratio(1, 3),
                        limits.parameters.region,
                        limits.functions,
                        &mut arithmetic()
                    )
                    .unwrap()
                    .unwrap(),
                ratio(1, 3)
            );
            assert_eq!(
                answer.event().to_bytes(&()).unwrap(),
                observation.event().to_bytes(&()).unwrap()
            );
        }
    }
}

#[test]
fn probability_heads_match_all_finite_region_pairs_including_nonempty_zero_evidence() {
    use bumbledb::{ProbabilityValue, query};
    let dir = common::TempDir::new("probability-finite-query");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let base = Space::new(SpaceId([183; 32]), 2, &()).unwrap();
    let pieces = (0..4)
        .map(|world| DensityPiece {
            region: base.table(3, &[1 << world], &()).unwrap(),
            density: ratio(world, 6),
        })
        .collect::<Vec<_>>();
    let source = base
        .with_density(&pieces, LawLimits::default(), &mut arithmetic())
        .unwrap();
    db.write(common::work(), |tx| {
        for mask in 0..16 {
            tx.insert([&Region {
                id: mask,
                condition: source.table(3, &[mask], &())?,
            }])?;
            tx.insert([&Observation {
                id: mask,
                condition: source.table(3, &[mask], &())?,
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let query = query!(SourceSchema {
        (left, right, chance: Probability(a, b), opposite: Probability(!a, b)) |
            Region(id: left, condition: a), Observation(id: right, condition: b);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 256);
        let mass = |mask: u64| (0..4).filter(|bit| mask & (1 << bit) != 0).sum::<u64>();
        for row in 0..answers.len() {
            let (AnswerValue::U64(a), AnswerValue::U64(b)) =
                (answers.get(row, 0), answers.get(row, 1))
            else {
                panic!("ids")
            };
            for column in 2..4 {
                let AnswerValue::Probability(answer) = answers.get(row, column) else {
                    panic!("probability")
                };
                let ProbabilityValue::Fixed { observation, value } = answer.value() else {
                    panic!("fixed law")
                };
                let numerator = mass(if column == 2 { a & b } else { !a & b });
                assert_eq!(observation.numerator(), &ratio(numerator, 6));
                assert_eq!(observation.evidence_mass(), &ratio(mass(b), 6));
                assert_eq!(
                    value.as_ref(),
                    (mass(b) != 0).then(|| ratio(numerator, mass(b))).as_ref()
                );
                assert_eq!(answer.is_impossible(), mass(b) == 0);
                assert_eq!(answer.given().is_empty(), b == 0);
            }
        }
    }
}

#[test]
fn probability_admits_participating_operands_and_reuses_after_atomic_failure() {
    use bumbledb::{Answers, Error, query};
    let dir = common::TempDir::new("probability-participation");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let measured = integrated_two_draws(true);
    let foreign = Space::new(SpaceId([184; 32]), 1, &()).unwrap();
    let left = Region {
        id: 1,
        condition: measured.full(),
    };
    let wrong = Observation {
        id: 1,
        condition: foreign.empty(),
    };
    db.write(common::work(), |tx| {
        tx.insert([&left])?;
        tx.insert([&wrong])?;
        // No matching Region: this unrelated source never participates.
        tx.insert([&Observation {
            id: 9,
            condition: foreign.full(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(SourceSchema {
        (chance: Probability(Empty(a), b)) |
            Region(id: id, condition: a), Observation(id: id, condition: b);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let mut answers = Answers::new();
    for fallback in [false, true] {
        prepared.force_cursor_fallback(fallback);
        let failure = db
            .read(common::work(), |snapshot| {
                snapshot.execute(&mut prepared, &[] as &[BindValue], &mut answers)
            })
            .unwrap_err();
        let Error::EventFaults(faults) = failure else {
            panic!("context fault: {failure:?}")
        };
        assert_eq!(faults.len(), 1);
        assert_eq!(faults[0].operand, 1);
        assert!(answers.is_empty());
    }
    db.write(common::work(), |tx| {
        tx.delete([&wrong])?;
        tx.insert([&Observation {
            id: 1,
            condition: measured.full(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    db.read(common::work(), |snapshot| {
        snapshot.execute(&mut prepared, &[] as &[BindValue], &mut answers)
    })
    .unwrap();
    assert_eq!(answers.len(), 1);
    let AnswerValue::Probability(value) = answers.get(0, 0) else {
        panic!("observation")
    };
    assert!(!value.is_impossible());
    assert!(value.event().is_empty());
    // An unused lawless field is fine; writing it as evidence is not.
    let unused = query!(SourceSchema {
        (chance: Probability(a, a)) | Region(condition: a), Observation(condition: _);
    });
    let mut prepared = db.prepare(&unused, common::work()).unwrap();
    assert_eq!(
        db.read(common::work(), |snapshot| snapshot
            .execute_collect(&mut prepared, &[] as &[BindValue]))
            .unwrap()
            .len(),
        1
    );
    let missing = query!(SourceSchema {
        (chance: Probability(Empty(a), Full(a))) | Observation(id == 9, condition: a);
    });
    let mut prepared = db.prepare(&missing, common::work()).unwrap();
    assert!(matches!(
        db.read(common::work(), |snapshot| snapshot.execute(
            &mut prepared,
            &[] as &[BindValue],
            &mut answers
        )),
        Err(Error::Event(EventError::MissingLaw))
    ));
    assert!(answers.is_empty());
}

#[test]
fn probability_pair_identity_survives_duplicate_arms_and_numerical_equality() {
    use bumbledb::query;
    let dir = common::TempDir::new("probability-pair-identity");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let source = integrated_two_draws(true);
    db.write(common::work(), |tx| {
        for id in 0..3 {
            tx.insert([&Region {
                id,
                condition: source.coordinate((id % 2) as u8, &())?,
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(SourceSchema {
        (chance: Probability(a, Full(a))) | Region(condition: a);
        (chance: Probability(a, Full(a))) | Region(condition: a);
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(
            answers.len(),
            2,
            "same pair deduplicates, different events do not"
        );
        assert_ne!(answers.get(0, 0), answers.get(1, 0));
        for row in 0..2 {
            let AnswerValue::Probability(value) = answers.get(row, 0) else {
                panic!("probability")
            };
            let bumbledb::ProbabilityValue::Fixed { value, .. } = value.value() else {
                panic!("fixed")
            };
            assert_eq!(value, &Some(ratio(1, 2)));
        }
    }
}

#[test]
fn probability_groups_by_source_pairs_before_counting_bindings() {
    use bumbledb::query;
    let dir = common::TempDir::new("probability-group-key");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let source = integrated_two_draws(true);
    db.write(common::work(), |tx| {
        for id in 0..3 {
            tx.insert([&Region {
                id,
                condition: source.coordinate((id % 2) as u8, &())?,
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(SourceSchema {
        (chance: Probability(a, Full(a)), paths: Count) | Region(id: path, condition: a);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    for fallback in [false, true] {
        prepared.force_cursor_fallback(fallback);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 2);
        let mut counts = (0..answers.len())
            .map(|row| {
                assert!(matches!(answers.get(row, 0), AnswerValue::Probability(_)));
                let AnswerValue::U64(count) = answers.get(row, 1) else {
                    panic!("count")
                };
                count
            })
            .collect::<Vec<_>>();
        counts.sort_unstable();
        assert_eq!(counts, [1, 2]);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one owned-family lifecycle and exact-value oracle"
)]
fn expectation_query_retains_shared_parameter_and_excluded_zero_after_owner_release() {
    use bumbledb::{ExpectationValue, query};
    let dir = common::TempDir::new("expectation-family-query");
    let db = Db::create(dir.path(), SourceSchema, common::work())
        .unwrap()
        .unwrap();
    let source = parameterized_draws();
    let first = source.coordinate(0, &()).unwrap();
    let second = source.coordinate(1, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([
            &Region {
                id: 8,
                condition: second.clone(),
            },
            &Region {
                id: 2,
                condition: second.complement(),
            },
        ])?;
        tx.insert([&Observation {
            id: 1,
            condition: first.clone(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop((db, source));
    let db = Db::open(dir.path(), SourceSchema, common::work()).unwrap();
    let template = query!(SourceSchema {
        (expected: Expectation(value, region, evidence)) |
            Region(id: value, condition: region), Observation(id == 1, condition: evidence);
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
    drop((db, template));
    for answers in retained {
        assert_eq!(answers.len(), 1);
        let AnswerValue::Expectation(answer) = answers.get(0, 0) else {
            panic!("expectation")
        };
        let ExpectationValue::Parameter(observation) = answer.value() else {
            panic!("family")
        };
        let limits = bumbledb::event::ParameterSourceLimits::default();
        assert_eq!(
            observation
                .value_at(&ratio(0, 1), limits, &mut arithmetic())
                .unwrap(),
            None
        );
        assert_eq!(
            observation
                .value_at(&ratio(1, 3), limits, &mut arithmetic())
                .unwrap(),
            Some(ratio(4, 1))
        );
        assert_eq!(
            observation
                .value_at(&ratio(1, 1), limits, &mut arithmetic())
                .unwrap(),
            Some(ratio(8, 1))
        );
        assert_eq!(
            observation
                .numerator()
                .value_at(
                    &ratio(1, 3),
                    limits.parameters.region,
                    limits.functions,
                    &mut arithmetic()
                )
                .unwrap(),
            Some(ratio(4, 3))
        );
        assert_eq!(
            observation
                .evidence_mass()
                .value_at(
                    &ratio(1, 3),
                    limits.parameters.region,
                    limits.functions,
                    &mut arithmetic()
                )
                .unwrap(),
            Some(ratio(1, 3))
        );
        assert_eq!(
            answer.given().to_bytes(&()).unwrap(),
            first.to_bytes(&()).unwrap()
        );
    }
}
