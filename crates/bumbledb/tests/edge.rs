//! Edge-case pins from the design audits: cyclic containments, nullary
//! relations, serial exhaustion, wide enums, 1-byte compound guards, and
//! empty interned values — each a doc claim that previously rested on a
//! code-reading argument instead of a test. Plus the PRD 20 bind matrix:
//! precise per-position errors for every scalar/set misuse, and a valid
//! mixed bind through the public [`bumbledb::ParamArg`] surface.

use std::path::PathBuf;

use bumbledb::error::ValidationError;
use bumbledb::ir::{AggOp, Atom, FindTerm, ParamId, Query, Term, VarId};
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};
use bumbledb::{Db, Error, Fact, ParamArg, ResultBuffer, ResultValue, Value};

bumbledb::schema! {
    relation Alpha {
        id: u64 as AlphaId, serial,
        beta: u64 as BetaId,
    }
    relation Beta {
        id: u64 as BetaId, serial,
        alpha: u64 as AlphaId,
    }
    relation Node {
        id: u64 as NodeId, serial,
        parent: u64 as NodeId,
    }
    relation Gate {
        tag: str,
    }
    relation Blob {
        id: u64 as BlobId, serial,
        payload: bytes,
        name: str,
    }
    relation Posting {
        id: u64 as PostingId, serial,
        account: u64,
        amount: i64,
        memo: str,
    }

    Alpha(beta) <= Beta(id);
    Beta(alpha) <= Alpha(id);
    Node(parent) <= Node(id);
}

fn test_dir(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("bumbledb-edge-{tag}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("test dir");
    path
}

/// "Cyclic references insert without any staging concept"
/// (docs/architecture/10-data-model.md): A→B plus B→A in one delta, and a
/// self-referencing row — judgments run against the final state.
#[test]
fn cyclic_containments_insert_in_one_transaction() {
    let dir = test_dir("cyclic");
    let db = Db::create(&dir, schema()).expect("create");
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
    assert!(matches!(err, Error::ContainmentViolation { .. }));

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Empty strings and empty byte sequences intern, round-trip, and query —
/// the reverse dictionary entry is exactly the tag byte.
#[test]
fn empty_strings_and_bytes_round_trip() {
    let dir = test_dir("empty-intern");
    let db = Db::create(&dir, schema()).expect("create");
    let original = Blob {
        id: BlobId(1),
        payload: Vec::new(),
        name: String::new(),
    };
    db.write(|tx| tx.insert(&original)).expect("write");
    let back: Vec<Blob> = db
        .read(|snap| snap.scan_facts::<Blob>()?.collect())
        .expect("scan");
    assert_eq!(back, vec![original]);
}

/// An explicit `u64::MAX` serial exhausts the generator through the
/// public surface (typed error, not wraparound).
#[test]
fn explicit_max_serial_exhausts_the_generator() {
    let dir = test_dir("serial-max");
    let db = Db::create(&dir, schema()).expect("create");
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
    assert!(matches!(err, Error::SerialExhausted { .. }));
}

/// A 256-variant enum (every u8 ordinal valid) commits and scans back.
#[test]
fn wide_enum_through_commit_and_scan() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Wide".into(),
            fields: vec![FieldDescriptor {
                name: "v".into(),
                value_type: ValueType::Enum {
                    variants: (0..256).map(|i| format!("V{i}").into()).collect(),
                },
                generation: Generation::None,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("256 variants are legal");
    let dir = test_dir("wide-enum");
    let db = Db::create(&dir, &schema).expect("create");
    db.write(|tx| {
        tx.insert_dyn(RelationId(0), &[Value::Enum(0)])?;
        tx.insert_dyn(RelationId(0), &[Value::Enum(255)])?;
        Ok(())
    })
    .expect("write");
    let mut facts = db
        .read(|snap| snap.scan(RelationId(0))?.collect::<Result<Vec<_>, _>>())
        .expect("scan");
    facts.sort_by_key(|f| match f[0] {
        Value::Enum(o) => o,
        _ => unreachable!("one enum column"),
    });
    assert_eq!(facts, vec![vec![Value::Enum(0)], vec![Value::Enum(255)]]);

    // Bind-time enum range checking: a 256-variant enum accepts every u8,
    // so pin the rejection on a narrow one (2 variants): supplied ordinal
    // 5 is a typed ParamTypeMismatch, not a silent empty result.
    let narrow = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "N".into(),
            fields: vec![
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::Enum {
                        variants: ["A", "B"].iter().map(|v| Box::from(*v)).collect(),
                    },
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "n".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid");
    let dir2 = test_dir("enum-bind");
    let db2 = Db::create(&dir2, &narrow).expect("create");
    db2.write(|tx| {
        tx.insert_dyn(RelationId(0), &[Value::Enum(1), Value::U64(3)])
            .map(|_| ())
    })
    .expect("seed");
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: RelationId(0),
            bindings: vec![
                (FieldId(0), Term::Param(bumbledb::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let mut prepared = db2.prepare(&query).expect("prepare");
    let err = db2
        .read(|snap| {
            snap.execute_collect(&mut prepared, &[Value::Enum(5)])
                .map(|_| ())
        })
        .unwrap_err();
    assert!(matches!(err, Error::ParamTypeMismatch { .. }));
}

/// A compound key plus a containment over 1-byte fields (enum, bool): the
/// dense 2-byte guard shape in `U`/`R` keys, commit-judged.
#[test]
fn one_byte_compound_guards() {
    let status = ValueType::Enum {
        variants: ["On", "Off"].iter().map(|v| Box::from(*v)).collect(),
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
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
    }
    .validate()
    .expect("valid");
    let (switch, watcher) = (RelationId(0), RelationId(1));

    let dir = test_dir("byte-guards");
    let db = Db::create(&dir, &schema).expect("create");
    db.write(|tx| {
        tx.insert_dyn(switch, &[Value::Enum(1), Value::Bool(true)])?;
        tx.insert_dyn(watcher, &[Value::Enum(1), Value::Bool(true), Value::U64(7)])?;
        Ok(())
    })
    .expect("guarded insert commits");

    // The containment holds over the 2-byte guard: a watcher of a
    // missing pair aborts.
    let err = db
        .write(|tx| {
            tx.insert_dyn(
                watcher,
                &[Value::Enum(0), Value::Bool(false), Value::U64(1)],
            )?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::ContainmentViolation { .. }));

    // The target side holds too: deleting the required pair aborts.
    let err = db
        .write(|tx| {
            tx.delete_dyn(switch, &[Value::Enum(1), Value::Bool(true)])?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::ContainmentViolation { .. }));
}

/// Nullary use of a relation as a nonemptiness gate, end to end: a
/// zero-binding atom gates a query, composed with a global Count
/// (exercising the zero-arity group through the public surface).
#[test]
fn zero_binding_gate_with_global_count() {
    let dir = test_dir("gate-count");
    let db = Db::create(&dir, schema()).expect("create");
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
    let query = Query {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![
            Atom {
                relation: Node::RELATION,
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            },
            Atom {
                relation: Gate::RELATION,
                bindings: vec![],
            },
        ],
        negated: vec![],
        predicates: vec![],
    };
    let mut prepared = db.prepare(&query).expect("prepare");

    // Gate empty: no rows at all (empty-input aggregate = empty set).
    let rows = db
        .read(|snap| snap.execute_collect(&mut prepared, &[]))
        .expect("execute");
    assert!(rows.is_empty(), "an empty gate empties the query");

    db.write(|tx| {
        tx.insert(&Gate {
            tag: "open".to_owned(),
        })
    })
    .expect("open the gate");
    let rows = db
        .read(|snap| snap.execute_collect(&mut prepared, &[]))
        .expect("execute");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.get(0, 0), bumbledb::ResultValue::U64(2));
}

/// Q(id) :- Posting(id, account = ?set1, amount = ?0, memo = ?2) — one
/// set among two scalars, the PRD 20 bind-matrix shape.
fn mixed_params_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: Posting::RELATION,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(1))),
                (FieldId(2), Term::Param(ParamId(0))),
                (FieldId(3), Term::Param(ParamId(2))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    }
}

/// The PRD 20 bind matrix through the public [`ParamArg`] surface:
/// scalar-where-set, set-where-scalar, a mistyped set element, and
/// non-dense param ids each raise their precise error; a valid mixed
/// bind (two scalars, one set) executes.
#[test]
#[allow(clippy::too_many_lines)] // the bind matrix: one case per arm, linear
fn bind_matrix_raises_precise_errors_and_mixed_binds_execute() {
    let dir = test_dir("bind-matrix");
    let db = Db::create(&dir, schema()).expect("create");
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
                    memo: memo.to_owned(),
                })?;
                ids.push(id);
            }
            Ok(ids)
        })
        .expect("seed");

    let mut prepared = db.prepare(&mixed_params_query()).expect("prepare");
    db.read(|snap| {
        let rent = Value::String(Box::from(&b"rent"[..]));

        // A valid mixed bind — two scalars, one deduplicated set —
        // executes through both the reusable-buffer and collect paths.
        let args = [
            ParamArg::Scalar(Value::I64(5)),
            ParamArg::Set(&[Value::U64(10), Value::U64(11), Value::U64(11)]),
            ParamArg::Scalar(rent.clone()),
        ];
        let mut out = ResultBuffer::new();
        snap.execute_args(&mut prepared, &args, &mut out)?;
        let mut got: Vec<u64> = (0..out.len())
            .map(|row| {
                let ResultValue::U64(id) = out.get(row, 0) else {
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
                    ParamArg::Scalar(Value::I64(5)),
                    ParamArg::Scalar(Value::U64(10)),
                    ParamArg::Scalar(rent.clone()),
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
                    ParamArg::Scalar(rent.clone()),
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
                    ParamArg::Scalar(Value::I64(5)),
                    ParamArg::Set(&[Value::U64(10), Value::I64(3)]),
                    ParamArg::Scalar(rent.clone()),
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
    gapped.atoms[0].bindings[1] = (FieldId(1), Term::Var(VarId(1)));
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

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
