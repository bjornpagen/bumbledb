use bumbledb::{
    AnswerValue, BindValue, Db, Fact, WorkContext,
    event::{CoordinateMap, Error, Limits, Space, SpaceId},
    ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId},
    schema::FieldId,
};

mod common;

bumbledb::schema! {
    pub MapSchema;
    relation Mapped { id: u64, condition: event }
    relation Expected { id: u64, condition: event }
    Mapped(id) -> Mapped;
    Expected(id) -> Expected;
}

#[test]
fn map_results_persist_join_and_outlive_maps_and_database() {
    let dir = common::TempDir::new("event-map-results");
    let db = Db::create(dir.path(), MapSchema, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([1; 32]), 1, &()).unwrap();
    let target = Space::new(SpaceId([2; 32]), 2, &()).unwrap();
    let map = CoordinateMap::coordinates(&source, &target, &[0, 0], &()).unwrap();
    let coordinate = source.coordinate(0, &()).unwrap();
    let inputs = [
        source.empty(),
        source.full(),
        coordinate.clone(),
        coordinate.complement(),
    ];
    let expected = Space::with_order(SpaceId([2; 32]), &[1, 0], Limits::default(), &()).unwrap();
    let masks = [0, 9, 8, 1];
    db.write(common::work(), |tx| {
        for (index, input) in inputs.iter().enumerate() {
            tx.insert([&Mapped {
                id: index as u64,
                condition: map.image(input, &common::work())?,
            }])?;
            tx.insert([&Expected {
                id: index as u64,
                condition: expected.table(3, &[masks[index]], &())?,
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop((map, source, target, expected, inputs, coordinate, db));

    let db = Db::open(dir.path(), MapSchema, common::work()).unwrap();
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(Mapped::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
            Atom {
                source: AtomSource::Edb(Expected::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 4);
        for row in 0..answers.len() {
            assert_eq!(answers.get(row, 0), answers.get(row, 1));
            let AnswerValue::U64(id) = answers.get(row, 0) else {
                panic!("id")
            };
            let AnswerValue::Event(event) = answers.get(row, 2) else {
                panic!("Event")
            };
            let mask = masks[usize::try_from(id).unwrap()];
            for world in 0..4 {
                assert_eq!(event.contains(world).unwrap(), mask & (1 << world) != 0);
            }
        }
        retained.push(answers);
    }
    drop(db);
    for answers in retained {
        for row in 0..answers.len() {
            let AnswerValue::Event(event) = answers.get(row, 2) else {
                panic!("Event")
            };
            let identity = CoordinateMap::identity(&event.space(), &()).unwrap();
            assert_eq!(identity.image(event, &()).unwrap(), *event);
        }
    }
}

#[test]
fn native_cancellation_applies_to_map_admission_and_every_image_mode() {
    let source = Space::new(SpaceId([1; 32]), 2, &()).unwrap();
    let target = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
    let map = CoordinateMap::coordinates(&source, &target, &[0], &()).unwrap();
    let work = WorkContext::new();
    work.cancel();
    assert!(matches!(
        CoordinateMap::identity(&source, &work),
        Err(Error::Cancelled)
    ));
    assert_eq!(map.image(&source.empty(), &work), Err(Error::Cancelled));
    assert_eq!(map.pullback(&target.full(), &work), Err(Error::Cancelled));
    assert_eq!(
        map.universal_image(&source.full(), &work),
        Err(Error::Cancelled)
    );
    assert_eq!(
        map.nonvacuous_image(&source.empty(), &work),
        Err(Error::Cancelled)
    );
    assert!(matches!(
        map.certify_surjective(&work),
        Err(Error::Cancelled)
    ));
}
