//! Schema-macro integration tests (docs/architecture/60-api.md): the
//! `schema!` macro is *exactly* sugar — its
//! expansion constructs and fingerprints identically to a hand-built
//! descriptor of the same declaration.

use bumbledb::schema::fingerprint::fingerprint;
use bumbledb::schema::{
    ConstraintDescriptor, ConstraintId, FieldDescriptor, FieldId, Generation, RelationDescriptor,
    RelationId, SchemaDescriptor, ValueType,
};
use bumbledb::{Db, Fact};

bumbledb::schema! {
    relation Holder {
        id: u64 as HolderId, serial,
        region: u64,
        name: str,
        unique(id, region),
    }
    relation Account {
        id: u64 as AccountId, serial, unique,
        holder: u64 as HolderId, fk(Holder.id),
        region: u64,
        status: enum Status { Active, Closed },
        active: bool,
        balance: i64,
        memo: bytes,
        unique(holder, status),
        fk(holder, region -> Holder.id_region),
    }
}

/// The same declaration, hand-built through the descriptor contract.
fn hand_built() -> bumbledb::schema::Schema {
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
                        name: "region".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "id_region".into(),
                    fields: Box::new([FieldId(0), FieldId(1)]),
                }],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "holder".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "region".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "status".into(),
                        value_type: ValueType::Enum {
                            variants: ["Active", "Closed"].iter().map(|v| Box::from(*v)).collect(),
                        },
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "active".into(),
                        value_type: ValueType::Bool,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "balance".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "memo".into(),
                        value_type: ValueType::Bytes,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![
                    // Declared order mirrors runtime::build_schema: per-field
                    // fks, then compound uniques, then compound fks. (The
                    // redundant `unique` on the serial id was dropped.)
                    ConstraintDescriptor::ForeignKey {
                        name: "holder_fk".into(),
                        fields: Box::new([FieldId(1)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(0),
                    },
                    ConstraintDescriptor::Unique {
                        name: "holder_status".into(),
                        fields: Box::new([FieldId(1), FieldId(3)]),
                    },
                    // The compound-FK inheritance pattern: (holder, region)
                    // targets Holder's compound unique `id_region` — id 1,
                    // after Holder's auto-unique.
                    ConstraintDescriptor::ForeignKey {
                        name: "holder_region_fk".into(),
                        fields: Box::new([FieldId(1), FieldId(2)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(1),
                    },
                ],
            },
        ],
    }
    .validate()
    .expect("hand-built declaration is valid")
}

#[test]
fn macro_output_is_exactly_sugar() {
    let generated = schema();
    let manual = hand_built();
    assert_eq!(fingerprint(generated), fingerprint(&manual));
}

#[test]
fn serial_generates_the_visible_auto_unique() {
    let schema = schema();
    let account = schema.relation(Account::RELATION);
    // Constraint 0 is the auto-unique on the serial id, named after it.
    assert_eq!(
        account.constraint(ConstraintId(0)),
        &ConstraintDescriptor::Unique {
            name: "id".into(),
            fields: Box::new([FieldId(0)]),
        }
    );
    assert!(account.unique_constraints().contains(&ConstraintId(0)));
}

#[test]
fn compound_unique_and_fk_clauses_land() {
    let schema = schema();
    let account = schema.relation(Account::RELATION);
    let names: Vec<&str> = account
        .constraints()
        .iter()
        .map(ConstraintDescriptor::name)
        .collect();
    assert_eq!(
        names,
        vec!["id", "holder_fk", "holder_status", "holder_region_fk"]
    );
    assert_eq!(
        account.constraint(ConstraintId(2)),
        &ConstraintDescriptor::Unique {
            name: "holder_status".into(),
            fields: Box::new([FieldId(1), FieldId(3)]),
        }
    );
    assert!(matches!(
        account.constraint(ConstraintId(3)),
        ConstraintDescriptor::ForeignKey {
            target_relation: RelationId(0),
            target_constraint: ConstraintId(1),
            ..
        }
    ));
}

#[test]
fn typed_round_trip_through_fact_bytes() {
    let dir = std::env::temp_dir().join("bumbledb-macro-round-trip");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("test dir");
    let db = Db::create(&dir, schema()).expect("create");

    let original = Account {
        id: AccountId(7),
        holder: HolderId(3),
        region: 2,
        status: Status::Closed,
        active: true,
        balance: -12_345,
        memo: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };
    // Write the holder + account (interning the strings and the memo
    // through the delta), then decode back through a snapshot.
    db.write(|tx| {
        tx.insert(&Holder {
            id: HolderId(3),
            region: 2,
            name: "alice".to_owned(),
        })?;
        tx.insert(&original)?;
        Ok(())
    })
    .expect("write");

    db.read(|snap| {
        // encode_read finds the committed interned values now.
        let mut bytes = Vec::new();
        assert!(original.encode_read(snap, &mut bytes).expect("encode"));
        let decoded = Account::decode(snap, &bytes).expect("decode");
        assert_eq!(decoded, original);

        // A never-interned value reports itself instead of encoding.
        let ghost = Holder {
            id: HolderId(9),
            region: 2,
            name: "nobody".to_owned(),
        };
        let mut bytes = Vec::new();
        assert!(!ghost.encode_read(snap, &mut bytes).expect("encode"));
        Ok(())
    })
    .expect("read");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
