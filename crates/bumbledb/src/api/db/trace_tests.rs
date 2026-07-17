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

fn names(events: &[obs::TraceEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.name).collect()
}

/// The write-path capture contract.
#[test]
fn write_path_traces_phases_with_counts() {
    let dir = TempDir::new("db-trace-write");
    let db = Db::create(dir.path(), schema()).expect("create");
    db.write(|tx| {
        tx.insert_dyn(R, &[Value::U64(99)])?;
        Ok(())
    })
    .expect("seed");

    // Three inserts + one delete: the six phase spans, in order, with
    // the counts from the delta's own entries.
    obs::start_capture();
    db.write(|tx| {
        for v in 0..3 {
            tx.insert_dyn(R, &[Value::U64(v)])?;
        }
        tx.delete_dyn(R, &[Value::U64(99)])?;
        Ok(())
    })
    .expect("write");
    let events = obs::finish_capture();
    let phase_order: Vec<&str> = events
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
            .contains(&e.name)
        })
        .map(|e| e.name)
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
    let by_name = |n: &str| events.iter().find(|e| e.name == n).expect("phase");
    assert_eq!(by_name(obs::names::APPLY_DELETES).a0, 1);
    assert_eq!(by_name(obs::names::APPLY_INSERTS).a0, 3);
    assert_eq!(by_name(obs::names::COMMIT).a0, 1, "commit changed flag");
    assert_eq!(by_name(obs::names::WRITE_TXN).a0, 1, "committed flag");

    // A net-no-op write: commit_noop, no phase spans.
    obs::start_capture();
    db.write(|tx| {
        tx.insert_dyn(R, &[Value::U64(0)])?; // already present
        Ok(())
    })
    .expect("noop write");
    let noop = obs::finish_capture();
    let noop_names = names(&noop);
    assert!(
        noop_names.contains(&obs::names::COMMIT_NOOP),
        "{noop_names:?}"
    );
    assert!(!noop_names.contains(&obs::names::LMDB_COMMIT));
    assert!(!noop_names.contains(&obs::names::APPLY_DELETES));
}

/// A redundant insert is never judged (PRD 05, the net-disposition
/// delta): the delta records nothing for a committed fact, so the
/// source-side judgment runs zero probes on its behalf — the trace's
/// `JUDGMENT_SOURCE` arg is the probe count.
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
    let db = Db::create(dir.path(), containment_schema).expect("create");
    db.write(|tx| {
        tx.insert_dyn(TARGET, &[Value::U64(5)])?;
        tx.insert_dyn(CLAIM, &[Value::U64(5)])?;
        Ok(())
    })
    .expect("seed");

    // The redundant insert beside an unrelated genuine change (which
    // keeps the delta nonempty; Extra has no outgoing statements): the
    // source-side judgment probes nothing.
    obs::start_capture();
    db.write(|tx| {
        tx.insert_dyn(CLAIM, &[Value::U64(5)])?;
        tx.insert_dyn(EXTRA, &[Value::U64(1)])?;
        Ok(())
    })
    .expect("write");
    let events = obs::finish_capture();
    let source = events
        .iter()
        .find(|e| e.name == obs::names::JUDGMENT_SOURCE)
        .expect("judgment span");
    assert_eq!(source.a0, 0, "zero probes for the redundant insert");

    // Contrast: a genuinely added source costs exactly its one probe.
    obs::start_capture();
    db.write(|tx| {
        tx.insert_dyn(TARGET, &[Value::U64(6)])?;
        tx.insert_dyn(CLAIM, &[Value::U64(6)])?;
        Ok(())
    })
    .expect("write");
    let events = obs::finish_capture();
    let source = events
        .iter()
        .find(|e| e.name == obs::names::JUDGMENT_SOURCE)
        .expect("judgment span");
    assert_eq!(source.a0, 1, "one probe for the genuine insert");
}

/// A fresh-only no-op commit does not move
/// the generation, so a prepared query's next execution memo-hits —
/// the counters-only flush invalidated nothing.
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
    let db = Db::create(dir.path(), fresh_schema).expect("create");
    let rel = RelationId(0);
    // Resolve once, mint per row: the witness is the untyped mint handle.
    let id_field = db.fresh_field(rel, FieldId(0)).expect("fresh field");
    db.write(|tx| {
        let id = tx.alloc_at(id_field)?;
        tx.insert_dyn(rel, &[Value::U64(id), Value::U64(42)])
            .map(|_| ())
    })
    .expect("seed");
    assert_eq!(db.generation().expect("generation").value(), 1);

    // Q(id, v) :- S(id, v) — a full-scan free join that builds views.
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
    db.read(|snap| snap.execute_collect(&mut prepared, &[]).map(|_| ()))
        .expect("first execute builds");

    // The no-op commit: an escaped allocation, no facts.
    let escaped = db.write(|tx| tx.alloc_at(id_field)).expect("bare alloc");
    assert_eq!(escaped, 1);
    assert_eq!(
        db.generation().expect("generation").value(),
        1,
        "a counters-only commit is not a state change"
    );

    // The next execution memo-hits: nothing was evicted or rebuilt.
    obs::start_capture();
    db.read(|snap| snap.execute_collect(&mut prepared, &[]).map(|_| ()))
        .expect("second execute");
    let events = obs::finish_capture();
    let ns = names(&events);
    assert!(ns.contains(&obs::names::VIEW_MEMO_HIT), "{ns:?}");
    assert!(!ns.contains(&obs::names::VIEW_BUILD), "{ns:?}");
    assert!(!ns.contains(&obs::names::IMAGE_BUILD), "{ns:?}");

    // And the escaped id persisted: the next allocation continues.
    let next = db.write(|tx| tx.alloc_at(id_field)).expect("alloc");
    assert_eq!(next, 2);
}

#[test]
fn bulk_load_traces_one_span_per_chunk() {
    let dir = TempDir::new("db-trace-bulk");
    let db = Db::create(dir.path(), schema()).expect("create");
    // 2.5 chunks: 4096 + 4096 + 2048.
    let n = 4096 * 2 + 2048;
    obs::start_capture();
    let loaded = db
        .bulk_load_dyn(R, (0..n).map(|v| vec![Value::U64(v)]))
        .expect("bulk");
    let events = obs::finish_capture();
    assert_eq!(loaded, n);
    let chunks: Vec<&obs::TraceEvent> = events
        .iter()
        .filter(|e| e.name == obs::names::BULK_CHUNK)
        .collect();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks.iter().map(|c| c.a0).sum::<u64>(), n);
    assert_eq!(chunks.iter().map(|c| c.a1).sum::<u64>(), n);
}

/// `compact`'s durability chain runs to its end. Power-loss semantics
/// cannot be pinned in-process; what CAN be is that the parent-dirent
/// sync path executes — `COMPACT_DURABLE` records only after the copied
/// file, the `dest` dirent contents, and `dest`'s own entry in its
/// parent directory have all been fsynced, so the event's presence is
/// the pin.
#[test]
fn compact_records_its_completed_durability_chain() {
    let dir = TempDir::new("db-trace-compact");
    let db = Db::create(dir.path(), schema()).expect("create");
    db.write(|tx| {
        tx.insert_dyn(R, &[Value::U64(1)])?;
        Ok(())
    })
    .expect("seed");

    let dest = dir.path().join("compacted");
    obs::start_capture();
    db.compact(&dest).expect("compact");
    let events = obs::finish_capture();
    let durable = events
        .iter()
        .find(|e| e.name == obs::names::COMPACT_DURABLE)
        .expect("the durability-chain event");
    assert_eq!(durable.a0, 2, "dest dirent + parent dirent, both synced");
}

#[test]
fn an_aborting_write_records_no_lmdb_commit() {
    let dir = TempDir::new("db-trace-abort");
    let db = Db::create(dir.path(), schema()).expect("create");
    obs::start_capture();
    let result: Result<()> = db.write(|tx| {
        tx.insert_dyn(R, &[Value::U64(1)])?;
        Err(crate::error::Error::Overflow(
            crate::error::OverflowKind::Aggregate { find: 0 },
        ))
    });
    let events = obs::finish_capture();
    assert!(result.is_err());
    let ns = names(&events);
    assert!(!ns.contains(&obs::names::LMDB_COMMIT), "{ns:?}");
    let write_txn = events
        .iter()
        .find(|e| e.name == obs::names::WRITE_TXN)
        .expect("write_txn span");
    assert_eq!(write_txn.a0, 0, "aborted flag");
}
