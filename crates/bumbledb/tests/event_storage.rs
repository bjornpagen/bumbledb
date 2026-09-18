use bumbledb::{
    AnswerValue, BindValue, Db, Event, Fact, Value, WorkContext,
    event::{BoolOp4, Limits, Space, SpaceId},
    ir::{Atom, AtomSource, FindTerm, ParamId, Query, Rule, Term, VarId},
    schema::FieldId,
};

mod common;

bumbledb::schema! {
    pub EventSchema;
    relation Region { id: u64, condition: event }
    relation Observation { id: u64, condition: event }
    Region(id) -> Region;
    Observation(id) -> Observation;
}

fn space(reverse: bool) -> Space {
    let order = if reverse { [2, 1, 0] } else { [0, 1, 2] };
    Space::with_order(SpaceId([42; 32]), &order, Limits::default(), &()).unwrap()
}

fn query(bindings: Vec<(FieldId, Term)>, finds: Vec<FindTerm>) -> Query {
    Query::single(Rule {
        finds,
        atoms: vec![Atom {
            source: AtomSource::Edb(Region::RELATION),
            bindings,
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn all() -> Query {
    query(
        vec![(FieldId(1), Term::Var(VarId(0)))],
        vec![FindTerm::Var(VarId(0))],
    )
}

fn bytes(event: &Event) -> Vec<u8> {
    event.to_bytes(&()).unwrap()
}

#[test]
fn canonical_facts_deduplicate_reopen_and_retain_owned_results() {
    let dir = common::TempDir::new("event-owned-storage");
    let db = Db::create(dir.path(), EventSchema, common::work())
        .unwrap()
        .unwrap();
    let first = space(false).coordinate(0, &()).unwrap();
    let other = space(true).coordinate(0, &()).unwrap();
    assert_ne!(first, other, "scoped handles differ before alignment");
    assert_eq!(bytes(&first), bytes(&other));
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
                condition: other.clone()
            }])?
            .changed(),
            0
        );
        let read = tx.get(RegionById { id: 1 })?.unwrap();
        assert_eq!(bytes(&read.condition), bytes(&first));
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(db);

    let db = Db::open(dir.path(), EventSchema, common::work()).unwrap();
    let mut prepared = db.prepare(&all(), common::work()).unwrap();
    let answers = db
        .read(common::work(), |snapshot| {
            let facts: Vec<Region> = snapshot.scan_facts()?.collect::<bumbledb::Result<_>>()?;
            assert_eq!(facts.len(), 1);
            assert_eq!(bytes(&facts[0].condition), bytes(&first));
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    drop(prepared);
    drop(db);
    let AnswerValue::Event(retained) = answers.get(0, 0) else {
        panic!("owned Event output")
    };
    assert_eq!(bytes(retained), bytes(&first));
    assert_eq!(retained.complement().count(&()).unwrap(), 4);
}

#[test]
fn free_join_uses_shared_semantic_identity_on_resident_and_cursor_paths() {
    let dir = common::TempDir::new("event-free-join");
    let db = Db::create(dir.path(), EventSchema, common::work())
        .unwrap()
        .unwrap();
    let a = space(false);
    let b = space(true);
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            condition: a.coordinate(0, &())?,
        }])?;
        tx.insert([&Region {
            id: 2,
            condition: a.coordinate(1, &())?,
        }])?;
        tx.insert([&Region {
            id: 3,
            condition: a.empty(),
        }])?;
        tx.insert([&Region {
            id: 4,
            condition: a.full(),
        }])?;
        tx.insert([&Observation {
            id: 10,
            condition: b.coordinate(0, &())?,
        }])?;
        tx.insert([&Observation {
            id: 20,
            condition: b.empty(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let mut joined = query(
        vec![
            (FieldId(0), Term::Var(VarId(1))),
            (FieldId(1), Term::Var(VarId(0))),
        ],
        vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(0))],
    );
    joined.rules[0].atoms.push(Atom {
        source: AtomSource::Edb(Observation::RELATION),
        bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
    });
    for fallback in [false, true] {
        let mut prepared = db.prepare(&joined, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        db.read(common::work(), |snapshot| {
            let answers = snapshot.execute_collect(&mut prepared, &[] as &[BindValue])?;
            let mut ids: Vec<_> = (0..answers.len())
                .map(|row| {
                    let AnswerValue::U64(id) = answers.get(row, 0) else {
                        panic!("id")
                    };
                    let AnswerValue::Event(event) = answers.get(row, 1) else {
                        panic!("Event")
                    };
                    assert_eq!(event.count(&())?, if id == 1 { 4 } else { 0 });
                    Ok(id)
                })
                .collect::<bumbledb::Result<_>>()?;
            ids.sort_unstable();
            assert_eq!(
                ids,
                [1, 3],
                "equal masses are not Event equality; empty is a value"
            );
            Ok(())
        })
        .unwrap();
    }
}

#[test]
fn independently_owned_literals_and_parameters_match_canonical_values() {
    let dir = common::TempDir::new("event-bindings");
    let db = Db::create(dir.path(), EventSchema, common::work())
        .unwrap()
        .unwrap();
    let a = space(false).coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            condition: a,
        }])
    })
    .unwrap()
    .unwrap();
    let b = space(true).coordinate(0, &()).unwrap();
    for fallback in [false, true] {
        let q = query(
            vec![(FieldId(1), Term::Param(ParamId(0)))],
            vec![FindTerm::Count],
        );
        let mut prepared = db.prepare(&q, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        db.read(common::work(), |snapshot| {
            let answers = snapshot.execute_collect(&mut prepared, &[BindValue::Event(&b)])?;
            assert_eq!(answers.get(0, 0), AnswerValue::U64(1));
            Ok(())
        })
        .unwrap();
        db.clear_cache();
        db.read(common::work(), |snapshot| {
            let answers = snapshot.execute_collect(&mut prepared, &[BindValue::Event(&b)])?;
            assert_eq!(answers.get(0, 0), AnswerValue::U64(1));
            Ok(())
        })
        .unwrap();
        let q = query(
            vec![(FieldId(1), Term::Literal(Value::Event(b.clone())))],
            vec![FindTerm::Count],
        );
        let mut prepared = db.prepare(&q, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        db.read(common::work(), |snapshot| {
            let answers = snapshot.execute_collect(&mut prepared, &[] as &[BindValue])?;
            assert_eq!(answers.get(0, 0), AnswerValue::U64(1));
            Ok(())
        })
        .unwrap();
    }
}

#[test]
fn rollback_and_concurrent_publication_preserve_whole_fact_identity() {
    let dir = common::TempDir::new("event-publication");
    let db = Db::create(dir.path(), EventSchema, common::work())
        .unwrap()
        .unwrap();
    let failed = db.write(common::work(), |tx| -> bumbledb::Result<()> {
        tx.insert([&Region {
            id: 99,
            condition: space(false).full(),
        }])?;
        Err(bumbledb::event::Error::Cancelled.into())
    });
    assert!(failed.is_err());
    std::thread::scope(|scope| {
        for reverse in [false, true, false, true] {
            let db = &db;
            scope.spawn(move || {
                let condition = space(reverse).coordinate(0, &()).unwrap();
                db.write(common::work(), |tx| {
                    tx.insert([&Region { id: 1, condition }])
                })
                .unwrap()
                .unwrap();
            });
        }
    });
    db.read(common::work(), |snapshot| {
        let facts: Vec<Region> = snapshot.scan_facts()?.collect::<bumbledb::Result<_>>()?;
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].id, 1);
        assert_eq!(facts[0].condition.count(&())?, 4);
        Ok(())
    })
    .unwrap();
}

#[test]
fn event_sets_collapse_semantic_duplicates_and_keep_equal_mass_regions_distinct() {
    let dir = common::TempDir::new("event-sets");
    let db = Db::create(dir.path(), EventSchema, common::work())
        .unwrap()
        .unwrap();
    let a = space(false);
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            condition: a.coordinate(0, &())?,
        }])?;
        tx.insert([&Region {
            id: 2,
            condition: a.coordinate(1, &())?,
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let q = query(
        vec![(FieldId(1), Term::ParamSet(ParamId(0)))],
        vec![FindTerm::Count],
    );
    let values = [
        Value::Event(a.coordinate(0, &()).unwrap()),
        Value::Event(space(true).coordinate(0, &()).unwrap()),
    ];
    for fallback in [false, true] {
        let mut prepared = db.prepare(&q, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        for _ in 0..2 {
            db.clear_cache();
            db.read(common::work(), |snapshot| {
                let answers =
                    snapshot.execute_collect(&mut prepared, &[bumbledb::ParamArg::Set(&values)])?;
                assert_eq!(answers.get(0, 0), AnswerValue::U64(1));
                Ok(())
            })
            .unwrap();
        }
    }
}

#[test]
fn event_runtime_keys_never_supply_a_public_order() {
    use bumbledb::ir::{CmpOp, Comparison};
    let dir = common::TempDir::new("event-order");
    let db = Db::create(dir.path(), EventSchema, common::work())
        .unwrap()
        .unwrap();
    let mut q = all();
    q.rules[0]
        .conditions
        .push(bumbledb::ConditionTree::Leaf(Comparison {
            lhs: Term::Var(VarId(0)),
            op: CmpOp::Lt,
            rhs: Term::Literal(Value::Event(space(false).full())),
        }));
    assert!(matches!(
        db.prepare(&q, common::work()),
        Err(bumbledb::Error::Validation(
            bumbledb::ValidationError::OrderComparisonOnEvent { .. }
        ))
    ));
}

#[test]
fn canonical_event_corruption_and_cancellation_are_not_nonmatches() {
    use bumbledb::canonical::{CanonicalRow, RowError};
    use bumbledb::schema::{FieldDescriptor, ValueType};
    let fields = [FieldDescriptor {
        name: "e".into(),
        value_type: ValueType::Event,
    }];
    let a = space(false);
    for value in [a.empty(), a.full(), a.coordinate(0, &()).unwrap()] {
        let row = CanonicalRow::encode(&fields, &[Value::Event(value)], &common::work()).unwrap();
        // u16 arity, Event tag, u64 length, four-byte family, version byte.
        let mut corrupt = row.as_bytes().to_vec();
        corrupt[15] = 255;
        assert!(matches!(
            CanonicalRow::parse(&fields, &corrupt, &common::work()),
            Err(RowError::Event { .. })
        ));
        for end in 1..row.len() {
            assert!(CanonicalRow::parse(&fields, &row[..end], &common::work()).is_err());
        }
    }
    let work = WorkContext::new();
    work.cancel();
    assert!(matches!(
        CanonicalRow::encode(&fields, &[Value::Event(a.full())], &work),
        Err(RowError::Work(_))
    ));
    // Shape operations remain available on values returned without a database.
    let x = a.coordinate(0, &()).unwrap();
    assert_eq!(
        x.apply(BoolOp4::OR, &x.complement(), &()).unwrap(),
        a.full()
    );
}

#[test]
fn version_one_row_fixture_pins_the_graph_and_field_envelope() {
    use bumbledb::canonical::CanonicalRow;
    use bumbledb::schema::{FieldDescriptor, ValueType};
    let hex = include_str!("fixtures/event-v1-row.hex").trim();
    let fixture: Vec<u8> = hex
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let fields = [FieldDescriptor {
        name: "e".into(),
        value_type: ValueType::Event,
    }];
    let source = Space::new(SpaceId([42; 32]), 2, &()).unwrap();
    let row = CanonicalRow::encode(
        &fields,
        &[Value::Event(source.coordinate(0, &()).unwrap())],
        &common::work(),
    )
    .unwrap();
    assert_eq!(row.as_bytes(), fixture);
    let parsed = CanonicalRow::parse(&fields, &fixture, &common::work()).unwrap();
    let mut reader = bumbledb::RowReader::new(parsed.as_bytes()).unwrap();
    let event = reader.next_event().unwrap();
    for world in 0..4 {
        assert_eq!(event.contains(world).unwrap(), world & 1 == 1);
    }

    let work = common::work();
    let mut reader = bumbledb::RowReader::with_work(&fixture, &work).unwrap();
    work.cancel();
    assert!(reader.next_event().is_err());
}

#[test]
fn one_decoded_row_aligns_its_event_fields_and_keeps_foreign_sources_separate() {
    use bumbledb::canonical::{CanonicalRow, decode};
    use bumbledb::schema::{FieldDescriptor, ValueType};

    let fields: Vec<_> = ["left", "right", "foreign"]
        .into_iter()
        .map(|name| FieldDescriptor {
            name: name.into(),
            value_type: ValueType::Event,
        })
        .collect();
    let left = space(false).coordinate(0, &()).unwrap();
    let right = space(true).coordinate(1, &()).unwrap();
    let foreign = Space::new(SpaceId([43; 32]), 3, &()).unwrap().empty();
    let work = common::work();
    let row = CanonicalRow::encode(
        &fields,
        &[
            Value::Event(left),
            Value::Event(right),
            Value::Event(foreign),
        ],
        &work,
    )
    .unwrap();

    let mut reader = bumbledb::RowReader::with_work(row.as_bytes(), &work).unwrap();
    let a = reader.next_event().unwrap();
    // Cloning a partially consumed reader retains its existing namespace.
    let mut cloned = reader.clone();
    let b = reader.next_event().unwrap();
    let c = reader.next_event().unwrap();
    reader.finish().unwrap();
    assert_eq!(b, cloned.next_event().unwrap());
    let decoded = decode(&fields, row.as_bytes(), &work).unwrap();
    drop(row);

    let check = |a: &Event, b: &Event, foreign: &Event| {
        let intersection = a.apply(BoolOp4::AND, b, &()).unwrap();
        assert_eq!(intersection.count(&()).unwrap(), 2);
        for world in 0..8 {
            assert_eq!(intersection.contains(world).unwrap(), world & 3 == 3);
        }
        assert!(a.apply(BoolOp4::AND, foreign, &()).is_err());
        assert!(foreign.apply(BoolOp4::AND, a, &()).is_err());
    };
    check(&a, &b, &c);
    let [Value::Event(a), Value::Event(b), Value::Event(c)] = decoded.values() else {
        panic!("three owned Event fields")
    };
    check(a, b, c);
}

#[test]
fn event_pages_own_values_and_cancelled_delivery_does_not_advance() {
    let dir = common::TempDir::new("event-pages");
    let db = Db::create(dir.path(), EventSchema, common::work())
        .unwrap()
        .unwrap();
    let source = space(false);
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            condition: source.full(),
        }])?;
        tx.insert([&Region {
            id: 2,
            condition: source.empty(),
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let mut prepared = db.prepare(&all(), common::work()).unwrap();
    let complete = db
        .read(common::work(), |snapshot| {
            prepared.execute_complete(snapshot, &[] as &[BindValue])
        })
        .unwrap();
    drop(prepared);
    drop(db);
    drop(source);
    let mut cursor = complete.into_cursor(1);
    let stopped = common::work();
    stopped.cancel();
    assert!(cursor.next_page(&stopped).is_err());
    let mut counts = Vec::new();
    while let Some(page) = cursor.next_page(&common::work()).unwrap() {
        let AnswerValue::Event(value) = page.rows.get(0, 0) else {
            panic!("Event page")
        };
        counts.push(value.count(&()).unwrap());
    }
    counts.sort_unstable();
    assert_eq!(counts, [0, 8]);
}
