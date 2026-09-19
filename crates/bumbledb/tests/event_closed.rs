//! Ground Events retain portable schema identity and bind into the same query
//! namespace as ordinary facts, literals and parameters.
use bumbledb::{
    AnswerValue, BindValue, Db, InstanceBuilder, Value,
    event::{Limits, Space, SpaceId},
    ir::{Atom, AtomSource, FindTerm, ParamId, Query, Rule, Term, VarId},
    schema::{
        FieldDescriptor, FieldId, RelationDescriptor, RelationId, Row, SchemaDescriptor, Side,
        StatementDescriptor, ValueType,
    },
};

mod common;

fn space(reverse: bool) -> Space {
    Space::with_order(
        SpaceId([127; 32]),
        if reverse { &[1, 0] } else { &[0, 1] },
        Limits::default(),
        &(),
    )
    .unwrap()
}
fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
    }
}
fn side(relation: u32, fields: &[u16]) -> Side {
    Side {
        relation: RelationId(relation),
        projection: fields
            .iter()
            .copied()
            .map(FieldId)
            .collect::<Vec<_>>()
            .into(),
        selection: Box::new([]),
    }
}
fn key(relation: u32, fields: &[u16]) -> StatementDescriptor {
    StatementDescriptor::Functionality {
        relation: RelationId(relation),
        projection: side(relation, fields).projection,
    }
}
fn schema(reverse: bool) -> SchemaDescriptor {
    let base = space(reverse);
    let a = base.coordinate(0, &()).unwrap();
    let aliases = space(!reverse);
    let values = [a.clone(), a.complement(), base.empty()];
    let alias = [
        aliases.coordinate(0, &()).unwrap(),
        aliases.full(),
        aliases.empty(),
    ];
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Catalog".into(),
                fields: vec![
                    field("group", ValueType::U64),
                    field("when", ValueType::Event),
                    field("alias", ValueType::Event),
                    field("tag", ValueType::U64),
                ],
                extension: Some(
                    values
                        .into_iter()
                        .zip(alias)
                        .enumerate()
                        .map(|(i, (value, alias))| Row {
                            handle: format!("Row{i}").into(),
                            values: vec![
                                Value::U64(7),
                                Value::Event(value),
                                Value::Event(alias),
                                Value::U64(11),
                            ]
                            .into_boxed_slice(),
                        })
                        .collect(),
                ),
            },
            RelationDescriptor {
                name: "Observation".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("group", ValueType::U64),
                    field("when", ValueType::Event),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "Gate".into(),
                fields: vec![field("id", ValueType::U64)],
                extension: None,
            },
        ],
        statements: vec![key(0, &[1, 2]), key(1, &[0]), key(2, &[0])],
    }
}
fn atom(relation: u32, bindings: &[(u16, Term)]) -> Atom {
    Atom {
        source: AtomSource::Edb(RelationId(relation)),
        bindings: bindings
            .iter()
            .map(|(field, term)| (FieldId(*field), term.clone()))
            .collect(),
    }
}
fn query(atoms: Vec<Atom>, finds: Vec<FindTerm>) -> Query {
    Query::single(Rule {
        finds,
        atoms,
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn closed_events_join_across_owners_caches_cursors_and_reopen() {
    let dir = common::TempDir::new("event-closed-query");
    let db = Db::create(dir.path(), schema(false), common::work())
        .unwrap()
        .unwrap();
    let base = space(true);
    let a = base.coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert_dyn(
            RelationId(1),
            [
                vec![Value::U64(1), Value::U64(7), Value::Event(a.clone())],
                vec![Value::U64(2), Value::U64(7), Value::Event(base.empty())],
            ],
        )?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(db);
    let db = Db::open(dir.path(), schema(true), common::work()).unwrap();
    db.read(common::work(), |snapshot| {
        let rows = snapshot
            .scan(RelationId(0))?
            .collect::<bumbledb::Result<Vec<_>>>()?;
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0][4], Value::U64(11));
        let Value::Event(event) = &rows[0][2] else {
            panic!("Event")
        };
        assert_eq!(event.to_bytes(&())?, a.to_bytes(&())?);
        Ok(())
    })
    .unwrap();
    let join = query(
        vec![
            atom(0, &[(0, Term::Var(VarId(0))), (2, Term::Var(VarId(1)))]),
            atom(1, &[(2, Term::Var(VarId(1)))]),
        ],
        vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
    );
    let mut retained = None;
    for fallback in [false, true] {
        let mut prepared = db.prepare(&join, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        for _ in 0..2 {
            let answers = db
                .read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            assert_eq!(answers.len(), 2);
            let mut ids = (0..answers.len())
                .map(|i| {
                    let AnswerValue::U64(id) = answers.get(i, 0) else {
                        panic!("id")
                    };
                    let AnswerValue::Event(value) = answers.get(i, 1) else {
                        panic!("Event")
                    };
                    assert_eq!(value.count(&()).unwrap(), if id == 0 { 2 } else { 0 });
                    id
                })
                .collect::<Vec<_>>();
            ids.sort_unstable();
            assert_eq!(ids, [0, 2]);
            retained = Some(answers);
            db.clear_cache();
        }
    }
    drop(db);
    let answers = retained.unwrap();
    for i in 0..answers.len() {
        let AnswerValue::Event(value) = answers.get(i, 1) else {
            panic!("owned output")
        };
        assert_eq!(
            value.count(&()).unwrap() + value.complement().count(&()).unwrap(),
            4
        );
    }
}

#[test]
fn closed_literals_parameters_and_same_row_equality_survive_ground_folding() {
    let dir = common::TempDir::new("event-closed-bindings");
    let db = Db::create(dir.path(), schema(false), common::work())
        .unwrap()
        .unwrap();
    db.write(common::work(), |tx| {
        tx.insert_dyn(RelationId(2), (0..3).map(|id| vec![Value::U64(id)]))?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let value = space(true).coordinate(0, &()).unwrap();
    for term in [
        Term::Literal(Value::Event(value.clone())),
        Term::Param(ParamId(0)),
    ] {
        let bindings: Vec<BindValue<'_>> = if matches!(term, Term::Param(_)) {
            vec![BindValue::Event(&value)]
        } else {
            vec![]
        };
        let q = query(vec![atom(0, &[(2, term)])], vec![FindTerm::Count]);
        for fallback in [false, true] {
            let mut prepared = db.prepare(&q, common::work()).unwrap();
            prepared.force_cursor_fallback(fallback);
            let answers = db
                .read(common::work(), |s| {
                    s.execute_collect(&mut prepared, &bindings)
                })
                .unwrap();
            assert_eq!(answers.get(0, 0), AnswerValue::U64(1));
        }
    }
    // The repeated Event variable is local to the closed atom. Its equality
    // can be folded only because both fields share the row's checked registry.
    let folded = query(
        vec![
            atom(2, &[(0, Term::Var(VarId(0)))]),
            atom(
                0,
                &[
                    (0, Term::Var(VarId(0))),
                    (2, Term::Var(VarId(1))),
                    (3, Term::Var(VarId(1))),
                    (4, Term::Literal(Value::U64(11))),
                ],
            ),
        ],
        vec![FindTerm::Var(VarId(0))],
    );
    let mut prepared = db.prepare(&folded, common::work()).unwrap();
    let answers = db
        .read(common::work(), |s| {
            s.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    let mut ids = (0..answers.len())
        .map(|i| match answers.get(i, 0) {
            AnswerValue::U64(id) => id,
            _ => panic!("id"),
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    assert_eq!(ids, [0, 2]);
}

#[test]
fn open_to_closed_event_coverage_checks_context_and_scalar_groups() {
    let mut theory = schema(false);
    theory.statements.push(StatementDescriptor::Containment {
        source: side(1, &[1, 2]),
        target: side(0, &[1, 2]),
    });
    let dir = common::TempDir::new("event-closed-target");
    let db = Db::create(dir.path(), theory, common::work())
        .unwrap()
        .unwrap();
    let base = space(true);
    db.write(common::work(), |tx| {
        tx.insert_dyn(
            RelationId(1),
            [vec![
                Value::U64(1),
                Value::U64(7),
                Value::Event(base.full()),
            ]],
        )?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert_dyn(
            RelationId(1),
            [vec![
                Value::U64(2),
                Value::U64(8),
                Value::Event(base.full()),
            ]],
        )?;
        Ok(())
    }));
    let foreign = Space::new(SpaceId([128; 32]), 2, &()).unwrap();
    assert!(matches!(
        db.write(common::work(), |tx| {
            tx.insert_dyn(
                RelationId(1),
                [vec![
                    Value::U64(3),
                    Value::U64(7),
                    Value::Event(foreign.empty()),
                ]],
            )?;
            Ok(())
        }),
        Err(bumbledb::Error::Event(
            bumbledb::event::Error::SpaceMismatch
        ))
    ));
}

#[test]
fn closed_to_open_coverage_supports_admitted_instances_and_atomic_repair() {
    let mut theory = schema(false);
    theory.statements.push(key(1, &[1, 2]));
    theory.statements.push(StatementDescriptor::Containment {
        source: side(0, &[1, 2]),
        target: side(1, &[1, 2]),
    });
    let dir = common::TempDir::new("event-closed-source");
    assert!(matches!(
        Db::create(dir.path(), theory.clone(), common::work()).unwrap(),
        bumbledb::Admission::Rejected(_)
    ));
    let base = space(true);
    let old = vec![Value::U64(1), Value::U64(7), Value::Event(base.full())];
    let mut builder = InstanceBuilder::new(theory, common::work()).unwrap();
    builder.load_dyn(RelationId(1), [&old]).unwrap();
    let instance = builder.admit().unwrap().unwrap();
    let db = Db::from_instance(dir.path(), &instance, common::work()).unwrap();
    drop(instance);
    common::expect_rejected(db.write(common::work(), |tx| {
        tx.delete_dyn(RelationId(1), [&old])?;
        Ok(())
    }));
    let a = base.coordinate(0, &()).unwrap();
    let replacement = [
        vec![Value::U64(2), Value::U64(7), Value::Event(a.clone())],
        vec![Value::U64(3), Value::U64(7), Value::Event(a.complement())],
    ];
    db.write(common::work(), |tx| {
        tx.delete_dyn(RelationId(1), [&old])?;
        tx.insert_dyn(RelationId(1), &replacement)?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    common::expect_rejected(db.write(common::work(), |tx| {
        tx.delete_dyn(RelationId(1), [&replacement[0]])?;
        Ok(())
    }));
}

#[test]
fn closed_point_reads_use_portable_equality_and_preserve_empty_key_refusal() {
    use bumbledb::schema::StatementId;
    let value = space(true).coordinate(0, &()).unwrap();
    let key = [Value::U64(7), Value::Event(value)];
    // The implied closed id key precedes the three declared keys.
    let statement = StatementId(1);
    let builder = InstanceBuilder::new(schema(false), common::work()).unwrap();
    assert!(
        builder
            .get_dyn(RelationId(0), statement, &key)
            .unwrap()
            .is_some()
    );
    let instance = builder.admit().unwrap().unwrap();
    assert!(
        instance
            .get_dyn(RelationId(0), statement, &key, &common::work())
            .unwrap()
            .is_some()
    );
    let dir = common::TempDir::new("event-closed-get");
    let db = Db::from_instance(dir.path(), &instance, common::work()).unwrap();
    db.read(common::work(), |s| {
        let row = s.get_dyn(RelationId(0), statement, &key)?.unwrap();
        assert_eq!(row[0], Value::U64(0));
        assert!(
            s.get_dyn(
                RelationId(0),
                statement,
                &[Value::U64(7), Value::Event(space(true).full())]
            )?
            .is_none()
        );
        assert!(matches!(
            s.get_dyn(
                RelationId(0),
                statement,
                &[Value::U64(7), Value::Event(space(true).empty())]
            ),
            Err(bumbledb::Error::FactShape(
                bumbledb::error::FactShapeError::EmptyEventLookup { .. }
            ))
        ));
        Ok(())
    })
    .unwrap();
    db.write(common::work(), |tx| {
        assert!(
            tx.get_dyn_with_work(RelationId(0), statement, &key, common::work())?
                .is_some()
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    let stopped = common::work();
    stopped.cancel();
    assert!(
        instance
            .get_dyn(RelationId(0), statement, &key, &stopped)
            .is_err()
    );
}

mod macro_ground {
    bumbledb::schema! {
        pub Ground;
        closed relation Catalog as CatalogId { value: event, tag: u64 } = {
            Empty { value: b"\x42\x45\x56\x54\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00", tag: 9 },
        };
        Catalog(value) -> Catalog;
    }
    #[test]
    fn event_accessors_are_owned_and_the_schema_macro_reads_ground_rows() {
        use bumbledb::schema::{Theory as _, ValidateDescriptor as _};
        let value = Catalog::Empty.value();
        assert!(value.is_empty());
        assert_eq!(Catalog::Empty.tag(), 9);
        Ground.descriptor().validate().unwrap();
        let dir = super::common::TempDir::new("event-closed-macro");
        let db = bumbledb::Db::create(dir.path(), Ground, super::common::work())
            .unwrap()
            .unwrap();
        let q = bumbledb::query!(Ground { (a, t) | Catalog(value: a, tag: t); });
        let mut prepared = db.prepare(&q, super::common::work()).unwrap();
        let answers = db
            .read(super::common::work(), |s| {
                s.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 1);
        assert_eq!(answers.get(0, 1), bumbledb::AnswerValue::U64(9));
        let bumbledb::AnswerValue::Event(actual) = answers.get(0, 0) else {
            panic!("Event")
        };
        assert_eq!(actual.to_bytes(&()).unwrap(), value.to_bytes(&()).unwrap());
    }
}
