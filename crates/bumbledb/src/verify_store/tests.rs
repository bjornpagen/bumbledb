//! Fixture stores with each desync class hand-injected through raw LMDB
use super::*;
use crate::encoding::{
    InternId, ValueRef, encode_fact, encode_interval_u64, encode_u64, fact_hash,
};
use crate::error::{CorruptionError, Direction, Violation};
use crate::schema::Schema;
use crate::storage::keys::{StatKind, key};
use crate::testutil::TempDir;
use bumbledb_theory::Value;
use bumbledb_theory::schema::StatementId;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor,
    Side, StatementDescriptor, ValueType,
};

const HOLDER: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const ACCOUNT: RelationId = RelationId(2);
const CLAIM: RelationId = RelationId(3);

const HOLDER_KEY: StatementId = StatementId(0);
const BOOKING_KEY: StatementId = StatementId(1);
const ACCOUNT_HOLDER: StatementId = StatementId(2);
const CLAIM_BOOKING: StatementId = StatementId(3);

fn judgment_containment(schema: &Schema, statement: StatementId, fact: Box<[u8]>) -> StoreFinding {
    StoreFinding::Judgment(Violation::containment(
        schema.cite(statement),
        Direction::TargetRequired,
        fact,
    ))
}

fn judgment_capacity(
    schema: &Schema,
    statement: StatementId,
    fact: Box<[u8]>,
    measure: u128,
) -> StoreFinding {
    StoreFinding::Judgment(Violation::capacity(schema.cite(statement), fact, measure))
}

#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
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
                    selection: Box::new([(
                        FieldId(1),
                        bumbledb_theory::schema::LiteralSet::One(Value::U64(0)),
                    )]),
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

fn fixture(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), schema())
        .expect("create")
        .expect("accepted");
    let facts: &[(RelationId, Vec<Value>)] = &[
        (HOLDER, vec![Value::U64(1), Value::String("alice".into())]),
        (HOLDER, vec![Value::U64(2), Value::String("bob".into())]),
        (
            BOOKING,
            vec![
                Value::U64(7),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(0, 10).expect("nonempty interval"),
                ),
            ],
        ),
        (
            BOOKING,
            vec![
                Value::U64(7),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(20, 30).expect("nonempty interval"),
                ),
            ],
        ),
        (ACCOUNT, vec![Value::U64(1), Value::U64(0)]),
        (ACCOUNT, vec![Value::U64(2), Value::U64(1)]),
        (
            CLAIM,
            vec![
                Value::U64(7),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(2, 8).expect("nonempty interval"),
                ),
            ],
        ),
    ];
    for (rel, values) in facts {
        db.write(|tx| tx.insert_dyn(*rel, [values]).map(|_| ()))
            .expect("insert")
            .unwrap();
    }
    db.write(|tx| {
        tx.delete_dyn(HOLDER, [&[Value::U64(2), Value::String("bob".into())]])
            .map(|_| ())
    })
    .expect("delete")
    .unwrap();
    (dir, db)
}

fn fixture_with_healthy_sibling(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let control_tag = format!("{tag}-control");
    let (_control_dir, control) = fixture(&control_tag);
    assert_eq!(
        control
            .verify_store()
            .expect("verify healthy sibling")
            .findings()
            .to_vec(),
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
    let db = Db::create(dir.path(), canonical_field_schema())
        .expect("create canonical store")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(
            RelationId(0),
            [&[
                Value::Bool(true),
                Value::FixedBytes(vec![1, 2, 3, 4, 5].into_boxed_slice()),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(10, 20).expect("nonempty interval"),
                ),
            ]],
        )
        .map(|_| ())
    })
    .expect("insert canonical fact")
    .unwrap();
    (dir, db)
}

fn canonical_field_fixture_with_healthy_sibling(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let control_tag = format!("{tag}-control");
    let (_control_dir, control) = canonical_field_fixture(&control_tag);
    assert_eq!(
        control
            .verify_store()
            .expect("verify healthy sibling")
            .findings()
            .to_vec(),
        vec![]
    );
    canonical_field_fixture(tag)
}

fn raw_write(db: &Db<SchemaDescriptor>, f: impl FnOnce(&mut crate::storage::env::WriteTxn<'_>)) {
    let mut txn = db.env().write_txn().expect("raw txn");
    f(&mut txn);
    txn.commit().expect("raw commit");
}

fn replace_fact_bytes(
    db: &Db<SchemaDescriptor>,
    rel: RelationId,
    row_id: u64,
    mutate: impl FnOnce(&mut Vec<u8>),
) {
    raw_write(db, |txn| {
        let data = txn.env().data();
        let f = keys::fact_key(rel, row_id).to_vec();
        let mut fact = data
            .get(txn.raw(), &f)
            .expect("raw get")
            .expect("live fact")
            .to_vec();
        let old_m = keys::membership_key(rel, &fact_hash(&fact)).to_vec();
        mutate(&mut fact);
        let new_m = keys::membership_key(rel, &fact_hash(&fact)).to_vec();
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
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ],
        db.schema().relation(BOOKING).layout(),
        &mut out,
    );
    out
}

fn booking_determinant(room: u64, start: u64, end: u64) -> Vec<u8> {
    let mut determinant = Vec::new();
    determinant.extend_from_slice(&encode_u64(room));
    determinant.extend_from_slice(&encode_interval_u64(
        bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
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
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ],
        db.schema().relation(CLAIM).layout(),
        &mut out,
    );
    out
}

fn delete_target_rows(
    db: &Db<SchemaDescriptor>,
    rel: RelationId,
    row_id: u64,
    determinants: &[(StatementId, Vec<u8>)],
    remaining_rows: u64,
) {
    raw_write(db, |txn| {
        let data = txn.env().data();
        let f = keys::fact_key(rel, row_id).to_vec();
        let fact = data
            .get(txn.raw(), &f)
            .expect("raw get")
            .expect("live fact")
            .to_vec();
        let m = keys::membership_key(rel, &fact_hash(&fact)).to_vec();
        assert!(data.delete(txn.raw_mut(), &f).expect("raw delete"));
        assert!(data.delete(txn.raw_mut(), &m).expect("raw delete"));
        for (sid, determinant) in determinants {
            let u = key(|b| keys::determinant_key(b, rel, *sid, determinant));
            assert!(data.delete(txn.raw_mut(), &u).expect("raw delete"));
        }
        let count = keys::stat_key(rel, StatKind::RowCount).to_vec();
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
    assert_eq!(report.findings().to_vec(), Vec::new());

    assert_eq!(report.dangling_intern_ids(), 1);
}

#[test]
fn malformed_keys_in_every_swept_namespace_are_contextual_findings() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-malformed-namespaces");
    let keys = [
        vec![keys::Namespace::Fact.tag()],
        vec![keys::Namespace::Membership.tag()],
        vec![keys::Namespace::Determinant.tag()],
        vec![keys::Namespace::Reverse.tag()],
        vec![keys::Namespace::Stat.tag()],
        vec![keys::Namespace::Fresh.tag()],
    ];
    raw_write(&db, |txn| {
        let data = txn.env().data();
        for key in &keys {
            data.put(txn.raw_mut(), key, &[])
                .expect("plant malformed key");
        }
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: keys[0].clone().into(),
                what: "F key length",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: keys[1].clone().into(),
                what: "M key length",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: keys[2].clone().into(),
                what: "U key length",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: keys[3].clone().into(),
                what: "R key shape",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: keys[4].clone().into(),
                what: "S key length",
            }),
            // The Q pass runs after the counters (pass order).
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: keys[5].clone().into(),
                what: "Q key length",
            }),
        ]
    );
}

#[test]
fn namespace_schema_ownership_is_rechecked() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-namespace-ownership");
    let unknown = RelationId(99);
    let f = keys::fact_key(unknown, 0).to_vec();
    let m = keys::membership_key(unknown, &[0x11; 32]).to_vec();
    let u_wrong_statement = key(|b| keys::determinant_key(b, HOLDER, BOOKING_KEY, &encode_u64(1)));
    let u_unknown_relation = key(|b| keys::determinant_key(b, unknown, HOLDER_KEY, &encode_u64(1)));
    let r_wrong_source = key(|b| keys::reverse_key(b, ACCOUNT_HOLDER, &encode_u64(1), HOLDER, 0));
    let r_unknown_statement =
        key(|b| keys::reverse_key(b, StatementId(99), &encode_u64(1), ACCOUNT, 0));
    let s = keys::stat_key(unknown, StatKind::RowCount).to_vec();
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
                .expect("plant foreign namespace key");
        }
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: f.into(),
                what: "F key relation",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: m.into(),
                what: "M key relation",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: u_wrong_statement.into(),
                what: "U key statement",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: u_unknown_relation.into(),
                what: "U key relation",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: r_wrong_source.into(),
                what: "R key source relation",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: r_unknown_statement.into(),
                what: "R key statement",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: s.into(),
                what: "S key relation",
            }),
        ]
    );
}

#[test]
fn namespace_row_images_are_width_checked() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-namespace-values");
    let m = keys::membership_key(BOOKING, &[0x22; 32]).to_vec();
    let u =
        key(|b| keys::determinant_key(b, BOOKING, BOOKING_KEY, &booking_determinant(99, 0, 10)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &m, &[])
            .expect("plant malformed M value");
        data.put(txn.raw_mut(), &u, &[])
            .expect("plant malformed U value");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: m.into(),
                what: "M row id",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: u.into(),
                what: "U row id",
            }),
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
    let control = Db::create(control_dir.path(), decl.clone())
        .expect("create control")
        .expect("accepted");
    assert_eq!(
        control
            .verify_store()
            .expect("verify healthy sibling")
            .findings()
            .to_vec(),
        vec![]
    );
    let dir = TempDir::new("verify-s-shape");
    let db = Db::create(dir.path(), decl)
        .expect("create")
        .expect("accepted");
    let malformed_value = keys::stat_key(RelationId(0), StatKind::RowCount).to_vec();
    let mut unknown_kind = keys::stat_key(RelationId(0), StatKind::RowIdHighWater).to_vec();
    *unknown_kind.last_mut().expect("stat kind") = 9;
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &malformed_value, &[])
            .expect("plant malformed counter");
        data.put(txn.raw_mut(), &unknown_kind, 0u64.to_le_bytes().as_slice())
            .expect("plant unknown stat");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: malformed_value.into(),
                what: "S counter value",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: unknown_kind.into(),
                what: "S stat kind",
            }),
        ]
    );
}

#[test]
fn wrong_fact_width_is_a_contextual_finding() {
    let (_dir, db) = canonical_field_fixture_with_healthy_sibling("verify-wrong-fact-width");
    replace_fact_bytes(&db, RelationId(0), 0, |fact| {
        fact.pop();
    });
    let f = keys::fact_key(RelationId(0), 0).to_vec();
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(CorruptionError::Malformed {
            key: f.into(),
            what: "F fact width",
        })]
    );
}

#[test]
fn noncanonical_field_encodings_are_each_found() {
    let (_dir, db) = canonical_field_fixture_with_healthy_sibling("verify-field-encodings");
    replace_fact_bytes(&db, RelationId(0), 0, |fact| {
        fact[0] = 2;
        fact[8] = 1;
        fact[9..17].copy_from_slice(&10u64.to_be_bytes());
        fact[17..25].copy_from_slice(&10u64.to_be_bytes());
    });
    let f = keys::fact_key(RelationId(0), 0).to_vec();
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: f.clone().into(),
                what: "F fact bool",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: f.clone().into(),
                what: "F fact fixed bytes padding",
            }),
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: f.into(),
                what: "F fact interval",
            }),
        ]
    );
}

#[test]
fn intern_id_at_or_beyond_the_counter_is_found_with_fact_context() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-intern-bound");
    replace_fact_bytes(&db, HOLDER, 1, |fact| {
        fact[8..16].copy_from_slice(&99u64.to_be_bytes());
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::InternBeyondNextId {
                relation: HOLDER,
                row_id: 1,
                intern_id: InternId::from_raw(99),
                next_id: InternId::from_raw(2),
            }),
            StoreFinding::Corruption(CorruptionError::DanglingInternId(InternId::from_raw(99))),
        ]
    );
}

#[test]
fn a_sentinel_intern_id_is_malformed_not_a_named_id() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-intern-sentinel");
    replace_fact_bytes(&db, HOLDER, 1, |fact| {
        fact[8..16].copy_from_slice(&u64::MAX.to_be_bytes());
    });
    let f = keys::fact_key(HOLDER, 1).to_vec();
    let findings = db.verify_store().expect("verify").findings().to_vec();
    assert_eq!(
        findings,
        vec![StoreFinding::Corruption(CorruptionError::Malformed {
            key: f.into(),
            what: "F intern sentinel",
        })]
    );
    assert!(
        !findings.iter().any(|finding| matches!(
            finding,
            StoreFinding::Corruption(CorruptionError::DanglingInternId(id)) if id.is_sentinel()
        )),
        "sentinel must not appear as a dangling intern id, got {findings:?}"
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
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(CorruptionError::Malformed {
            key: malformed.into(),
            what: "dict reverse id",
        })]
    );
}

fn dict_reverse_key(id: u64) -> Vec<u8> {
    let mut key = vec![1u8];
    key.extend_from_slice(&id.to_be_bytes());
    key
}

fn dict_forward_key(raw: &[u8]) -> Vec<u8> {
    let mut key = vec![0u8];
    key.extend_from_slice(blake3::hash(raw).as_bytes());
    key
}

#[test]
fn a_referenced_id_without_a_reverse_entry_is_the_finding() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-dict-liveness");
    raw_write(&db, |txn| {
        assert!(
            txn.env()
                .dict()
                .delete(txn.raw_mut(), &dict_reverse_key(0))
                .expect("delete reverse entry"),
            "the fixture interned alice at id 0"
        );
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(CorruptionError::DanglingInternId(
            InternId::from_raw(0)
        ))]
    );
}

#[test]
fn a_rebound_forward_entry_is_the_finding() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-dict-rebound");
    raw_write(&db, |txn| {
        txn.env()
            .dict()
            .put(
                txn.raw_mut(),
                &dict_forward_key(b"alice"),
                1u64.to_be_bytes().as_slice(),
            )
            .expect("rebind forward entry");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::DictForwardDesync {
                intern_id: InternId::from_raw(0),
                forward: Some(InternId::from_raw(1)),
            }
        )]
    );
}

#[test]
fn a_reverse_id_at_or_beyond_the_counter_is_the_finding() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-dict-next-id");
    raw_write(&db, |txn| {
        txn.put_dict_next_id(1).expect("regress the counter");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(CorruptionError::DictNextIdLow {
            stored: InternId::from_raw(1),
            reverse_id: InternId::from_raw(1),
        })]
    );
}

#[test]
fn a_regressed_fresh_next_value_is_the_finding() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-q-low");
    let q = keys::fresh_key(HOLDER, FieldId(0)).to_vec();
    raw_write(&db, |txn| {
        txn.env()
            .data()
            .put(txn.raw_mut(), &q, 1u64.to_le_bytes().as_slice())
            .expect("regress Q");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::FreshNextValueLow {
                relation: HOLDER,
                field: FieldId(0),
                stored: 1,
                max_fresh: 1,
            }
        )]
    );
}

#[test]
fn an_absent_fresh_sequence_is_found_against_the_tally() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-q-absent");
    let q = keys::fresh_key(HOLDER, FieldId(0)).to_vec();
    raw_write(&db, |txn| {
        assert!(
            txn.env().data().delete(txn.raw_mut(), &q).expect("delete"),
            "the fixture committed fresh values"
        );
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::FreshNextValueLow {
                relation: HOLDER,
                field: FieldId(0),
                stored: 0,
                max_fresh: 1,
            }
        )]
    );
}

/// Finding 033, the exhausted-sequence corner: the exemption keys on the STORED
/// next-value being exhausted, never on the tally alone — a row holding an
/// explicit `u64::MAX` must not mask a regressed `Q` underneath it (`reserve`
/// would re-issue every id between the regression and the ceiling).
#[test]
fn a_max_row_does_not_mask_a_regressed_fresh_next_value() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-q-max-masked");
    db.write(|tx| {
        tx.insert_dyn(
            HOLDER,
            [&[Value::U64(u64::MAX), Value::String("mallory".into())]],
        )
        .map(|_| ())
    })
    .expect("insert the exhausting row")
    .unwrap();

    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![]
    );
    let q = keys::fresh_key(HOLDER, FieldId(0)).to_vec();
    raw_write(&db, |txn| {
        txn.env()
            .data()
            .put(txn.raw_mut(), &q, 7u64.to_le_bytes().as_slice())
            .expect("regress Q");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::FreshNextValueLow {
                relation: HOLDER,
                field: FieldId(0),
                stored: 7,
                max_fresh: u64::MAX,
            }
        )]
    );
}

#[test]
fn missing_membership_is_found_from_the_fact_side() {
    let (_dir, db) = fixture("verify-missing-m");
    let fact = booking_bytes(&db, 7, 0, 10);
    let m = keys::membership_key(BOOKING, &fact_hash(&fact)).to_vec();
    raw_write(&db, |txn| {
        let data = txn.env().data();
        assert!(data.delete(txn.raw_mut(), &m).expect("raw delete"));
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::FactWithoutMembership {
                relation: BOOKING,
                row_id: 0,
                membership_key: m.into(),
            }
        )]
    );
}

#[test]
fn orphan_membership_is_found_from_the_entry_side() {
    let (_dir, db) = fixture("verify-orphan-m");
    let m = keys::membership_key(BOOKING, &[0xAB; 32]).to_vec();
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &m, 99u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::MembershipWithoutFact {
                relation: BOOKING,
                row_id: 99,
                membership_key: m.into(),
            }
        )]
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
        report.findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::FactWithoutDeterminant {
                relation: BOOKING,
                statement: BOOKING_KEY,
                row_id: 0,
                determinant_key: u.into(),
            }),
            // desync convicts twice, once per broken invariant.
            judgment_containment(db.schema(), CLAIM_BOOKING, claim_bytes(&db, 7, 2, 8).into()),
        ]
    );
}

#[test]
fn orphan_determinant_is_found_from_the_entry_side() {
    let (_dir, db) = fixture("verify-orphan-u");

    let u =
        key(|b| keys::determinant_key(b, BOOKING, BOOKING_KEY, &booking_determinant(99, 0, 10)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &u, 42u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::DeterminantWithoutFact {
                relation: BOOKING,
                statement: BOOKING_KEY,
                determinant_key: u.into(),
            }
        )]
    );
}

#[test]
fn determinant_key_byte_flip_is_found_against_the_live_fact() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-u-key-image");

    let u = key(|b| keys::determinant_key(b, BOOKING, BOOKING_KEY, &booking_determinant(5, 0, 10)));
    raw_write(&db, |txn| {
        txn.env()
            .data()
            .put(txn.raw_mut(), &u, 0u64.to_le_bytes().as_slice())
            .expect("plant perturbed U key");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::DeterminantWithoutFact {
                relation: BOOKING,
                statement: BOOKING_KEY,
                determinant_key: u.into(),
            }
        )]
    );
}

#[test]
fn a_u_entry_under_a_fresh_row_key_is_the_finding() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-fresh-row-u");
    let u = key(|b| keys::determinant_key(b, HOLDER, HOLDER_KEY, &encode_u64(1)));
    raw_write(&db, |txn| {
        txn.env()
            .data()
            .put(txn.raw_mut(), &u, 1u64.to_le_bytes().as_slice())
            .expect("plant fresh-row U entry");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::FreshRowDeterminantEntry {
                relation: HOLDER,
                statement: HOLDER_KEY,
                determinant_key: u.into(),
            }
        )]
    );
}

#[test]
fn a_fresh_row_id_disagreeing_with_the_fresh_field_is_the_finding() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-fresh-row-desync");
    replace_fact_bytes(&db, HOLDER, 1, |fact| {
        fact[..8].copy_from_slice(&0u64.to_be_bytes());
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(CorruptionError::FreshRowDesync {
            relation: HOLDER,
            row_id: 1,
            fresh: 0,
        })]
    );
}

#[test]
fn a_stored_high_water_on_a_fresh_keyed_relation_is_the_finding() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-fresh-row-high-water");
    let water = keys::stat_key(HOLDER, StatKind::RowIdHighWater).to_vec();
    raw_write(&db, |txn| {
        txn.env()
            .data()
            .put(txn.raw_mut(), &water, 9u64.to_le_bytes().as_slice())
            .expect("plant fresh-keyed high-water");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(CorruptionError::Malformed {
            key: water.into(),
            what: "S high-water on a fresh-keyed relation",
        })]
    );
}

#[test]
fn pointwise_overlap_is_found_by_the_ordered_walk() {
    let (_dir, db) = fixture("verify-pointwise-overlap");

    // overlapping (7, [0, 10)): the invariant no namespace pairing sees,

    let fact = booking_bytes(&db, 7, 5, 15);
    let row_id = 2u64;
    let f = keys::fact_key(BOOKING, row_id).to_vec();
    let m = keys::membership_key(BOOKING, &fact_hash(&fact)).to_vec();
    let u = key(|b| keys::determinant_key(b, BOOKING, BOOKING_KEY, &booking_determinant(7, 5, 15)));
    let count = keys::stat_key(BOOKING, StatKind::RowCount).to_vec();
    let water = keys::stat_key(BOOKING, StatKind::RowIdHighWater).to_vec();
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
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::PointwiseOverlap {
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
            }
        )]
    );
}

#[test]
fn a_coherently_deleted_scalar_target_is_a_judgment_violation() {
    let (_dir, db) = fixture("verify-judgment-scalar");

    delete_target_rows(&db, HOLDER, 1, &[], 0);
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![judgment_containment(
            db.schema(),
            ACCOUNT_HOLDER,
            account_bytes(&db, 1, 0).into(),
        )]
    );
}

#[test]
fn a_coherently_deleted_coverage_segment_is_a_judgment_violation() {
    let (_dir, db) = fixture("verify-judgment-coverage");

    delete_target_rows(
        &db,
        BOOKING,
        0,
        &[(BOOKING_KEY, booking_determinant(7, 0, 10))],
        1,
    );
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![judgment_containment(
            db.schema(),
            CLAIM_BOOKING,
            claim_bytes(&db, 7, 2, 8).into(),
        )]
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
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::FactWithoutReverseEdge {
                statement: ACCOUNT_HOLDER,
                relation: ACCOUNT,
                row_id: 0,
                reverse_key: r.into(),
            }
        )]
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
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::ReverseEdgeWithoutFact {
                statement: ACCOUNT_HOLDER,
                reverse_key: r.into(),
            }
        )]
    );
}

#[test]
fn edge_whose_source_left_its_selection_is_an_orphan() {
    let (_dir, db) = fixture("verify-orphan-r-phi");

    let r = key(|b| keys::reverse_key(b, ACCOUNT_HOLDER, &encode_u64(2), ACCOUNT, 1));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &r, &[]).expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::ReverseEdgeWithoutFact {
                statement: ACCOUNT_HOLDER,
                reverse_key: r.into(),
            }
        )]
    );
}

#[test]
fn reverse_key_byte_flip_is_found_against_the_live_source() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-r-key-image");

    let r = key(|b| keys::reverse_key(b, ACCOUNT_HOLDER, &encode_u64(3), ACCOUNT, 0));
    raw_write(&db, |txn| {
        txn.env()
            .data()
            .put(txn.raw_mut(), &r, &[])
            .expect("plant perturbed R key");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::ReverseEdgeWithoutFact {
                statement: ACCOUNT_HOLDER,
                reverse_key: r.into(),
            }
        )]
    );
}

#[test]
fn wrong_row_count_is_found_against_the_scan() {
    let (_dir, db) = fixture("verify-wrong-s");
    let count = keys::stat_key(BOOKING, StatKind::RowCount).to_vec();
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &count, 99u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(CorruptionError::RowCountDesync {
            relation: BOOKING,
            stored: 99,
            counted: 2,
        })]
    );
}

#[test]
fn low_high_water_is_found_against_the_max_row_id() {
    let (_dir, db) = fixture("verify-low-water");
    let water = keys::stat_key(BOOKING, StatKind::RowIdHighWater).to_vec();
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &water, 0u64.to_le_bytes().as_slice())
            .expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::RowIdHighWaterLow {
                relation: BOOKING,
                stored: 0,
                max_row_id: 1,
            }
        )]
    );
}

#[test]
fn absent_counters_are_found_against_the_fact_tally() {
    let (_dir, db) = fixture_with_healthy_sibling("verify-absent-counters");
    let count = keys::stat_key(CLAIM, StatKind::RowCount).to_vec();
    let water = keys::stat_key(CLAIM, StatKind::RowIdHighWater).to_vec();
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
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::RowCountDesync {
                relation: CLAIM,
                stored: 0,
                counted: 1,
            }),
            StoreFinding::Corruption(CorruptionError::RowIdHighWaterLow {
                relation: CLAIM,
                stored: 0,
                max_row_id: 0,
            }),
        ]
    );
}

#[test]
fn a_stored_row_for_a_closed_relation_is_the_finding() {
    let dir = TempDir::new("verify-closed");
    let decl = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: Some(Box::new([bumbledb_theory::schema::Row {
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
    let db = Db::create(dir.path(), decl)
        .expect("create")
        .expect("accepted");
    let currency = RelationId(0);
    let fact = db
        .schema()
        .relation(currency)
        .body()
        .closed_rows()
        .expect("closed")[0]
        .fact
        .to_vec();
    let f = keys::fact_key(currency, 0).to_vec();
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &f, &fact).expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::ClosedRelationEntry {
                relation: currency,
                key: f.into(),
            }
        )]
    );
}

#[test]
fn membership_and_determinant_entries_for_a_closed_relation_are_findings() {
    let decl = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: Some(Box::new([bumbledb_theory::schema::Row {
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
    let control = Db::create(control_dir.path(), decl.clone())
        .expect("create control")
        .expect("accepted");
    assert_eq!(
        control
            .verify_store()
            .expect("verify healthy sibling")
            .findings()
            .to_vec(),
        vec![]
    );

    let dir = TempDir::new("verify-closed-m-u");
    let db = Db::create(dir.path(), decl)
        .expect("create")
        .expect("accepted");
    let currency = RelationId(0);
    let fact = &db
        .schema()
        .relation(currency)
        .body()
        .closed_rows()
        .expect("closed")[0]
        .fact;
    let m = keys::membership_key(currency, &fact_hash(fact)).to_vec();
    let u = key(|b| keys::determinant_key(b, currency, StatementId(0), &encode_u64(0)));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &m, 0u64.to_le_bytes().as_slice())
            .expect("plant M");
        data.put(txn.raw_mut(), &u, 0u64.to_le_bytes().as_slice())
            .expect("plant U");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::ClosedRelationEntry {
                relation: currency,
                key: m.into(),
            }),
            StoreFinding::Corruption(CorruptionError::ClosedRelationEntry {
                relation: currency,
                key: u.into(),
            }),
        ]
    );
}

fn closed_subset_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(Box::new([
                    bumbledb_theory::schema::Row {
                        handle: "Low".into(),
                        values: Box::new([Value::Bool(false)]),
                    },
                    bumbledb_theory::schema::Row {
                        handle: "Med".into(),
                        values: Box::new([Value::Bool(true)]),
                    },
                    bumbledb_theory::schema::Row {
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
    let dir = TempDir::new("verify-closed-r");
    let db = Db::create(dir.path(), closed_subset_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| tx.insert_dyn(RelationId(1), [&[Value::U64(1)]]).map(|_| ()))
        .expect("a legal closed reference commits")
        .unwrap();
    let r = key(|b| keys::reverse_key(b, StatementId(1), &encode_u64(1), RelationId(1), 0));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &r, &[]).expect("raw put");
    });
    let report = db.verify_store().expect("verify");
    assert_eq!(
        report.findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::ClosedRelationEntry {
                relation: RelationId(0),
                key: r.into(),
            }
        )]
    );
}

#[test]
fn a_planted_source_outside_the_member_set_is_a_judgment_violation() {
    let dir = TempDir::new("verify-closed-member");
    let db = Db::create(dir.path(), closed_subset_schema())
        .expect("create")
        .expect("accepted");
    let alert = RelationId(1);
    let mut fact = Vec::new();
    encode_fact(
        &[ValueRef::U64(9)],
        db.schema().relation(alert).layout(),
        &mut fact,
    );
    let f = keys::fact_key(alert, 0).to_vec();
    let m = keys::membership_key(alert, &fact_hash(&fact)).to_vec();
    let count = keys::stat_key(alert, StatKind::RowCount).to_vec();
    let water = keys::stat_key(alert, StatKind::RowIdHighWater).to_vec();
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
        report.findings().to_vec(),
        vec![judgment_containment(
            db.schema(),
            StatementId(1),
            fact.into(),
        )]
    );
}

#[test]
fn an_uncovered_domain_quantification_is_a_judgment_violation() {
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

    let db = Db::create_store_without_admission(dir.path(), decl).expect("fixture");
    let severities = db
        .schema()
        .relation(RelationId(0))
        .body()
        .closed_rows()
        .expect("closed");
    let expected: Vec<StoreFinding> = severities
        .iter()
        .map(|row| judgment_containment(db.schema(), StatementId(3), row.fact.clone()))
        .collect();
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        expected
    );
    for severity in 0..3u64 {
        db.write(|tx| {
            tx.insert_dyn(RelationId(2), [&[Value::U64(severity), Value::U64(10)]])
                .map(|_| ())
        })
        .expect("handlers commit")
        .unwrap();
    }
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![]
    );
}

const M_HOLDER: RelationId = RelationId(0);
const M_ACCOUNT: RelationId = RelationId(1);

const M_CAPACITY: StatementId = StatementId(1);

fn marks_schema() -> SchemaDescriptor {
    let plain = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![plain("id"), plain("tag")],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![plain("holder"), plain("kind"), plain("num")],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: M_HOLDER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: Side {
                    relation: M_HOLDER,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                weight: bumbledb_theory::schema::Weight::Unit,
                lo: 1,
                hi: Some(bumbledb_theory::schema::Bound::Lit(2)),
                source: Side {
                    relation: M_ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(
                        FieldId(1),
                        bumbledb_theory::schema::LiteralSet::One(Value::U64(1)),
                    )]),
                },
            },
        ],
    }
}

fn marks_fixture(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), marks_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(M_HOLDER, [&[Value::U64(1), Value::U64(0)]])?;
        tx.insert_dyn(M_ACCOUNT, [&[Value::U64(1), Value::U64(1), Value::U64(0)]])
            .map(|_| ())
    })
    .expect("green base commit")
    .unwrap();
    (dir, db)
}

#[test]
fn a_marked_store_verifies_clean() {
    let (_dir, db) = marks_fixture("verify-marks-clean");
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![]
    );
}

#[test]
fn a_closed_parent_capacity_group_is_remeasured_by_the_marks_pass() {
    let pool = RelationId(0);
    let device = RelationId(1);

    let capacity = StatementId(1);
    let plain = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::None,
    };
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(Box::new([bumbledb_theory::schema::Row {
                    handle: "P0".into(),
                    values: Box::new([Value::U64(9)]),
                }])),
                name: "Pool".into(),
                fields: vec![plain("cap")],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![plain("pool"), plain("num")],
            },
        ],

        statements: vec![StatementDescriptor::Capacity {
            target: Side {
                relation: pool,
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
            weight: bumbledb_theory::schema::Weight::Unit,
            lo: 1,
            hi: Some(bumbledb_theory::schema::Bound::Lit(2)),
            source: Side {
                relation: device,
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    };
    let dir = TempDir::new("verify-marks-closed-parent");

    let db = Db::create_store_without_admission(dir.path(), decl).expect("fixture");
    db.write(|tx| {
        tx.insert_dyn(device, [&[Value::U64(0), Value::U64(0)]])
            .map(|_| ())
    })
    .expect("one device inside the window")
    .unwrap();
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![],
        "the green closed-parent store sweeps clean"
    );

    let r = key(|b| keys::reverse_key(b, capacity, &encode_u64(0), device, 0));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        assert!(
            data.delete(txn.raw_mut(), &r).expect("raw delete"),
            "the fixture wrote this capacity edge"
        );
    });
    let parent_fact: Box<[u8]> = db
        .schema()
        .relation(pool)
        .body()
        .closed_rows()
        .expect("closed")[0]
        .fact
        .clone();
    let report = db.verify_store().expect("verify");
    let findings = report.findings();
    assert!(
        findings.contains(&judgment_capacity(db.schema(), capacity, parent_fact, 0)),
        "the marks pass re-measures the sealed axiom's group, got {findings:?}"
    );
    assert!(
        findings.iter().any(|f| matches!(
            f,
            StoreFinding::Corruption(CorruptionError::FactWithoutReverseEdge { .. })
        )),
        "the missing edge itself is also found, got {findings:?}"
    );
    assert_eq!(findings.len(), 2, "exactly the two findings: {findings:?}");
}

#[test]
fn a_missing_capacity_edge_is_found_and_the_group_remeasured() {
    let (_dir, db) = marks_fixture("verify-marks-capacity-edge");
    let child_key = encode_u64(1);
    let r = key(|b| keys::reverse_key(b, M_CAPACITY, &child_key, M_ACCOUNT, 0));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        assert!(
            data.delete(txn.raw_mut(), &r).expect("raw delete"),
            "the fixture wrote this capacity edge"
        );
    });
    let holder_fact = {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(1), ValueRef::U64(0)],
            db.schema().relation(M_HOLDER).layout(),
            &mut bytes,
        );
        bytes
    };
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            judgment_capacity(db.schema(), M_CAPACITY, holder_fact.into(), 0),
            StoreFinding::Corruption(CorruptionError::FactWithoutReverseEdge {
                statement: M_CAPACITY,
                relation: M_ACCOUNT,
                row_id: 0,
                reverse_key: r.into(),
            }),
        ]
    );
}

#[test]
fn a_stray_capacity_edge_is_convicted() {
    let (_dir, db) = marks_fixture("verify-marks-stray-capacity");
    let child_key = encode_u64(9);
    let r = key(|b| keys::reverse_key(b, M_CAPACITY, &child_key, M_ACCOUNT, 77));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &r, &[]).expect("plant stray edge");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(
            CorruptionError::ReverseEdgeWithoutFact {
                statement: M_CAPACITY,
                reverse_key: r.into(),
            }
        )]
    );
}

const W_POOL: RelationId = RelationId(0);
const W_DEVICE: RelationId = RelationId(1);

const W_CAPACITY: StatementId = StatementId(1);

fn weighted_fixture(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let plain = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::None,
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Pool".into(),
                fields: vec![plain("id"), plain("supply")],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![plain("pool"), plain("watts"), plain("num")],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: W_POOL,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: Side {
                    relation: W_POOL,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                weight: bumbledb_theory::schema::Weight::Field(FieldId(1)),
                lo: 5,
                hi: Some(bumbledb_theory::schema::Bound::Lit(100)),
                source: Side {
                    relation: W_DEVICE,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
        ],
    };
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), schema)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(W_POOL, [&[Value::U64(1), Value::U64(100)]])?;
        tx.insert_dyn(W_DEVICE, [&[Value::U64(1), Value::U64(60), Value::U64(0)]])
            .map(|_| ())
    })
    .expect("green weighted base commit")
    .unwrap();
    (dir, db)
}

#[test]
fn a_weighted_store_verifies_clean() {
    let (_dir, db) = weighted_fixture("verify-weight-clean");
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![]
    );
}

#[test]
fn a_desynced_weight_slot_is_convicted_never_repaired() {
    let (_dir, db) = weighted_fixture("verify-weight-desync");
    let child_key = encode_u64(1);
    let r = key(|b| keys::reverse_key(b, W_CAPACITY, &child_key, W_DEVICE, 0));
    let planted = 61u64.to_le_bytes();
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &r, &planted)
            .expect("corrupt the weight slot");
    });
    let report = db.verify_store().expect("verify");
    let findings = report.findings();
    assert!(
        !findings.is_empty(),
        "a diverged weight slot must be convicted"
    );
    for finding in findings {
        assert!(
            matches!(
                finding,
                StoreFinding::Corruption(CorruptionError::ReverseEdgeWeightDesync {
                    statement: W_CAPACITY,
                    reverse_key,
                    stored,
                    derived,
                }) if **reverse_key == *r && **stored == planted[..] && **derived != planted[..]
            ),
            "every finding names the diverged edge with stored vs derived, got {finding:?}"
        );
    }
}

#[test]
fn a_foreign_relation_capacity_edge_is_convicted_never_a_panic() {
    let (_dir, db) = weighted_fixture("verify-weight-foreign-edge");
    let child_key = encode_u64(1);
    let r = key(|b| keys::reverse_key(b, W_CAPACITY, &child_key, W_POOL, 0));
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &r, &[])
            .expect("plant foreign-relation edge");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![StoreFinding::Corruption(CorruptionError::Malformed {
            key: r.into(),
            what: "R key source relation",
        })]
    );
}

#[test]
fn a_wrong_width_capacity_child_is_convicted_never_a_panic() {
    let (_dir, db) = weighted_fixture("verify-weight-wrong-width-child");
    let child_key = encode_u64(1);
    let f = keys::fact_key(W_DEVICE, 77).to_vec();
    let r = key(|b| keys::reverse_key(b, W_CAPACITY, &child_key, W_DEVICE, 77));

    let planted: Vec<u8> = [encode_u64(1), encode_u64(60)].concat();
    raw_write(&db, |txn| {
        let data = txn.env().data();
        data.put(txn.raw_mut(), &f, &planted)
            .expect("plant wrong-width fact");
        data.put(txn.raw_mut(), &r, &[])
            .expect("plant its capacity edge");
    });
    assert_eq!(
        db.verify_store().expect("verify").findings().to_vec(),
        vec![
            StoreFinding::Corruption(CorruptionError::Malformed {
                key: f.into(),
                what: "F fact width",
            }),
            StoreFinding::Corruption(CorruptionError::RowCountDesync {
                relation: W_DEVICE,
                stored: 1,
                counted: 2,
            }),
            StoreFinding::Corruption(CorruptionError::RowIdHighWaterLow {
                relation: W_DEVICE,
                stored: 1,
                max_row_id: 77,
            }),
        ]
    );
}

fn fixed_lane_fixture(tag: &str) -> (TempDir, Db<SchemaDescriptor>) {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "FixedLane".into(),
            fields: vec![
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "lane".into(),
                    value_type: ValueType::FixedInterval {
                        element: IntervalElement::U64,
                        width: 5,
                    },
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    };
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), schema)
        .expect("create fixed-lane store")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(
            RelationId(0),
            [&[
                Value::Bool(true),
                Value::IntervalU64(bumbledb_theory::Interval::<u64>::new(10, 15).expect("width 5")),
            ]],
        )
        .map(|_| ())
    })
    .expect("insert fixed-lane fact")
    .unwrap();
    (dir, db)
}

#[test]
fn fixed_width_start_at_or_past_the_bound_at_rest_is_convicted() {
    for (tag, corrupt_start) in [
        ("verify-fixed-start-at-bound", u64::MAX - 5),
        ("verify-fixed-start-overflow", u64::MAX),
    ] {
        let (_dir, db) = fixed_lane_fixture(tag);

        assert_eq!(
            db.verify_store()
                .expect("verify healthy")
                .findings()
                .to_vec(),
            vec![]
        );
        replace_fact_bytes(&db, RelationId(0), 0, |fact| {
            // The lane field's one stored word sits after the bool byte's

            let len = fact.len();
            fact[len - 8..].copy_from_slice(&corrupt_start.to_be_bytes());
        });
        let f = keys::fact_key(RelationId(0), 0).to_vec();
        assert_eq!(
            db.verify_store().expect("verify").findings().to_vec(),
            vec![StoreFinding::Corruption(CorruptionError::Malformed {
                key: f.into(),
                what: "F fact fixed interval start",
            })],
            "corrupt start {corrupt_start} must convict"
        );
    }
}
