use bumbledb::error::ValidationError;
use bumbledb::ir::{Atom, FindTerm, ParamId, Query, Rule, Term, VarId};
use bumbledb::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, Row, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};
use bumbledb::{AnswerValue, Answers, BindValue, Db, Error, Fact, ParamArg, Value};

mod common;

bumbledb::schema! {
    pub Ledger;

    relation Alpha {
        id: u64 as AlphaId,
        beta: u64 as BetaId,
    }
    relation Beta {
        id: u64 as BetaId,
        alpha: u64 as AlphaId,
    }
    relation Node {
        id: u64 as NodeId,
        parent: u64 as NodeId,
    }
    relation Gate {
        tag: str,
    }
    relation Blob {
        id: u64 as BlobId,
        payload: bytes<16>,
        name: str,
    }
    relation Posting {
        id: u64 as PostingId,
        account: u64,
        amount: i64,
        memo: str,
    }

    Alpha(beta) <= Beta(id);
    Beta(alpha) <= Alpha(id);
    Node(parent) <= Node(id);
}

#[test]
fn cyclic_containments_insert_in_one_transaction() {
    let dir = common::TempDir::new("edge-cyclic");
    let db = Db::create(dir.path(), Ledger)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert([&Alpha {
            id: AlphaId(1),
            beta: BetaId(2),
        }])?;
        tx.insert([&Beta {
            id: BetaId(2),
            alpha: AlphaId(1),
        }])?;

        tx.insert([&Node {
            id: NodeId(9),
            parent: NodeId(9),
        }])?;
        Ok(())
    })
    .expect("cycle commits: source judgments run against the final state")
    .unwrap();

    // And the failure half: a cycle missing one side aborts whole.
    let _ = common::expect_rejected(db.write(|tx| {
        tx.insert([&Alpha {
            id: AlphaId(5),
            beta: BetaId(99),
        }])?;
        Ok(())
    }));
}

#[test]
fn empty_strings_and_bytes_round_trip() {
    let dir = common::TempDir::new("edge-empty-intern");
    let db = Db::create(dir.path(), Ledger)
        .expect("create")
        .expect("accepted");
    let original = Blob {
        id: BlobId(1),
        payload: [0u8; 16],
        name: "",
    };
    db.write(|tx| tx.insert([&original]))
        .expect("write")
        .unwrap();

    db.read(|snap| {
        let back: Vec<Blob> = snap.scan_facts()?.collect::<Result<_, _>>()?;
        assert_eq!(back, vec![original]);
        Ok(())
    })
    .expect("scan");
}

// The reserve-exhaustion half of the old `explicit_max_fresh_exhausts_the
// _generator` test retired with the fresh machinery (E-NO-RESERVE); the
// legality of the extreme explicit id survives.
#[test]
fn explicit_max_id_is_a_legal_value() {
    let dir = common::TempDir::new("edge-fresh-max");
    let db = Db::create(dir.path(), Ledger)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert([&Node {
            id: NodeId(u64::MAX),
            parent: NodeId(u64::MAX),
        }])
    })
    .expect("explicit MAX is a legal value")
    .unwrap();
}

#[test]
fn cap_wide_closed_vocabulary_through_commit_and_scan() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(
                    (0..256)
                        .map(|i| Row {
                            handle: format!("V{i}").into(),
                            values: Box::new([]),
                        })
                        .collect(),
                ),
                name: "Wide".into(),
                fields: vec![],
            },
            RelationDescriptor {
                extension: None,
                name: "Ref".into(),
                fields: vec![FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                }],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    };
    let dir = common::TempDir::new("edge-wide-vocabulary");
    let db = Db::create(dir.path(), schema)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(RelationId(1), [&[Value::U64(0)]])?;
        tx.insert_dyn(RelationId(1), [&[Value::U64(255)]])?;
        Ok(())
    })
    .expect("write")
    .unwrap();
    let mut facts = db
        .read(|snap| snap.scan(RelationId(1))?.collect::<Result<Vec<_>, _>>())
        .expect("scan");
    facts.sort_by_key(|f| match f[0] {
        Value::U64(id) => id,
        _ => unreachable!("one reference column"),
    });
    assert_eq!(facts, vec![vec![Value::U64(0)], vec![Value::U64(255)]]);

    let _ = common::expect_rejected(db.write(|tx| {
        tx.insert_dyn(RelationId(1), [&[Value::U64(256)]])?;
        Ok(())
    }));
}

#[test]
fn one_byte_compound_determinants() {
    let status = ValueType::Bool;
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Switch".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "state".into(),
                        value_type: status,
                    },
                    FieldDescriptor {
                        name: "armed".into(),
                        value_type: ValueType::Bool,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Watcher".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "state".into(),
                        value_type: status,
                    },
                    FieldDescriptor {
                        name: "armed".into(),
                        value_type: ValueType::Bool,
                    },
                    FieldDescriptor {
                        name: "note".into(),
                        value_type: ValueType::U64,
                    },
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: RelationId(1),
                    projection: Box::new([FieldId(0), FieldId(1)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: RelationId(0),
                    projection: Box::new([FieldId(0), FieldId(1)]),
                    selection: Box::new([]),
                },
            },
        ],
    };
    let (switch, watcher) = (RelationId(0), RelationId(1));

    let dir = common::TempDir::new("edge-byte-determinants");
    let db = Db::create(dir.path(), schema)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(switch, [&[Value::Bool(true), Value::Bool(true)]])?;
        tx.insert_dyn(
            watcher,
            [&[Value::Bool(true), Value::Bool(true), Value::U64(7)]],
        )?;
        Ok(())
    })
    .expect("validated insert commits")
    .unwrap();

    let _ = common::expect_rejected(db.write(|tx| {
        tx.insert_dyn(
            watcher,
            [&[Value::Bool(false), Value::Bool(false), Value::U64(1)]],
        )?;
        Ok(())
    }));

    let _ = common::expect_rejected(db.write(|tx| {
        tx.delete_dyn(switch, [&[Value::Bool(true), Value::Bool(true)]])?;
        Ok(())
    }));
}

#[test]
fn zero_binding_gate_with_global_count() {
    let dir = common::TempDir::new("edge-gate-count");
    let db = Db::create(dir.path(), Ledger)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert([&Node {
            id: NodeId(1),
            parent: NodeId(1),
        }])?;
        tx.insert([&Node {
            id: NodeId(2),
            parent: NodeId(1),
        }])?;
        Ok(())
    })
    .expect("seed")
    .unwrap();

    let query = Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(Node::RELATION),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(Gate::RELATION),
                bindings: vec![],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = db.prepare(&query).expect("prepare");

    let answers = db
        .read(|snap| snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue]))
        .expect("execute");
    assert!(answers.is_empty(), "an empty gate empties the query");

    db.write(|tx| tx.insert([&Gate { tag: "open" }]))
        .expect("open the gate")
        .unwrap();
    let answers = db
        .read(|snap| snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue]))
        .expect("execute");
    assert_eq!(answers.len(), 1);
    assert_eq!(answers.get(0, 0), bumbledb::AnswerValue::U64(2));
}

fn mixed_params_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(Posting::RELATION),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(1))),
                (FieldId(2), Term::Param(ParamId(0))),
                (FieldId(3), Term::Param(ParamId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
fn bind_matrix_raises_precise_errors_and_mixed_binds_execute() {
    let dir = common::TempDir::new("edge-bind-matrix");
    let db = Db::create(dir.path(), Ledger)
        .expect("create")
        .expect("accepted");
    let ids = db
        .write(|tx| {
            let mut ids = Vec::new();
            for (next, (account, amount, memo)) in [
                (10u64, 5i64, "rent"),
                (11, 5, "rent"),
                (12, 5, "rent"),
                (10, 6, "rent"),
                (11, 5, "food"),
            ]
            .into_iter()
            .enumerate()
            {
                let id = PostingId(next as u64);
                tx.insert([&Posting {
                    id,
                    account,
                    amount,
                    memo,
                }])?;
                ids.push(id);
            }
            Ok(ids)
        })
        .expect("seed")
        .unwrap()
        .value;

    let mut prepared = db.prepare(&mixed_params_query()).expect("prepare");
    db.read(|snap| {
        let args = [
            ParamArg::Scalar(BindValue::I64(5)),
            ParamArg::Set(&[Value::U64(10), Value::U64(11), Value::U64(11)]),
            ParamArg::Scalar(BindValue::Str("rent")),
        ];
        let mut out = Answers::new();
        snap.execute(&mut prepared, &args, &mut out)?;
        let mut got: Vec<u64> = (0..out.len())
            .map(|answer| {
                let AnswerValue::U64(id) = out.get(answer, 0) else {
                    panic!("column 0 is the posting id");
                };
                id
            })
            .collect();
        got.sort_unstable();
        assert_eq!(
            got,
            vec![ids[0].0, ids[1].0],
            "accounts 10 and 11, amount 5, rent"
        );
        let collected = snap.execute_collect(&mut prepared, &args)?;
        assert_eq!(collected.len(), 2);

        let err = snap
            .execute_collect(
                &mut prepared,
                &[
                    ParamArg::Scalar(BindValue::I64(5)),
                    ParamArg::Scalar(BindValue::U64(10)),
                    ParamArg::Scalar(BindValue::Str("rent")),
                ],
            )
            .unwrap_err();
        assert!(
            matches!(err, Error::ParamSetExpected { param } if param.0 == 1),
            "{err:?}"
        );

        let err = snap
            .execute_collect(
                &mut prepared,
                &[
                    ParamArg::Set(&[Value::I64(5)]),
                    ParamArg::Set(&[Value::U64(10)]),
                    ParamArg::Scalar(BindValue::Str("rent")),
                ],
            )
            .unwrap_err();
        assert!(
            matches!(err, Error::ParamScalarExpected { param } if param.0 == 0),
            "{err:?}"
        );

        let err = snap
            .execute_collect(
                &mut prepared,
                &[
                    ParamArg::Scalar(BindValue::I64(5)),
                    ParamArg::Set(&[Value::U64(10), Value::I64(3)]),
                    ParamArg::Scalar(BindValue::Str("rent")),
                ],
            )
            .unwrap_err();
        assert!(
            matches!(
                err,
                Error::ParamElementTypeMismatch { param, element: 1, .. } if param.0 == 1
            ),
            "{err:?}"
        );

        // ...and the query stays bindable after every rejection.
        let again = snap.execute_collect(&mut prepared, &args)?;
        assert_eq!(again.len(), 2);
        Ok(())
    })
    .expect("read");

    let mut gapped = mixed_params_query();
    gapped.rules_mut()[0].atoms[0].bindings[1] = (FieldId(1), Term::Var(VarId(1)));
    let Err(err) = db.prepare(&gapped).map(|_| ()) else {
        panic!("a gapped param id space must fail to prepare");
    };
    assert!(
        matches!(
            err,
            Error::Validation(ValidationError::ParamIdGap { param }) if param.0 == 1
        ),
        "{err:?}"
    );
}
