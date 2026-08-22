use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, StatementDescriptor, ValueType};
use bumbledb::{ConditionalWrite, Db, Error, FieldId, RelationId, Value};

use super::{BOOKING, MARKER, schema};
use crate::differential::{
    ConditionalVerdict, Verdict, engine_write, engine_write_from, naive_write_from,
};
use crate::fixture::{TempDir, field, side};
use crate::naive::{Delta, NaiveDb};

fn pair(room: u64, span: (u64, u64), reference: u64) -> Delta {
    Delta {
        deletes: vec![],
        inserts: vec![
            (
                BOOKING,
                vec![
                    Value::U64(room),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(span.0, span.1).expect("nonempty interval"),
                    ),
                    Value::U64(reference),
                ],
            ),
            (MARKER, vec![Value::U64(reference)]),
        ],
    }
}

fn prepared_world(tag: &str) -> (TempDir, Db<SchemaDescriptor>, NaiveDb) {
    let descriptor = schema();
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
    let mut naive = NaiveDb::new(&descriptor);
    let seed = pair(0, (1, 4), 3);
    assert_eq!(engine_write(&db, &seed), Verdict::Committed);
    naive.apply(&seed).expect("the seed pair commits");
    assert_eq!(db.generation().expect("generation").value(), 1);
    assert_eq!(naive.generation(), 1);
    (dir, db, naive)
}

#[test]
fn the_interleaved_second_sequence_aborts_with_the_payload() {
    let (_dir, db, mut naive) = prepared_world("witness-interleave");
    let first = pair(1, (6, 9), 4);
    let second = pair(2, (10, 12), 5);

    db.read(|instance| {
        let witness = instance.witness()?;
        let witnessed = naive.generation();

        let engine_first =
            db.read(|inner| Ok(engine_write_from(&db, &inner.witness()?, &first)))?;
        let naive_first = naive_write_from(&mut naive, witnessed, &first);
        assert_eq!(engine_first, ConditionalVerdict::Committed);
        assert_eq!(naive_first, ConditionalVerdict::Committed);

        let engine_second = engine_write_from(&db, &witness, &second);
        let naive_second = naive_write_from(&mut naive, witnessed, &second);
        assert_eq!(engine_second, naive_second);
        assert_eq!(
            engine_second,
            ConditionalVerdict::Moved {
                witnessed: 1,
                current: 2,
            }
        );

        let raw = db
            .write_from(&witness, |_| Ok(()))
            .expect("conditional write");
        assert!(
            matches!(
                raw,
                ConditionalWrite::Moved { witnessed, current }
                    if witnessed.value() == 1 && current.value() == 2
            ),
            "expected Moved {{ 1, 2 }}: {raw:?}"
        );
        Ok(())
    })
    .expect("read");

    assert_eq!(naive.relation(MARKER).len(), 2);
    assert_eq!(db.generation().expect("generation").value(), 2);
}

#[test]
fn a_noop_commit_between_read_and_write_does_not_abort() {
    let (_dir, db, mut naive) = prepared_world("witness-noop");
    let follow = pair(1, (6, 9), 4);

    db.read(|instance| {
        let witness = instance.witness()?;
        let witnessed = naive.generation();

        let noop = Delta {
            deletes: vec![(MARKER, vec![Value::U64(77)])],
            inserts: vec![],
        };
        assert_eq!(engine_write(&db, &noop), Verdict::Committed);
        naive.apply(&noop).expect("a no-op delete commits");
        assert_eq!(db.generation().expect("generation").value(), 1, "no bump");
        assert_eq!(naive.generation(), 1, "no bump");

        let engine = engine_write_from(&db, &witness, &follow);
        let model = naive_write_from(&mut naive, witnessed, &follow);
        assert_eq!(engine, ConditionalVerdict::Committed);
        assert_eq!(model, ConditionalVerdict::Committed);
        Ok(())
    })
    .expect("read");
}

/// Scenario (c): a witness snapshot of another database is rejected typed
/// (`ForeignWitness` — the prepared-query identity check on the write side),
/// before anything happens: the clock never moves.
#[test]
fn a_foreign_snapshot_is_rejected_typed() {
    let descriptor = schema();
    let dir = TempDir::new("witness-foreign-a");
    let foreign_dir = TempDir::new("witness-foreign-b");
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
    let foreign = Db::create(foreign_dir.path(), descriptor)
        .expect("create foreign store")
        .expect("accepted");

    foreign
        .read(|instance| {
            let raw = db.write_from(&instance.witness()?, |_| Ok(())).unwrap_err();
            assert!(
                matches!(raw, Error::ForeignWitness),
                "expected ForeignWitness: {raw:?}"
            );
            Ok(())
        })
        .expect("read");
    assert_eq!(
        db.generation().expect("generation").value(),
        0,
        "nothing happened"
    );
}

#[test]
fn write_from_with_no_intervening_commit_is_write() {
    let descriptor = schema();
    let dir_w = TempDir::new("witness-plain");
    let dir_f = TempDir::new("witness-witnessed");
    let db_w = Db::create(dir_w.path(), descriptor.clone())
        .expect("create plain store")
        .expect("accepted");
    let db_f = Db::create(dir_f.path(), descriptor)
        .expect("create witnessed store")
        .expect("accepted");

    let ops = vec![
        pair(0, (1, 4), 3),
        Delta {
            deletes: vec![],
            inserts: vec![(
                BOOKING,
                vec![
                    Value::U64(1),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(6, 9).expect("nonempty interval"),
                    ),
                    Value::U64(8),
                ],
            )],
        },
        pair(2, (10, 12), 5),
    ];
    for delta in &ops {
        let plain = engine_write(&db_w, delta);
        let witnessed = db_f
            .read(|instance| Ok(engine_write_from(&db_f, &instance.witness()?, delta)))
            .expect("read");

        match (plain, witnessed) {
            (Verdict::Committed, ConditionalVerdict::Committed) => {}
            (Verdict::Aborted(a), ConditionalVerdict::Aborted(b)) => assert_eq!(a, b),
            other => panic!("write and write_from diverged: {other:?}"),
        }
    }

    for rel in [BOOKING, MARKER] {
        let scan = |db: &Db<SchemaDescriptor>| -> Vec<Vec<Value>> {
            db.read(|snap| snap.scan(rel)?.collect::<bumbledb::Result<Vec<_>>>())
                .expect("scan")
        };
        assert_eq!(scan(&db_w), scan(&db_f));
    }
    assert_eq!(
        db_w.generation().expect("generation"),
        db_f.generation().expect("generation")
    );
}

fn register_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Register".into(),
            fields: vec![
                field("slot", ValueType::U64),
                field("value", ValueType::U64),
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    }
}

const REGISTER: RelationId = RelationId(0);

const MAINTENANCE_SOURCE: RelationId = RelationId(0);
const MAINTENANCE_DERIVED: RelationId = RelationId(1);

fn maintenance_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Source".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("selected", ValueType::Bool),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Derived".into(),
                fields: vec![field("source", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: MAINTENANCE_SOURCE,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: MAINTENANCE_DERIVED,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(MAINTENANCE_DERIVED, &[0], &[]),
                target: side(MAINTENANCE_SOURCE, &[0], &[]),
            },
        ],
    }
}

fn source(id: u64, selected: bool) -> Vec<Value> {
    vec![Value::U64(id), Value::Bool(selected)]
}

fn maintenance_world(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), maintenance_schema())
        .expect("create maintenance store")
        .expect("accepted");
    (dir, db)
}

fn assert_generation_moved(outcome: &ConditionalWrite<()>) {
    assert!(
        matches!(outcome, ConditionalWrite::Moved { .. }),
        "expected Moved, got {outcome:?}"
    );
}

/// Update-where is snapshot-shaped: movement after predicate evaluation refuses
/// the entire replacement delta before its closure runs.
#[test]
fn update_where_refuses_generation_movement() {
    let (_dir, db) = maintenance_world("witness-update-where");
    db.write(|tx| {
        tx.insert_dyn(MAINTENANCE_SOURCE, [&source(1, false)])?;
        tx.insert_dyn(MAINTENANCE_SOURCE, [&source(2, false)])?;
        Ok(())
    })
    .expect("seed sources")
    .unwrap();

    db.read(|instance| {
        let matches: Vec<_> = instance
            .scan(MAINTENANCE_SOURCE)?
            .collect::<bumbledb::Result<_>>()?;
        db.write(|tx| {
            tx.insert_dyn(MAINTENANCE_SOURCE, [&source(3, false)])?;
            Ok(())
        })?
        .unwrap();
        let ran = std::cell::Cell::new(false);
        let error = db
            .write_from(&instance.witness()?, |tx| {
                ran.set(true);
                for fact in &matches {
                    let Value::U64(id) = fact[0] else {
                        unreachable!("source id is u64")
                    };
                    tx.delete_dyn(MAINTENANCE_SOURCE, [fact])?;
                    tx.insert_dyn(MAINTENANCE_SOURCE, [&source(id, true)])?;
                }
                Ok(())
            })
            .expect("conditional write");
        assert_generation_moved(&error);
        assert!(!ran.get(), "a moved witness must not run the closure");
        Ok(())
    })
    .expect("update-where witness");
}

/// Insert-select likewise cannot publish answers from a snapshot after the
/// source generation has moved.
#[test]
fn insert_select_refuses_generation_movement() {
    let (_dir, db) = maintenance_world("witness-insert-select");
    db.write(|tx| {
        tx.insert_dyn(MAINTENANCE_SOURCE, [&source(1, true)])?;
        Ok(())
    })
    .expect("seed source")
    .unwrap();

    db.read(|instance| {
        let selected: Vec<u64> = instance
            .scan(MAINTENANCE_SOURCE)?
            .map(|fact| {
                let fact = fact?;
                let (Value::U64(id), Value::Bool(true)) = (&fact[0], &fact[1]) else {
                    unreachable!("the seed is selected")
                };
                Ok(*id)
            })
            .collect::<bumbledb::Result<_>>()?;
        db.write(|tx| {
            tx.insert_dyn(MAINTENANCE_SOURCE, [&source(2, true)])?;
            Ok(())
        })?
        .unwrap();
        let ran = std::cell::Cell::new(false);
        let error = db
            .write_from(&instance.witness()?, |tx| {
                ran.set(true);
                for id in &selected {
                    tx.insert_dyn(MAINTENANCE_DERIVED, [&[Value::U64(*id)]])?;
                }
                Ok(())
            })
            .expect("conditional write");
        assert_generation_moved(&error);
        assert!(!ran.get(), "a moved witness must not run the closure");
        Ok(())
    })
    .expect("insert-select witness");
}

#[test]
fn snapshot_read_modify_write_refuses_generation_movement() {
    let (_dir, db) = maintenance_world("witness-read-modify-write");
    db.write(|tx| {
        tx.insert_dyn(MAINTENANCE_SOURCE, [&source(1, false)])?;
        Ok(())
    })
    .expect("seed source")
    .unwrap();

    db.read(|instance| {
        let old = instance
            .scan(MAINTENANCE_SOURCE)?
            .next()
            .expect("one source")?;
        db.write(|tx| {
            tx.delete_dyn(MAINTENANCE_SOURCE, [&old])?;
            tx.insert_dyn(MAINTENANCE_SOURCE, [&source(1, true)])?;
            Ok(())
        })?
        .unwrap();
        let ran = std::cell::Cell::new(false);
        let error = db
            .write_from(&instance.witness()?, |tx| {
                ran.set(true);
                tx.delete_dyn(MAINTENANCE_SOURCE, [&old])?;
                tx.insert_dyn(MAINTENANCE_SOURCE, [&source(1, true)])?;
                Ok(())
            })
            .expect("conditional write");
        assert_generation_moved(&error);
        assert!(!ran.get(), "a moved witness must not run the closure");
        Ok(())
    })
    .expect("read-modify-write witness");
}

/// The dependency net owns soundness independently of the witness: deleting a
/// source while its derived fact survives is rejected by final-state
/// containment, and the rejected deletion changes nothing.
#[test]
fn stale_derived_fact_is_rejected_after_source_movement() {
    let (_dir, db) = maintenance_world("witness-stale-derived");
    db.write(|tx| {
        tx.insert_dyn(MAINTENANCE_SOURCE, [&source(1, true)])?;
        tx.insert_dyn(MAINTENANCE_DERIVED, [&[Value::U64(1)]])?;
        Ok(())
    })
    .expect("seed sound derived fact")
    .unwrap();

    let _ = match db.write(|tx| {
        tx.delete_dyn(MAINTENANCE_SOURCE, [&source(1, true)])?;
        Ok(())
    }) {
        Ok(bumbledb::Admission::Rejected(violations)) => violations,
        other => panic!("expected admission rejection, got {other:?}"),
    };
    let sources = db
        .read(|snap| Ok(snap.scan(MAINTENANCE_SOURCE)?.count()))
        .expect("scan sources");
    let derived = db
        .read(|snap| Ok(snap.scan(MAINTENANCE_DERIVED)?.count()))
        .expect("scan derived");
    assert_eq!((sources, derived), (1, 1), "the refused delete was atomic");
}

fn increment(db: &Db<SchemaDescriptor>) -> u64 {
    let mut retries = 0;
    loop {
        let attempt = db.read(|instance| {
            let mut value = None;
            for fact in instance.scan(REGISTER)? {
                let fact = fact?;
                if fact[0] == Value::U64(0) {
                    let Value::U64(current) = fact[1] else {
                        unreachable!("value is u64 by schema");
                    };
                    value = Some(current);
                }
            }
            let current = value.expect("slot 0 is seeded");
            db.write_from(&instance.witness()?, |tx| {
                tx.delete_dyn(REGISTER, [&[Value::U64(0), Value::U64(current)]])?;
                tx.insert_dyn(REGISTER, [&[Value::U64(0), Value::U64(current + 1)]])?;
                Ok(())
            })
        });
        match attempt {
            Ok(bumbledb::ConditionalWrite::Accepted(_)) => return retries,
            Ok(bumbledb::ConditionalWrite::Rejected(violations)) => {
                panic!("increment rejected: {violations:?}")
            }
            Ok(bumbledb::ConditionalWrite::Moved { .. }) => retries += 1,
            Err(other) => panic!("increment refused: {other:?}"),
        }
    }
}

#[test]
fn two_threads_of_witnessed_increments_equal_the_serial_schedule() {
    const PER_THREAD: u64 = 64;
    let dir = TempDir::new("witness-threads");
    let db = Db::create(dir.path(), register_schema())
        .expect("create engine store")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(REGISTER, [&[Value::U64(0), Value::U64(0)]])?;
        Ok(())
    })
    .expect("seed slot 0")
    .unwrap();

    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                barrier.wait();
                for _ in 0..PER_THREAD {
                    increment(&db);
                }
            });
        }
    });

    let facts = db
        .read(|snap| snap.scan(REGISTER)?.collect::<bumbledb::Result<Vec<_>>>())
        .expect("scan");
    assert_eq!(
        facts,
        vec![vec![Value::U64(0), Value::U64(2 * PER_THREAD)]],
        "the retried schedule serialized"
    );
}
