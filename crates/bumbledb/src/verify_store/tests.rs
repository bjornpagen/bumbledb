//! Fixture stores with each desync class hand-injected through raw LMDB
//! writes on the environment handle — the sweeps must produce exactly
//! their finding variant, and a clean store an empty report with the
//! dictionary statistic populated. Every injected key is derived through
//! `storage::keys` (never a second slicer).

use super::*;
use crate::encoding::{ValueRef, encode_fact, encode_interval_u64, encode_u64, fact_hash};
use crate::error::Direction;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor,
    Side, StatementDescriptor, ValueType,
};
use crate::storage::keys::{StatKind, key};
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
/// Account(holder, kind) ⊆ Holder under the σ `kind == 0` (checking);
/// Claim(room, span) ⊆ Booking(room, during) — the coverage-form
/// containment (the target's pointwise key carries the interval).
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one descriptor literal, four relations
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
                        value_type: ValueType::U64,
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
                    selection: Box::new([(FieldId(1), Value::U64(0))]),
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
        (
            BOOKING,
            vec![
                Value::U64(7),
                Value::IntervalU64(crate::Interval::<u64>::new(0, 10).expect("nonempty interval")),
            ],
        ),
        (
            BOOKING,
            vec![
                Value::U64(7),
                Value::IntervalU64(crate::Interval::<u64>::new(20, 30).expect("nonempty interval")),
            ],
        ),
        (ACCOUNT, vec![Value::U64(1), Value::U64(0)]),
        (ACCOUNT, vec![Value::U64(2), Value::U64(1)]),
        (
            CLAIM,
            vec![
                Value::U64(7),
                Value::IntervalU64(crate::Interval::<u64>::new(2, 8).expect("nonempty interval")),
            ],
        ),
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

/// Every newly added corruption fixture is paired with the same populated
/// store left untouched. This makes the raw injector's no-false-positive
/// control explicit rather than relying only on the suite's general clean
/// fixture.
fn fixture_with_healthy_sibling(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let control_tag = format!("{tag}-control");
    let (_control_dir, control) = fixture(&control_tag);
    assert_eq!(
        control
            .verify_store()
            .expect("verify healthy sibling")
            .findings,
        vec![]
    );
    fixture(tag)
}

fn canonical_field_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Canonical".into(),
            fields: vec![
                FieldDescriptor {
                    name: "flag".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "digest".into(),
                    value_type: ValueType::FixedBytes { len: 5 },
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
        }],
        statements: vec![],
    }
}

fn canonical_field_fixture(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), canonical_field_schema()).expect("create canonical store");
    db.write(|tx| {
        tx.insert_dyn(
            RelationId(0),
            &[
                Value::Bool(true),
                Value::FixedBytes(vec![1, 2, 3, 4, 5].into_boxed_slice()),
                Value::IntervalU64(crate::Interval::<u64>::new(10, 20).expect("nonempty interval")),
            ],
        )
        .map(|_| ())
    })
    .expect("insert canonical fact");
    (dir, db)
}

fn canonical_field_fixture_with_healthy_sibling(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let control_tag = format!("{tag}-control");
    let (_control_dir, control) = canonical_field_fixture(&control_tag);
    assert_eq!(
        control
            .verify_store()
            .expect("verify healthy sibling")
            .findings,
        vec![]
    );
    canonical_field_fixture(tag)
}

/// The test-only raw-write handle: one LMDB write transaction over the
/// open environment, bypassing the delta — exactly the desync injector
/// the sweeps exist to catch.
fn raw_write(db: &Db<SchemaDescriptor>, f: impl FnOnce(&mut crate::storage::env::WriteTxn<'_>)) {
    let mut txn = db.env().write_txn().expect("raw txn");
    f(&mut txn);
    txn.commit().expect("raw commit");
}

/// Replaces one F value while keeping its M image coherent. Callers choose a
/// field that is not projected by U, or use a relation with no keys.
fn replace_fact_bytes(
    db: &Db<SchemaDescriptor>,
    rel: RelationId,
    row_id: u64,
    mutate: impl FnOnce(&mut Vec<u8>),
) {
    raw_write(db, |txn| {
        let data = txn.env().data();
        let f = key(|b| keys::fact_key(b, rel, row_id));
        let mut fact = data
            .get(txn.raw(), &f)
            .expect("raw get")
            .expect("live fact")
            .to_vec();
        let old_m = key(|b| keys::membership_key(b, rel, &fact_hash(&fact)));
        mutate(&mut fact);
        let new_m = key(|b| keys::membership_key(b, rel, &fact_hash(&fact)));
        assert!(data.delete(txn.raw_mut(), &old_m).expect("delete old M"));
        data.put(txn.raw_mut(), &f, &fact).expect("replace F");
        data.put(txn.raw_mut(), &new_m, row_id.to_le_bytes().as_slice())
            .expect("replace M");
    });
}

fn booking_bytes(db: &Db<SchemaDescriptor>, room: u64, start: u64, end: u64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(room),
            ValueRef::IntervalU64(
                crate::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ],
        db.schema().relation(BOOKING).layout(),
        &mut out,
    );
    out
}

/// `enc(room) ‖ enc(start ‖ end)` — the Booking key statement's determinant.
fn booking_determinant(room: u64, start: u64, end: u64) -> Vec<u8> {
    let mut determinant = Vec::new();
    determinant.extend_from_slice(&encode_u64(room));
    determinant.extend_from_slice(&encode_interval_u64(
        crate::Interval::<u64>::new(start, end).expect("nonempty interval"),
    ));
    determinant
}

fn account_bytes(db: &Db<SchemaDescriptor>, holder: u64, kind: u64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_fact(
        &[ValueRef::U64(holder), ValueRef::U64(kind)],
        db.schema().relation(ACCOUNT).layout(),
        &mut out,
    );
    out
}

fn claim_bytes(db: &Db<SchemaDescriptor>, room: u64, start: u64, end: u64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(room),
            ValueRef::IntervalU64(
                crate::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ],
        db.schema().relation(CLAIM).layout(),
        &mut out,
    );
    out
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
    determinants: &[(StatementId, Vec<u8>)],
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
        for (sid, determinant) in determinants {
            let u = key(|b| keys::determinant_key(b, rel, *sid, determinant));
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
fn malformed_keys_in_every_swept_namespace_are_contextual_findings() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-malformed-namespaces");
    let keys = [
        vec![keys::NS_FACT],
        vec![keys::NS_MEMBERSHIP],
        vec![keys::NS_DETERMINANT],
        vec![keys::NS_REVERSE],
        vec![keys::NS_STAT],
    ];
    raw_write(&db, |txn| {
        let data = txn.env().data();
        for key in &keys {
            data.put(txn.raw_mut(), key, &[])
                .expect("plant malformed key");
        }
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![
            StoreFinding::Malformed {
                key: keys[0].clone().into(),
                what: "F key length",
            },
            StoreFinding::Malformed {
                key: keys[1].clone().into(),
                what: "M key length",
            },
            StoreFinding::Malformed {
                key: keys[2].clone().into(),
                what: "U key length",
            },
            StoreFinding::Malformed {
                key: keys[3].clone().into(),
                what: "R key shape",
            },
            StoreFinding::Malformed {
                key: keys[4].clone().into(),
                what: "S key length",
            },
        ]
    );
}

#[test]
fn namespace_schema_ownership_is_rechecked() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-namespace-ownership");
    let unknown = RelationId(99);
    let f = key(|b| keys::fact_key(b, unknown, 0));
    let m = key(|b| keys::membership_key(b, unknown, &[0x11; 32]));
    let u_wrong_statement = key(|b| keys::determinant_key(b, HOLDER, BOOKING_KEY, &encode_u64(1)));
    let u_unknown_relation = key(|b| keys::determinant_key(b, unknown, HOLDER_KEY, &encode_u64(1)));
    let r_wrong_source = key(|b| keys::reverse_key(b, ACCOUNT_HOLDER, &encode_u64(1), HOLDER, 0));
    let r_unknown_statement =
        key(|b| keys::reverse_key(b, StatementId(99), &encode_u64(1), ACCOUNT, 0));
    let s = key(|b| keys::stat_key(b, unknown, StatKind::RowCount));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        for key in [
            &f,
            &m,
            &u_wrong_statement,
            &u_unknown_relation,
            &r_wrong_source,
            &r_unknown_statement,
            &s,
        ] {
            data.put(txn.raw_mut(), key, &[])
                .expect("plant foreign key");
        }
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![
            StoreFinding::Malformed {
                key: f.into(),
                what: "F key relation",
            },
            StoreFinding::Malformed {
                key: m.into(),
                what: "M key relation",
            },
            StoreFinding::Malformed {
                key: u_wrong_statement.into(),
                what: "U key statement",
            },
            StoreFinding::Malformed {
                key: u_unknown_relation.into(),
                what: "U key relation",
            },
            StoreFinding::Malformed {
                key: r_wrong_source.into(),
                what: "R key source relation",
            },
            StoreFinding::Malformed {
                key: r_unknown_statement.into(),
                what: "R key statement",
            },
            StoreFinding::Malformed {
                key: s.into(),
                what: "S key relation",
            },
        ]
    );
}

#[test]
fn namespace_row_images_are_width_checked() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-namespace-values");
    let m = key(|b| keys::membership_key(b, BOOKING, &[0x22; 32]));
    let u = key(|b| keys::determinant_key(b, HOLDER, HOLDER_KEY, &encode_u64(99)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &m, &[])
            .expect("plant malformed M value");
        data.put(txn.raw_mut(), &u, &[])
            .expect("plant malformed U value");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![
            StoreFinding::Malformed {
                key: m.into(),
                what: "M row id",
            },
            StoreFinding::Malformed {
                key: u.into(),
                what: "U row id",
            },
        ]
    );
}

#[test]
fn counter_value_and_stat_kind_are_width_and_domain_checked() {
    let decl = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Empty".into(),
            fields: vec![],
        }],
        statements: vec![],
    };
    let control_dir = TempDir::new("verify-s-shape-control");
    let control = Db::create(control_dir.path(), decl.clone()).expect("create control");
    assert_eq!(
        control
            .verify_store()
            .expect("verify healthy sibling")
            .findings,
        vec![]
    );
    let dir = TempDir::new("verify-s-shape");
    let db = Db::create(dir.path(), decl).expect("create");
    let malformed_value = key(|b| keys::stat_key(b, RelationId(0), StatKind::RowCount));
    let mut unknown_kind = key(|b| keys::stat_key(b, RelationId(0), StatKind::RowIdHighWater));
    *unknown_kind.last_mut().expect("stat kind") = 9;
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &malformed_value, &[])
            .expect("plant malformed counter");
        data.put(txn.raw_mut(), &unknown_kind, 0u64.to_le_bytes().as_slice())
            .expect("plant unknown stat");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![
            StoreFinding::Malformed {
                key: malformed_value.into(),
                what: "S counter value",
            },
            StoreFinding::Malformed {
                key: unknown_kind.into(),
                what: "S stat kind",
            },
        ]
    );
}

#[test]
fn wrong_fact_width_is_a_contextual_finding() {
    let (_dir, db) = canonical_field_fixture_with_healthy_sibling("verify-wrong-fact-width");
    replace_fact_bytes(&db, RelationId(0), 0, |fact| {
        fact.pop();
    });
    let f = key(|b| keys::fact_key(b, RelationId(0), 0));
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![StoreFinding::Malformed {
            key: f.into(),
            what: "F fact width",
        }]
    );
}

#[test]
fn noncanonical_field_encodings_are_each_found() {
    let (_dir, db) = canonical_field_fixture_with_healthy_sibling("verify-field-encodings");
    replace_fact_bytes(&db, RelationId(0), 0, |fact| {
        // bool byte; bytes<5>'s third pad byte; equal interval halves.
        fact[0] = 2;
        fact[8] = 1;
        fact[9..17].copy_from_slice(&10u64.to_be_bytes());
        fact[17..25].copy_from_slice(&10u64.to_be_bytes());
    });
    let f = key(|b| keys::fact_key(b, RelationId(0), 0));
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![
            StoreFinding::Malformed {
                key: f.clone().into(),
                what: "F fact bool",
            },
            StoreFinding::Malformed {
                key: f.clone().into(),
                what: "F fact fixed bytes padding",
            },
            StoreFinding::Malformed {
                key: f.into(),
                what: "F fact interval",
            },
        ]
    );
}

#[test]
fn intern_id_at_or_beyond_the_counter_is_found_with_fact_context() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-intern-bound");
    replace_fact_bytes(&db, HOLDER, 0, |fact| {
        fact[8..16].copy_from_slice(&99u64.to_be_bytes());
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![StoreFinding::InternBeyondNextId {
            relation: HOLDER,
            row_id: 0,
            intern_id: 99,
            next_id: 2,
        }]
    );
}

#[test]
fn malformed_dictionary_reverse_key_is_a_finding() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-malformed-dict-reverse");
    let malformed = [1u8, 7];
    raw_write(&db, |txn| {
        txn.env()
            .dict()
            .put(txn.raw_mut(), &malformed, b"bad")
            .expect("plant malformed reverse key");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![StoreFinding::Malformed {
            key: malformed.into(),
            what: "dict reverse id",
        }]
    );
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
fn missing_determinant_is_found_from_the_fact_side() {
    let (_dir, db) = fixture("verify-missing-u");
    let u = key(|b| keys::determinant_key(b, BOOKING, BOOKING_KEY, &booking_determinant(7, 0, 10)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        assert!(data.delete(txn.raw_mut(), &u).expect("raw delete"));
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![
            StoreFinding::FactWithoutDeterminant {
                relation: BOOKING,
                statement: BOOKING_KEY,
                row_id: 0,
                determinant_key: u.into(),
            },
            // The deleted determinant entry is also the segment covering claim
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
fn orphan_determinant_is_found_from_the_entry_side() {
    let (_dir, db) = fixture("verify-orphan-u");
    // A determinant for a fact that does not exist, pointing at a row that
    // does not exist either.
    let u =
        key(|b| keys::determinant_key(b, BOOKING, BOOKING_KEY, &booking_determinant(99, 0, 10)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &u, 42u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings,
        vec![StoreFinding::DeterminantWithoutFact {
            relation: BOOKING,
            statement: BOOKING_KEY,
            determinant_key: u.into(),
        }]
    );
}

#[test]
fn determinant_key_byte_flip_is_found_against_the_live_fact() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-u-key-image");
    // Holder row 0 re-derives determinant 1. Keep its correct U entry and
    // plant a one-byte-perturbed determinant 3 pointing at the same live row.
    let mut determinant = encode_u64(1).to_vec();
    *determinant.last_mut().expect("word") ^= 2;
    let u = key(|b| keys::determinant_key(b, HOLDER, HOLDER_KEY, &determinant));
    raw_write(&db, |txn| {
        txn.env()
            .data()
            .put(txn.raw_mut(), &u, 0u64.to_le_bytes().as_slice())
            .expect("plant perturbed U key");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![StoreFinding::DeterminantWithoutFact {
            relation: HOLDER,
            statement: HOLDER_KEY,
            determinant_key: u.into(),
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
    let u = key(|b| keys::determinant_key(b, BOOKING, BOOKING_KEY, &booking_determinant(7, 5, 15)));
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
            first: key(|b| keys::determinant_key(
                b,
                BOOKING,
                BOOKING_KEY,
                &booking_determinant(7, 0, 10)
            ))
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
    // requiring it: the scalar determinant probe misses.
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
        &[(BOOKING_KEY, booking_determinant(7, 0, 10))],
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
fn reverse_key_byte_flip_is_found_against_the_live_source() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-r-key-image");
    // Account row 0 requires holder 1. Its correct edge stays present; this
    // perturbed key names holder 3 but still points at that live source row.
    let r = key(|b| keys::reverse_key(b, ACCOUNT_HOLDER, &encode_u64(3), ACCOUNT, 0));
    raw_write(&db, |txn| {
        txn.env()
            .data()
            .put(txn.raw_mut(), &r, &[])
            .expect("plant perturbed R key");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
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
fn absent_counters_are_found_against_the_fact_tally() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-absent-counters");
    let count = key(|b| keys::stat_key(b, CLAIM, StatKind::RowCount));
    let water = key(|b| keys::stat_key(b, CLAIM, StatKind::RowIdHighWater));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        assert!(
            data.delete(txn.raw_mut(), &count)
                .expect("delete row count")
        );
        assert!(
            data.delete(txn.raw_mut(), &water)
                .expect("delete high-water")
        );
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![
            StoreFinding::RowCountDesync {
                relation: CLAIM,
                stored: 0,
                counted: 1,
            },
            StoreFinding::RowIdHighWaterLow {
                relation: CLAIM,
                stored: 0,
                max_row_id: 0,
            },
        ]
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

#[test]
fn membership_and_determinant_entries_for_a_closed_relation_are_findings() {
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
    let control_dir = TempDir::new("verify-closed-m-u-control");
    let control = Db::create(control_dir.path(), decl.clone()).expect("create control");
    assert_eq!(
        control
            .verify_store()
            .expect("verify healthy sibling")
            .findings,
        vec![]
    );

    let dir = TempDir::new("verify-closed-m-u");
    let db = Db::create(dir.path(), decl).expect("create");
    let currency = RelationId(0);
    let fact = &db.schema().relation(currency).extension().expect("closed")[0].fact;
    let m = key(|b| keys::membership_key(b, currency, &fact_hash(fact)));
    let u = key(|b| keys::determinant_key(b, currency, StatementId(0), &encode_u64(0)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &m, 0u64.to_le_bytes().as_slice())
            .expect("plant M");
        data.put(txn.raw_mut(), &u, 0u64.to_le_bytes().as_slice())
            .expect("plant U");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings,
        vec![
            StoreFinding::ClosedRelationEntry {
                relation: currency,
                key: m.into(),
            },
            StoreFinding::ClosedRelationEntry {
                relation: currency,
                key: u.into(),
            },
        ]
    );
}

// --- Compiled subsets (docs/architecture/30-dependencies.md): the
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
