use super::*;
use crate::ir::Value;
use crate::obs;
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, Side, StatementDescriptor,
    ValueType,
};

fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![FieldDescriptor {
                name: "v".into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            }],
        }],
        statements: vec![],
    }
}

const R: RelationId = RelationId(0);

fn names(events: &[obs::TraceEvent]) -> Vec<obs::TracePoint> {
    events.iter().map(|e| e.point()).collect()
}

#[test]
fn write_path_traces_phases_with_counts() {
    let dir = TempDir::new("db-trace-write");
    let db = Db::create(dir.path(), schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(R, [&[Value::U64(99)]])?;
        Ok(())
    })
    .expect("seed")
    .expect("accepted");

    obs::start_capture();
    db.write(|tx| {
        for v in 0..3 {
            tx.insert_dyn(R, [&[Value::U64(v)]])?;
        }
        tx.delete_dyn(R, [&[Value::U64(99)]])?;
        Ok(())
    })
    .expect("write")
    .expect("accepted");
    let events = obs::finish_capture();
    let phase_order: Vec<obs::TracePoint> = events
        .iter()
        .filter(|e| {
            [
                obs::names::APPLY_DELETES,
                obs::names::APPLY_INSERTS,
                obs::names::JUDGMENT_SOURCE,
                obs::names::JUDGMENT_TARGET,
                obs::names::COUNTERS_FLUSH,
                obs::names::LMDB_COMMIT,
            ]
            .contains(&e.point())
        })
        .map(|e| e.point())
        .collect();
    assert_eq!(
        phase_order,
        vec![
            obs::names::APPLY_DELETES,
            obs::names::APPLY_INSERTS,
            obs::names::JUDGMENT_SOURCE,
            obs::names::JUDGMENT_TARGET,
            obs::names::COUNTERS_FLUSH,
            obs::names::LMDB_COMMIT,
        ],
        "the canonical order, recorded in drop order per phase"
    );
    let by_name = |n| events.iter().find(|e| e.point() == n).expect("phase");
    assert_eq!(by_name(obs::names::APPLY_DELETES).a0(), 1);
    assert_eq!(by_name(obs::names::APPLY_INSERTS).a0(), 3);
    assert_eq!(by_name(obs::names::COMMIT).a0(), 1, "commit changed flag");
    assert_eq!(
        by_name(obs::names::WRITE_TXN).args(),
        obs::TraceArgs::Flag(true),
        "committed flag"
    );

    obs::start_capture();
    db.write(|tx| {
        tx.insert_dyn(R, [&[Value::U64(0)]])?;
        Ok(())
    })
    .expect("noop write")
    .expect("accepted");
    let noop = obs::finish_capture();
    let noop_names = names(&noop);
    assert!(
        noop_names.contains(&obs::names::COMMIT_NOOP),
        "{noop_names:?}"
    );
    assert!(!noop_names.contains(&obs::names::LMDB_COMMIT));
    assert!(!noop_names.contains(&obs::names::APPLY_DELETES));
}

#[test]
fn a_redundant_insert_costs_zero_source_side_probes() {
    const TARGET: RelationId = RelationId(0);
    const CLAIM: RelationId = RelationId(1);
    const EXTRA: RelationId = RelationId(2);
    let field = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::None,
    };
    let containment_schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![field("id")],
            },
            RelationDescriptor {
                extension: None,
                name: "Claim".into(),
                fields: vec![field("holder")],
            },
            RelationDescriptor {
                extension: None,
                name: "Extra".into(),
                fields: vec![field("v")],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: TARGET,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: CLAIM,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: TARGET,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
        ],
    };
    let dir = TempDir::new("db-trace-redundant-insert");
    let db = Db::create(dir.path(), containment_schema)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(TARGET, [&[Value::U64(5)]])?;
        tx.insert_dyn(CLAIM, [&[Value::U64(5)]])?;
        Ok(())
    })
    .expect("seed")
    .expect("accepted");

    obs::start_capture();
    db.write(|tx| {
        tx.insert_dyn(CLAIM, [&[Value::U64(5)]])?;
        tx.insert_dyn(EXTRA, [&[Value::U64(1)]])?;
        Ok(())
    })
    .expect("write")
    .expect("accepted");
    let events = obs::finish_capture();
    let source = events
        .iter()
        .find(|e| e.point() == obs::names::JUDGMENT_SOURCE)
        .expect("judgment span");
    assert_eq!(source.a0(), 0, "zero probes for the redundant insert");

    obs::start_capture();
    db.write(|tx| {
        tx.insert_dyn(TARGET, [&[Value::U64(6)]])?;
        tx.insert_dyn(CLAIM, [&[Value::U64(6)]])?;
        Ok(())
    })
    .expect("write")
    .expect("accepted");
    let events = obs::finish_capture();
    let source = events
        .iter()
        .find(|e| e.point() == obs::names::JUDGMENT_SOURCE)
        .expect("judgment span");
    assert_eq!(source.a0(), 1, "one probe for the genuine insert");
}

#[test]
fn a_noop_fresh_commit_keeps_the_view_memo_valid() {
    let fresh_schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "S".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    };
    let dir = TempDir::new("db-trace-noop-fresh");
    let db = Db::create(dir.path(), fresh_schema)
        .expect("create")
        .expect("accepted");
    let rel = RelationId(0);

    let id_field = db.fresh_field(rel, FieldId(0)).expect("fresh field");
    db.write(|tx| {
        let id = tx.reserve_at(id_field, 1)?.start().expect("nonempty");
        tx.insert_dyn(rel, [&[Value::U64(id), Value::U64(42)]])
            .map(|_| ())
    })
    .expect("seed")
    .expect("accepted");
    assert_eq!(db.generation().expect("generation").value(), 1);

    let query = crate::ir::Query::single(crate::ir::Rule {
        finds: vec![
            crate::ir::FindTerm::Var(crate::ir::VarId(0)),
            crate::ir::FindTerm::Var(crate::ir::VarId(1)),
        ],
        atoms: vec![crate::ir::Atom {
            source: crate::ir::AtomSource::Edb(rel),
            bindings: vec![
                (FieldId(0), crate::ir::Term::Var(crate::ir::VarId(0))),
                (FieldId(1), crate::ir::Term::Var(crate::ir::VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = db.prepare(&query).expect("prepare");
    db.read(|snap| {
        snap.execute_collect(&mut prepared, &[] as &[crate::BindValue])
            .map(|_| ())
    })
    .expect("first execute builds");

    let escaped = db
        .write(|tx| Ok(tx.reserve_at(id_field, 1)?.start().expect("nonempty")))
        .expect("bare reserve")
        .expect("accepted")
        .value;
    assert_eq!(escaped, 1);
    assert_eq!(
        db.generation().expect("generation").value(),
        1,
        "a counters-only commit is not a state change"
    );

    obs::start_capture();
    db.read(|snap| {
        snap.execute_collect(&mut prepared, &[] as &[crate::BindValue])
            .map(|_| ())
    })
    .expect("second execute");
    let events = obs::finish_capture();
    let ns = names(&events);
    assert!(ns.contains(&obs::names::VIEW_MEMO_HIT), "{ns:?}");
    assert!(!ns.contains(&obs::names::VIEW_BUILD), "{ns:?}");
    assert!(!ns.contains(&obs::names::IMAGE_BUILD), "{ns:?}");

    let next = db
        .write(|tx| Ok(tx.reserve_at(id_field, 1)?.start().expect("nonempty")))
        .expect("reserve")
        .expect("accepted")
        .value;
    assert_eq!(next, 2);
}

#[test]
fn a_collection_insert_is_one_write() {
    let dir = TempDir::new("db-trace-insert");
    let db = Db::create(dir.path(), schema())
        .expect("create")
        .expect("accepted");
    let n = 8_192u64;
    obs::start_capture();
    let loaded = db
        .write(|tx| {
            tx.insert_dyn(R, (0..n).map(|v| vec![Value::U64(v)]))
                .map(crate::MutationReport::changed)
        })
        .expect("insert")
        .unwrap()
        .value;
    let events = obs::finish_capture();
    assert_eq!(loaded, n);
    let writes = events
        .iter()
        .filter(|e| e.point() == obs::names::WRITE_TXN)
        .count();
    assert_eq!(
        writes, 1,
        "a collection insert is one commit, not chunked writes"
    );
}

/// Power-loss semantics cannot be pinned in-process; what CAN be is that the
/// parent-dirent sync path executes — `COMPACT_DURABLE` records only after the
/// copied file, the `dest` dirent contents, and `dest`'s own entry in its
/// parent directory have all been fsynced, so the event's presence is the pin.
#[test]
fn compact_records_its_completed_durability_chain() {
    let dir = TempDir::new("db-trace-compact");
    let db = Db::create(dir.path(), schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(R, [&[Value::U64(1)]])?;
        Ok(())
    })
    .expect("seed")
    .expect("accepted");

    let dest = dir.path().join("compacted");
    obs::start_capture();
    db.compact(&dest).expect("compact");
    let events = obs::finish_capture();
    let durable = events
        .iter()
        .find(|e| e.point() == obs::names::COMPACT_DURABLE)
        .expect("the durability-chain event");
    assert_eq!(durable.a0(), 2, "dest dirent + parent dirent, both synced");
}

/// `Db::create`'s birth dirent chain (finding 022) — compact's pin, applied to
/// the other site of the one mechanism: `CREATE_DURABLE` records only after the
/// store directory and its parent have both been fsynced behind the initialize
/// commit, so the event's presence pins that the create-path syncs executed.
#[test]
fn create_records_its_completed_durability_chain() {
    let dir = TempDir::new("db-trace-create");
    obs::start_capture();
    let _db = Db::create(dir.path(), schema())
        .expect("create")
        .expect("accepted");
    let events = obs::finish_capture();
    let durable = events
        .iter()
        .find(|e| e.point() == obs::names::CREATE_DURABLE)
        .expect("the durability-chain event");
    assert_eq!(durable.a0(), 2, "store dirent + parent dirent, both synced");
}

#[test]
fn an_aborted_write_burns_escaped_ids_exactly_once_panic_included() {
    let fresh_schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "S".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    };
    let dir = TempDir::new("db-trace-abort-burn");
    let db = Db::create(dir.path(), fresh_schema)
        .expect("create")
        .expect("accepted");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    let flush_counts = |events: &[obs::TraceEvent]| {
        (
            events
                .iter()
                .filter(|e| e.point() == obs::names::COUNTERS_FLUSH)
                .count(),
            events
                .iter()
                .filter(|e| e.point() == obs::names::LMDB_COMMIT)
                .count(),
        )
    };

    obs::start_capture();
    let aborted = db.write(|tx| {
        tx.reserve_at(id_field, 1)?.start().expect("nonempty");
        Err::<(), _>(crate::error::Error::Overflow(
            crate::error::OverflowKind::Aggregate {
                find: crate::error::FindIndex(0),
            },
        ))
    });
    let events = obs::finish_capture();
    assert!(aborted.is_err());
    assert_eq!(
        flush_counts(&events),
        (1, 1),
        "the Err abort burns exactly once"
    );

    // The panicked write: the guard's drop is the only flush.
    obs::start_capture();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = db.write(|tx| -> Result<()> {
            assert_eq!(
                tx.reserve_at(id_field, 1)?.start().expect("nonempty"),
                1,
                "past the Err abort's burn"
            );
            panic!("mid-closure");
        });
    }));
    let events = obs::finish_capture();
    assert!(unwound.is_err(), "the closure's panic propagates");
    assert_eq!(
        flush_counts(&events),
        (1, 1),
        "the panicked write burns exactly once"
    );

    db.write(|tx| {
        assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 2);
        Ok(())
    })
    .expect("mint after both aborts")
    .expect("accepted");
}

#[test]
fn verify_store_traces_every_namespace_pass_in_order() {
    let dir = TempDir::new("db-trace-verify");
    let db = Db::create(dir.path(), schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(R, [&[Value::U64(1)]])?;
        Ok(())
    })
    .expect("seed")
    .expect("accepted");

    obs::start_capture();
    let report = db.verify_store().expect("verify");
    let events = obs::finish_capture();
    assert!(report.findings().is_empty(), "a clean store");

    let pass_order: Vec<obs::TracePoint> = events
        .iter()
        .filter(|e| {
            [
                obs::names::VERIFY_FACTS,
                obs::names::VERIFY_MEMBERSHIP,
                obs::names::VERIFY_DETERMINANTS,
                obs::names::VERIFY_REVERSE,
                obs::names::VERIFY_MARKS,
                obs::names::VERIFY_COUNTERS,
                obs::names::VERIFY_FRESH,
                obs::names::VERIFY_DICT,
            ]
            .contains(&e.point())
        })
        .map(|e| e.point())
        .collect();
    assert_eq!(
        pass_order,
        vec![
            obs::names::VERIFY_FACTS,
            obs::names::VERIFY_MEMBERSHIP,
            obs::names::VERIFY_DETERMINANTS,
            obs::names::VERIFY_REVERSE,
            obs::names::VERIFY_MARKS,
            obs::names::VERIFY_COUNTERS,
            obs::names::VERIFY_FRESH,
            obs::names::VERIFY_DICT,
        ],
        "the canonical sweep order",
    );
    let outer = events
        .iter()
        .find(|e| e.point() == obs::names::VERIFY_STORE)
        .expect("the outer sweep span");
    assert_eq!(outer.a0(), 0, "a clean store raises no findings");
    for e in &events {
        assert!(e.start_ns() >= outer.start_ns());
        assert!(e.start_ns() + e.dur_ns() <= outer.start_ns() + outer.dur_ns());
        assert_eq!(
            e.a0(),
            0,
            "every pass raised zero findings on a clean store"
        );
    }
}

#[test]
fn an_aborting_write_records_no_lmdb_commit() {
    let dir = TempDir::new("db-trace-abort");
    let db = Db::create(dir.path(), schema())
        .expect("create")
        .expect("accepted");
    obs::start_capture();
    let result = db.write(|tx| {
        tx.insert_dyn(R, [&[Value::U64(1)]])?;
        Err::<(), _>(crate::error::Error::Overflow(
            crate::error::OverflowKind::Aggregate {
                find: crate::error::FindIndex(0),
            },
        ))
    });
    let events = obs::finish_capture();
    assert!(result.is_err());
    let ns = names(&events);
    assert!(!ns.contains(&obs::names::LMDB_COMMIT), "{ns:?}");
    let write_txn = events
        .iter()
        .find(|e| e.point() == obs::names::WRITE_TXN)
        .expect("write_txn span");
    assert_eq!(
        write_txn.args(),
        obs::TraceArgs::None,
        "aborted write never sets the committed flag — distinct from Flag(false) and Count(0)"
    );
}

#[test]
fn point_reads_trace_hits_and_misses() {
    let dir = TempDir::new("db-trace-point-read");
    let keyed = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Entry".into(),
            fields: vec![
                FieldDescriptor {
                    name: "k".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([bumbledb_theory::schema::FieldId(0)]),
        }],
    };
    let db = Db::create(dir.path(), keyed)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(RelationId(0), [&[Value::U64(7), Value::U64(70)]])?;
        Ok(())
    })
    .expect("seed")
    .expect("accepted");

    obs::start_capture();
    let mut out = Vec::new();
    db.read(|snap| {
        let hit = snap.get_dyn(
            RelationId(0),
            bumbledb_theory::schema::StatementId(0),
            &[Value::U64(7)],
        )?;
        assert_eq!(hit, Some(vec![Value::U64(7), Value::U64(70)]));
        assert!(!snap.get_dyn_into(
            RelationId(0),
            bumbledb_theory::schema::StatementId(0),
            &[Value::U64(9)],
            &mut out
        )?);
        Ok(())
    })
    .expect("reads");
    let events = obs::finish_capture();
    let point_reads: Vec<u64> = events
        .iter()
        .filter(|e| e.point() == obs::names::POINT_READ)
        .map(|e| e.a0())
        .collect();
    assert_eq!(
        point_reads,
        vec![1, 0],
        "one span per get, hit then miss in a0"
    );
}
