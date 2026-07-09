//! Fixture stores with each desync class hand-injected through raw LMDB
//! writes on the environment handle — the sweeps must produce exactly
//! their finding variant, and a clean store an empty report with the
//! dictionary statistic populated. Every injected key is derived through
//! `storage::keys` (never a second slicer).

use super::*;
use crate::encoding::{encode_fact, encode_interval_u64, encode_u64, fact_hash, ValueRef};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor,
    Side, StatementDescriptor, ValueType,
};
use crate::storage::keys::{KeyBuf, StatKind, MAX_KEY};
use crate::testutil::TempDir;
use crate::value::Value;

const HOLDER: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const ACCOUNT: RelationId = RelationId(2);
/// Materialized statement order: the serial auto-FD on `Holder.id` first,
/// then the declared statements in declaration order.
const BOOKING_KEY: StatementId = StatementId(1);
const ACCOUNT_HOLDER: StatementId = StatementId(2);

/// Holder(id serial, name str) — scalar key, string field for the
/// dictionary statistic; Booking(room, during) with a pointwise key;
/// Account(holder, kind) ⊆ Holder under the σ `kind == checking`.
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                name: "Booking".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "room".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "holder".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "kind".into(),
                        value_type: ValueType::Enum {
                            variants: Box::new(["checking".into(), "savings".into()]),
                        },
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: BOOKING,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(FieldId(1), Value::Enum(0))]),
                },
                target: Side {
                    relation: HOLDER,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

/// A committed store: holders 1 "alice" (row 0) and 2 "bob" (row 1, then
/// deleted — "bob" is the dangling dictionary entry), bookings (7, [0,10))
/// and (7, [20,30)) at rows 0 and 1, accounts (1, checking) at row 0
/// (inside σ — one `R` edge) and (2, savings) at row 1 (outside σ — no
/// edge). One insert per commit pins the row ids.
fn fixture(tag: &str) -> (TempDir, Db<'static>) {
    let schema = Box::leak(Box::new(schema()));
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), schema).expect("create");
    let facts: &[(RelationId, Vec<Value>)] = &[
        (
            HOLDER,
            vec![Value::U64(1), Value::String("alice".as_bytes().into())],
        ),
        (
            HOLDER,
            vec![Value::U64(2), Value::String("bob".as_bytes().into())],
        ),
        (BOOKING, vec![Value::U64(7), Value::IntervalU64(0, 10)]),
        (BOOKING, vec![Value::U64(7), Value::IntervalU64(20, 30)]),
        (ACCOUNT, vec![Value::U64(1), Value::Enum(0)]),
        (ACCOUNT, vec![Value::U64(2), Value::Enum(1)]),
    ];
    for (rel, values) in facts {
        db.write(|tx| tx.insert_dyn(*rel, values).map(|_| ()))
            .expect("insert");
    }
    db.write(|tx| {
        tx.delete_dyn(
            HOLDER,
            &[Value::U64(2), Value::String("bob".as_bytes().into())],
        )
        .map(|_| ())
    })
    .expect("delete");
    (dir, db)
}

/// The test-only raw-write handle: one LMDB write transaction over the
/// open environment, bypassing the delta — exactly the desync injector
/// the sweeps exist to catch.
fn raw_write(db: &Db<'_>, f: impl FnOnce(&mut crate::storage::env::WriteTxn<'_>)) {
    let mut txn = db.env().write_txn().expect("raw txn");
    f(&mut txn);
    txn.commit().expect("raw commit");
}

fn booking_bytes(db: &Db<'_>, room: u64, start: u64, end: u64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_fact(
        &[ValueRef::U64(room), ValueRef::IntervalU64(start, end)],
        db.schema().relation(BOOKING).layout(),
        &mut out,
    );
    out
}

/// `enc(room) ‖ enc(start ‖ end)` — the Booking key statement's guard.
fn booking_guard(room: u64, start: u64, end: u64) -> Vec<u8> {
    let mut guard = Vec::new();
    guard.extend_from_slice(&encode_u64(room));
    guard.extend_from_slice(&encode_interval_u64(start, end));
    guard
}

fn key(write: impl FnOnce(&mut KeyBuf) -> usize) -> Vec<u8> {
    let mut buf: KeyBuf = [0; MAX_KEY];
    let len = write(&mut buf);
    buf[..len].to_vec()
}

#[test]
fn clean_store_reports_nothing_and_counts_the_leak() {
    let (_dir, db) = fixture("verify-clean");
    let report = db.verify_store().expect("verify");
    assert_eq!(report.findings, Vec::new());
    // "bob" was interned, then its one referencing fact deleted: the
    // accepted leak, counted, never a finding.
    assert_eq!(report.dangling_intern_ids, 1);
}

#[test]
fn missing_membership_is_found_from_the_fact_side() {
    let (_dir, db) = fixture("verify-missing-m");
    let fact = booking_bytes(&db, 7, 0, 10);
    let m = key(|b| keys::membership_key(b, BOOKING, &fact_hash(&fact)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        assert!(data.delete(txn.raw_mut(), &m).expect("raw delete"));
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::FactWithoutMembership {
            relation: BOOKING,
            row_id: 0,
            membership_key: m.into(),
        }]
    );
}

#[test]
fn orphan_membership_is_found_from_the_entry_side() {
    let (_dir, db) = fixture("verify-orphan-m");
    let m = key(|b| keys::membership_key(b, BOOKING, &[0xAB; 32]));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &m, 99u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::MembershipWithoutFact {
            relation: BOOKING,
            row_id: 99,
            membership_key: m.into(),
        }]
    );
}

#[test]
fn missing_guard_is_found_from_the_fact_side() {
    let (_dir, db) = fixture("verify-missing-u");
    let u = key(|b| keys::guard_key(b, BOOKING, BOOKING_KEY, &booking_guard(7, 0, 10)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        assert!(data.delete(txn.raw_mut(), &u).expect("raw delete"));
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::FactWithoutGuard {
            relation: BOOKING,
            statement: BOOKING_KEY,
            row_id: 0,
            guard_key: u.into(),
        }]
    );
}

#[test]
fn orphan_guard_is_found_from_the_entry_side() {
    let (_dir, db) = fixture("verify-orphan-u");
    // A guard for a fact that does not exist, pointing at a row that
    // does not exist either.
    let u = key(|b| keys::guard_key(b, BOOKING, BOOKING_KEY, &booking_guard(99, 0, 10)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &u, 42u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::GuardWithoutFact {
            relation: BOOKING,
            statement: BOOKING_KEY,
            guard_key: u.into(),
        }]
    );
}

#[test]
fn pointwise_overlap_is_found_by_the_ordered_walk() {
    let (_dir, db) = fixture("verify-pointwise-overlap");
    // A fully consistent third booking (7, [5, 15)) injected raw — F, M,
    // U, and both S counters all coherent — whose only defect is
    // overlapping (7, [0, 10)): the invariant no namespace pairing sees,
    // only the ordered walk.
    let fact = booking_bytes(&db, 7, 5, 15);
    let row_id = 2u64;
    let f = key(|b| keys::fact_key(b, BOOKING, row_id));
    let m = key(|b| keys::membership_key(b, BOOKING, &fact_hash(&fact)));
    let u = key(|b| keys::guard_key(b, BOOKING, BOOKING_KEY, &booking_guard(7, 5, 15)));
    let count = key(|b| keys::stat_key(b, BOOKING, StatKind::RowCount));
    let water = key(|b| keys::stat_key(b, BOOKING, StatKind::RowIdHighWater));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &f, &fact).expect("raw put");
        data.put(txn.raw_mut(), &m, row_id.to_le_bytes().as_slice())
            .expect("raw put");
        data.put(txn.raw_mut(), &u, row_id.to_le_bytes().as_slice())
            .expect("raw put");
        data.put(txn.raw_mut(), &count, 3u64.to_le_bytes().as_slice())
            .expect("raw put");
        data.put(txn.raw_mut(), &water, 3u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::PointwiseOverlap {
            relation: BOOKING,
            statement: BOOKING_KEY,
            first: key(|b| keys::guard_key(b, BOOKING, BOOKING_KEY, &booking_guard(7, 0, 10)))
                .into(),
            second: u.into(),
        }]
    );
}

#[test]
fn missing_reverse_edge_is_found_from_the_fact_side() {
    let (_dir, db) = fixture("verify-missing-r");
    let r = key(|b| keys::reverse_key(b, ACCOUNT_HOLDER, &encode_u64(1), ACCOUNT, 0));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        assert!(data.delete(txn.raw_mut(), &r).expect("raw delete"));
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::FactWithoutReverseEdge {
            statement: ACCOUNT_HOLDER,
            relation: ACCOUNT,
            row_id: 0,
            reverse_key: r.into(),
        }]
    );
}

#[test]
fn orphan_reverse_edge_is_found_from_the_edge_side() {
    let (_dir, db) = fixture("verify-orphan-r");
    let r = key(|b| keys::reverse_key(b, ACCOUNT_HOLDER, &encode_u64(9), ACCOUNT, 77));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &r, &[]).expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::ReverseEdgeWithoutFact {
            statement: ACCOUNT_HOLDER,
            reverse_key: r.into(),
        }]
    );
}

#[test]
fn edge_whose_source_left_its_selection_is_an_orphan() {
    let (_dir, db) = fixture("verify-orphan-r-phi");
    // An edge for the savings account (row 1) — the fact is live and
    // re-derives the key bytes, but sits outside σ: φ is re-checked,
    // not just liveness.
    let r = key(|b| keys::reverse_key(b, ACCOUNT_HOLDER, &encode_u64(2), ACCOUNT, 1));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &r, &[]).expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::ReverseEdgeWithoutFact {
            statement: ACCOUNT_HOLDER,
            reverse_key: r.into(),
        }]
    );
}

#[test]
fn wrong_row_count_is_found_against_the_scan() {
    let (_dir, db) = fixture("verify-wrong-s");
    let count = key(|b| keys::stat_key(b, BOOKING, StatKind::RowCount));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &count, 99u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::RowCountDesync {
            relation: BOOKING,
            stored: 99,
            counted: 2,
        }]
    );
}

#[test]
fn low_high_water_is_found_against_the_max_row_id() {
    let (_dir, db) = fixture("verify-low-water");
    let water = key(|b| keys::stat_key(b, BOOKING, StatKind::RowIdHighWater));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &water, 0u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::RowIdHighWaterLow {
            relation: BOOKING,
            stored: 0,
            max_row_id: 1,
        }]
    );
}
