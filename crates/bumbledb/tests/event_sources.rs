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
