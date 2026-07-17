//! The `SchemaSpec` bindings contract (`docs/architecture/70-api.md`
//! § the `SchemaSpec` bindings contract), pinned from the public surface:
//!
//! - **Fingerprint parity** — one theory exercising every descriptor
//!   construct (both closed tiers, `fresh`, `str`, `bytes<N>`, general
//!   and fixed-width intervals, all three statement forms, `==`, a
//!   literal-set selection, every legal window spelling), built through
//!   the `schema!` macro and through [`bumbledb::SchemaSpec`], yields
//!   IDENTICAL fingerprints: macro and spec produce indistinguishable
//!   descriptors.
//! - **The spec-path ban table** — name-resolution failures are typed
//!   and COMPLETE (every unresolvable name enumerated), and window
//!   canonicity errors name the canonical spelling verbatim, mirroring
//!   the macro's expansion errors.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::spec::{
    FieldSpec, LiteralAt, LiteralSetSpec, LiteralSpec, RelationSpec, RowSpec, SideSpec, SpecIssue,
    StatementSide, StatementSpec, WindowSpec,
};
use bumbledb::schema::{IntervalElement, ValueType, fingerprint::fingerprint};
use bumbledb::{Interval, SchemaSpec, Theory, Value};

bumbledb::schema! {
    pub Everything;

    closed relation Status as StatusId = { Open, Frozen };

    closed relation Kind as KindId {
        mastered: bool,
        span: interval<u64>,
    } = {
        DirectPass { mastered: true, span: 1..3 },
        Failed     { mastered: false, span: 3..5 },
    };

    relation Holder { id: u64 as HolderId, fresh, name: str, digest: bytes<16> }

    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        kind: u64 as KindId,
        status: u64 as StatusId,
        active: interval<i64> as ActiveDuring,
        lease: interval<u64, 7> as Lease,
    }

    relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }

    SavingsTerms(account) -> SavingsTerms;
    Account(holder) <= Holder(id);
    Account(kind) <= Kind(id);
    Account(status) <= Status(id);
    Account(id | status == Frozen) == SavingsTerms(account);
    Holder(id | name == {"alpha", "beta"}) <= Holder(id);
    Holder(id) <={0..3} Account(holder);
    Holder(id) <={2..*} Account(holder | status == Frozen);
    Holder(id) <={1} Account(holder | status == Open);
    Holder(id) <={0} Account(holder | kind == Failed);
    Holder(id) <={1..4} Account(holder | kind == DirectPass);
}

/// A field spec with no newtype and no fresh mark.
fn field(name: &str, value_type: ValueType) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        value_type,
        newtype: None,
        fresh: false,
    }
}

/// A side with no selection.
fn side(relation: &str, projection: &[&str]) -> SideSpec {
    SideSpec {
        relation: relation.into(),
        projection: projection.iter().map(|f| (*f).into()).collect(),
        selection: Vec::new(),
    }
}

/// A side with one `field == Handle` selection.
fn side_selected(relation: &str, projection: &[&str], field: &str, handle: &str) -> SideSpec {
    SideSpec {
        selection: vec![(
            field.into(),
            LiteralSetSpec::One(LiteralSpec::Handle(handle.into())),
        )],
        ..side(relation, projection)
    }
}

/// The spec twin of the `Everything` schema above, statement for
/// statement — names where the macro wrote names, `bidirectional: true`
/// where it wrote `==`, one `WindowSpec` per legal window spelling.
#[expect(
    clippy::too_many_lines,
    reason = "one construct-complete theory, clearer kept together"
)]
fn everything_spec() -> SchemaSpec {
    let interval_u64 = ValueType::Interval {
        element: IntervalElement::U64,
        width: None,
    };
    SchemaSpec {
        relations: vec![
            RelationSpec {
                name: "Status".into(),
                newtype: Some("StatusId".into()),
                fields: Vec::new(),
                extension: Some(vec![
                    RowSpec {
                        handle: "Open".into(),
                        values: Vec::new(),
                    },
                    RowSpec {
                        handle: "Frozen".into(),
                        values: Vec::new(),
                    },
                ]),
            },
            RelationSpec {
                name: "Kind".into(),
                newtype: Some("KindId".into()),
                fields: vec![
                    field("mastered", ValueType::Bool),
                    field("span", interval_u64.clone()),
                ],
                extension: Some(vec![
                    RowSpec {
                        handle: "DirectPass".into(),
                        values: vec![
                            LiteralSpec::Value(Value::Bool(true)),
                            LiteralSpec::Value(Value::IntervalU64(
                                Interval::<u64>::new(1, 3).expect("nonempty"),
                            )),
                        ],
                    },
                    RowSpec {
                        handle: "Failed".into(),
                        values: vec![
                            LiteralSpec::Value(Value::Bool(false)),
                            LiteralSpec::Value(Value::IntervalU64(
                                Interval::<u64>::new(3, 5).expect("nonempty"),
                            )),
                        ],
                    },
                ]),
            },
            RelationSpec {
                name: "Holder".into(),
                newtype: None,
                fields: vec![
                    FieldSpec {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        newtype: Some("HolderId".into()),
                        fresh: true,
                    },
                    field("name", ValueType::String),
                    field("digest", ValueType::FixedBytes { len: 16 }),
                ],
                extension: None,
            },
            RelationSpec {
                name: "Account".into(),
                newtype: None,
                fields: vec![
                    FieldSpec {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        newtype: Some("AccountId".into()),
                        fresh: true,
                    },
                    FieldSpec {
                        newtype: Some("HolderId".into()),
                        ..field("holder", ValueType::U64)
                    },
                    FieldSpec {
                        newtype: Some("KindId".into()),
                        ..field("kind", ValueType::U64)
                    },
                    FieldSpec {
                        newtype: Some("StatusId".into()),
                        ..field("status", ValueType::U64)
                    },
                    FieldSpec {
                        newtype: Some("ActiveDuring".into()),
                        ..field(
                            "active",
                            ValueType::Interval {
                                element: IntervalElement::I64,
                                width: None,
                            },
                        )
                    },
                    FieldSpec {
                        newtype: Some("Lease".into()),
                        ..field(
                            "lease",
                            ValueType::Interval {
                                element: IntervalElement::U64,
                                width: Some(7),
                            },
                        )
                    },
                ],
                extension: None,
            },
            RelationSpec {
                name: "SavingsTerms".into(),
                newtype: None,
                fields: vec![
                    FieldSpec {
                        newtype: Some("AccountId".into()),
                        ..field("account", ValueType::U64)
                    },
                    field("rate_bps", ValueType::I64),
                ],
                extension: None,
            },
        ],
        statements: vec![
            StatementSpec::Fd {
                relation: "SavingsTerms".into(),
                projection: vec!["account".into()],
            },
            StatementSpec::Containment {
                source: side("Account", &["holder"]),
                target: side("Holder", &["id"]),
                bidirectional: false,
            },
            StatementSpec::Containment {
                source: side("Account", &["kind"]),
                target: side("Kind", &["id"]),
                bidirectional: false,
            },
            StatementSpec::Containment {
                source: side("Account", &["status"]),
                target: side("Status", &["id"]),
                bidirectional: false,
            },
            StatementSpec::Containment {
                source: side_selected("Account", &["id"], "status", "Frozen"),
                target: side("SavingsTerms", &["account"]),
                bidirectional: true,
            },
            StatementSpec::Containment {
                source: SideSpec {
                    selection: vec![(
                        "name".into(),
                        LiteralSetSpec::Many(vec![
                            LiteralSpec::Value(Value::String(Box::from("alpha".as_bytes()))),
                            LiteralSpec::Value(Value::String(Box::from("beta".as_bytes()))),
                        ]),
                    )],
                    ..side("Holder", &["id"])
                },
                target: side("Holder", &["id"]),
                bidirectional: false,
            },
            StatementSpec::Cardinality {
                target: side("Holder", &["id"]),
                window: WindowSpec::Range { lo: 0, hi: 3 },
                source: side("Account", &["holder"]),
            },
            StatementSpec::Cardinality {
                target: side("Holder", &["id"]),
                window: WindowSpec::Floor(2),
                source: side_selected("Account", &["holder"], "status", "Frozen"),
            },
            StatementSpec::Cardinality {
                target: side("Holder", &["id"]),
                window: WindowSpec::Exact(1),
                source: side_selected("Account", &["holder"], "status", "Open"),
            },
            StatementSpec::Cardinality {
                target: side("Holder", &["id"]),
                window: WindowSpec::Exact(0),
                source: side_selected("Account", &["holder"], "kind", "Failed"),
            },
            StatementSpec::Cardinality {
                target: side("Holder", &["id"]),
                window: WindowSpec::Range { lo: 1, hi: 4 },
                source: side_selected("Account", &["holder"], "kind", "DirectPass"),
            },
        ],
    }
}

// The seam roster: every literal construct the token→`Value` seam can
// emit rides a SELECTION at least once (bool, u64, negative i64, str,
// bytes<N>, general u64/i64 intervals, a width-matched fixed interval,
// a handle, a non-string literal set), the fixed family covers the i64
// element, closed rows carry i64/interval/bytes/u64 values, and an FD
// takes a multi-field determinant — the constructs `Everything` misses.
bumbledb::schema! {
    pub Seam;

    closed relation Grade as GradeId {
        points: i64,
        window: interval<i64>,
        tag: bytes<2>,
        code: u64,
    } = {
        Low  { points: -3, window: -5..-2, tag: b"lo", code: 7 },
        High { points: 9,  window: 2..4,   tag: b"hi", code: 8 },
    };

    relation Item {
        id: u64 as ItemId, fresh,
        flag: bool,
        count: u64,
        delta: i64,
        label: str,
        mark: bytes<2>,
        span_u: interval<u64>,
        span_i: interval<i64>,
        lease: interval<i64, 3> as LeaseI,
        grade: u64 as GradeId,
    }

    Item(id, flag) -> Item;
    Item(id | flag == true) <= Item(id);
    Item(id | count == 5) <= Item(id);
    Item(id | delta == -7) <= Item(id);
    Item(id | label == "alpha") <= Item(id);
    Item(id | mark == b"ok") <= Item(id);
    Item(id | span_u == 1..3) <= Item(id);
    Item(id | span_i == -4..-1) <= Item(id);
    Item(id | lease == 1..4) <= Item(id);
    Item(id | grade == Low) <= Item(id);
    Item(id | count == {2, 4}) <= Item(id);
    Item(id | label == "a\"b\n\u{1F41D}") <= Item(id);
    Item(id | mark == b"\xFF\x00") <= Item(id);
    Item(id | span_u == 5..18446744073709551615) <= Item(id);
    Item(id | delta == {-9223372036854775808, 3}) <= Item(id);
}

/// A side with one `field == literal` (plain-value) selection.
fn side_valued(relation: &str, projection: &[&str], field: &str, literal: Value) -> SideSpec {
    SideSpec {
        selection: vec![(
            field.into(),
            LiteralSetSpec::One(LiteralSpec::Value(literal)),
        )],
        ..side(relation, projection)
    }
}

/// The spec twin of the `Seam` schema above.
#[expect(
    clippy::too_many_lines,
    reason = "one construct-complete theory, clearer kept together"
)]
fn seam_spec() -> SchemaSpec {
    let contain = |source: SideSpec| StatementSpec::Containment {
        source,
        target: side("Item", &["id"]),
        bidirectional: false,
    };
    SchemaSpec {
        relations: vec![
            RelationSpec {
                name: "Grade".into(),
                newtype: Some("GradeId".into()),
                fields: vec![
                    field("points", ValueType::I64),
                    field(
                        "window",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                            width: None,
                        },
                    ),
                    field("tag", ValueType::FixedBytes { len: 2 }),
                    field("code", ValueType::U64),
                ],
                extension: Some(vec![
                    RowSpec {
                        handle: "Low".into(),
                        values: vec![
                            LiteralSpec::Value(Value::I64(-3)),
                            LiteralSpec::Value(Value::IntervalI64(
                                Interval::<i64>::new(-5, -2).expect("nonempty"),
                            )),
                            LiteralSpec::Value(Value::FixedBytes(Box::from(&b"lo"[..]))),
                            LiteralSpec::Value(Value::U64(7)),
                        ],
                    },
                    RowSpec {
                        handle: "High".into(),
                        values: vec![
                            LiteralSpec::Value(Value::I64(9)),
                            LiteralSpec::Value(Value::IntervalI64(
                                Interval::<i64>::new(2, 4).expect("nonempty"),
                            )),
                            LiteralSpec::Value(Value::FixedBytes(Box::from(&b"hi"[..]))),
                            LiteralSpec::Value(Value::U64(8)),
                        ],
                    },
                ]),
            },
            RelationSpec {
                name: "Item".into(),
                newtype: None,
                fields: vec![
                    FieldSpec {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        newtype: Some("ItemId".into()),
                        fresh: true,
                    },
                    field("flag", ValueType::Bool),
                    field("count", ValueType::U64),
                    field("delta", ValueType::I64),
                    field("label", ValueType::String),
                    field("mark", ValueType::FixedBytes { len: 2 }),
                    field(
                        "span_u",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                            width: None,
                        },
                    ),
                    field(
                        "span_i",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                            width: None,
                        },
                    ),
                    FieldSpec {
                        newtype: Some("LeaseI".into()),
                        ..field(
                            "lease",
                            ValueType::Interval {
                                element: IntervalElement::I64,
                                width: Some(3),
                            },
                        )
                    },
                    FieldSpec {
                        newtype: Some("GradeId".into()),
                        ..field("grade", ValueType::U64)
                    },
                ],
                extension: None,
            },
        ],
        statements: vec![
            StatementSpec::Fd {
                relation: "Item".into(),
                projection: vec!["id".into(), "flag".into()],
            },
            contain(side_valued("Item", &["id"], "flag", Value::Bool(true))),
            contain(side_valued("Item", &["id"], "count", Value::U64(5))),
            contain(side_valued("Item", &["id"], "delta", Value::I64(-7))),
            contain(side_valued(
                "Item",
                &["id"],
                "label",
                Value::String(Box::from("alpha".as_bytes())),
            )),
            contain(side_valued(
                "Item",
                &["id"],
                "mark",
                Value::FixedBytes(Box::from(&b"ok"[..])),
            )),
            contain(side_valued(
                "Item",
                &["id"],
                "span_u",
                Value::IntervalU64(Interval::<u64>::new(1, 3).expect("nonempty")),
            )),
            contain(side_valued(
                "Item",
                &["id"],
                "span_i",
                Value::IntervalI64(Interval::<i64>::new(-4, -1).expect("nonempty")),
            )),
            contain(side_valued(
                "Item",
                &["id"],
                "lease",
                Value::IntervalI64(Interval::<i64>::new(1, 4).expect("nonempty")),
            )),
            contain(side_selected("Item", &["id"], "grade", "Low")),
            contain(SideSpec {
                selection: vec![(
                    "count".into(),
                    LiteralSetSpec::Many(vec![
                        LiteralSpec::Value(Value::U64(2)),
                        LiteralSpec::Value(Value::U64(4)),
                    ]),
                )],
                ..side("Item", &["id"])
            }),
            contain(side_valued(
                "Item",
                &["id"],
                "label",
                Value::String(Box::from("a\"b\n\u{1F41D}".as_bytes())),
            )),
            contain(side_valued(
                "Item",
                &["id"],
                "mark",
                Value::FixedBytes(Box::from(&[0xFF, 0x00][..])),
            )),
            contain(side_valued(
                "Item",
                &["id"],
                "span_u",
                Value::IntervalU64(Interval::<u64>::new(5, u64::MAX).expect("the ray")),
            )),
            contain(SideSpec {
                selection: vec![(
                    "delta".into(),
                    LiteralSetSpec::Many(vec![
                        LiteralSpec::Value(Value::I64(i64::MIN)),
                        LiteralSpec::Value(Value::I64(3)),
                    ]),
                )],
                ..side("Item", &["id"])
            }),
        ],
    }
}

#[test]
fn the_seam_roster_spec_and_macro_produce_one_fingerprint() {
    let macro_descriptor = Seam.descriptor();
    let spec_descriptor = seam_spec().descriptor().expect("the twin spec resolves");
    assert_eq!(
        spec_descriptor, macro_descriptor,
        "spec lowering and macro expansion emit the same descriptor"
    );
    let macro_schema = macro_descriptor.validate().expect("the theory seals");
    let spec_schema = spec_descriptor.validate().expect("the twin seals");
    assert_eq!(
        fingerprint(&spec_schema),
        fingerprint(&macro_schema),
        "one theory, one fingerprint — whichever surface built it"
    );
}

#[test]
fn an_empty_spec_lowers_and_seals() {
    let spec = SchemaSpec {
        relations: Vec::new(),
        statements: Vec::new(),
    };
    let descriptor = spec.descriptor().expect("nothing to resolve");
    descriptor.validate().expect("the empty theory seals");
}

#[test]
fn the_extension_row_cap_is_the_validators_not_the_lowerings() {
    let closed = |rows: usize| SchemaSpec {
        relations: vec![RelationSpec {
            name: "Status".into(),
            newtype: Some("StatusId".into()),
            fields: Vec::new(),
            extension: Some(
                (0..rows)
                    .map(|idx| RowSpec {
                        handle: format!("H{idx}").into(),
                        values: Vec::new(),
                    })
                    .collect(),
            ),
        }],
        statements: Vec::new(),
    };
    // 256 rows: at the cap, lowers and seals.
    closed(256)
        .descriptor()
        .expect("resolves")
        .validate()
        .expect("at the cap the theory seals");
    // 257 rows: lowers (the cap is semantic), sealed rejection is typed.
    let over = closed(257).descriptor().expect("names still resolve");
    assert!(
        over.validate().is_err(),
        "beyond the cap the validator rejects"
    );
}

#[test]
fn a_row_with_extra_values_is_rejected_not_silently_truncated() {
    let mut spec = everything_spec();
    let rows = spec.relations[1]
        .extension
        .as_mut()
        .expect("Kind is closed");
    // Kind declares two columns; this row now supplies three literals.
    rows[0].values.push(LiteralSpec::Value(Value::Bool(false)));
    // The lowering must not silently drop the third literal: the column
    // zip cannot represent it, so the spec is rejected typed (the
    // short-row case stays the engine validator's arity check).
    let error = spec
        .descriptor()
        .expect_err("an over-wide row never lowers");
    assert_eq!(
        error.issues(),
        [SpecIssue::RowArityExcess {
            relation: 1,
            row: 0,
            name: "Kind".into(),
            declared: 2,
            supplied: 3,
        }],
        "the rejection names the offending row and both arities"
    );
}

#[test]
fn the_spec_and_the_macro_produce_one_fingerprint() {
    let macro_descriptor = Everything.descriptor();
    let spec_descriptor = everything_spec()
        .descriptor()
        .expect("the twin spec resolves");
    assert_eq!(
        spec_descriptor, macro_descriptor,
        "spec lowering and macro expansion emit the same descriptor"
    );
    let macro_schema = macro_descriptor.validate().expect("the theory seals");
    let spec_schema = spec_descriptor.validate().expect("the twin seals");
    assert_eq!(
        fingerprint(&spec_schema),
        fingerprint(&macro_schema),
        "one theory, one fingerprint — whichever surface built it"
    );
}

#[test]
fn unresolvable_names_are_enumerated_completely_never_first_only() {
    let mut spec = everything_spec();
    spec.statements.push(StatementSpec::Fd {
        relation: "Nowhere".into(),
        projection: vec!["id".into()],
    });
    spec.statements.push(StatementSpec::Containment {
        source: side("Account", &["nope"]),
        target: side("Holder", &["id"]),
        bidirectional: false,
    });
    spec.statements.push(StatementSpec::Containment {
        source: side_selected("Account", &["id"], "status", "Thawed"),
        target: side("Holder", &["id"]),
        bidirectional: false,
    });
    let error = spec.descriptor().expect_err("three unresolvable names");
    let issues = error.issues();
    assert!(
        issues.contains(&SpecIssue::UnknownRelation {
            statement: 11,
            relation: "Nowhere".into()
        }),
        "the unknown relation is cited: {issues:?}"
    );
    assert!(
        issues.contains(&SpecIssue::UnknownField {
            statement: 12,
            relation: "Account".into(),
            field: "nope".into()
        }),
        "the unknown field is cited: {issues:?}"
    );
    assert!(
        issues.contains(&SpecIssue::UnknownHandle {
            closed: "Status".into(),
            handle: "Thawed".into(),
            at: LiteralAt::Selection {
                statement: 13,
                side: StatementSide::Source,
                binding: 0,
                literal: 0
            }
        }),
        "the unknown handle is cited: {issues:?}"
    );
    assert_eq!(issues.len(), 3, "every issue, nothing else: {issues:?}");
}

#[test]
fn a_handle_on_a_non_reference_field_is_typed() {
    let mut spec = everything_spec();
    spec.statements.push(StatementSpec::Containment {
        source: side_selected("Account", &["id"], "holder", "Frozen"),
        target: side("Holder", &["id"]),
        bidirectional: false,
    });
    let error = spec.descriptor().expect_err("HolderId names no vocabulary");
    assert_eq!(
        error.issues(),
        [SpecIssue::NotAHandleField {
            relation: "Account".into(),
            field: "holder".into(),
            handle: "Frozen".into(),
            at: LiteralAt::Selection {
                statement: 11,
                side: StatementSide::Source,
                binding: 0,
                literal: 0
            }
        }]
    );
}

/// One banned window spelling per row of the ban table
/// (`docs/architecture/70-api.md` § the canonical-utterance law), each
/// error's `Display` naming the canonical form the author pastes back.
#[test]
fn the_window_ban_table_rejects_at_spec_construction_naming_the_canonical_form() {
    let banned: [(WindowSpec, SpecIssue, &str); 5] = [
        (
            WindowSpec::Range { lo: 4, hi: 2 },
            SpecIssue::WindowInverted {
                statement: 11,
                lo: 4,
                hi: 2,
            },
            "an exact count is `{n}`",
        ),
        (
            WindowSpec::Range { lo: 2, hi: 2 },
            SpecIssue::WindowExactRespelled {
                statement: 11,
                count: 2,
            },
            "an exact count is written `{2}`",
        ),
        (
            WindowSpec::Range { lo: 0, hi: 0 },
            SpecIssue::WindowExclusionRespelled { statement: 11 },
            "the exclusion is written `{0}`",
        ),
        (
            WindowSpec::Floor(0),
            SpecIssue::WindowVacuous { statement: 11 },
            "delete the statement",
        ),
        (
            WindowSpec::Floor(1),
            SpecIssue::WindowContainmentRespelled { statement: 11 },
            "drop the annotation and write the containment",
        ),
    ];
    for (window, expected, canonical) in banned {
        let mut spec = everything_spec();
        spec.statements.push(StatementSpec::Cardinality {
            target: side("Holder", &["id"]),
            window,
            source: side("Account", &["holder"]),
        });
        let error = spec.descriptor().expect_err("a banned spelling");
        assert_eq!(
            error.issues(),
            std::slice::from_ref(&expected),
            "for {window:?}"
        );
        let rendered = error.to_string();
        assert!(
            rendered.contains(canonical),
            "{window:?} names its canonical form: {rendered}"
        );
    }
}

#[test]
fn degenerate_literal_sets_are_banned_naming_the_bare_spelling() {
    for (many, needle) in [
        (Vec::new(), "write no binding"),
        (
            vec![LiteralSpec::Handle("Frozen".into())],
            "a one-element set is the bare literal",
        ),
    ] {
        let mut spec = everything_spec();
        spec.statements.push(StatementSpec::Containment {
            source: SideSpec {
                selection: vec![("status".into(), LiteralSetSpec::Many(many))],
                ..side("Account", &["id"])
            },
            target: side("Holder", &["id"]),
            bidirectional: false,
        });
        let error = spec.descriptor().expect_err("a degenerate set");
        let rendered = error.to_string();
        assert!(rendered.contains(needle), "{rendered}");
    }
}
