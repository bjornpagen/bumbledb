//! The generation-witness differential (PRD 18): `Db::write_from`
//! against the naive counter compare — verdict and payload identical,
//! including the typed error (the direction-divergence lesson applied
//! from birth). Four scenarios: an interleaved read-compute-write pair
//! aborts the loser with the right generations; a no-op commit between
//! read and write does NOT abort (state-changing generations only); a
//! foreign snapshot is rejected typed; and `write_from` with no
//! intervening commit behaves identically to `write`, byte-for-byte
//! (row-id-ordered scans plus the generation — this schema has no `str`
//! field, so decoded values determine the canonical fact bytes). Plus
//! the one real-concurrency test the engine permits itself.

use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, StatementDescriptor, ValueType};
use bumbledb::{Db, Error, FieldId, RelationId, Value};

use super::{schema, BOOKING, MARKER};
use crate::differential::{
    engine_write, engine_write_from, naive_write_from, ConditionalVerdict, Verdict,
};
use crate::fixture::{field, TempDir};
use crate::naive::{Delta, NaiveDb};

/// One consistent Booking+Marker pair insert.
fn pair(room: u64, span: (u64, u64), reference: u64) -> Delta {
    Delta {
        deletes: vec![],
        inserts: vec![
            (
                BOOKING,
                vec![
                    Value::U64(room),
                    Value::IntervalU64(span.0, span.1),
                    Value::U64(reference),
                ],
            ),
            (MARKER, vec![Value::U64(reference)]),
        ],
    }
}

/// Seeds one committed pair through both oracles so both clocks read 1.
fn seeded(tag: &str) -> (TempDir, Db<SchemaDescriptor>, NaiveDb) {
    let descriptor = schema();
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);
    let seed = pair(0, (1, 4), 3);
    assert_eq!(engine_write(&db, &seed), Verdict::Committed);
    naive.apply(&seed).expect("the seed pair commits");
    assert_eq!(db.generation().expect("generation"), 1);
    assert_eq!(naive.generation(), 1);
    (dir, db, naive)
}

/// Scenario (a): two read-compute-write sequences witness the same
/// generation; the first lands, the second aborts with the payload
/// naming both generations — on both oracles, identically.
#[test]
fn the_interleaved_second_sequence_aborts_with_the_payload() {
    let (_dir, db, mut naive) = seeded("witness-interleave");
    let first = pair(1, (6, 9), 4);
    let second = pair(2, (10, 12), 5);

    db.read(|witness| {
        let witnessed = naive.generation();
        // The first sequence: a fresh witness of the same generation —
        // commits, moving the clock to 2.
        let engine_first = db.read(|snap| Ok(engine_write_from(&db, snap, &first)))?;
        let naive_first = naive_write_from(&mut naive, witnessed, &first);
        assert_eq!(engine_first, ConditionalVerdict::Committed);
        assert_eq!(naive_first, ConditionalVerdict::Committed);
        // The second sequence, still on the old witness: aborts before
        // any page is touched, verdict and generations identical.
        let engine_second = engine_write_from(&db, witness, &second);
        let naive_second = naive_write_from(&mut naive, witnessed, &second);
        assert_eq!(engine_second, naive_second);
        assert_eq!(
            engine_second,
            ConditionalVerdict::Moved {
                witnessed: 1,
                current: 2,
            }
        );
        // The typed identity on the raw error — ids, never strings.
        let raw = db.write_from(witness, |_| Ok(())).unwrap_err();
        assert!(
            matches!(
                raw,
                Error::GenerationMoved {
                    witnessed: 1,
                    current: 2,
                }
            ),
            "expected GenerationMoved {{ 1, 2 }}: {raw:?}"
        );
        Ok(())
    })
    .expect("read");

    // The aborted delta dropped whole: the second pair never landed.
    assert_eq!(naive.relation(MARKER).len(), 2);
    assert_eq!(db.generation().expect("generation"), 2);
}

/// Scenario (b): a no-op commit (a delete of an absent fact — the delta
/// nets to nothing) advances no generation and trips no witness.
#[test]
fn a_noop_commit_between_read_and_write_does_not_abort() {
    let (_dir, db, mut naive) = seeded("witness-noop");
    let follow = pair(1, (6, 9), 4);

    db.read(|witness| {
        let witnessed = naive.generation();
        // The intervening no-op commit, on both sides.
        let noop = Delta {
            deletes: vec![(MARKER, vec![Value::U64(77)])],
            inserts: vec![],
        };
        assert_eq!(engine_write(&db, &noop), Verdict::Committed);
        naive.apply(&noop).expect("a no-op delete commits");
        assert_eq!(db.generation().expect("generation"), 1, "no bump");
        assert_eq!(naive.generation(), 1, "no bump");
        // The witness holds: state-changing generations only.
        let engine = engine_write_from(&db, witness, &follow);
        let model = naive_write_from(&mut naive, witnessed, &follow);
        assert_eq!(engine, ConditionalVerdict::Committed);
        assert_eq!(model, ConditionalVerdict::Committed);
        Ok(())
    })
    .expect("read");
}

/// Scenario (c): a witness snapshot of another database is rejected
/// typed (`ForeignSnapshot` — the prepared-query identity guard on the
/// write side), before anything happens: the clock never moves.
#[test]
fn a_foreign_snapshot_is_rejected_typed() {
    let descriptor = schema();
    let dir = TempDir::new("witness-foreign-a");
    let foreign_dir = TempDir::new("witness-foreign-b");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let foreign = Db::create(foreign_dir.path(), descriptor).expect("create foreign store");

    foreign
        .read(|snap| {
            let raw = db.write_from(snap, |_| Ok(())).unwrap_err();
            assert!(
                matches!(raw, Error::ForeignSnapshot),
                "expected ForeignSnapshot: {raw:?}"
            );
            Ok(())
        })
        .expect("read");
    assert_eq!(db.generation().expect("generation"), 0, "nothing happened");
}

/// Scenario (d): `write_from` on a fresh witness with no intervening
/// commit behaves byte-identically to `write` — per-op verdicts (typed
/// aborts included, payloads compared), the row-id-ordered scans of
/// both relations, and the final generation.
#[test]
fn write_from_with_no_intervening_commit_is_write() {
    let descriptor = schema();
    let dir_w = TempDir::new("witness-plain");
    let dir_f = TempDir::new("witness-witnessed");
    let db_w = Db::create(dir_w.path(), descriptor.clone()).expect("create plain store");
    let db_f = Db::create(dir_f.path(), descriptor).expect("create witnessed store");

    // The same op sequence: a committing pair, a violating lone insert
    // (containment source side), a committing second pair.
    let ops = vec![
        pair(0, (1, 4), 3),
        Delta {
            deletes: vec![],
            inserts: vec![(
                BOOKING,
                vec![Value::U64(1), Value::IntervalU64(6, 9), Value::U64(8)],
            )],
        },
        pair(2, (10, 12), 5),
    ];
    for delta in &ops {
        let plain = engine_write(&db_w, delta);
        let witnessed = db_f
            .read(|snap| Ok(engine_write_from(&db_f, snap, delta)))
            .expect("read");
        // Verdict parity across the two entry points, payloads included.
        match (plain, witnessed) {
            (Verdict::Committed, ConditionalVerdict::Committed) => {}
            (Verdict::Aborted(a), ConditionalVerdict::Aborted(b)) => assert_eq!(a, b),
            other => panic!("write and write_from diverged: {other:?}"),
        }
    }

    // Byte identity: row-id-ordered scans (no str fields — decoded
    // values determine the canonical bytes) and the generation clock.
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

/// Register(slot, value) with the key (slot): the increment fixture for
/// the concurrency test.
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

/// One read-compute-write increment of slot 0, retried on
/// `GenerationMoved` — the retry loop is HOST policy, living here in
/// the test, never in the engine.
fn increment(db: &Db<SchemaDescriptor>) -> u64 {
    let mut retries = 0;
    loop {
        let attempt = db.read(|snap| {
            let mut value = None;
            for fact in snap.scan(REGISTER)? {
                let fact = fact?;
                if fact[0] == Value::U64(0) {
                    let Value::U64(current) = fact[1] else {
                        unreachable!("value is u64 by schema");
                    };
                    value = Some(current);
                }
            }
            let current = value.expect("slot 0 is seeded");
            db.write_from(snap, |tx| {
                tx.delete_dyn(REGISTER, &[Value::U64(0), Value::U64(current)])?;
                tx.insert_dyn(REGISTER, &[Value::U64(0), Value::U64(current + 1)])?;
                Ok(())
            })
        });
        match attempt {
            Ok(()) => return retries,
            Err(Error::GenerationMoved { .. }) => retries += 1,
            Err(other) => panic!("increment refused: {other:?}"),
        }
    }
}

/// The one real-concurrency test the engine permits itself: two host
/// threads interleave read-compute-write sequences over one relation;
/// with each sequence witnessed and host-retried, the final state
/// equals a serial execution of the retried schedule — 128 increments
/// land as exactly 128, no lost update representable.
#[test]
fn two_threads_of_witnessed_increments_equal_the_serial_schedule() {
    const PER_THREAD: u64 = 64;
    let dir = TempDir::new("witness-threads");
    let db = Db::create(dir.path(), register_schema()).expect("create engine store");
    db.write(|tx| {
        tx.insert_dyn(REGISTER, &[Value::U64(0), Value::U64(0)])?;
        Ok(())
    })
    .expect("seed slot 0");

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
