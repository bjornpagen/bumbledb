use super::*;
use crate::ir::Value;
use crate::obs;
use crate::schema::{FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType};
use crate::testutil::TempDir;

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "R".into(),
            fields: vec![FieldDescriptor {
                name: "v".into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("fixture")
}

const R: RelationId = RelationId(0);

fn names(events: &[obs::TraceEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.name).collect()
}

/// PRD 04's write-path capture contract.
#[test]
fn write_path_traces_phases_with_counts() {
    let dir = TempDir::new("db-trace-write");
    let schema = schema();
    let db = Db::create(dir.path(), &schema).expect("create");
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

/// PRD 01 (docs/hardening): a serial-only no-op commit does not move
/// the generation, so a prepared query's next execution memo-hits —
/// the counters-only flush invalidated nothing.
#[test]
fn a_noop_serial_commit_keeps_the_view_memo_valid() {
    let serial_schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "S".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Serial,
                },
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("fixture");
    let dir = TempDir::new("db-trace-noop-serial");
    let db = Db::create(dir.path(), &serial_schema).expect("create");
    let rel = RelationId(0);
    db.write(|tx| {
        let id = tx.alloc_dyn(rel, FieldId(0))?;
        tx.insert_dyn(rel, &[Value::U64(id), Value::U64(42)])
            .map(|_| ())
    })
    .expect("seed");
    assert_eq!(db.generation().expect("generation"), 1);

    // Q(id, v) :- S(id, v) — a full-scan free join that builds views.
    let query = crate::ir::Query {
        finds: vec![
            crate::ir::FindTerm::Var(crate::ir::VarId(0)),
            crate::ir::FindTerm::Var(crate::ir::VarId(1)),
        ],
        atoms: vec![crate::ir::Atom {
            relation: rel,
            bindings: vec![
                (FieldId(0), crate::ir::Term::Var(crate::ir::VarId(0))),
                (FieldId(1), crate::ir::Term::Var(crate::ir::VarId(1))),
            ],
        }],
        predicates: vec![],
    };
    let mut prepared = db.prepare(&query).expect("prepare");
    db.read(|snap| snap.execute_collect(&mut prepared, &[]).map(|_| ()))
        .expect("first execute builds");

    // The no-op commit: an escaped allocation, no facts.
    let escaped = db
        .write(|tx| tx.alloc_dyn(rel, FieldId(0)))
        .expect("bare alloc");
    assert_eq!(escaped, 1);
    assert_eq!(
        db.generation().expect("generation"),
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
    let next = db.write(|tx| tx.alloc_dyn(rel, FieldId(0))).expect("alloc");
    assert_eq!(next, 2);
}

#[test]
fn bulk_load_traces_one_span_per_chunk() {
    let dir = TempDir::new("db-trace-bulk");
    let schema = schema();
    let db = Db::create(dir.path(), &schema).expect("create");
    // 2.5 chunks: 4096 + 4096 + 2048.
    let n = 4096 * 2 + 2048;
    obs::start_capture();
    let loaded = db
        .bulk_load(R, (0..n).map(|v| vec![Value::U64(v)]))
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

#[test]
fn an_aborting_write_records_no_lmdb_commit() {
    let dir = TempDir::new("db-trace-abort");
    let schema = schema();
    let db = Db::create(dir.path(), &schema).expect("create");
    obs::start_capture();
    let result: Result<()> = db.write(|tx| {
        tx.insert_dyn(R, &[Value::U64(1)])?;
        Err(crate::error::Error::Overflow { find: 0 })
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
