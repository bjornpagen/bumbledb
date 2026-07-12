//! Fixture stores with each desync class hand-injected through raw LMDB
//! writes on the environment handle — the sweeps must produce exactly
//! their finding variant, and a clean store an empty report with the
//! dictionary statistic populated. Every injected key is derived through
//! `storage::keys` (never a second slicer).

use super::*;
use crate::encoding::{encode_fact, encode_interval_u64, encode_u64, fact_hash, ValueRef};
use crate::error::Direction;
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
const CLAIM: RelationId = RelationId(3);
/// Materialized statement order: the fresh auto-FD on `Holder.id` first,
/// then the declared statements in declaration order.
const HOLDER_KEY: StatementId = StatementId(0);
const BOOKING_KEY: StatementId = StatementId(1);
const ACCOUNT_HOLDER: StatementId = StatementId(2);
const CLAIM_BOOKING: StatementId = StatementId(3);

/// Holder(id fresh, name str) — scalar key, string field for the
/// dictionary statistic; Booking(room, during) with a pointwise key;
/// Account(holder, kind) ⊆ Holder under the σ `kind == checking`;
/// Claim(room, span) ⊆ Booking(room, during) — the coverage-form
/// containment (the target's pointwise key carries the interval).
#[allow(clippy::too_many_lines)] // one descriptor literal, four relations
fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
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
                extension: None,
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
            RelationDescriptor {
                extension: None,
                name: "Claim".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "room".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "span".into(),
                        value_type: ValueType::Interval {
                            element: IntervalElement::U64,
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
            StatementDescriptor::Containment {
                source: Side {
                    relation: CLAIM,
                    projection: Box::new([FieldId(0), FieldId(1)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: BOOKING,
                    projection: Box::new([FieldId(0), FieldId(1)]),
                    selection: Box::new([]),
                },
            },
        ],
    }
}

/// A committed store: holders 1 "alice" (row 0) and 2 "bob" (row 1, then
/// deleted — "bob" is the dangling dictionary entry), bookings (7, [0,10))
/// and (7, [20,30)) at rows 0 and 1, accounts (1, checking) at row 0
/// (inside σ — one `R` edge) and (2, savings) at row 1 (outside σ — no
/// edge), and claim (7, [2,8)) at row 0 (covered by booking (7, [0,10))).
/// One insert per commit pins the row ids.
fn fixture(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), schema()).expect("create");
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
        (CLAIM, vec![Value::U64(7), Value::IntervalU64(2, 8)]),
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
fn raw_write(db: &Db<SchemaDescriptor>, f: impl FnOnce(&mut crate::storage::env::WriteTxn<'_>)) {
    let mut txn = db.env().write_txn().expect("raw txn");
    f(&mut txn);
    txn.commit().expect("raw commit");
}

fn booking_bytes(db: &Db<SchemaDescriptor>, room: u64, start: u64, end: u64) -> Vec<u8> {
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

fn account_bytes(db: &Db<SchemaDescriptor>, holder: u64, kind: u8) -> Vec<u8> {
    let mut out = Vec::new();
    encode_fact(
        &[ValueRef::U64(holder), ValueRef::Enum(kind)],
        db.schema().relation(ACCOUNT).layout(),
        &mut out,
    );
    out
}

fn claim_bytes(db: &Db<SchemaDescriptor>, room: u64, start: u64, end: u64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_fact(
        &[ValueRef::U64(room), ValueRef::IntervalU64(start, end)],
        db.schema().relation(CLAIM).layout(),
        &mut out,
    );
    out
}

fn key(write: impl FnOnce(&mut KeyBuf) -> usize) -> Vec<u8> {
    let mut buf: KeyBuf = [0; MAX_KEY];
    let len = write(&mut buf);
    buf[..len].to_vec()
}

/// Deletes one fact's `F`/`M`/`U` rows *coherently* — every namespace
/// pairing stays consistent, and the `S` row count is re-pinned to the
/// surviving cardinality — so every namespace sweep passes. This is
/// exactly the corruption class only the global judgment re-verification
/// can convict: a target gone from every namespace while a source fact
/// still requires it. (`R` rows: neither fixture target relation has
/// outgoing statements, so there are none to remove.)
fn delete_target_rows(
    db: &Db<SchemaDescriptor>,
    rel: RelationId,
    row_id: u64,
    guards: &[(StatementId, Vec<u8>)],
    remaining_rows: u64,
) {
    raw_write(db, |txn| {
        let data = txn.env().data();
        let f = key(|b| keys::fact_key(b, rel, row_id));
        let fact = data
            .get(txn.raw(), &f)
            .expect("raw get")
            .expect("live fact")
            .to_vec();
        let m = key(|b| keys::membership_key(b, rel, &fact_hash(&fact)));
        assert!(data.delete(txn.raw_mut(), &f).expect("raw delete"));
        assert!(data.delete(txn.raw_mut(), &m).expect("raw delete"));
        for (sid, guard) in guards {
            let u = key(|b| keys::guard_key(b, rel, *sid, guard));
            assert!(data.delete(txn.raw_mut(), &u).expect("raw delete"));
        }
        let count = key(|b| keys::stat_key(b, rel, StatKind::RowCount));
        data.put(
            txn.raw_mut(),
            &count,
            remaining_rows.to_le_bytes().as_slice(),
        )
        .expect("raw put");
    });
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
        vec![
            StoreFinding::FactWithoutGuard {
                relation: BOOKING,
                statement: BOOKING_KEY,
                row_id: 0,
                guard_key: u.into(),
            },
            // The deleted guard entry is also the segment covering claim
            // (7, [2,8)) — the coverage walk judges the `U` state, so the
            // desync convicts twice, once per broken invariant.
            StoreFinding::JudgmentViolation {
                statement: CLAIM_BOOKING,
                direction: Direction::TargetRequired,
                fact: claim_bytes(&db, 7, 2, 8).into(),
            },
        ]
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
fn a_coherently_deleted_scalar_target_is_a_judgment_violation() {
    let (_dir, db) = fixture("verify-judgment-scalar");
    // Holder 1 removed from every namespace at once — no namespace sweep
    // sees it, but account (1, checking) is a live source inside σ still
    // requiring it: the scalar guard probe misses.
    delete_target_rows(&db, HOLDER, 0, &[(HOLDER_KEY, encode_u64(1).to_vec())], 0);
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::JudgmentViolation {
            statement: ACCOUNT_HOLDER,
            direction: Direction::TargetRequired,
            fact: account_bytes(&db, 1, 0).into(),
        }]
    );
}

#[test]
fn a_coherently_deleted_coverage_segment_is_a_judgment_violation() {
    let (_dir, db) = fixture("verify-judgment-coverage");
    // Booking (7, [0,10)) removed from every namespace at once — booking
    // (7, [20,30)) survives, so the store stays namespace-coherent, but
    // claim (7, [2,8)) is no longer covered: the coverage walk gaps.
    delete_target_rows(
        &db,
        BOOKING,
        0,
        &[(BOOKING_KEY, booking_guard(7, 0, 10))],
        1,
    );
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::JudgmentViolation {
            statement: CLAIM_BOOKING,
            direction: Direction::TargetRequired,
            fact: claim_bytes(&db, 7, 2, 8).into(),
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

#[test]
fn a_stored_row_for_a_closed_relation_is_the_finding() {
    // Currency { minor_units: u64 } = { Usd(2) }: closed relations are
    // virtual — the store holds no rows for them — so a raw-injected `F`
    // entry is itself the one finding: the entry is exempt from every
    // coherence walk (no membership/tally convictions ride along).
    let dir = TempDir::new("verify-closed");
    let decl = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: Some(Box::new([crate::schema::Row {
                handle: "Usd".into(),
                values: Box::new([Value::U64(2)]),
            }])),
            name: "Currency".into(),
            fields: vec![FieldDescriptor {
                name: "minor_units".into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            }],
        }],
        statements: vec![],
    };
    let db = Db::create(dir.path(), decl).expect("create");
    let currency = RelationId(0);
    let fact = db.schema().relation(currency).extension().expect("closed")[0]
        .fact
        .to_vec();
    let f = key(|b| keys::fact_key(b, currency, 0));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &f, &fact).expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::ClosedRelationEntry {
            relation: currency,
            key: f.into(),
        }]
    );
}

// --- Compiled subsets (docs/prd-comptime/04-compiled-subsets.md): the
// closed-target and constant-source arms of the sweep.

/// Severity closed {pages: bool} = Low(false) | Med(true) | High(true),
/// Alert(severity) <= Severity(id) — the closed-target statement.
fn closed_subset_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(Box::new([
                    crate::schema::Row {
                        handle: "Low".into(),
                        values: Box::new([Value::Bool(false)]),
                    },
                    crate::schema::Row {
                        handle: "Med".into(),
                        values: Box::new([Value::Bool(true)]),
                    },
                    crate::schema::Row {
                        handle: "High".into(),
                        values: Box::new([Value::Bool(true)]),
                    },
                ])),
                name: "Severity".into(),
                fields: vec![FieldDescriptor {
                    name: "pages".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                }],
            },
            RelationDescriptor {
                extension: None,
                name: "Alert".into(),
                fields: vec![FieldDescriptor {
                    name: "severity".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                }],
            },
        ],
        // Materialized: Severity's closed auto-key (0), the containment (1).
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
    }
}

#[test]
fn an_r_entry_naming_a_closed_target_statement_is_the_finding() {
    // Closed-target statements never emit R traffic: a stored edge's very
    // existence is the finding, attributed to the closed target.
    let dir = TempDir::new("verify-closed-r");
    let db = Db::create(dir.path(), closed_subset_schema()).expect("create");
    db.write(|tx| tx.insert_dyn(RelationId(1), &[Value::U64(1)]).map(|_| ()))
        .expect("a legal closed reference commits");
    let r = key(|b| keys::reverse_key(b, StatementId(1), &encode_u64(1), RelationId(1), 0));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &r, &[]).expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::ClosedRelationEntry {
            relation: RelationId(0),
            key: r.into(),
        }]
    );
}

#[test]
fn a_planted_source_outside_the_member_set_is_a_judgment_violation() {
    // The corruption class only the global judgment sees: a coherent
    // F/M/S triple whose closed reference no commit could have admitted.
    let dir = TempDir::new("verify-closed-member");
    let db = Db::create(dir.path(), closed_subset_schema()).expect("create");
    let alert = RelationId(1);
    let mut fact = Vec::new();
    encode_fact(
        &[ValueRef::U64(9)],
        db.schema().relation(alert).layout(),
        &mut fact,
    );
    let f = key(|b| keys::fact_key(b, alert, 0));
    let m = key(|b| keys::membership_key(b, alert, &fact_hash(&fact)));
    let count = key(|b| keys::stat_key(b, alert, StatKind::RowCount));
    let water = key(|b| keys::stat_key(b, alert, StatKind::RowIdHighWater));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &f, &fact).expect("raw put");
        data.put(txn.raw_mut(), &m, 0u64.to_le_bytes().as_slice())
            .expect("raw put");
        data.put(txn.raw_mut(), &count, 1u64.to_le_bytes().as_slice())
            .expect("raw put");
        data.put(txn.raw_mut(), &water, 1u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::JudgmentViolation {
            statement: StatementId(1),
            direction: Direction::TargetRequired,
            fact: fact.into(),
        }]
    );
}

#[test]
fn an_uncovered_domain_quantification_is_a_judgment_violation() {
    // Severity(id) <= Handler(severity): the constant source has no F
    // rows, so only the extension-source walk can re-verify it globally —
    // an empty store violates it three times over, and covering all three
    // severities clears the report. Commit-time judgment never sees the
    // empty store (no delta touches Handler), which is exactly why the
    // sweeper owns this class.
    let dir = TempDir::new("verify-closed-domain");
    let mut decl = closed_subset_schema();
    decl.relations.push(RelationDescriptor {
        extension: None,
        name: "Handler".into(),
        fields: vec![
            FieldDescriptor {
                name: "severity".into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            },
            FieldDescriptor {
                name: "priority".into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            },
        ],
    });
    decl.statements.insert(
        0,
        StatementDescriptor::Functionality {
            relation: RelationId(2),
            projection: Box::new([FieldId(0)]),
        },
    );
    decl.statements.push(StatementDescriptor::Containment {
        source: Side {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
            selection: Box::new([]),
        },
        target: Side {
            relation: RelationId(2),
            projection: Box::new([FieldId(0)]),
            selection: Box::new([]),
        },
    });
    // Materialized: closed auto-key (0), Handler key (1), Alert
    // containment (2), the domain statement (3).
    let db = Db::create(dir.path(), decl).expect("create");
    let severities = db
        .schema()
        .relation(RelationId(0))
        .extension()
        .expect("closed");
    let expected: Vec<StoreFinding> = severities
        .iter()
        .map(|row| StoreFinding::JudgmentViolation {
            statement: StatementId(3),
            direction: Direction::TargetRequired,
            fact: row.fact.clone(),
        })
        .collect();
    assert_eq!(db.verify_store().expect("verify").findings, expected);
    for severity in 0..3u64 {
        db.write(|tx| {
            tx.insert_dyn(RelationId(2), &[Value::U64(severity), Value::U64(10)])
                .map(|_| ())
        })
        .expect("handlers commit");
    }
    assert_eq!(db.verify_store().expect("verify").findings, vec![]);
}
