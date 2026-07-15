//! Schema-macro expansion tests (docs/architecture/70-api.md): the
//! `schema!` macro is exactly sugar — its expansion resolves to the same
//! validated schema a hand-built [`SchemaDescriptor`] does, statements in
//! source order with `==` lowered to two adjacent containments (`A <= B`
//! first) and selection literals typed in the macro (closed-relation
//! handles as declaration-order row ids).
//!
//! The example schema is `docs/architecture/30-dependencies.md`'s, minus
//! its illustrative `Employment` line (that relation is not declared).

use bumbledb::schema::fingerprint::fingerprint;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, LiteralSet, RelationDescriptor,
    RelationId, Row, SchemaDescriptor, Side, StatementDescriptor, StatementId, StatementView,
    ValueType,
};
use bumbledb::{Db, Fact, Interval, Value};

mod common;

/// The macro's declared schema, validated — what the assertions inspect.
/// The engine itself takes [`Ledger`]; validation runs inside
/// `Db::create`/`Db::open`.
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

    relation Holder  { id: u64 as HolderId, fresh, name: str }
    relation Account {
        id: u64 as AccountId, fresh,
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
        generation: Generation::None,
    }
}

fn fresh_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    }
}

/// `Account(id | kind == Savings)` — the selected side of the `==`, its
/// handle literal resolved to the declaration-order row id.
fn savings_accounts() -> Side {
    Side {
        relation: RelationId(2),
        projection: Box::new([FieldId(0)]),
        selection: Box::new([(FieldId(2), LiteralSet::One(Value::U64(1)))]),
    }
}

/// `SavingsTerms(account)`.
fn savings_terms_side() -> Side {
    Side {
        relation: RelationId(3),
        projection: Box::new([FieldId(0)]),
        selection: Box::new([]),
    }
}

/// The same declaration, hand-built through the descriptor contract.
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
    let descriptors: Vec<StatementDescriptor> = (0..8)
        .map(|id| match schema.statement(StatementId(id)) {
            StatementView::Key(_, statement) => StatementDescriptor::Functionality {
                relation: statement.relation,
                projection: statement.projection.clone(),
            },
            StatementView::Containment(_, statement) => StatementDescriptor::Containment {
                source: statement.source.clone(),
                target: statement.target.clone(),
            },
            StatementView::Cardinality(..) | StatementView::Order(..) => {
                unreachable!("this fixture declares keys and containments only")
            }
        })
        .collect();
    // Materialized order: the two fresh auto-FDs first (Holder.id,
    // Account.id), then Kind's closed auto-key, then the declared
    // statements in source order — the `==` contributing its two
    // containments adjacently, `A <= B` first.
    assert_eq!(descriptors.len(), 8);
    assert_eq!(
        descriptors[0],
        StatementDescriptor::Functionality {
            relation: RelationId(1),
            projection: Box::new([FieldId(0)]),
        }
    );
    assert_eq!(
        descriptors[1],
        StatementDescriptor::Functionality {
            relation: RelationId(2),
            projection: Box::new([FieldId(0)]),
        }
    );
    assert_eq!(
        descriptors[2],
        StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }
    );
    assert_eq!(
        descriptors[3],
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
        descriptors[4],
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
        descriptors[5],
        StatementDescriptor::Containment {
            source: savings_accounts(),
            target: savings_terms_side(),
        }
    );
    assert_eq!(
        descriptors[6],
        StatementDescriptor::Containment {
            source: savings_terms_side(),
            target: savings_accounts(),
        }
    );
    assert_eq!(
        descriptors[7],
        StatementDescriptor::Functionality {
            relation: RelationId(3),
            projection: Box::new([FieldId(0)]),
        }
    );
}

#[test]
fn the_equality_pair_seals_mirror_links() {
    // The macro's `==` lowers to ids 5 and 6, which seal pointing at each
    // other; every FD and the one-way containments carry `None`.
    let schema = declared();
    let mirrors: Vec<Option<StatementId>> = (0..8)
        .map(|id| match schema.statement(StatementId(id)) {
            StatementView::Key(_, _)
            | StatementView::Cardinality(..)
            | StatementView::Order(..) => None,
            StatementView::Containment(_, statement) => statement.mirror,
        })
        .collect();
    assert_eq!(
        mirrors,
        vec![
            None,
            None,
            None,
            None,
            None,
            Some(StatementId(6)),
            Some(StatementId(5)),
            None
        ]
    );
}

#[test]
fn fact_structs_carry_host_types() {
    // interval<i64> without `as`: the fact field is the raw engine value.
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
fn typed_round_trip_through_fact_bytes() {
    let dir = common::TempDir::new("macro-round-trip");
    let db = Db::create(dir.path(), Ledger).expect("create");

    let original = Account {
        id: AccountId(7),
        holder: HolderId(3),
        kind: Kind::Checking.id(),
        active: Interval::<i64>::new(-100, 100).expect("nonempty"),
    };
    // Write the holder + account (interning the string through the delta),
    // then decode back through a snapshot.
    db.write(|tx| {
        tx.insert(&Holder {
            id: HolderId(3),
            name: "alice",
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
            name: "nobody",
        };
        let mut bytes = Vec::new();
        assert!(!ghost.encode_read(snap, &mut bytes).expect("encode"));
        Ok(())
    })
    .expect("read");
}

#[test]
fn id_constants_are_declaration_order_named_data() {
    // The macro emits declaration-order id constants on the theory
    // (docs/architecture/70-api.md § id constants): per relation, per
    // field — the Rust host never writes a magic number into an
    // `ir::Query`.
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
    // The vocabulary's numbers live on the host enum (no per-handle
    // constants exist): the weld is `id`/`from_id`.
    assert_eq!(Kind::Checking.id(), KindId(0));
    assert_eq!(Kind::Savings.id(), KindId(1));
}

#[test]
fn the_manifest_is_the_constants_runtime_twin() {
    // The manifest (docs/architecture/70-api.md § the manifest): the
    // same numbers as plain data, reachable from the theory, for hosts
    // that cannot see Rust constants. No serde anywhere — the value is
    // the surface.
    use bumbledb::Theory as _;
    let manifest = Ledger.manifest();
    assert_eq!(manifest.relations.len(), 4);
    let account = &manifest.relations[2];
    assert_eq!(&*account.name, "Account");
    assert_eq!(account.id, Ledger::ACCOUNT);
    let kind = &account.fields[2];
    assert_eq!(&*kind.name, "kind");
    assert_eq!(kind.id, Ledger::ACCOUNT_KIND);
    // The reference field is structurally a u64; the vocabulary rides
    // the closed relation's extension table, row id = declaration index.
    assert_eq!(kind.value_type, ValueType::U64);
    let vocabulary = &manifest.relations[0];
    assert_eq!(&*vocabulary.name, "Kind");
    let rows = vocabulary.extension.as_ref().expect("Kind is closed");
    assert_eq!(&*rows[1].handle, "Savings");
    assert_eq!(rows[1].id, 1);
}

mod interval_newtype {
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
    use bumbledb::Value;
    use bumbledb::schema::{FieldId, StatementId, StatementView};

    bumbledb::schema! {
        pub Telemetry;

        relation Sensor {
            id: u64 as SensorId, fresh,
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
        // Statement 0 is Sensor.id's fresh auto-FD; 1 is the containment.
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
                    bumbledb::schema::LiteralSet::One(Value::String(Box::from(&b"north"[..])))
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
    //! `bytes<N>` emits `[u8; N]` — owned, `Copy`, lifetime-free — and
    //! takes `as NewType` (order-free, like interval newtypes: a
    //! digest's lexicographic order is an encoding artifact). The typed
    //! round-trip goes fact → canonical bytes → fact with zero
    //! dictionary traffic.

    use bumbledb::{Db, Fact as _};

    bumbledb::schema! {
        pub Content;

        relation Object {
            id: u64 as ObjectId, fresh,
            hash: bytes<32> as ContentHash,
            head: bytes<9>,
        }

        Object(hash) -> Object;
    }

    #[test]
    fn fixed_bytes_round_trip_through_the_typed_surface() {
        let dir = crate::common::TempDir::new("macro-fixed-bytes");
        let db = Db::create(dir.path(), Content).expect("create");
        let mut digest = [0u8; 32];
        digest[31] = 0x2A;
        let original = Object {
            id: ObjectId(1),
            hash: ContentHash(digest),
            head: [7u8; 9],
        };
        db.write(|tx| tx.insert(&original)).expect("write");
        db.read(|snap| {
            let back: Vec<Object> = snap.scan_facts()?.collect::<Result<_, _>>()?;
            assert_eq!(back, vec![original.clone()]);
            Ok(())
        })
        .expect("scan");
        // The newtype is Copy and order-free; the value round-trips
        // by identity.
        let copied: ContentHash = original.hash;
        assert_eq!(copied, ContentHash(digest));
        // The bytes<32> key determinants writes: a second object under the
        // same hash is a functionality violation.
        let err = db
            .write(|tx| {
                tx.insert(&Object {
                    id: ObjectId(2),
                    hash: ContentHash(digest),
                    head: [8u8; 9],
                })?;
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(err, bumbledb::Error::CommitRejected { .. }));
        // encode_read is infallible for bytes<N> (no dictionary miss
        // exists for an inline value).
        db.read(|snap| {
            let mut bytes = Vec::new();
            assert!(original.encode_read(snap, &mut bytes).expect("encode"));
            let decoded = Object::decode(snap, &bytes).expect("decode");
            assert_eq!(decoded, original);
            Ok(())
        })
        .expect("read");
    }
}

mod two_schemas_per_module {
    //! Two `schema!` invocations coexist in one module — their `pub Name;`
    //! headers disambiguate (the old one-invocation-per-module limit died
    //! with the magic `schema()` constructor).

    use bumbledb::Db;

    bumbledb::schema! {
        pub LedgerA;
        relation Alpha { id: u64 as AlphaId, fresh, note: str }
    }
    bumbledb::schema! {
        pub LedgerB;
        relation Beta { id: u64 as BetaId, fresh }
    }

    #[test]
    fn two_schemas_coexist_in_one_module() {
        let dir_a = crate::common::TempDir::new("macro-two-schemas-a");
        let dir_b = crate::common::TempDir::new("macro-two-schemas-b");
        let db_a = Db::create(dir_a.path(), LedgerA).expect("create A");
        let db_b = Db::create(dir_b.path(), LedgerB).expect("create B");
        db_a.write(|tx| {
            let id = tx.alloc::<AlphaId>()?;
            tx.insert(&Alpha { id, note: "a" }).map(|_| ())
        })
        .expect("write A");
        db_b.write(|tx| {
            let id = tx.alloc::<BetaId>()?;
            tx.insert(&Beta { id }).map(|_| ())
        })
        .expect("write B");
    }
}

mod closed_relations {
    //! The emission (`docs/architecture/70-api.md` § the `schema!`
    //! grammar): closed relations declare their extension in the schema.
    //! The host enum is an emission, not a type — the engine's vocabulary
    //! is relational, projected into a Rust enum so rustc's pattern
    //! checking keeps working; the weld test pinning `id`/`from_id` is
    //! EMITTED per closed relation (`__bumbledb_weld_status`,
    //! `__bumbledb_weld_kind` — running in this very module), so it cannot
    //! be forgotten for a new theory. No fact struct and no `Fact` impl
    //! exist for `Status`/`Kind` — closed relations are unwritable.

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
            id: u64 as SubmissionId, fresh,
            status: u64 as StatusId,
            kind: u64 as KindId,
        }

        Submission(status) <= Status(id);
        Submission(kind | status == Frozen) <= Kind(id | mastered == true);
    }

    /// The two grammar tiers expand, and the emitted descriptor
    /// round-trips through `validate()` — the tie to the declaration
    /// roster.
    #[test]
    fn the_two_tiers_expand_and_validate() {
        Review
            .descriptor()
            .validate()
            .expect("the declared schema is valid");
        // And through the real boundary: `Db::create` validates and
        // fingerprints the same descriptor.
        let dir = crate::common::TempDir::new("macro-closed-relations");
        Db::create(dir.path(), Review).expect("create");
    }

    /// The descriptor carries the extension: ground axioms in declaration
    /// order, values through the same literal machine as statement
    /// selections (declared columns only — the synthetic id is
    /// `validate()`'s to prepend).
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

    /// A bare handle in a selection resolves through the field's newtype
    /// to its owning closed relation's declaration-order row id; the
    /// synthetic id field is addressable as `id` — `FieldId(0)` — on both
    /// statement sides.
    #[test]
    fn handles_resolve_to_declaration_order_row_ids() {
        let schema = Review
            .descriptor()
            .validate()
            .expect("the declared schema is valid");
        // Materialized order: Submission.id's fresh auto-FD, the two
        // closed auto-keys (Status, Kind), then the declared containments.
        assert_eq!(schema.keys().len() + schema.containments().len(), 5);
        let StatementView::Containment(_, statement) = schema.statement(StatementId(4)) else {
            panic!("the second declared statement is a containment");
        };
        let source = &statement.source;
        let target = &statement.target;
        // `Submission(kind | status == Frozen)` — Frozen is row id 1.
        assert_eq!(source.relation, Review::SUBMISSION);
        assert_eq!(source.projection[..], [Review::SUBMISSION_KIND]);
        assert_eq!(
            source.selection[..],
            [(
                Review::SUBMISSION_STATUS,
                bumbledb::schema::LiteralSet::One(Value::U64(1))
            )]
        );
        // `Kind(id | mastered == true)` — the synthetic id at FieldId(0),
        // the declared column shifted to FieldId(1).
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

    /// The id constants see the sealed shape: the synthetic id at
    /// `FieldId(0)`, declared columns shifted.
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

    /// The host-enum weld, hand-checked (the exhaustive sibling is the
    /// emitted test): `id`/`from_id` are `const fn` — usable in const
    /// contexts — and the ids are the declaration-order row ids.
    #[test]
    fn the_host_enum_welds_to_row_ids() {
        const FROZEN: StatusId = Status::Frozen.id();
        assert_eq!(FROZEN, StatusId(1));
        assert_eq!(Kind::from_id(Kind::DirectPass.id()), Some(Kind::DirectPass));
        assert_eq!(Kind::from_id(KindId(2)), None);
        // The host enum is the constant namespace: matching stays
        // rustc-checked for exhaustiveness.
        let mastered = match Kind::Failed {
            Kind::DirectPass => true,
            Kind::Failed => false,
        };
        assert!(!mastered);
    }

    /// The manifest carries the extension — the vocabulary as plain data,
    /// for foreign surfaces that take their numbers as values.
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
        // The field list opens with the synthetic id the sealed schema
        // answers to.
        assert_eq!(&*kind.fields[0].name, "id");
        let rows = kind.extension.as_ref().expect("Kind is closed");
        assert_eq!(rows[0].values[..], [("mastered".into(), Value::Bool(true))]);
        assert_eq!(manifest.relations[2].extension, None);
    }
}

mod discriminated_union {
    //! PRD 05's survival criterion: the discriminated-union pattern
    //! (docs/cookbook.md recipe 2) with its arms discriminated by a
    //! closed reference — the discriminator's type change (inline enum →
    //! closed relation) left the DU theorems intact: totality and arm
    //! validity are the same `==` statement over `kind == Det`, now a
    //! row-id selection.

    use bumbledb::Db;

    bumbledb::schema! {
        pub Graph;

        closed relation GK as GKId = { Det, Custom };

        relation Parent { id: u64 as ParentId, fresh, kind: u64 as GKId }
        relation DetArm { parent: u64 as ParentId }

        DetArm(parent) -> DetArm;
        Parent(kind) <= GK(id);
        Parent(id | kind == Det) == DetArm(parent);
    }

    #[test]
    fn the_du_pattern_survives_the_closed_discriminator() {
        // The theory validates through the real boundary.
        let dir = crate::common::TempDir::new("macro-du-closed");
        let db = Db::create(dir.path(), Graph).expect("the DU theory validates");

        // Arm-consistent: the Det parent and its arm row in one commit.
        db.write(|tx| {
            let id: ParentId = tx.alloc()?;
            tx.insert(&Parent {
                id,
                kind: GK::Det.id(),
            })?;
            tx.insert(&DetArm { parent: id })?;
            Ok(())
        })
        .expect("a Det parent with its arm commits");

        // Arm-violating: a Det parent with no arm row — totality aborts.
        let err = db
            .write(|tx| {
                let id: ParentId = tx.alloc()?;
                tx.insert(&Parent {
                    id,
                    kind: GK::Det.id(),
                })?;
                Ok(())
            })
            .unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::CommitRejected { .. }),
            "{err:?}"
        );

        // The other arm's parent is unconstrained by the Det statement.
        db.write(|tx| {
            let id: ParentId = tx.alloc()?;
            tx.insert(&Parent {
                id,
                kind: GK::Custom.id(),
            })?;
            Ok(())
        })
        .expect("a Custom parent needs no Det arm");
    }
}

mod invalid_declaration {
    //! Semantic validation lives in `Db::create`/`Db::open`, not the
    //! macro: a declaration the grammar accepts but the acceptance gate
    //! refuses surfaces as the typed `SchemaError` — no panic path.

    use bumbledb::Db;
    use bumbledb::error::SchemaError;

    bumbledb::schema! {
        pub Duplicated;
        relation Parent { id: u64 as ParentId, fresh }
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
                bumbledb::Error::Schema(SchemaError::DuplicateStatement { .. })
            ),
            "{err:?}"
        );
    }
}

mod equality_reverse_key {
    use bumbledb::error::SchemaError;
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
            bumbledb::Error::Schema(SchemaError::NoMatchingTargetKey {
                statement: StatementId(2),
                target,
                projection,
                available,
            }) if target == Source::RELATION
                && *projection == [FieldId(0)]
                && available.is_empty()
        ));
    }
}

mod keyed_equality {
    use bumbledb::error::Direction;
    use bumbledb::schema::StatementId;
    use bumbledb::{Db, Error, Violation};

    bumbledb::schema! {
        pub KeyedEquality;

        relation Source { a: u64, b: i64, c: bool, note: str }
        relation Target { x: u64, y: i64, z: bool, weight: i64 }

        Source(a, b, c) -> Source;
        // Same exact target field set, deliberately declared in another order.
        Target(z, x, y) -> Target;
        Source(a, b, c) == Target(x, y, z);
    }

    fn assert_containment(error: Error, expected: StatementId) {
        let Error::CommitRejected { violations } = error else {
            panic!("expected containment rejection, got {error}");
        };
        let [
            Violation::Containment {
                statement,
                direction,
                ..
            },
        ] = violations.as_slice()
        else {
            panic!("expected one containment violation, got {violations:?}");
        };
        assert_eq!(*statement, expected);
        assert_eq!(*direction, Direction::SourceUnsatisfied);
    }

    #[test]
    fn three_field_reordered_key_equality_validates_and_enforces_both_directions() {
        let dir = crate::common::TempDir::new("macro-keyed-equality");
        let db = Db::create(dir.path(), KeyedEquality)
            .expect("both projected products resolve to declared keys");

        // Forward existence: Source's projected tuple has no Target witness.
        let error = db
            .write(|tx| {
                tx.insert(&Source {
                    a: 7,
                    b: -3,
                    c: true,
                    note: "source-only",
                })
            })
            .expect_err("forward equality containment is enforced");
        assert_containment(error, StatementId(2));

        // Reverse existence: Target's projected tuple has no Source witness.
        let error = db
            .write(|tx| {
                tx.insert(&Target {
                    x: 7,
                    y: -3,
                    z: true,
                    weight: 99,
                })
            })
            .expect_err("reverse equality containment is enforced");
        assert_containment(error, StatementId(3));

        // The selected projections correspond; whole facts do not. Their
        // unprojected payloads even have different structural types, which is
        // legal because `==` is projected-view equality, not row equality.
        db.write(|tx| {
            tx.insert(&Source {
                a: 7,
                b: -3,
                c: true,
                note: "payloads may differ",
            })?;
            tx.insert(&Target {
                x: 7,
                y: -3,
                z: true,
                weight: 99,
            })
        })
        .expect("one witness on each keyed projection commits");

        // Injectivity: another Target fact with the same projected product
        // but a different payload is rejected by Target's reordered key.
        let error = db
            .write(|tx| {
                tx.insert(&Target {
                    x: 7,
                    y: -3,
                    z: true,
                    weight: 100,
                })
            })
            .expect_err("the key makes the witness unique");
        assert!(matches!(
            error,
            Error::CommitRejected { violations }
                if matches!(violations.as_slice(), [Violation::Functionality {
                    statement: StatementId(1),
                    ..
                }])
        ));
    }
}

mod redundant_superkey_warning {
    use bumbledb::schema::{RelationId, SchemaWarning, StatementId};
    use bumbledb::{Db, Error, Interval, Theory as _, Violation};

    bumbledb::schema! {
        pub RedundantKeys;

        relation Window { id: u64, span: interval<i64>, payload: i64 }

        Window(id) -> Window;
        Window(id, span) -> Window;
    }

    #[test]
    fn redundant_superkey_warns_without_weakening_either_enforcement_plan() {
        let schema = RedundantKeys
            .descriptor()
            .validate()
            .expect("the redundant superkey remains accepted");
        let [
            SchemaWarning::RedundantSuperkey {
                relation,
                key,
                implied_by,
            },
        ] = schema.warnings()
        else {
            panic!("expected exactly one redundant-superkey warning");
        };
        let keys = schema.relation(RelationId(0)).keys();
        assert_eq!(*relation, RelationId(0));
        assert_eq!((*key, *implied_by), (keys[1], keys[0]));

        let dir = crate::common::TempDir::new("macro-redundant-superkey");
        let db = Db::create(dir.path(), RedundantKeys).expect("warning is non-fatal");
        let error = db
            .write(|tx| {
                tx.insert(&Window {
                    id: 7,
                    span: Interval::<i64>::new(0, 5).expect("interval"),
                    payload: 10,
                })?;
                tx.insert(&Window {
                    id: 7,
                    span: Interval::<i64>::new(3, 8).expect("interval"),
                    payload: 20,
                })
            })
            .expect_err("both determinant plans reject the overlapping duplicate id");
        let Error::CommitRejected { violations } = error else {
            panic!("expected functionality violations, got {error:?}");
        };
        assert_eq!(violations.as_slice().len(), 2);
        assert!(matches!(
            violations.as_slice(),
            [
                Violation::Functionality {
                    statement: StatementId(0),
                    ..
                },
                Violation::Functionality {
                    statement: StatementId(1),
                    ..
                }
            ]
        ));
    }
}

mod extension_forms {
    //! The dependency-vocabulary extension's grammar: literal-set
    //! selections, the cardinality window (`in lo..hi per`, `*` the
    //! no-ceiling spelling), and the order mark with its `by` chain
    //! (`docs/architecture/30-dependencies.md`).

    use bumbledb::schema::{LiteralSet, StatementDescriptor, StatementView};
    use bumbledb::{StatementId, Theory as _, Value};

    bumbledb::schema! {
        pub Tracker;

        closed relation Priority as PriorityId {
            weight: u64,
        } = {
            Low  { weight: 10 },
            High { weight: 20 },
        };

        relation Parent { id: u64 as ParentId, fresh }
        relation Task {
            parent: u64 as ParentId,
            pos:    u64,
            prio:   u64 as PriorityId,
            state:  u64,
        }

        Task(parent | state == {1, 2}) in 1..3 per Parent(id);
        Task(parent) in 1..* per Parent(id);
        order Task(pos) per Task(parent);
        order Task(pos) per Task(parent) by prio -> Priority(weight);
    }

    /// The macro lowers every extension form: the set selection lands as
    /// `LiteralSet::Many`, the window bounds land verbatim (`*` = None),
    /// and the `by` hop resolves the closed relation's synthetic id as
    /// its key.
    #[test]
    fn the_extension_forms_lower_and_validate() {
        let schema = Tracker
            .descriptor()
            .validate()
            .expect("the declared schema is valid");
        // Materialized order: Parent.id's fresh auto-key, Priority's
        // closed auto-key, then the four declared statements.
        assert!(matches!(
            schema.statement(StatementId(2)),
            StatementView::Cardinality(_, _)
        ));
        let window = &schema.windows()[0];
        assert_eq!((window.lo, window.hi), (1, Some(3)));
        assert_eq!(
            window.source.selection[..],
            [(
                Tracker::TASK_STATE,
                LiteralSet::Many(Box::new([Value::U64(1), Value::U64(2)]))
            )]
        );
        let star = &schema.windows()[1];
        assert_eq!((star.lo, star.hi), (1, None));

        assert!(matches!(
            schema.statement(StatementId(4)),
            StatementView::Order(_, _)
        ));
        let plain = &schema.orders()[0];
        assert_eq!(plain.position, Tracker::TASK_POS);
        assert_eq!(plain.grouping[..], [Tracker::TASK_PARENT]);
        assert!(plain.ranking.is_none());

        let ranked = &schema.orders()[1];
        let chain = ranked.ranking.as_ref().expect("the chain sealed");
        assert_eq!(chain.link, Tracker::TASK_PRIO);
        assert_eq!(chain.hops[0].relation, Tracker::PRIORITY);
        // The inferred hop key: the closed relation's synthetic id.
        assert_eq!(chain.hops[0].key, Tracker::PRIORITY_ID);
        assert_eq!(chain.hops[0].read, Tracker::PRIORITY_WEIGHT);
    }

    /// A singleton braced set is the equality spelling — `{1}` lowers to
    /// `LiteralSet::One`, so the descriptor is canonical by construction.
    #[test]
    fn a_braced_singleton_lowers_to_the_equality() {
        let descriptor = Tracker.descriptor();
        let Some(StatementDescriptor::Cardinality { source, .. }) = descriptor.statements.first()
        else {
            panic!("the first declared statement is the window");
        };
        assert!(matches!(source.selection[0].1, LiteralSet::Many(_)));
        // The sibling singleton spelling, from a fresh invocation:
        bumbledb::schema! {
            pub Solo;

            relation Parent { id: u64 as SoloParentId, fresh }
            relation Task { parent: u64 as SoloParentId, state: u64 }

            Task(parent | state == {7}) <= Parent(id);
        }
        let descriptor = Solo.descriptor();
        let Some(StatementDescriptor::Containment { source, .. }) = descriptor.statements.first()
        else {
            panic!("the declared statement is a containment");
        };
        assert_eq!(source.selection[0].1, LiteralSet::One(Value::U64(7)));
    }
}
