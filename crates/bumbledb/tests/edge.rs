//! Edge-case pins from the design audits: cyclic containments, nullary
//! relations, fresh exhaustion, cap-wide closed vocabularies, 1-byte
//! compound determinants, and empty interned values — each a doc claim that
//! previously rested on a code-reading argument instead of a test. Plus the PRD 20 bind matrix:
//! precise per-position errors for every scalar/set misuse, and a valid
//! mixed bind through the public [`bumbledb::ParamArg`] surface.

use bumbledb::error::ValidationError;
use bumbledb::ir::{AggOp, Atom, FindTerm, ParamId, Query, Rule, Term, VarId};
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Row, SchemaDescriptor,
    Side, StatementDescriptor, ValueType,
};
use bumbledb::{AnswerValue, Answers, BindValue, Db, Error, Fact, ParamArg, Value};

mod common;

bumbledb::schema! {
    pub Ledger;

    relation Alpha {
        id: u64 as AlphaId, fresh,
        beta: u64 as BetaId,
    }
    relation Beta {
        id: u64 as BetaId, fresh,
        alpha: u64 as AlphaId,
    }
    relation Node {
        id: u64 as NodeId, fresh,
        parent: u64 as NodeId,
    }
    relation Gate {
        tag: str,
    }
    relation Blob {
        id: u64 as BlobId, fresh,
        payload: bytes<16>,
        name: str,
    }
    relation Posting {
        id: u64 as PostingId, fresh,
        account: u64,
        amount: i64,
        memo: str,
    }

    Alpha(beta) <= Beta(id);
    Beta(alpha) <= Alpha(id);
    Node(parent) <= Node(id);
}

/// "Cyclic references insert without any staging concept"
/// (docs/architecture/10-data-model.md): A→B plus B→A in one delta, and a
/// self-referencing row — judgments run against the final state.
#[test]
fn cyclic_containments_insert_in_one_transaction() {
    let dir = common::TempDir::new("edge-cyclic");
    let db = Db::create(dir.path(), Ledger).expect("create");
    db.write(|tx| {
        tx.insert(&Alpha {
            id: AlphaId(1),
            beta: BetaId(2),
        })?;
        tx.insert(&Beta {
            id: BetaId(2),
            alpha: AlphaId(1),
        })?;
        // The self-loop: a row referencing itself.
        tx.insert(&Node {
            id: NodeId(9),
            parent: NodeId(9),
        })?;
        Ok(())
    })
    .expect("cycle commits: source judgments run against the final state");

    // And the failure half: a cycle missing one side aborts whole.
    let err = db
        .write(|tx| {
            tx.insert(&Alpha {
                id: AlphaId(5),
                beta: BetaId(99), // no such Beta
            })?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::CommitRejected { .. }));
}

/// Empty strings intern and round-trip (the reverse dictionary entry is
/// the empty byte string), and a bytes<16> payload rides inline beside
/// them — no dictionary traffic for the fixed-width field.
#[test]
fn empty_strings_and_bytes_round_trip() {
    let dir = common::TempDir::new("edge-empty-intern");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let original = Blob {
        id: BlobId(1),
        payload: [0u8; 16],
        name: "",
    };
    db.write(|tx| tx.insert(&original)).expect("write");
    // The scanned views borrow the snapshot, so the comparison happens
    // inside the read closure.
    db.read(|snap| {
        let back: Vec<Blob> = snap.scan_facts()?.collect::<Result<_, _>>()?;
        assert_eq!(back, vec![original.clone()]);
        Ok(())
    })
    .expect("scan");
}

/// An explicit `u64::MAX` fresh exhausts the generator through the
/// public surface (typed error, not wraparound).
#[test]
fn explicit_max_fresh_exhausts_the_generator() {
    let dir = common::TempDir::new("edge-fresh-max");
    let db = Db::create(dir.path(), Ledger).expect("create");
    db.write(|tx| {
        tx.insert(&Node {
            id: NodeId(u64::MAX),
            parent: NodeId(u64::MAX),
        })
    })
    .expect("explicit MAX is a legal value");
    let err = db
        .write(|tx| {
            let _: NodeId = tx.alloc()?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::FreshExhausted { .. }));
}

/// A cap-wide closed vocabulary (256 rows — `MAX_EXTENSION_ROWS`)
/// validates; references to its first and last rows commit and scan
/// back; and a reference one past the roster is an ordinary containment
/// violation — no range-check error class exists for row ids. (Bind-time
/// enum range checking died with the inline enum type: a reference field
/// is structurally a u64, and an out-of-roster bind is simply absent.)
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
                    generation: Generation::None,
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
    let db = Db::create(dir.path(), schema).expect("create");
    db.write(|tx| {
        tx.insert_dyn(RelationId(1), &[Value::U64(0)])?;
        tx.insert_dyn(RelationId(1), &[Value::U64(255)])?;
        Ok(())
    })
    .expect("write");
    let mut facts = db
        .read(|snap| snap.scan(RelationId(1))?.collect::<Result<Vec<_>, _>>())
        .expect("scan");
    facts.sort_by_key(|f| match f[0] {
        Value::U64(id) => id,
        _ => unreachable!("one reference column"),
    });
    assert_eq!(facts, vec![vec![Value::U64(0)], vec![Value::U64(255)]]);

    // Row 256 does not exist: past the roster is the same containment
    // violation as any dangling reference.
    let err = db
        .write(|tx| {
            tx.insert_dyn(RelationId(1), &[Value::U64(256)])?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::CommitRejected { .. }));
}

/// A compound key plus a containment over 1-byte fields (bool, bool):
/// the dense 2-byte determinant shape in `U`/`R` keys, commit-judged.
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
                        value_type: status.clone(),
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "armed".into(),
                        value_type: ValueType::Bool,
                        generation: Generation::None,
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
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "armed".into(),
                        value_type: ValueType::Bool,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "note".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![
            // Switch(state, armed) -> Switch
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            // Watcher(state, armed) <= Switch(state, armed)
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
    let db = Db::create(dir.path(), schema).expect("create");
    db.write(|tx| {
        tx.insert_dyn(switch, &[Value::Bool(true), Value::Bool(true)])?;
        tx.insert_dyn(
            watcher,
            &[Value::Bool(true), Value::Bool(true), Value::U64(7)],
        )?;
        Ok(())
    })
    .expect("validated insert commits");

    // The containment holds over the 2-byte determinant: a watcher of a
    // missing pair aborts.
    let err = db
        .write(|tx| {
            tx.insert_dyn(
                watcher,
                &[Value::Bool(false), Value::Bool(false), Value::U64(1)],
            )?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::CommitRejected { .. }));

    // The target side holds too: deleting the required pair aborts.
    let err = db
        .write(|tx| {
            tx.delete_dyn(switch, &[Value::Bool(true), Value::Bool(true)])?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::CommitRejected { .. }));
}

/// Nullary use of a relation as a nonemptiness gate, end to end: a
/// zero-binding atom gates a query, composed with a global Count
/// (exercising the zero-arity group through the public surface).
#[test]
fn zero_binding_gate_with_global_count() {
    let dir = common::TempDir::new("edge-gate-count");
    let db = Db::create(dir.path(), Ledger).expect("create");
    db.write(|tx| {
        tx.insert(&Node {
            id: NodeId(1),
            parent: NodeId(1),
        })?;
        tx.insert(&Node {
            id: NodeId(2),
            parent: NodeId(1),
        })?;
        Ok(())
    })
    .expect("seed");

    // Count(nodes) gated on Gate being nonempty.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
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

    // Gate empty: no answers at all (empty-input aggregate = empty set).
    let answers = db
        .read(|snap| snap.execute_collect(&mut prepared, &[]))
        .expect("execute");
    assert!(answers.is_empty(), "an empty gate empties the query");

    db.write(|tx| tx.insert(&Gate { tag: "open" }))
        .expect("open the gate");
    let answers = db
        .read(|snap| snap.execute_collect(&mut prepared, &[]))
        .expect("execute");
    assert_eq!(answers.len(), 1);
    assert_eq!(answers.get(0, 0), bumbledb::AnswerValue::U64(2));
}

/// Q(id) :- Posting(id, account = ?set1, amount = ?0, memo = ?2) — one
/// set among two scalars, the PRD 20 bind-matrix shape.
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

/// The PRD 20 bind matrix through the public [`ParamArg`] surface:
/// scalar-where-set, set-where-scalar, a mistyped set element, and
/// non-dense param ids each raise their precise error; a valid mixed
/// bind (two scalars, one set) executes.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the bind matrix: one case per arm, linear
fn bind_matrix_raises_precise_errors_and_mixed_binds_execute() {
    let dir = common::TempDir::new("edge-bind-matrix");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let ids = db
        .write(|tx| {
            let mut ids = Vec::new();
            for (account, amount, memo) in [
                (10u64, 5i64, "rent"),
                (11, 5, "rent"),
                (12, 5, "rent"),
                (10, 6, "rent"),
                (11, 5, "food"),
            ] {
                let id: PostingId = tx.alloc()?;
                tx.insert(&Posting {
                    id,
                    account,
                    amount,
                    memo,
                })?;
                ids.push(id);
            }
            Ok(ids)
        })
        .expect("seed");

    let mut prepared = db.prepare(&mixed_params_query()).expect("prepare");
    db.read(|snap| {
        // A valid mixed bind — two scalars, one deduplicated set —
        // executes through both the reusable-buffer and collect paths.
        let args = [
            ParamArg::Scalar(BindValue::I64(5)),
            ParamArg::Set(&[Value::U64(10), Value::U64(11), Value::U64(11)]),
            ParamArg::Scalar(BindValue::Str("rent")),
        ];
        let mut out = Answers::new();
        snap.execute_args(&mut prepared, &args, &mut out)?;
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
        let collected = snap.execute_collect_args(&mut prepared, &args)?;
        assert_eq!(collected.len(), 2);

        // Scalar where the set is expected: the precise per-position error.
        let err = snap
            .execute_collect_args(
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

        // Set where a scalar is expected.
        let err = snap
            .execute_collect_args(
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

        // A mistyped set element names its position.
        let err = snap
            .execute_collect_args(
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
        let again = snap.execute_collect_args(&mut prepared, &args)?;
        assert_eq!(again.len(), 2);
        Ok(())
    })
    .expect("read");

    // Non-dense param ids are a prepare-time validation error: a gap is
    // a positional slot whose supplied value is never type-checked.
    let mut gapped = mixed_params_query();
    gapped.rules[0].atoms[0].bindings[1] = (FieldId(1), Term::Var(VarId(1)));
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
