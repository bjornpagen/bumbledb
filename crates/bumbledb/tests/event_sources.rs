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
