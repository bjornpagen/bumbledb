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
