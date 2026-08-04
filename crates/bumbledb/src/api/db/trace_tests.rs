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

/// `Db::create`'s birth dirent chain (finding 022) — compact's pin,
/// applied to the other site of the one mechanism: `CREATE_DURABLE`
/// records only after the store directory and its parent have both been
/// fsynced behind the initialize commit, so the event's presence pins
/// that the create-path syncs executed.
#[test]
fn create_records_its_completed_durability_chain() {
    let dir = TempDir::new("db-trace-create");
    obs::start_capture();
    let _db = Db::create(dir.path(), schema()).expect("create");
    let events = obs::finish_capture();
    let durable = events
        .iter()
        .find(|e| e.name == obs::names::CREATE_DURABLE)
        .expect("the durability-chain event");
    assert_eq!(durable.a0, 2, "store dirent + parent dirent, both synced");
}

/// Exactly one burn per termination (`EscapedIdBurn`): an `Err`-aborted
/// and a PANICKED write each advance the escaped `Q` marks through
/// exactly one counters-only commit — never zero (the mint continues
/// past every escaped id), never two (one `COUNTERS_FLUSH` span, one
/// `LMDB_COMMIT` span: the guard owns the whole closure region, and
/// `commit()` was never reached to flush a second time).
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
    let db = Db::create(dir.path(), fresh_schema).expect("create");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    let flush_counts = |events: &[obs::TraceEvent]| {
        (
            events
                .iter()
                .filter(|e| e.name == obs::names::COUNTERS_FLUSH)
                .count(),
            events
                .iter()
                .filter(|e| e.name == obs::names::LMDB_COMMIT)
                .count(),
        )
    };

    // The Err abort: the guard's one counters-only commit.
    obs::start_capture();
    let aborted: Result<()> = db.write(|tx| {
        tx.alloc_at(id_field)?;
        Err(crate::error::Error::Overflow(
            crate::error::OverflowKind::Aggregate { find: 0 },
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
        let _: Result<()> = db.write(|tx| {
            assert_eq!(tx.alloc_at(id_field)?, 1, "past the Err abort's burn");
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

    // Never zero: both aborts' ids are gone; the mint continues past.
    db.write(|tx| {
        assert_eq!(tx.alloc_at(id_field)?, 2);
        Ok(())
    })
    .expect("mint after both aborts");
}

/// Lane I2 — the integrity sweep, formerly wholly dark. `verify_store`
/// records one outer span containing one span per namespace pass, in the
/// canonical order; a clean store raises no findings, so every pass span
/// and the outer span charge zero.
#[test]
fn verify_store_traces_every_namespace_pass_in_order() {
    let dir = TempDir::new("db-trace-verify");
    let db = Db::create(dir.path(), schema()).expect("create");
    db.write(|tx| {
        tx.insert_dyn(R, &[Value::U64(1)])?;
        Ok(())
    })
    .expect("seed");

    obs::start_capture();
    let report = db.verify_store().expect("verify");
    let events = obs::finish_capture();
    assert!(report.findings.is_empty(), "a clean store");

    // The pass spans, in the canonical sweep order, each inside the outer.
    let pass_order: Vec<&str> = events
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
            .contains(&e.name)
        })
        .map(|e| e.name)
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
        .find(|e| e.name == obs::names::VERIFY_STORE)
        .expect("the outer sweep span");
    assert_eq!(outer.a0, 0, "a clean store raises no findings");
    for e in &events {
        assert!(e.start_ns >= outer.start_ns);
        assert!(e.start_ns + e.dur_ns <= outer.start_ns + outer.dur_ns);
        assert_eq!(e.a0, 0, "every pass raised zero findings on a clean store");
    }
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

/// The snapshot point-read surface is lit (the formerly wholly dark
/// keyed-get lane): one `point_read` span per get, hit/miss riding a0
/// — the owned and pooled entries alike.
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
    let db = Db::create(dir.path(), keyed).expect("create");
    db.write(|tx| {
        tx.insert_dyn(RelationId(0), &[Value::U64(7), Value::U64(70)])?;
        Ok(())
    })
    .expect("seed");

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
        .filter(|e| e.name == obs::names::POINT_READ)
        .map(|e| e.a0)
        .collect();
    assert_eq!(
        point_reads,
        vec![1, 0],
        "one span per get, hit then miss in a0"
    );
}
