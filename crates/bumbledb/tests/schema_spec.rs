use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::spec::{
    BoundSpec, CapacityWindowSpec, ClosedSpec, FaceNewtype, FieldSpec, LiteralAt, LiteralSetSpec,
    LiteralSpec, RelationSpec, RowSpec, SideSpec, SpecIssue, StatementSide, StatementSpec,
    WeightSpec,
};
use bumbledb::schema::{
    FixedIntervalElement, IntervalElement, ValueType, fingerprint::fingerprint,
};
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

    relation Holder { id: u64 as HolderId, name: str, digest: bytes<16>, cap: u64 }

    relation Account {
        id: u64 as AccountId,
        holder: u64 as HolderId,
        kind: u64 as KindId,
        status: u64 as StatusId,
        active: interval<i64> as ActiveDuring,
        lease: interval<u64, 7> as Lease,
        balance: u64,
    }

    relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }

    SavingsTerms(account) -> SavingsTerms;
    Account(holder) <= Holder(id);
    Account(kind) <= Kind(id);
    Account(status) <= Status(id);
    Account(id | status == Frozen) == SavingsTerms(account);
    Holder(id | name == {"alpha", "beta"}) <= Holder(id);
    Holder(id) <={0..3} Account(holder);
    Holder(id) <=[balance]{2..*} Account(holder | status == Frozen);
    Holder(id) <={1} Account(holder | status == Open);
    Holder(id) <={0} Account(holder | kind == Failed);
    Holder(id) <={1..4} Account(holder | kind == DirectPass);
    Holder(id) <=[balance]{0..cap} Account(holder);
    Holder(id) <=[Duration(active)]{0..720} Account(holder);
    Holder(id) <=[balance]{1..*} Account(holder | status == Open);
}

fn field(name: &str, value_type: ValueType) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        value_type,
        newtype: None,
    }
}

fn side(relation: &str, projection: &[&str]) -> SideSpec {
    SideSpec {
        relation: relation.into(),
        projection: projection.iter().map(|f| (*f).into()).collect(),
        selection: Vec::new(),
    }
}

fn side_selected(relation: &str, projection: &[&str], field: &str, handle: &str) -> SideSpec {
    SideSpec {
        selection: vec![(
            field.into(),
            LiteralSetSpec::One(LiteralSpec::Handle(handle.into())),
        )],
        ..side(relation, projection)
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one construct-complete theory, clearer kept together"
)]
fn everything_spec() -> SchemaSpec {
    let interval_u64 = ValueType::Interval {
        element: IntervalElement::U64,
    };
    SchemaSpec {
        relations: vec![
            RelationSpec {
                name: "Status".into(),
                fields: Vec::new(),
                closed: Some(ClosedSpec {
                    newtype: "StatusId".into(),
                    rows: vec![
                        RowSpec {
                            handle: "Open".into(),
                            values: Vec::new(),
                        },
                        RowSpec {
                            handle: "Frozen".into(),
                            values: Vec::new(),
                        },
                    ],
                }),
            },
            RelationSpec {
                name: "Kind".into(),
                fields: vec![
                    field("mastered", ValueType::Bool),
                    field("span", interval_u64),
                ],
                closed: Some(ClosedSpec {
                    newtype: "KindId".into(),
                    rows: vec![
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
                    ],
                }),
            },
            RelationSpec {
                name: "Holder".into(),
                fields: vec![
                    FieldSpec {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        newtype: Some("HolderId".into()),
                    },
                    field("name", ValueType::String),
                    field("digest", ValueType::FixedBytes { len: 16 }),
                    field("cap", ValueType::U64),
                ],
                closed: None,
            },
            RelationSpec {
                name: "Account".into(),
                fields: vec![
                    FieldSpec {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        newtype: Some("AccountId".into()),
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
                            },
                        )
                    },
                    FieldSpec {
                        newtype: Some("Lease".into()),
                        ..field(
                            "lease",
                            ValueType::FixedInterval {
                                element: FixedIntervalElement::U64,
                                width: 7,
                            },
                        )
                    },
                    field("balance", ValueType::U64),
                ],
                closed: None,
            },
            RelationSpec {
                name: "SavingsTerms".into(),
                fields: vec![
                    FieldSpec {
                        newtype: Some("AccountId".into()),
                        ..field("account", ValueType::U64)
                    },
                    field("rate_bps", ValueType::I64),
                ],
                closed: None,
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
                            LiteralSpec::Value(Value::String(Box::from("alpha"))),
                            LiteralSpec::Value(Value::String(Box::from("beta"))),
                        ]),
                    )],
                    ..side("Holder", &["id"])
                },
                target: side("Holder", &["id"]),
                bidirectional: false,
            },
            StatementSpec::Capacity {
                target: side("Holder", &["id"]),
                weight: WeightSpec::Unit,
                window: CapacityWindowSpec::Range {
                    lo: BoundSpec::Lit(0),
                    hi: BoundSpec::Lit(3),
                },
                source: side("Account", &["holder"]),
            },
            StatementSpec::Capacity {
                target: side("Holder", &["id"]),
                weight: WeightSpec::Field("balance".into()),
                window: CapacityWindowSpec::Floor(BoundSpec::Lit(2)),
                source: side_selected("Account", &["holder"], "status", "Frozen"),
            },
            StatementSpec::Capacity {
                target: side("Holder", &["id"]),
                weight: WeightSpec::Unit,
                window: CapacityWindowSpec::Exact(BoundSpec::Lit(1)),
                source: side_selected("Account", &["holder"], "status", "Open"),
            },
            StatementSpec::Capacity {
                target: side("Holder", &["id"]),
                weight: WeightSpec::Unit,
                window: CapacityWindowSpec::Exact(BoundSpec::Lit(0)),
                source: side_selected("Account", &["holder"], "kind", "Failed"),
            },
            StatementSpec::Capacity {
                target: side("Holder", &["id"]),
                weight: WeightSpec::Unit,
                window: CapacityWindowSpec::Range {
                    lo: BoundSpec::Lit(1),
                    hi: BoundSpec::Lit(4),
                },
                source: side_selected("Account", &["holder"], "kind", "DirectPass"),
            },
            StatementSpec::Capacity {
                target: side("Holder", &["id"]),
                weight: WeightSpec::Field("balance".into()),
                window: CapacityWindowSpec::Range {
                    lo: BoundSpec::Lit(0),
                    hi: BoundSpec::Field("cap".into()),
                },
                source: side("Account", &["holder"]),
            },
            StatementSpec::Capacity {
                target: side("Holder", &["id"]),
                weight: WeightSpec::Duration("active".into()),
                window: CapacityWindowSpec::Range {
                    lo: BoundSpec::Lit(0),
                    hi: BoundSpec::Lit(720),
                },
                source: side("Account", &["holder"]),
            },
            StatementSpec::Capacity {
                target: side("Holder", &["id"]),
                weight: WeightSpec::Field("balance".into()),
                window: CapacityWindowSpec::Floor(BoundSpec::Lit(1)),
                source: side_selected("Account", &["holder"], "status", "Open"),
            },
        ],
    }
}

// The seam roster: every literal construct the token→`Value` seam can
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
        id: u64 as ItemId,
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

fn side_valued(relation: &str, projection: &[&str], field: &str, literal: Value) -> SideSpec {
    SideSpec {
        selection: vec![(
            field.into(),
            LiteralSetSpec::One(LiteralSpec::Value(literal)),
        )],
        ..side(relation, projection)
    }
}

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
                fields: vec![
                    field("points", ValueType::I64),
                    field(
                        "window",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                    field("tag", ValueType::FixedBytes { len: 2 }),
                    field("code", ValueType::U64),
                ],
                closed: Some(ClosedSpec {
                    newtype: "GradeId".into(),
                    rows: vec![
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
                    ],
                }),
            },
            RelationSpec {
                name: "Item".into(),
                fields: vec![
                    FieldSpec {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        newtype: Some("ItemId".into()),
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
                        },
                    ),
                    field(
                        "span_i",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                    FieldSpec {
                        newtype: Some("LeaseI".into()),
                        ..field(
                            "lease",
                            ValueType::FixedInterval {
                                element: FixedIntervalElement::I64,
                                width: 3,
                            },
                        )
                    },
                    FieldSpec {
                        newtype: Some("GradeId".into()),
                        ..field("grade", ValueType::U64)
                    },
                ],
                closed: None,
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
                Value::String(Box::from("alpha")),
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
                Value::String(Box::from("a\"b\n\u{1F41D}")),
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
            fields: Vec::new(),
            closed: Some(ClosedSpec {
                newtype: "StatusId".into(),
                rows: (0..rows)
                    .map(|idx| RowSpec {
                        handle: format!("H{idx}").into(),
                        values: Vec::new(),
                    })
                    .collect(),
            }),
        }],
        statements: Vec::new(),
    };

    closed(256)
        .descriptor()
        .expect("resolves")
        .validate()
        .expect("at the cap the theory seals");

    let over = closed(257).descriptor().expect("names still resolve");
    assert!(
        over.validate().is_err(),
        "beyond the cap the validator rejects"
    );
}

#[test]
fn a_row_with_extra_values_is_rejected_not_silently_truncated() {
    let mut spec = everything_spec();
    let rows = &mut spec.relations[1]
        .closed
        .as_mut()
        .expect("Kind is closed")
        .rows;

    rows[0].values.push(LiteralSpec::Value(Value::Bool(false)));
    // The lowering must not silently drop the third literal: the column

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
        target: side("SavingsTerms", &["account"]),
        bidirectional: false,
    });
    let error = spec.descriptor().expect_err("three unresolvable names");
    let issues = error.issues();
    assert!(
        issues.contains(&SpecIssue::UnknownRelation {
            statement: 14,
            relation: "Nowhere".into()
        }),
        "the unknown relation is cited: {issues:?}"
    );
    assert!(
        issues.contains(&SpecIssue::UnknownField {
            statement: 15,
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
                statement: 16,
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
        target: side("SavingsTerms", &["account"]),
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
                statement: 14,
                side: StatementSide::Source,
                binding: 0,
                literal: 0
            }
        }]
    );
}

#[test]
fn paired_faces_with_disagreeing_newtypes_are_rejected_typed() {
    let mut spec = everything_spec();

    spec.statements.push(StatementSpec::Containment {
        source: side("Account", &["holder"]),
        target: side("Kind", &["id"]),
        bidirectional: false,
    });
    let error = spec.descriptor().expect_err("two labels disagree");
    assert_eq!(
        error.issues(),
        [SpecIssue::StatementNewtypeMismatch {
            statement: 14,
            position: 0,
            source: FaceNewtype {
                relation: "Account".into(),
                field: "holder".into(),
                newtype: Some("HolderId".into()),
            },
            target: FaceNewtype {
                relation: "Kind".into(),
                field: "id".into(),
                newtype: Some("KindId".into()),
            },
        }],
        "the rejection cites both faces with their labels"
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("`Account.holder` (`HolderId`)")
            && rendered.contains("`Kind.id` (`KindId`)"),
        "the error names both faces: {rendered}"
    );
}

#[test]
fn a_labeled_face_never_pairs_with_a_bare_one() {
    let mut spec = everything_spec();

    spec.statements.push(StatementSpec::Containment {
        source: side("SavingsTerms", &["account"]),
        target: side("SavingsTerms", &["rate_bps"]),
        bidirectional: false,
    });
    let error = spec
        .descriptor()
        .expect_err("labeled↔bare is the mismatch too");
    assert_eq!(
        error.issues(),
        [SpecIssue::StatementNewtypeMismatch {
            statement: 14,
            position: 0,
            source: FaceNewtype {
                relation: "SavingsTerms".into(),
                field: "account".into(),
                newtype: Some("AccountId".into()),
            },
            target: FaceNewtype {
                relation: "SavingsTerms".into(),
                field: "rate_bps".into(),
                newtype: None,
            },
        }],
    );
    assert!(
        error
            .to_string()
            .contains("`SavingsTerms.rate_bps` (no newtype)"),
        "the bare face is cited as bare: {error}"
    );
}

#[test]
fn bare_faces_pair_with_bare_faces() {
    let mut spec = everything_spec();

    spec.statements.push(StatementSpec::Containment {
        source: side("Holder", &["name"]),
        target: side("Holder", &["name"]),
        bidirectional: false,
    });
    spec.descriptor().expect("bare pairs with bare");
}

#[test]
fn a_psi_selected_target_never_bypasses_the_coherence_check() {
    let mut spec = everything_spec();

    spec.statements.push(StatementSpec::Capacity {
        target: SideSpec {
            selection: vec![(
                "mastered".into(),
                LiteralSetSpec::One(LiteralSpec::Value(Value::Bool(true))),
            )],
            ..side("Kind", &["id"])
        },
        weight: WeightSpec::Unit,
        window: CapacityWindowSpec::Exact(BoundSpec::Lit(1)),
        source: side("Account", &["holder"]),
    });
    let error = spec
        .descriptor()
        .expect_err("the ψ-selected target's projection is still judged");
    assert_eq!(
        error.issues(),
        [SpecIssue::StatementNewtypeMismatch {
            statement: 14,
            position: 0,
            source: FaceNewtype {
                relation: "Account".into(),
                field: "holder".into(),
                newtype: Some("HolderId".into()),
            },
            target: FaceNewtype {
                relation: "Kind".into(),
                field: "id".into(),
                newtype: Some("KindId".into()),
            },
        }],
    );
}

/// The old spelling ban table is deleted: alternate window spellings
/// lower canonically through the spec normalization. Only genuinely
/// different semantics still refuse — inverted literal bounds here
/// (dependent floors and path weights keep their own tests below).
#[test]
fn inverted_bounds_refuse_and_the_old_ban_table_spellings_lower_canonically() {
    let mut spec = everything_spec();
    spec.statements.push(StatementSpec::Capacity {
        target: side("Holder", &["id"]),
        weight: WeightSpec::Unit,
        window: CapacityWindowSpec::Range {
            lo: BoundSpec::Lit(4),
            hi: BoundSpec::Lit(2),
        },
        source: side("Account", &["holder"]),
    });
    let error = spec.descriptor().expect_err("inverted literal bounds");
    assert_eq!(
        error.issues(),
        [SpecIssue::CapacityInverted {
            statement: 14,
            lo: 4,
            hi: 2,
        }],
    );

    for window in [
        CapacityWindowSpec::Range {
            lo: BoundSpec::Lit(2),
            hi: BoundSpec::Lit(2),
        },
        CapacityWindowSpec::Range {
            lo: BoundSpec::Lit(0),
            hi: BoundSpec::Lit(0),
        },
        CapacityWindowSpec::Floor(BoundSpec::Lit(0)),
        CapacityWindowSpec::Floor(BoundSpec::Lit(1)),
    ] {
        let mut spec = everything_spec();
        spec.statements.push(StatementSpec::Capacity {
            target: side("Holder", &["id"]),
            weight: WeightSpec::Unit,
            window: window.clone(),
            source: side("Account", &["holder"]),
        });
        if let Err(error) = spec.descriptor() {
            panic!("{window:?} lowers canonically, no ban table: {error}");
        }
    }
}

#[test]
fn the_weighted_floor_is_legal_where_the_unit_floor_is_banned() {
    let mut spec = everything_spec();
    spec.statements.push(StatementSpec::Capacity {
        target: side("Holder", &["id"]),
        weight: WeightSpec::Field("balance".into()),
        window: CapacityWindowSpec::Floor(BoundSpec::Lit(1)),
        source: side("Account", &["holder"]),
    });
    spec.descriptor()
        .expect("`<=[w]{1..*}` is the legal weighted floor");
}

/// The path BOUND refuses at the spec surface exactly as the weight does
/// (ruling 6, one law both slots): the typed `BoundPathRefused` naming the
/// pinned-column idiom — never the accidental `UnknownField` a dotted name fell
/// to before the symmetry landed.
#[test]
fn a_path_bound_is_refused_naming_the_pinned_column_idiom() {
    let mut spec = everything_spec();
    spec.statements.push(StatementSpec::Capacity {
        target: side("Holder", &["id"]),
        weight: WeightSpec::Field("balance".into()),
        window: CapacityWindowSpec::Range {
            lo: BoundSpec::Lit(0),
            hi: BoundSpec::Field("grid.supply".into()),
        },
        source: side("Account", &["holder"]),
    });
    let error = spec.descriptor().expect_err("a path bound");
    assert_eq!(
        error.issues(),
        [SpecIssue::BoundPathRefused {
            statement: 14,
            path: "grid.supply".into(),
        }],
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("pinned-column idiom"),
        "the refusal names the composition idiom: {rendered}"
    );
}

#[test]
fn a_path_weight_is_refused_naming_the_pinned_column_idiom() {
    let mut spec = everything_spec();
    spec.statements.push(StatementSpec::Capacity {
        target: side("Holder", &["id"]),
        weight: WeightSpec::Field("kind.mastered".into()),
        window: CapacityWindowSpec::Range {
            lo: BoundSpec::Lit(0),
            hi: BoundSpec::Lit(3),
        },
        source: side("Account", &["holder"]),
    });
    let error = spec.descriptor().expect_err("a path weight");
    assert_eq!(
        error.issues(),
        [SpecIssue::WeightPathRefused {
            statement: 14,
            path: "kind.mastered".into(),
        }],
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("pinned-column idiom"),
        "the refusal names the composition idiom: {rendered}"
    );
}

/// Dependent bounds are hi-slot only (ruled 2026-07-24, C6): a dependent floor
/// is a typed refusal naming the ruling.
#[test]
fn a_dependent_floor_is_refused_hi_slot_only() {
    let mut spec = everything_spec();
    spec.statements.push(StatementSpec::Capacity {
        target: side("Holder", &["id"]),
        weight: WeightSpec::Field("balance".into()),
        window: CapacityWindowSpec::Range {
            lo: BoundSpec::Field("cap".into()),
            hi: BoundSpec::Lit(9),
        },
        source: side("Account", &["holder"]),
    });
    let error = spec.descriptor().expect_err("a dependent floor");
    assert_eq!(
        error.issues(),
        [SpecIssue::CapacityDependentFloor { statement: 14 }],
    );
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
