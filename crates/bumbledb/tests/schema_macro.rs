use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint;
use bumbledb::schema::{
    FieldDescriptor, FieldId, IntervalElement, LiteralSet, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, StatementId, StatementView, ValueType,
};
use bumbledb::{Db, Fact, Interval, Value};

mod common;

fn declared() -> bumbledb::Schema {
    use bumbledb::Theory as _;
    Ledger
        .descriptor()
        .validate()
        .expect("the declared schema is valid")
}

bumbledb::schema! {
    pub Ledger;

    closed relation Kind as KindId = { Checking, Savings };

    relation Holder  { id: u64 as HolderId, name: str }
    relation Account {
        id: u64 as AccountId,
        holder: u64 as HolderId,
        kind: u64 as KindId,
        active: interval<i64>,
    }
    relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }

    Account(holder) <= Holder(id);
    Account(kind) <= Kind(id);
    Account(id | kind == Savings) == SavingsTerms(account);
    SavingsTerms(account) -> SavingsTerms;
}

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
    }
}

fn fresh_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
    }
}

fn savings_accounts() -> Side {
    Side {
        relation: RelationId(2),
        projection: Box::new([FieldId(0)]),
        selection: Box::new([(FieldId(2), LiteralSet::One(Value::U64(1)))]),
    }
}

fn savings_terms_side() -> Side {
    Side {
        relation: RelationId(3),
        projection: Box::new([FieldId(0)]),
        selection: Box::new([]),
    }
}

fn hand_built() -> bumbledb::schema::Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Checking".into(),
                        values: Box::new([]),
                    },
                    Row {
                        handle: "Savings".into(),
                        values: Box::new([]),
                    },
                ])),
                name: "Kind".into(),
                fields: vec![],
            },
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![fresh_field("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![
                    fresh_field("id"),
                    field("holder", ValueType::U64),
                    field("kind", ValueType::U64),
                    field(
                        "active",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
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
                    relation: RelationId(2),
                    projection: Box::new([FieldId(1)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: RelationId(1),
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: RelationId(2),
                    projection: Box::new([FieldId(2)]),
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
                relation: RelationId(3),
                projection: Box::new([FieldId(0)]),
            },
        ],
    }
    .validate()
    .expect("hand-built declaration is valid")
}

#[test]
fn macro_output_is_exactly_sugar() {
    assert_eq!(fingerprint(&declared()), fingerprint(&hand_built()));
}

#[test]
fn statements_land_in_source_order_with_equality_lowered() {
    let schema = declared();
    let descriptors: Vec<StatementDescriptor> = (0..6)
        .map(|id| match schema.statement(StatementId(id)) {
            StatementView::Key(_, statement) => StatementDescriptor::Functionality {
                relation: statement.relation,
                projection: statement.projection.clone(),
            },
            StatementView::Containment(_, statement) => StatementDescriptor::Containment {
                source: statement.source.clone(),
                target: statement.target.clone(),
            },
            StatementView::Capacity(..) => {
                unreachable!("this fixture declares keys and containments only")
            }
        })
        .collect();

    // The closed relation's auto-handle key first, then the declared
    // statements in source order (there are no fresh-implied keys).
    assert_eq!(descriptors.len(), 6);
    assert_eq!(
        descriptors[0],
        StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }
    );
    assert_eq!(
        descriptors[1],
        StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(2),
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }
    );
    assert_eq!(
        descriptors[2],
        StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(2),
                projection: Box::new([FieldId(2)]),
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
        StatementDescriptor::Containment {
            source: savings_accounts(),
            target: savings_terms_side(),
        }
    );
    assert_eq!(
        descriptors[4],
        StatementDescriptor::Containment {
            source: savings_terms_side(),
            target: savings_accounts(),
        }
    );
    assert_eq!(
        descriptors[5],
        StatementDescriptor::Functionality {
            relation: RelationId(3),
            projection: Box::new([FieldId(0)]),
        }
    );
}

#[test]
fn the_equality_pair_seals_mirror_links() {
    let schema = declared();
    let mirrors: Vec<Option<StatementId>> = (0..6)
        .map(|id| match schema.statement(StatementId(id)) {
            StatementView::Key(_, _) | StatementView::Capacity(..) => None,
            StatementView::Containment(_, statement) => statement.mirror_id(&schema),
        })
        .collect();
    assert_eq!(
        mirrors,
        vec![
            None,
            None,
            None,
            Some(StatementId(4)),
            Some(StatementId(3)),
            None
        ]
    );
}

#[test]
fn fact_structs_carry_host_types() {
    let account = Account {
        id: AccountId(1),
        holder: HolderId(2),
        kind: Kind::Savings.id(),
        active: Interval::<i64>::new(-5, 5).expect("nonempty"),
    };
    assert_eq!(account.active.start(), -5);
    assert_eq!(account.active.end(), 5);
    assert_eq!(Account::RELATION, RelationId(2));
    let holder = Holder {
        id: HolderId(2),
        name: "alice",
    };
    let terms = SavingsTerms {
        account: AccountId(1),
        rate_bps: 250,
    };
    assert_eq!(holder.id, account.holder);
    assert_eq!(terms.account, account.id);
}

#[test]
fn fact_and_key_structs_are_value_types() {
    // decoded fact is a value: reusable after insertion, set-member,

    let holder = Holder {
        id: HolderId(2),
        name: "alice",
    };
    let copied = holder;
    assert_eq!(holder, copied);
    let key = SavingsTermsByAccount {
        account: AccountId(1),
    };
    let mut facts = std::collections::HashSet::new();
    assert!(facts.insert(holder));
    assert!(!facts.insert(copied));
    let mut keys = std::collections::HashSet::new();
    assert!(keys.insert(key));
    assert!(!keys.insert(key));
}

#[test]
fn typed_round_trip_through_fact_bytes() {
    let dir = common::TempDir::new("macro-round-trip");
    let db = Db::create(dir.path(), Ledger)
        .expect("create")
        .expect("accepted");

    let original = Account {
        id: AccountId(7),
        holder: HolderId(3),
        kind: Kind::Checking.id(),
        active: Interval::<i64>::new(-100, 100).expect("nonempty"),
    };

    db.write(|tx| {
        tx.insert([&Holder {
            id: HolderId(3),
            name: "alice",
        }])?;
        tx.insert([&original])?;
        Ok(())
    })
    .expect("write")
    .unwrap();

    db.read(|snap| {
        // The stored canonical row decodes back to the exact value: the
        // generated `Fact::decode` walks the real stored bytes (text
        // borrows the snapshot's pages — no dictionary, no copy).
        let decoded: Vec<Account> = snap.scan_facts()?.collect::<Result<_, _>>()?;
        assert_eq!(decoded, vec![original]);

        // A fact never written is absent through the same typed encode
        // path (the Probe/intern-lookup surface is deleted with the
        // dictionary: absence is a set answer, not a codec verdict).
        let ghost = Holder {
            id: HolderId(9),
            name: "nobody",
        };
        assert!(!snap.contains(&ghost)?);
        Ok(())
    })
    .expect("read");
}

#[test]
fn id_constants_are_declaration_order_named_data() {
    assert_eq!(Ledger::KIND, RelationId(0));
    assert_eq!(Ledger::HOLDER, RelationId(1));
    assert_eq!(Ledger::ACCOUNT, RelationId(2));
    assert_eq!(Ledger::SAVINGS_TERMS, RelationId(3));
    assert_eq!(Ledger::KIND_ID, FieldId(0));
    assert_eq!(Ledger::HOLDER_ID, FieldId(0));
    assert_eq!(Ledger::HOLDER_NAME, FieldId(1));
    assert_eq!(Ledger::ACCOUNT_KIND, FieldId(2));
    assert_eq!(Ledger::ACCOUNT_ACTIVE, FieldId(3));
    assert_eq!(Ledger::SAVINGS_TERMS_RATE_BPS, FieldId(1));

    assert_eq!(Kind::Checking.id(), KindId(0));
    assert_eq!(Kind::Savings.id(), KindId(1));
}

#[test]
fn the_manifest_is_the_constants_runtime_twin() {
    use bumbledb::Theory as _;
    let manifest = Ledger.manifest();
    assert_eq!(manifest.relations.len(), 4);
    let account = &manifest.relations[2];
    assert_eq!(&*account.name, "Account");
    assert_eq!(account.id, Ledger::ACCOUNT);
    let kind = &account.fields[2];
    assert_eq!(&*kind.name, "kind");
    assert_eq!(kind.id, Ledger::ACCOUNT_KIND);

    assert_eq!(kind.value_type, ValueType::U64);
    let vocabulary = &manifest.relations[0];
    assert_eq!(&*vocabulary.name, "Kind");
    let rows = vocabulary.extension.as_ref().expect("Kind is closed");
    assert_eq!(&*rows[1].handle, "Savings");
    assert_eq!(rows[1].id, 1);
}

mod interval_newtype {
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{FieldId, IntervalElement, ValueType};
    use bumbledb::{Fact, Interval};

    bumbledb::schema! {
        pub Bookings;

        relation Booking {
            room: u64 as RoomId,
            active: interval<i64> as ActiveDuring,
            window: interval<u64>,
        }
        Booking(room, active) -> Booking;
    }

    #[test]
    fn interval_as_newtype_wraps_the_interval() {
        let booking = Booking {
            room: RoomId(7),
            active: ActiveDuring(Interval::<i64>::new(-10, 20).expect("nonempty")),
            window: Interval::<u64>::new(0, Interval::<u64>::MAX_END).expect("nonempty"),
        };
        assert_eq!(booking.active.0.start(), -10);
        assert_eq!(booking.active.0.end(), 20);

        let schema = {
            use bumbledb::Theory as _;
            Bookings
                .descriptor()
                .validate()
                .expect("the declared schema is valid")
        };
        let relation = schema.relation(Booking::RELATION);
        assert_eq!(
            relation.field(FieldId(1)).value_type,
            ValueType::Interval {
                element: IntervalElement::I64
            }
        );
        assert_eq!(
            relation.field(FieldId(2)).value_type,
            ValueType::Interval {
                element: IntervalElement::U64
            }
        );
    }
}

mod selection_literals {
    use bumbledb::Value;
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{FieldId, StatementId, StatementView};

    bumbledb::schema! {
        pub Telemetry;

        relation Sensor {
            id: u64 as SensorId,
            span: interval<i64>,
            offset: i64,
            live: bool,
            label: str,
            tag: bytes<1>,
        }
        relation Reading { sensor: u64 as SensorId }

        Reading(sensor) <= Sensor(id | span == -10..10, offset == -3, live == true, label == "north", tag == b"\x01");
    }

    #[test]
    fn every_literal_kind_resolves_typed() {
        let schema = {
            use bumbledb::Theory as _;
            Telemetry
                .descriptor()
                .validate()
                .expect("the declared schema is valid")
        };

        let StatementView::Containment(_, statement) = schema.statement(StatementId(1)) else {
            panic!("the declared statement is a containment");
        };
        let target = &statement.target;
        assert_eq!(
            target.selection[..],
            [
                (
                    FieldId(1),
                    bumbledb::schema::LiteralSet::One(Value::IntervalI64(
                        bumbledb::Interval::<i64>::new(-10, 10).expect("nonempty interval")
                    ))
                ),
                (
                    FieldId(2),
                    bumbledb::schema::LiteralSet::One(Value::I64(-3))
                ),
                (
                    FieldId(3),
                    bumbledb::schema::LiteralSet::One(Value::Bool(true))
                ),
                (
                    FieldId(4),
                    bumbledb::schema::LiteralSet::One(Value::String(Box::from("north")))
                ),
                (
                    FieldId(5),
                    bumbledb::schema::LiteralSet::One(Value::FixedBytes(Box::from(&b"\x01"[..])))
                ),
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
            label: "north",
            tag: [0x01],
        };
        let reading = Reading {
            sensor: SensorId(1),
        };
        assert_eq!(reading.sensor, sensor.id);
    }
}

mod fixed_bytes_host_type {

    use bumbledb::Db;

    bumbledb::schema! {
        pub Content;

        relation Object {
            id: u64 as ObjectId,
            hash: bytes<32> as ContentHash,
            head: bytes<9>,
        }

        Object(hash) -> Object;
    }

    #[test]
    fn fixed_bytes_round_trip_through_the_typed_surface() {
        let dir = crate::common::TempDir::new("macro-fixed-bytes");
        let db = Db::create(dir.path(), Content)
            .expect("create")
            .expect("accepted");
        let mut digest = [0u8; 32];
        digest[31] = 0x2A;
        let original = Object {
            id: ObjectId(1),
            hash: ContentHash(digest),
            head: [7u8; 9],
        };
        db.write(|tx| tx.insert([&original]))
            .expect("write")
            .unwrap();
        db.read(|snap| {
            let back: Vec<Object> = snap.scan_facts()?.collect::<Result<_, _>>()?;
            assert_eq!(back, vec![original]);
            Ok(())
        })
        .expect("scan");

        let copied: ContentHash = original.hash;
        assert_eq!(copied, ContentHash(digest));

        let _ = crate::common::expect_rejected(db.write(|tx| {
            tx.insert([&Object {
                id: ObjectId(2),
                hash: ContentHash(digest),
                head: [8u8; 9],
            }])?;
            Ok(())
        }));

        db.read(|snap| {
            // The committed fixed-bytes row is reachable through the typed
            // encode path (append_values → canonical row), not just the
            // scan above: membership re-encodes the exact value.
            assert!(snap.contains(&original)?);
            assert!(!snap.contains(&Object {
                id: ObjectId(1),
                hash: ContentHash([0u8; 32]),
                head: [7u8; 9],
            })?);
            Ok(())
        })
        .expect("read");
    }
}

mod two_schemas_per_module {

    use bumbledb::Db;

    bumbledb::schema! {
        pub LedgerA;
        relation Alpha { id: u64 as AlphaId, note: str }
    }
    bumbledb::schema! {
        pub LedgerB;
        relation Beta { id: u64 as BetaId }
    }

    #[test]
    fn two_schemas_coexist_in_one_module() {
        let dir_a = crate::common::TempDir::new("macro-two-schemas-a");
        let dir_b = crate::common::TempDir::new("macro-two-schemas-b");
        let db_a = Db::create(dir_a.path(), LedgerA)
            .expect("create A")
            .expect("accepted");
        let db_b = Db::create(dir_b.path(), LedgerB)
            .expect("create B")
            .expect("accepted");
        db_a.write(|tx| {
            tx.insert([&Alpha {
                id: AlphaId(1),
                note: "a",
            }])
            .map(|_| ())
        })
        .expect("write A")
        .unwrap();
        db_b.write(|tx| tx.insert([&Beta { id: BetaId(1) }]).map(|_| ()))
            .expect("write B")
            .unwrap();
    }
}

mod closed_relations {

    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{FieldId, RelationId, Row, StatementId, StatementView};
    use bumbledb::{Db, Theory as _, Value};

    bumbledb::schema! {
        pub Review;

        closed relation Status as StatusId = { Open, Frozen, Closed };
        closed relation Kind as KindId {
            mastered: bool,
        } = {
            DirectPass { mastered: true },
            Failed     { mastered: false },
        };

        relation Submission {
            id: u64 as SubmissionId,
            status: u64 as StatusId,
            kind: u64 as KindId,
        }

        Submission(status) <= Status(id);
        Submission(kind | status == Frozen) <= Kind(id | mastered == true);
    }

    #[test]
    fn the_two_tiers_expand_and_validate() {
        Review
            .descriptor()
            .validate()
            .expect("the declared schema is valid");

        let dir = crate::common::TempDir::new("macro-closed-relations");
        Db::create(dir.path(), Review)
            .expect("create")
            .expect("accepted");
    }

    #[test]
    fn the_descriptor_carries_the_extension() {
        let descriptor = Review.descriptor();
        let status = &descriptor.relations[0];
        assert!(status.fields.is_empty());
        let row = |handle: &str, values: &[Value]| Row {
            handle: handle.into(),
            values: values.into(),
        };
        assert_eq!(
            status.extension.as_deref(),
            Some(&[row("Open", &[]), row("Frozen", &[]), row("Closed", &[])][..])
        );
        let kind = &descriptor.relations[1];
        assert_eq!(kind.fields.len(), 1);
        assert_eq!(&*kind.fields[0].name, "mastered");
        assert_eq!(
            kind.extension.as_deref(),
            Some(
                &[
                    row("DirectPass", &[Value::Bool(true)]),
                    row("Failed", &[Value::Bool(false)]),
                ][..]
            )
        );
        assert_eq!(descriptor.relations[2].extension, None);
    }

    #[test]
    fn handles_resolve_to_declaration_order_row_ids() {
        let schema = Review
            .descriptor()
            .validate()
            .expect("the declared schema is valid");

        assert_eq!(schema.keys().len() + schema.containments().len(), 5);
        let StatementView::Containment(_, statement) = schema.statement(StatementId(4)) else {
            panic!("the second declared statement is a containment");
        };
        let source = &statement.source;
        let target = &statement.target;

        assert_eq!(source.relation, Review::SUBMISSION);
        assert_eq!(source.projection[..], [Review::SUBMISSION_KIND]);
        assert_eq!(
            source.selection[..],
            [(
                Review::SUBMISSION_STATUS,
                bumbledb::schema::LiteralSet::One(Value::U64(1))
            )]
        );

        assert_eq!(target.relation, Review::KIND);
        assert_eq!(target.projection[..], [FieldId(0)]);
        assert_eq!(
            target.selection[..],
            [(
                Review::KIND_MASTERED,
                bumbledb::schema::LiteralSet::One(Value::Bool(true))
            )]
        );
    }

    #[test]
    fn id_constants_address_the_sealed_field_list() {
        assert_eq!(Review::STATUS, RelationId(0));
        assert_eq!(Review::KIND, RelationId(1));
        assert_eq!(Review::SUBMISSION, RelationId(2));
        assert_eq!(Review::STATUS_ID, FieldId(0));
        assert_eq!(Review::KIND_ID, FieldId(0));
        assert_eq!(Review::KIND_MASTERED, FieldId(1));
        assert_eq!(Review::SUBMISSION_STATUS, FieldId(1));
    }

    #[test]
    fn the_host_enum_welds_to_row_ids() {
        const FROZEN: StatusId = Status::Frozen.id();
        assert_eq!(FROZEN, StatusId(1));
        assert_eq!(Kind::from_id(Kind::DirectPass.id()), Some(Kind::DirectPass));
        assert_eq!(Kind::from_id(KindId(2)), None);

        let mastered = match Kind::Failed {
            Kind::DirectPass => true,
            Kind::Failed => false,
        };
        assert!(!mastered);
    }

    #[test]
    fn the_manifest_carries_the_extension() {
        let manifest = Review.manifest();
        let status = &manifest.relations[0];
        assert_eq!(&*status.name, "Status");
        let rows = status.extension.as_ref().expect("Status is closed");
        assert_eq!(rows.len(), 3);
        assert_eq!(&*rows[1].handle, "Frozen");
        assert_eq!(rows[1].id, 1);
        assert!(rows[1].values.is_empty());
        let kind = &manifest.relations[1];

        assert_eq!(&*kind.fields[0].name, "id");
        let rows = kind.extension.as_ref().expect("Kind is closed");
        assert_eq!(rows[0].values[..], [("mastered".into(), Value::Bool(true))]);
        assert_eq!(manifest.relations[2].extension, None);
    }
}

mod closed_column_accessors {
    //! Declared columns project too (ruled 2026-07-23, R14 —
    use bumbledb::Theory as _;
    use bumbledb::schema::ValidateDescriptor as _;

    bumbledb::schema! {
        pub Fleet;

        closed relation Tier as TierId = { Free, Paid };
        closed relation Plan as PlanId {
            active: bool,
            rank:   u64,
            drift:  i64,
            tag:    bytes<2>,
            window: interval<u64, 7> as PlanWindow,
            tier:   u64 as TierId,
        } = {
            Basic { active: true,  rank: 1, drift: -3, tag: b"ba", window: 0..7,  tier: Free },
            Pro   { active: false, rank: 2, drift: 4,  tag: b"pr", window: 7..14, tier: Paid },
        };
    }

    #[test]
    fn every_column_projects_as_a_const_accessor() {
        const RANK: u64 = Plan::Pro.rank();
        const WINDOW: PlanWindow = Plan::Basic.window();
        Fleet
            .descriptor()
            .validate()
            .expect("the declared schema is valid");
        assert_eq!(RANK, 2);
        assert!(Plan::Basic.active());
        assert!(!Plan::Pro.active());
        assert_eq!(Plan::Basic.drift(), -3);
        assert_eq!(Plan::Pro.tag(), *b"pr");
        assert_eq!(WINDOW.0.bounds(), (0, 7));
        assert_eq!(Plan::Pro.tier(), Tier::Paid.id());
    }
}

mod discriminated_union {

    use bumbledb::Db;

    bumbledb::schema! {
        pub Graph;

        closed relation GK as GKId = { Det, Custom };

        relation Parent { id: u64 as ParentId, kind: u64 as GKId }
        relation DetArm { parent: u64 as ParentId }

        DetArm(parent) -> DetArm;
        Parent(kind) <= GK(id);
        Parent(id | kind == Det) == DetArm(parent);
    }

    #[test]
    fn the_du_pattern_survives_the_closed_discriminator() {
        let dir = crate::common::TempDir::new("macro-du-closed");
        let db = Db::create(dir.path(), Graph)
            .expect("the DU theory validates")
            .expect("accepted");

        db.write(|tx| {
            let id = ParentId(1);
            tx.insert([&Parent {
                id,
                kind: GK::Det.id(),
            }])?;
            tx.insert([&DetArm { parent: id }])?;
            Ok(())
        })
        .expect("a Det parent with its arm commits")
        .unwrap();

        let _ = crate::common::expect_rejected(db.write(|tx| {
            tx.insert([&Parent {
                id: ParentId(2),
                kind: GK::Det.id(),
            }])?;
            Ok(())
        }));

        db.write(|tx| {
            tx.insert([&Parent {
                id: ParentId(3),
                kind: GK::Custom.id(),
            }])?;
            Ok(())
        })
        .expect("a Custom parent needs no Det arm")
        .unwrap();
    }
}

mod invalid_declaration {

    use bumbledb::Db;
    use bumbledb::error::{SchemaError, StatementErrorKind};

    bumbledb::schema! {
        pub Duplicated;
        relation Parent { id: u64 as ParentId }
        relation Child { parent: u64 as ParentId }
        Child(parent) <= Parent(id);
        Child(parent) <= Parent(id);
    }

    #[test]
    fn invalid_declaration_is_a_typed_schema_error_from_create() {
        let dir = crate::common::TempDir::new("macro-invalid-declaration");
        let Err(err) = Db::create(dir.path(), Duplicated).map(|_| ()) else {
            panic!("a duplicate statement must fail validation at create");
        };
        assert!(
            matches!(
                err,
                bumbledb::Error::Schema(SchemaError::Statement {
                    kind: StatementErrorKind::DuplicateStatement { .. },
                    ..
                })
            ),
            "{err:?}"
        );
    }
}

mod equality_reverse_key {
    use bumbledb::error::{SchemaError, StatementErrorKind};
    use bumbledb::schema::{FieldId, StatementDescriptor, StatementId};
    use bumbledb::{Db, Fact as _, Theory as _};

    bumbledb::schema! {
        pub InvalidEquality;

        relation Source { a: u64 }
        relation Target { x: u64 }

        Target(x) -> Target;
        Source(a) == Target(x);
    }

    #[test]
    fn macro_equality_rejects_the_reverse_half_when_the_left_projection_is_not_a_key() {
        let descriptor = InvalidEquality.descriptor();
        let StatementDescriptor::Containment { target, .. } = &descriptor.statements[2] else {
            panic!("the cited reverse half is a containment");
        };
        assert_eq!(target.relation, Source::RELATION);
        assert_eq!(&*target.projection, &[FieldId(0)]);

        let dir = crate::common::TempDir::new("macro-equality-reverse-key");
        let Err(error) = Db::create(dir.path(), InvalidEquality).map(|_| ()) else {
            panic!("the reverse equality half must require Source(a) as a key");
        };
        assert!(matches!(
            error,
            bumbledb::Error::Schema(SchemaError::Statement {
                statement: StatementId(2),
                kind: StatementErrorKind::NoMatchingTargetKey {
                    target,
                    projection,
                    available,
                    ..
                },
            }) if target == Source::RELATION
                && *projection == [FieldId(0)]
                && available.is_empty()
        ));
    }
}

mod keyed_equality {
    use bumbledb::error::Direction;
    use bumbledb::schema::StatementId;
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::{Db, Theory, Violation};

    bumbledb::schema! {
        pub KeyedEquality;

        relation Source { a: u64, b: i64, c: bool, note: str }
        relation Target { x: u64, y: i64, z: bool, weight: i64 }

        Source(a, b, c) -> Source;

        Target(z, x, y) -> Target;
        Source(a, b, c) == Target(x, y, z);
    }

    fn assert_containment<T: std::fmt::Debug>(
        result: bumbledb::Result<bumbledb::Admission<T>>,
        expected: StatementId,
    ) {
        let violations = crate::common::expect_rejected(result);
        let [(Violation::Containment { direction, .. }, _)] = violations.as_slice() else {
            panic!("expected one containment violation, got {violations:?}");
        };
        let schema = KeyedEquality
            .descriptor()
            .validate()
            .expect("the test schema is valid");
        assert_eq!(violations.get(0).unwrap().statement_id(&schema), expected);
        assert_eq!(*direction, Direction::SourceUnsatisfied);
    }

    #[test]
    fn three_field_reordered_key_equality_validates_and_enforces_both_directions() {
        let dir = crate::common::TempDir::new("macro-keyed-equality");
        let db = Db::create(dir.path(), KeyedEquality)
            .expect("both projected products resolve to declared keys")
            .expect("accepted");

        assert_containment(
            db.write(|tx| {
                tx.insert([&Source {
                    a: 7,
                    b: -3,
                    c: true,
                    note: "source-only",
                }])
            }),
            StatementId(2),
        );

        assert_containment(
            db.write(|tx| {
                tx.insert([&Target {
                    x: 7,
                    y: -3,
                    z: true,
                    weight: 99,
                }])
            }),
            StatementId(3),
        );

        // The selected projections correspond; whole facts do not. Their

        db.write(|tx| {
            tx.insert([&Source {
                a: 7,
                b: -3,
                c: true,
                note: "payloads may differ",
            }])?;
            tx.insert([&Target {
                x: 7,
                y: -3,
                z: true,
                weight: 99,
            }])
        })
        .expect("one witness on each keyed projection commits")
        .unwrap();

        let violations = crate::common::expect_rejected(db.write(|tx| {
            tx.insert([&Target {
                x: 7,
                y: -3,
                z: true,
                weight: 100,
            }])
        }));
        assert!(matches!(
            violations.as_slice(),
            [(Violation::Functionality { .. }, _)]
                if violations.get(0).unwrap().statement_id(
                    &KeyedEquality
                        .descriptor()
                        .validate()
                        .expect("the test schema is valid"),
                ) == StatementId(1)
        ));
    }
}

mod redundant_superkey_enforcement {
    use bumbledb::schema::StatementId;
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::{Db, Interval, Theory as _, Violation};

    bumbledb::schema! {
        pub RedundantKeys;

        relation Window { id: u64, span: interval<i64>, payload: i64 }

        Window(id) -> Window;
        Window(id, span) -> Window;
    }

    #[test]
    fn a_redundant_superkey_still_enforces_both_keys() {
        let _schema = RedundantKeys
            .descriptor()
            .validate()
            .expect("the redundant superkey remains accepted");

        let dir = crate::common::TempDir::new("macro-redundant-superkey");
        let db = Db::create(dir.path(), RedundantKeys)
            .expect("warning is non-fatal")
            .expect("accepted");
        let error = db.write(|tx| {
            tx.insert([&Window {
                id: 7,
                span: Interval::<i64>::new(0, 5).expect("interval"),
                payload: 10,
            }])?;
            tx.insert([&Window {
                id: 7,
                span: Interval::<i64>::new(3, 8).expect("interval"),
                payload: 20,
            }])
        });
        let violations = crate::common::expect_rejected(error);
        assert_eq!(violations.as_slice().len(), 2);
        assert!(matches!(
            violations.as_slice(),
            [
                (Violation::Functionality { .. }, _),
                (Violation::Functionality { .. }, _),
            ] if {
                let schema = RedundantKeys
                    .descriptor()
                    .validate()
                    .expect("the test schema is valid");
                violations.get(0).unwrap().statement_id(&schema) == StatementId(0)
                    && violations.get(1).unwrap().statement_id(&schema) == StatementId(1)
            }
        ));
    }
}

mod extension_forms {

    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{Bound, LiteralSet, StatementDescriptor, StatementView, Weight};
    use bumbledb::{StatementId, Theory as _, Value};

    bumbledb::schema! {
        pub Tracker;

        closed relation Priority as PriorityId {
            weight: u64,
        } = {
            Low  { weight: 10 },
            High { weight: 20 },
        };

        relation Parent { id: u64 as ParentId }
        relation Task {
            parent: u64 as ParentId,
            pos:    u64,
            prio:   u64 as PriorityId,
            state:  u64,
        }

        Parent(id) <={1..3} Task(parent | state == {1, 2});
        Parent(id) <=[state]{2..*} Task(parent);
        Parent(id) <={4} Task(parent | state == 3);
        Parent(id) <={0} Task(parent | state == 9);
    }

    #[test]
    fn the_extension_forms_lower_and_validate() {
        let schema = Tracker
            .descriptor()
            .validate()
            .expect("the declared schema is valid");

        assert!(matches!(
            schema.statement(StatementId(2)),
            StatementView::Capacity(_, _)
        ));
        let cap = &schema.capacities()[0];
        assert_eq!(cap.lo, 1);
        assert_eq!(cap.hi.to_bound(), Some(Bound::Lit(3)));
        assert_eq!(cap.weight.to_weight(), Weight::Unit);
        assert_eq!(cap.target.relation, Tracker::PARENT);
        assert_eq!(cap.source.relation, Tracker::TASK);
        assert_eq!(
            cap.source.selection[..],
            [(
                Tracker::TASK_STATE,
                LiteralSet::Many(Box::new([Value::U64(1), Value::U64(2)]))
            )]
        );
        let star = &schema.capacities()[1];
        assert_eq!(star.lo, 2);
        assert_eq!(star.hi.to_bound(), None);
        assert_eq!(star.weight.to_weight(), Weight::Field(Tracker::TASK_STATE));
        let exact = &schema.capacities()[2];
        assert_eq!(exact.lo, 4);
        assert_eq!(exact.hi.to_bound(), Some(Bound::Lit(4)));
        let exclusion = &schema.capacities()[3];
        assert_eq!(exclusion.lo, 0);
        assert_eq!(exclusion.hi.to_bound(), Some(Bound::Lit(0)));
    }

    /// absence (ruled 2026-07-24, C4).
    #[test]
    fn the_capacity_descriptor_is_target_left() {
        let descriptor = Tracker.descriptor();
        let Some(StatementDescriptor::Capacity {
            source,
            weight,
            target,
            ..
        }) = descriptor.statements.first()
        else {
            panic!("the first declared statement is the capacity statement");
        };
        assert_eq!(*weight, Weight::Unit);
        assert_eq!(target.relation, Tracker::PARENT);
        assert_eq!(source.relation, Tracker::TASK);
        assert!(matches!(source.selection[0].1, LiteralSet::Many(_)));
    }
}

mod capacity_forms {

    use bumbledb::Theory as _;
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{Bound, FieldId, StatementDescriptor, Weight};

    bumbledb::schema! {
        pub Grid;

        relation Pool {
            id: u64 as PoolId,
            supply: u64,
        }
        relation Device {
            id: u64 as DeviceId,
            pool: u64 as PoolId,
            watts: u64,
            booked: interval<u64>,
        }

        Pool(id) <=[watts]{0..supply} Device(pool);
        Pool(id) <=[Duration(booked)]{0..720} Device(pool);
        Pool(id) <=[watts]{1..*} Device(pool);
    }

    #[test]
    fn the_weighted_forms_lower_and_validate() {
        let descriptor = Grid.descriptor();
        assert_eq!(
            descriptor.statements[..],
            [
                StatementDescriptor::Capacity {
                    target: bumbledb::schema::Side {
                        relation: Grid::POOL,
                        projection: Box::new([FieldId(0)]),
                        selection: Box::new([]),
                    },
                    weight: Weight::Field(Grid::DEVICE_WATTS),
                    lo: 0,
                    hi: Some(Bound::TargetField(Grid::POOL_SUPPLY)),
                    source: bumbledb::schema::Side {
                        relation: Grid::DEVICE,
                        projection: Box::new([Grid::DEVICE_POOL]),
                        selection: Box::new([]),
                    },
                },
                StatementDescriptor::Capacity {
                    target: bumbledb::schema::Side {
                        relation: Grid::POOL,
                        projection: Box::new([FieldId(0)]),
                        selection: Box::new([]),
                    },
                    weight: Weight::DurationOf(Grid::DEVICE_BOOKED),
                    lo: 0,
                    hi: Some(Bound::Lit(720)),
                    source: bumbledb::schema::Side {
                        relation: Grid::DEVICE,
                        projection: Box::new([Grid::DEVICE_POOL]),
                        selection: Box::new([]),
                    },
                },
                StatementDescriptor::Capacity {
                    target: bumbledb::schema::Side {
                        relation: Grid::POOL,
                        projection: Box::new([FieldId(0)]),
                        selection: Box::new([]),
                    },
                    weight: Weight::Field(Grid::DEVICE_WATTS),
                    lo: 1,
                    hi: None,
                    source: bumbledb::schema::Side {
                        relation: Grid::DEVICE,
                        projection: Box::new([Grid::DEVICE_POOL]),
                        selection: Box::new([]),
                    },
                },
            ]
        );
        let schema = descriptor.validate().expect("the weighted theory seals");

        let caps = schema.capacities();
        assert_eq!(caps.len(), 3);
        assert_eq!(
            caps[0].hi.to_bound(),
            Some(Bound::TargetField(Grid::POOL_SUPPLY))
        );
        assert_eq!(
            caps[1].weight.to_weight(),
            Weight::DurationOf(Grid::DEVICE_BOOKED)
        );
        assert_eq!(caps[2].lo, 1);
        assert_eq!(caps[2].hi.to_bound(), None);

        let dir = crate::common::TempDir::new("macro-capacity-forms");
        bumbledb::Db::create(dir.path(), Grid)
            .expect("create")
            .expect("accepted");
    }
}

#[allow(non_snake_case)]
mod duration_named_field {

    use bumbledb::Theory as _;
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{Bound, StatementDescriptor, Weight};

    bumbledb::schema! {
        pub Quota;

        relation Bucket {
            id: u64 as BucketId,
            Duration: u64,
        }
        relation Item {
            id: u64 as ItemId,
            bucket: u64 as BucketId,
            Duration: u64,
        }

        Bucket(id) <=[Duration]{0..Duration} Item(bucket);
    }

    #[test]
    fn a_field_named_duration_is_an_ordinary_ident_in_both_slots() {
        let descriptor = Quota.descriptor();
        let [StatementDescriptor::Capacity { weight, hi, .. }, ..] = &descriptor.statements[..]
        else {
            panic!("the declared capacity statement leads");
        };
        assert_eq!(*weight, Weight::Field(Quota::ITEM_DURATION));
        assert_eq!(*hi, Some(Bound::TargetField(Quota::BUCKET_DURATION)));
        descriptor.validate().expect("the theory seals");
    }
}

mod radix_literals {
    //! Integer literals are rustc's (ruled 2026-07-23, R8): the
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{Bound, FixedIntervalElement, LiteralSet, ValueType};
    use bumbledb::{Theory as _, Value};

    bumbledb::schema! {
        pub Radix;

        relation Chunk {
            digest: bytes<0x20>,
            span:   interval<u64, 1_0>,
        }
        relation Parent { id: u64 as ParentId }
        relation Task { parent: u64 as ParentId, state: u64 }

        Parent(id) <={0x2..0b100} Task(parent | state == 0o17);
    }

    #[test]
    fn every_integer_position_reads_rustc_literals() {
        let descriptor = Radix.descriptor();
        assert_eq!(
            descriptor.relations[0].fields[0].value_type,
            ValueType::FixedBytes { len: 32 }
        );
        assert_eq!(
            descriptor.relations[0].fields[1].value_type,
            ValueType::FixedInterval {
                element: FixedIntervalElement::U64,
                width: 10
            }
        );
        let schema = descriptor.validate().expect("the declared schema is valid");
        let cap = &schema.capacities()[0];
        assert_eq!(cap.lo, 2);
        assert_eq!(cap.hi.to_bound(), Some(Bound::Lit(4)));
        assert_eq!(
            cap.source.selection[..],
            [(Radix::TASK_STATE, LiteralSet::One(Value::U64(15)))]
        );
    }
}

mod fixed_width_intervals {

    //! `lean/Bumbledb/Values.lean: FixedU64.not_ray`). One stored word,
    use bumbledb::ir::{
        Atom, CmpOp, Comparison, ConditionTree, FindTerm, Query, Rule, Term, Value, VarId,
    };
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{FieldId, ValueType};
    use bumbledb::{AllenMask, AnswerValue, Db, Fact, Interval, Theory as _};

    bumbledb::schema! {
        pub Jukebox;

        relation Slot {
            playlist: u64,
            slot: interval<u64, 5> as SlotSpan,
            track: u64,
        }

        Slot(playlist, slot) -> Slot;
    }

    #[test]
    fn the_width_is_the_type_and_the_encoding_is_the_start() {
        let descriptor = Jukebox.descriptor();
        assert_eq!(
            descriptor.relations[0].fields[1].value_type,
            ValueType::FixedInterval {
                element: bumbledb::schema::FixedIntervalElement::U64,
                width: 5
            }
        );

        let schema = descriptor.validate().expect("valid");
        assert_eq!(schema.relations()[0].layout().fact_width(), 24);
    }

    #[test]
    fn typed_writes_check_the_declared_width_and_round_trip() {
        let dir = crate::common::TempDir::new("macro-fixed-interval");
        let db = Db::create(dir.path(), Jukebox)
            .expect("create")
            .expect("accepted");
        let slot = Slot {
            playlist: 1,
            slot: SlotSpan(Interval::<u64>::fixed(10, 5).expect("in-domain")),
            track: 77,
        };
        db.write(|tx| tx.insert([&slot])).expect("write").unwrap();
        db.read(|snap| {
            let back: Vec<Slot> = snap.scan_facts()?.collect::<Result<_, _>>()?;
            assert_eq!(back, vec![slot]);
            Ok(())
        })
        .expect("scan");

        let err = db
            .write(|tx| {
                tx.insert([&Slot {
                    playlist: 1,
                    slot: SlotSpan(Interval::<u64>::new(100, 107).expect("nonempty")),
                    track: 78,
                }])?;
                Ok(())
            })
            .unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::FactShape(_)),
            "a wrong-width interval is a typed shape error, got {err:?}"
        );
    }

    #[test]
    fn a_width_matched_ray_is_rejected_at_the_typed_boundary() {
        let dir = crate::common::TempDir::new("macro-fixed-ray");
        let db = Db::create(dir.path(), Jukebox)
            .expect("create")
            .expect("accepted");
        let err = db
            .write(|tx| {
                tx.insert([&Slot {
                    playlist: 1,
                    slot: SlotSpan(
                        Interval::<u64>::new(u64::MAX - 5, u64::MAX).expect("a legal general ray"),
                    ),
                    track: 79,
                }])?;
                Ok(())
            })
            .unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::FactShape(_)),
            "the width-matched ray must be a typed shape error, got {err:?}"
        );
    }

    #[test]
    fn the_fixed_pointwise_key_rejects_overlap_and_accepts_adjacency() {
        let dir = crate::common::TempDir::new("macro-fixed-pointwise");
        let db = Db::create(dir.path(), Jukebox)
            .expect("create")
            .expect("accepted");
        let slot = |playlist: u64, start: u64, track: u64| Slot {
            playlist,
            slot: SlotSpan(Interval::<u64>::fixed(start, 5).expect("in-domain")),
            track,
        };

        db.write(|tx| {
            tx.insert([&slot(1, 10, 1)])?;
            tx.insert([&slot(1, 15, 2)])?;
            tx.insert([&slot(2, 12, 3)])?;
            Ok(())
        })
        .expect("adjacency and cross-group starts are legal")
        .unwrap();

        let _ = crate::common::expect_rejected(db.write(|tx| tx.insert([&slot(1, 12, 4)])));
    }

    #[test]
    fn the_key_probe_lane_finds_an_exact_fixed_tuple() {
        let dir = crate::common::TempDir::new("macro-fixed-key-probe");
        let db = Db::create(dir.path(), Jukebox)
            .expect("create")
            .expect("accepted");
        db.write(|tx| {
            tx.insert([&Slot {
                playlist: 1,
                slot: SlotSpan(Interval::<u64>::fixed(10, 5).expect("in-domain")),
                track: 77,
            }])
        })
        .expect("write")
        .unwrap();
        let lookup = |start: u64| {
            Query::single(Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![Atom {
                    source: bumbledb::AtomSource::Edb(Slot::RELATION),
                    bindings: vec![
                        (FieldId(0), Term::Literal(Value::U64(1))),
                        (
                            FieldId(1),
                            Term::Literal(Value::IntervalU64(
                                Interval::<u64>::fixed(start, 5).expect("in-domain"),
                            )),
                        ),
                        (FieldId(2), Term::Var(VarId(0))),
                    ],
                }],
                negated: vec![],
                conditions: vec![],
            })
        };
        let mut hit = db.prepare(&lookup(10)).expect("prepare");
        let mut miss = db.prepare(&lookup(11)).expect("prepare");
        db.read(|snap| {
            let answers = snap.execute_collect(&mut hit, &[] as &[bumbledb::BindValue])?;
            assert_eq!(answers.len(), 1);
            assert_eq!(answers.get(0, 0), AnswerValue::U64(77));
            assert!(
                snap.execute_collect(&mut miss, &[] as &[bumbledb::BindValue])?
                    .is_empty()
            );
            Ok(())
        })
        .expect("key probe");
    }

    fn membership_query(point: u64) -> Query {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Edb(Slot::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Literal(Value::U64(1))),
                    (FieldId(1), Term::Literal(Value::U64(point))),
                    (FieldId(2), Term::Var(VarId(0))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        })
    }

    fn allen_meets_query() -> Query {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![
                Atom {
                    source: bumbledb::AtomSource::Edb(Slot::RELATION),
                    bindings: vec![
                        (FieldId(1), Term::Var(VarId(2))),
                        (FieldId(2), Term::Var(VarId(0))),
                    ],
                },
                Atom {
                    source: bumbledb::AtomSource::Edb(Slot::RELATION),
                    bindings: vec![
                        (FieldId(1), Term::Var(VarId(3))),
                        (FieldId(2), Term::Var(VarId(1))),
                    ],
                },
            ],
            negated: vec![],
            conditions: vec![ConditionTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: AllenMask::MEETS,
                },
                lhs: Term::Var(VarId(2)),
                rhs: Term::Var(VarId(3)),
            })],
        })
    }

    #[test]
    fn membership_and_allen_run_over_derived_bounds() {
        let dir = crate::common::TempDir::new("macro-fixed-kernels");
        let db = Db::create(dir.path(), Jukebox)
            .expect("create")
            .expect("accepted");
        db.write(|tx| {
            for (start, track) in [(10u64, 1u64), (15, 2), (25, 3)] {
                tx.insert([&Slot {
                    playlist: 1,
                    slot: SlotSpan(Interval::<u64>::fixed(start, 5).expect("in-domain")),
                    track,
                }])?;
            }
            Ok(())
        })
        .expect("write")
        .unwrap();

        // (`lean/Bumbledb/Query/Membership.lean: pointMem_fixed_u64`).
        let mut covers_12 = db.prepare(&membership_query(12)).expect("prepare");
        let mut covers_15 = db.prepare(&membership_query(15)).expect("prepare");
        let mut covers_20 = db.prepare(&membership_query(20)).expect("prepare");
        db.read(|snap| {
            let answers = snap.execute_collect(&mut covers_12, &[] as &[bumbledb::BindValue])?;
            assert_eq!(answers.len(), 1);
            assert_eq!(answers.get(0, 0), AnswerValue::U64(1));

            let answers = snap.execute_collect(&mut covers_15, &[] as &[bumbledb::BindValue])?;
            assert_eq!(answers.len(), 1);
            assert_eq!(answers.get(0, 0), AnswerValue::U64(2));

            let answers = snap.execute_collect(&mut covers_20, &[] as &[bumbledb::BindValue])?;
            assert!(answers.is_empty());
            Ok(())
        })
        .expect("membership");

        let mut meets = db.prepare(&allen_meets_query()).expect("prepare");
        db.read(|snap| {
            let answers = snap.execute_collect(&mut meets, &[] as &[bumbledb::BindValue])?;
            let mut pairs: Vec<(u64, u64)> = (0..answers.len())
                .map(|i| {
                    let (AnswerValue::U64(a), AnswerValue::U64(b)) =
                        (answers.get(i, 0), answers.get(i, 1))
                    else {
                        panic!("both columns are u64 tracks");
                    };
                    (a, b)
                })
                .collect();
            pairs.sort_unstable();
            assert_eq!(pairs, vec![(1, 2)], "[10,15) meets [15,20), and only it");
            Ok(())
        })
        .expect("allen");
    }
}

mod element_domain_typing {
    //! Q1: interval positions carry an element domain and never a width.
    //! `lean/Bumbledb/Schema.lean: Value.points_one_tag_u64`.
    use bumbledb::error::Direction;
    use bumbledb::ir::{
        Atom, CmpOp, Comparison, ConditionTree, FindTerm, Query, Rule, Term, VarId,
    };
    use bumbledb::schema::ValidateDescriptor as _;
    use bumbledb::schema::{FieldId, StatementId};
    use bumbledb::{AllenMask, AnswerValue, Db, Fact as _, Interval, Theory, Violation};

    bumbledb::schema! {
        pub Playlists;

        relation Playlist {
            id: u64 as PlaylistId,
            span: interval<u64>,
        }

        relation Slot {
            playlist: u64 as PlaylistId,
            slot: interval<u64, 1>,
            track: u64,
        }

        Playlist(id, span) -> Playlist;
        Slot(playlist, slot) -> Slot;
        Slot(playlist, slot) == Playlist(id, span);
    }

    fn unit(at: u64) -> Interval<u64> {
        Interval::<u64>::fixed(at, 1).expect("in-domain unit slot")
    }

    fn tile(db: &Db<Playlists>) -> PlaylistId {
        db.write(|tx| {
            let id = PlaylistId(1);
            tx.insert([&Playlist {
                id,
                span: Interval::<u64>::new(0, 3).expect("nonempty"),
            }])?;
            for (at, track) in [(0, 100), (1, 200), (2, 300)] {
                tx.insert([&Slot {
                    playlist: id,
                    slot: unit(at),
                    track,
                }])?;
            }
            Ok(id)
        })
        .expect("an exact tiling commits")
        .unwrap()
        .value
    }

    #[test]
    fn the_playlist_recipe_validates_and_a_tiling_commits() {
        let dir = crate::common::TempDir::new("macro-q1-tiling");
        let db = Db::create(dir.path(), Playlists)
            .expect("Q1: the recipe validates")
            .expect("accepted");
        let id = tile(&db);
        db.read(|snap| {
            let slots: Vec<Slot> = snap.scan_facts()?.collect::<Result<_, _>>()?;
            assert_eq!(slots.len(), 3);
            assert!(slots.iter().all(|s| s.playlist == id));
            Ok(())
        })
        .expect("scan");
    }

    #[test]
    fn a_gap_delta_aborts() {
        let dir = crate::common::TempDir::new("macro-q1-gap");
        let db = Db::create(dir.path(), Playlists)
            .expect("create")
            .expect("accepted");
        let violations = crate::common::expect_rejected(db.write(|tx| {
            let id = PlaylistId(1);
            tx.insert([&Playlist {
                id,
                span: Interval::<u64>::new(0, 3).expect("nonempty"),
            }])?;
            for (at, track) in [(0, 100), (2, 300)] {
                tx.insert([&Slot {
                    playlist: id,
                    slot: unit(at),
                    track,
                }])?;
            }
            Ok(())
        }));
        assert!(
            matches!(
                violations.as_slice(),
                [(
                    Violation::Containment {
                        direction: Direction::SourceUnsatisfied,
                        ..
                    },
                    _
                )]
            ),
            "the uncovered span point convicts the coverage direction, got {violations:?}"
        );
    }

    #[test]
    fn an_overlap_delta_aborts() {
        let dir = crate::common::TempDir::new("macro-q1-overlap");
        let db = Db::create(dir.path(), Playlists)
            .expect("create")
            .expect("accepted");
        let id = tile(&db);
        let violations = crate::common::expect_rejected(db.write(|tx| {
            tx.insert([&Slot {
                playlist: id,
                slot: unit(1),
                track: 999,
            }])
        }));
        assert!(
            matches!(
                violations.as_slice(),
                [(Violation::Functionality { .. }, _)]
                    if violations.get(0).unwrap().statement_id(
                        &Playlists
                            .descriptor()
                            .validate()
                            .expect("the test schema is valid"),
                    ) == StatementId(2)
            ),
            "the pointwise key convicts the overlap, got {violations:?}"
        );
    }

    #[test]
    fn a_slot_past_the_span_aborts() {
        let dir = crate::common::TempDir::new("macro-q1-past-end");
        let db = Db::create(dir.path(), Playlists)
            .expect("create")
            .expect("accepted");
        let id = tile(&db);
        let violations = crate::common::expect_rejected(db.write(|tx| {
            tx.insert([&Slot {
                playlist: id,
                slot: unit(3),
                track: 999,
            }])
        }));
        assert!(
            matches!(
                violations.as_slice(),
                [(
                    Violation::Containment {
                        direction: Direction::SourceUnsatisfied,
                        ..
                    },
                    _
                )]
            ),
            "the uncovered slot convicts the slot-side coverage, got {violations:?}"
        );
    }

    #[test]
    fn a_mixed_width_allen_query_classifies_with_hand_answers() {
        let dir = crate::common::TempDir::new("macro-q1-allen");
        let db = Db::create(dir.path(), Playlists)
            .expect("create")
            .expect("accepted");
        tile(&db);

        let query = |mask: AllenMask| {
            Query::single(Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![
                    Atom {
                        source: bumbledb::AtomSource::Edb(Playlist::RELATION),
                        bindings: vec![
                            (FieldId(0), Term::Var(VarId(3))),
                            (FieldId(1), Term::Var(VarId(1))),
                        ],
                    },
                    Atom {
                        source: bumbledb::AtomSource::Edb(Slot::RELATION),
                        bindings: vec![
                            (FieldId(0), Term::Var(VarId(3))),
                            (FieldId(1), Term::Var(VarId(2))),
                            (FieldId(2), Term::Var(VarId(0))),
                        ],
                    },
                ],
                negated: vec![],
                conditions: vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Allen { mask },
                    lhs: Term::Var(VarId(2)),
                    rhs: Term::Var(VarId(1)),
                })],
            })
        };
        let answers = |mask: AllenMask| -> Vec<u64> {
            let mut prepared = db.prepare(&query(mask)).expect("Q1: mixed widths classify");
            db.read(|snap| {
                let out = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
                let mut tracks: Vec<u64> = (0..out.len())
                    .map(|i| match out.get(i, 0) {
                        AnswerValue::U64(t) => t,
                        other => panic!("track answers are u64, got {other:?}"),
                    })
                    .collect();
                tracks.sort_unstable();
                Ok(tracks)
            })
            .expect("execute")
        };
        assert_eq!(
            answers(AllenMask::STARTS),
            vec![100],
            "slot [0,1) starts [0,3)"
        );
        assert_eq!(
            answers(AllenMask::DURING),
            vec![200],
            "slot [1,2) is during [0,3)"
        );
        assert_eq!(
            answers(AllenMask::FINISHES),
            vec![300],
            "slot [2,3) finishes [0,3)"
        );
        assert_eq!(
            answers(AllenMask::INTERSECTS),
            vec![100, 200, 300],
            "every unit slot intersects its span"
        );
        assert_eq!(answers(AllenMask::AFTER), Vec::<u64>::new());
    }
}

mod newtype_coherence_pass {
    bumbledb::schema! {
        pub BareFaces;

        relation Node { id: u64 }
        relation Edge { from: u64, to: u64 }

        Node(id) -> Node;
        Edge(from) <= Node(id);
        Edge(to)   <= Node(id);
    }

    #[test]
    fn bare_pairs_with_bare_and_the_theory_seals() {
        let dir = crate::common::TempDir::new("m5-bare-faces");
        bumbledb::Db::create(dir.path(), BareFaces)
            .expect("bare faces pair with bare faces — the coherence check passes")
            .expect("accepted");
    }
}
