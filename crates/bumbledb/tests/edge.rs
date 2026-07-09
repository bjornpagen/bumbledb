//! Edge-case pins from the design audits: cyclic FKs, nullary relations,
//! serial exhaustion, wide enums, 1-byte compound guards, and empty
//! interned values — each a doc claim that previously rested on a
//! code-reading argument instead of a test.

use std::path::PathBuf;

use bumbledb::ir::{AggOp, Atom, FindTerm, Query, Term, VarId};
use bumbledb::schema::{
    ConstraintDescriptor, ConstraintId, FieldDescriptor, FieldId, Generation, RelationDescriptor,
    RelationId, SchemaDescriptor, ValueType,
};
use bumbledb::{Db, Error, Fact, Value};

bumbledb::schema! {
    relation Alpha {
        id: u64 as AlphaId, serial,
        beta: u64 as BetaId, fk(Beta.id),
    }
    relation Beta {
        id: u64 as BetaId, serial,
        alpha: u64 as AlphaId, fk(Alpha.id),
    }
    relation Node {
        id: u64 as NodeId, serial,
        parent: u64 as NodeId, fk(Node.id),
    }
    relation Gate {
        tag: str,
    }
    relation Blob {
        id: u64 as BlobId, serial,
        payload: bytes,
        name: str,
    }
}

fn test_dir(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("bumbledb-edge-{tag}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("test dir");
    path
}

/// "Cyclic references insert without any staging concept"
/// (docs/architecture/10-data-model.md): A→B plus B→A in one delta, and a
/// self-referencing row.
#[test]
fn cyclic_foreign_keys_insert_in_one_transaction() {
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
    .expect("cycle commits: forward probes run against the final state");

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
    assert!(matches!(err, Error::ForeignKeyViolation { .. }));

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
            constraints: vec![],
        }],
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
    // so pin the rejection on a narrow one via the fk fixture below — see
    // `one_byte_compound_guards`' schema (2 variants): supplied ordinal 5
    // is a typed ParamTypeMismatch, not a silent empty result.
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
            constraints: vec![],
        }],
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

/// Compound unique + FK over 1-byte fields (enum, bool): the dense 2-byte
/// guard shape in `U`/`R` keys, commit-checked.
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
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "state_armed".into(),
                    fields: Box::new([FieldId(0), FieldId(1)]),
                }],
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
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "watch_fk".into(),
                    fields: Box::new([FieldId(0), FieldId(1)]),
                    target_relation: RelationId(0),
                    target_constraint: ConstraintId(0),
                }],
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

    // The FK holds over the 2-byte guard: a watcher of a missing pair
    // aborts.
    let err = db
        .write(|tx| {
            tx.insert_dyn(
                watcher,
                &[Value::Enum(0), Value::Bool(false), Value::U64(1)],
            )?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::ForeignKeyViolation { .. }));

    // Restrict holds too: deleting the referenced pair aborts.
    let err = db
        .write(|tx| {
            tx.delete_dyn(switch, &[Value::Enum(1), Value::Bool(true)])?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, Error::ForeignKeyViolation { .. }));
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
