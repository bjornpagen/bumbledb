//! Schema-macro expansion tests (docs/architecture/70-api.md): the
//! `schema!` macro is exactly sugar — its expansion resolves to the same
//! validated schema a hand-built [`SchemaDescriptor`] does, statements in
//! source order with `==` lowered to two adjacent containments (`A <= B`
//! first) and selection literals typed in the macro (enum variants as
//! ordinals).
//!
//! The example schema is `docs/architecture/30-dependencies.md`'s, minus
//! its illustrative `Employment` line (that relation is not declared).

use bumbledb::schema::fingerprint::fingerprint;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, LiteralValue, RelationDescriptor,
    RelationId, SchemaDescriptor, Side, StatementDescriptor, ValueType,
};
use bumbledb::{Db, Fact, Interval};

bumbledb::schema! {
    relation Holder  { id: u64 as HolderId, serial, name: str }
    relation Account {
        id: u64 as AccountId, serial,
        holder: u64 as HolderId,
        kind: enum Kind { Checking, Savings },
        active: interval<i64>,
    }
    relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }

    Account(holder) <= Holder(id);
    Account(id | kind == Savings) == SavingsTerms(account);
    SavingsTerms(account) -> SavingsTerms;
}

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn serial_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Serial,
    }
}

/// `Account(id | kind == Savings)` — the selected side of the `==`, its
/// variant literal resolved to the ordinal.
fn savings_accounts() -> Side {
    Side {
        relation: RelationId(1),
        projection: Box::new([FieldId(0)]),
        selection: Box::new([(FieldId(2), LiteralValue::Enum(1))]),
    }
}

/// `SavingsTerms(account)`.
fn savings_terms_side() -> Side {
    Side {
        relation: RelationId(2),
        projection: Box::new([FieldId(0)]),
        selection: Box::new([]),
    }
}

/// The same declaration, hand-built through the descriptor contract.
fn hand_built() -> bumbledb::schema::Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![serial_field("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    serial_field("id"),
                    field("holder", ValueType::U64),
                    field(
                        "kind",
                        ValueType::Enum {
                            variants: ["Checking", "Savings"]
                                .iter()
                                .map(|v| Box::from(*v))
                                .collect(),
                        },
                    ),
                    field(
                        "active",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                name: "SavingsTerms".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field("rate_bps", ValueType::I64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Containment {
                source: Side {
                    relation: RelationId(1),
                    projection: Box::new([FieldId(1)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: RelationId(0),
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
            StatementDescriptor::Containment {
                source: savings_accounts(),
                target: savings_terms_side(),
            },
            StatementDescriptor::Containment {
                source: savings_terms_side(),
                target: savings_accounts(),
            },
            StatementDescriptor::Functionality {
                relation: RelationId(2),
                projection: Box::new([FieldId(0)]),
            },
        ],
    }
    .validate()
    .expect("hand-built declaration is valid")
}

#[test]
fn macro_output_is_exactly_sugar() {
    assert_eq!(fingerprint(schema()), fingerprint(&hand_built()));
}

#[test]
fn statements_land_in_source_order_with_equality_lowered() {
    let descriptors: Vec<&StatementDescriptor> = schema()
        .statements()
        .iter()
        .map(|s| &s.descriptor)
        .collect();
    // Materialized order: the two serial auto-FDs first (Holder.id,
    // Account.id), then the declared statements in source order — the `==`
    // contributing its two containments adjacently, `A <= B` first.
    assert_eq!(descriptors.len(), 6);
    assert_eq!(
        descriptors[0],
        &StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }
    );
    assert_eq!(
        descriptors[1],
        &StatementDescriptor::Functionality {
            relation: RelationId(1),
            projection: Box::new([FieldId(0)]),
        }
    );
    assert_eq!(
        descriptors[2],
        &StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(1),
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }
    );
    assert_eq!(
        descriptors[3],
        &StatementDescriptor::Containment {
            source: savings_accounts(),
            target: savings_terms_side(),
        }
    );
    assert_eq!(
        descriptors[4],
        &StatementDescriptor::Containment {
            source: savings_terms_side(),
            target: savings_accounts(),
        }
    );
    assert_eq!(
        descriptors[5],
        &StatementDescriptor::Functionality {
            relation: RelationId(2),
            projection: Box::new([FieldId(0)]),
        }
    );
}

#[test]
fn fact_structs_carry_host_types() {
    // interval<i64> without `as`: the fact field is the raw engine value.
    let account = Account {
        id: AccountId(1),
        holder: HolderId(2),
        kind: Kind::Savings,
        active: Interval::<i64>::new(-5, 5).expect("nonempty"),
    };
    assert_eq!(account.active.start(), -5);
    assert_eq!(account.active.end(), 5);
    assert_eq!(Account::RELATION, RelationId(1));
    let holder = Holder {
        id: HolderId(2),
        name: "alice".to_owned(),
    };
    let terms = SavingsTerms {
        account: AccountId(1),
        rate_bps: 250,
    };
    assert_eq!(holder.id, account.holder);
    assert_eq!(terms.account, account.id);
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
        kind: Kind::Checking,
        active: Interval::<i64>::new(-100, 100).expect("nonempty"),
    };
    // Write the holder + account (interning the string through the delta),
    // then decode back through a snapshot.
    db.write(|tx| {
        tx.insert(&Holder {
            id: HolderId(3),
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

mod interval_newtype {
    use bumbledb::schema::{FieldId, IntervalElement, ValueType};
    use bumbledb::{Fact, Interval};

    bumbledb::schema! {
        relation Booking {
            room: u64 as RoomId,
            active: interval<i64> as ActiveDuring,
            window: interval<u64>,
        }
        Booking(room, active) -> Booking;
    }

    #[test]
    fn interval_as_newtype_wraps_the_interval() {
        // The generated newtype wraps `Interval<i64>`; the fact struct
        // carries it (and the raw engine value where `as` is absent).
        let booking = Booking {
            room: RoomId(7),
            active: ActiveDuring(Interval::<i64>::new(-10, 20).expect("nonempty")),
            window: Interval::<u64>::new(0, Interval::<u64>::MAX_END).expect("nonempty"),
        };
        assert_eq!(booking.active.0.start(), -10);
        assert_eq!(booking.active.0.end(), 20);
        // Both fields are interval-typed in the descriptor, and the schema
        // passes validation (the FD's one interval is its last position).
        let relation = schema().relation(Booking::RELATION);
        assert_eq!(
            relation.field(FieldId(1)).value_type,
            ValueType::Interval {
                element: IntervalElement::I64,
            }
        );
        assert_eq!(
            relation.field(FieldId(2)).value_type,
            ValueType::Interval {
                element: IntervalElement::U64,
            }
        );
    }
}

mod selection_literals {
    use bumbledb::schema::{FieldId, LiteralValue, StatementDescriptor};

    bumbledb::schema! {
        relation Sensor {
            id: u64 as SensorId, serial,
            span: interval<i64>,
            offset: i64,
            live: bool,
            label: str,
            tag: bytes,
        }
        relation Reading { sensor: u64 as SensorId }

        Reading(sensor) <= Sensor(id | span == -10..10, offset == -3, live == true, label == "north", tag == b"\x01");
    }

    #[test]
    fn every_literal_kind_resolves_typed() {
        // Statement 0 is Sensor.id's serial auto-FD; 1 is the containment.
        let StatementDescriptor::Containment { target, .. } = &schema().statements()[1].descriptor
        else {
            panic!("the declared statement is a containment");
        };
        assert_eq!(
            target.selection[..],
            [
                (FieldId(1), LiteralValue::IntervalI64(-10, 10)),
                (FieldId(2), LiteralValue::I64(-3)),
                (FieldId(3), LiteralValue::Bool(true)),
                (FieldId(4), LiteralValue::String(Box::from(&b"north"[..]))),
                (FieldId(5), LiteralValue::Bytes(Box::from(&b"\x01"[..]))),
            ]
        );
    }

    #[test]
    fn the_fact_structs_construct() {
        let sensor = Sensor {
            id: SensorId(1),
            span: bumbledb::Interval::<i64>::new(0, 10).expect("nonempty"),
            offset: -3,
            live: true,
            label: "north".to_owned(),
            tag: vec![0x01],
        };
        let reading = Reading {
            sensor: SensorId(1),
        };
        assert_eq!(reading.sensor, sensor.id);
    }
}
