//! The spec-lowering contract, exercised as a foreign host would:
//! plain data in, a descriptor or the COMPLETE typed issue list out.
//! The macro/spec fingerprint parity pin lives engine-side
//! (`crates/bumbledb/tests/schema_spec.rs`); this file pins the
//! theory-crate laws alone.
use bumbledb_theory::Value;
use bumbledb_theory::schema::spec::{
    ClosedSpec, FieldSpec, LiteralAt, LiteralSetSpec, LiteralSpec, RelationSpec, RowSpec,
    SchemaSpec, SideSpec, SpecIssue, StatementSpec,
};
use bumbledb_theory::schema::{FieldId, LiteralSet, RelationId, StatementDescriptor, ValueType};

fn field(name: &str, newtype: Option<&str>) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        value_type: ValueType::U64,
        newtype: newtype.map(Into::into),
    }
}

/// The fused closedness sum (ruled 2026-07-23, R7): a closed relation
/// carries its handle newtype by construction, so the handle namespace
/// is entered by plain iteration — a `Handle` literal resolves through
/// the referencing field's newtype to the declaration-order row id, and
/// the synthetic `id` addresses `FieldId(0)`.
#[test]
fn closed_spec_carries_the_handle_newtype_by_construction() {
    let spec = SchemaSpec {
        relations: vec![
            RelationSpec {
                name: "Status".into(),
                fields: vec![],
                closed: Some(ClosedSpec {
                    newtype: "StatusId".into(),
                    rows: vec![
                        RowSpec {
                            handle: "Active".into(),
                            values: vec![],
                        },
                        RowSpec {
                            handle: "Frozen".into(),
                            values: vec![],
                        },
                    ],
                }),
            },
            RelationSpec {
                name: "Account".into(),
                fields: vec![field("owner", None), field("status", Some("StatusId"))],
                closed: None,
            },
        ],
        statements: vec![StatementSpec::Containment {
            source: SideSpec {
                relation: "Account".into(),
                projection: vec!["status".into()],
                selection: vec![(
                    "status".into(),
                    LiteralSetSpec::One(LiteralSpec::Handle("Frozen".into())),
                )],
            },
            target: SideSpec {
                relation: "Status".into(),
                projection: vec!["id".into()],
                selection: vec![],
            },
            bidirectional: false,
        }],
    };
    let descriptor = spec.descriptor().expect("the spec lowers clean");

    // The closed arm lowers to the descriptor's extension rows.
    let rows = descriptor.relations[0]
        .extension
        .as_deref()
        .expect("closed lowers closed");
    assert_eq!(rows.len(), 2);
    assert_eq!(&*rows[1].handle, "Frozen");

    // The handle literal resolved to its declaration-order row id, and
    // the target's `id` addressed the synthetic FieldId(0).
    let StatementDescriptor::Containment { source, target } = &descriptor.statements[0] else {
        panic!("one containment lowered");
    };
    assert_eq!(source.relation, RelationId(1));
    assert_eq!(&*source.projection, [FieldId(1)]);
    assert_eq!(
        &*source.selection,
        [(FieldId(1), LiteralSet::One(Value::U64(1)))]
    );
    assert_eq!(target.relation, RelationId(0));
    assert_eq!(&*target.projection, [FieldId(0)]);
}

/// The one sealed-slot lookup (finding 126): the synthetic `id` carries
/// its relation's handle newtype into the coherence check, so pairing it
/// with a bare column is the mismatch — the same judgment handle
/// resolution reads, never a second scan's opinion.
#[test]
fn synthetic_id_newtype_rides_the_sealed_slot() {
    let spec = SchemaSpec {
        relations: vec![
            RelationSpec {
                name: "Status".into(),
                fields: vec![],
                closed: Some(ClosedSpec {
                    newtype: "StatusId".into(),
                    rows: vec![RowSpec {
                        handle: "Active".into(),
                        values: vec![],
                    }],
                }),
            },
            RelationSpec {
                name: "Account".into(),
                fields: vec![field("status", None)],
                closed: None,
            },
        ],
        statements: vec![StatementSpec::Containment {
            source: SideSpec {
                relation: "Account".into(),
                projection: vec!["status".into()],
                selection: vec![],
            },
            target: SideSpec {
                relation: "Status".into(),
                projection: vec!["id".into()],
                selection: vec![],
            },
            bidirectional: false,
        }],
    };
    let err = spec.descriptor().expect_err("the faces disagree");
    let [
        SpecIssue::StatementNewtypeMismatch {
            statement: 0,
            position: 0,
            source,
            target,
        },
    ] = err.issues()
    else {
        panic!("one mismatch issue, not {:?}", err.issues());
    };
    assert_eq!(source.newtype, None);
    assert_eq!(target.newtype.as_deref(), Some("StatusId"));
}

/// One round trip (finding 127): the issue list is COMPLETE in one pass
/// — an earlier broken row or statement never suppresses a later one's
/// diagnosis. Side lowering is total (placeholder-bearing, per the
/// `literal` law); the one final gate alone judges validity.
#[test]
fn the_issue_list_is_complete_in_one_pass() {
    let spec = SchemaSpec {
        relations: vec![
            RelationSpec {
                name: "Status".into(),
                fields: vec![],
                closed: Some(ClosedSpec {
                    newtype: "StatusId".into(),
                    rows: vec![RowSpec {
                        handle: "Active".into(),
                        // One value for zero declared columns: excess.
                        values: vec![LiteralSpec::Value(Value::U64(9))],
                    }],
                }),
            },
            RelationSpec {
                name: "Account".into(),
                fields: vec![field("status", Some("StatusId"))],
                closed: None,
            },
        ],
        statements: vec![
            StatementSpec::Fd {
                relation: "Ghost".into(),
                projection: vec!["x".into()],
            },
            StatementSpec::Containment {
                source: SideSpec {
                    relation: "Account".into(),
                    projection: vec!["status".into()],
                    selection: vec![(
                        "status".into(),
                        LiteralSetSpec::One(LiteralSpec::Handle("Missing".into())),
                    )],
                },
                target: SideSpec {
                    relation: "Status".into(),
                    projection: vec!["id".into()],
                    selection: vec![],
                },
                bidirectional: false,
            },
        ],
    };
    let err = spec.descriptor().expect_err("three independent issues");
    let [
        SpecIssue::RowArityExcess {
            relation: 0,
            row: 0,
            ..
        },
        SpecIssue::UnknownRelation { statement: 0, .. },
        SpecIssue::UnknownHandle {
            at: LiteralAt::Selection { statement: 1, .. },
            ..
        },
    ] = err.issues()
    else {
        panic!("the complete list in spec order, not {:?}", err.issues());
    };
}

/// Harmless equivalent capacity spellings normalize to one canonical law
/// instead of refusing (the ban tables are deleted): `{n..n}` is the
/// exact window, `{0..0}` the exclusion, and unit floors — including the
/// `{1..*}` existence window — are ordinary grouped-measure windows.
/// Inverted literal bounds and dependent floors still refuse.
#[test]
fn equivalent_capacity_spellings_normalize_and_real_errors_still_refuse() {
    use bumbledb_theory::schema::Bound;
    use bumbledb_theory::schema::spec::{BoundSpec, CapacityWindowSpec, WeightSpec};

    let with_window = |window: CapacityWindowSpec| SchemaSpec {
        relations: vec![
            RelationSpec {
                name: "Student".into(),
                fields: vec![field("id", None)],
                closed: None,
            },
            RelationSpec {
                name: "Attempt".into(),
                fields: vec![field("student", None)],
                closed: None,
            },
        ],
        statements: vec![StatementSpec::Capacity {
            target: SideSpec {
                relation: "Student".into(),
                projection: vec!["id".into()],
                selection: vec![],
            },
            weight: WeightSpec::Unit,
            window,
            source: SideSpec {
                relation: "Attempt".into(),
                projection: vec!["student".into()],
                selection: vec![],
            },
        }],
    };
    let lowered = |window: CapacityWindowSpec| {
        let descriptor = with_window(window).descriptor().expect("lowers clean");
        let StatementDescriptor::Capacity { lo, hi, .. } = descriptor.statements[0].clone() else {
            panic!("one capacity lowered");
        };
        (lo, hi)
    };

    // Exact window and its equal-bound range spelling are one law.
    assert_eq!(
        lowered(CapacityWindowSpec::Exact(BoundSpec::Lit(3))),
        (3, Some(Bound::Lit(3)))
    );
    assert_eq!(
        lowered(CapacityWindowSpec::Range {
            lo: BoundSpec::Lit(3),
            hi: BoundSpec::Lit(3)
        }),
        (3, Some(Bound::Lit(3)))
    );
    // The exclusion and its range spelling are one law.
    assert_eq!(
        lowered(CapacityWindowSpec::Exact(BoundSpec::Lit(0))),
        (0, Some(Bound::Lit(0)))
    );
    assert_eq!(
        lowered(CapacityWindowSpec::Range {
            lo: BoundSpec::Lit(0),
            hi: BoundSpec::Lit(0)
        }),
        (0, Some(Bound::Lit(0)))
    );
    // Unit floors are grouped-measure windows, not bans: existence and
    // at-least-N both lower; the vacuous floor lowers and judges trivially.
    assert_eq!(
        lowered(CapacityWindowSpec::Floor(BoundSpec::Lit(1))),
        (1, None)
    );
    assert_eq!(
        lowered(CapacityWindowSpec::Floor(BoundSpec::Lit(4))),
        (4, None)
    );
    assert_eq!(
        lowered(CapacityWindowSpec::Floor(BoundSpec::Lit(0))),
        (0, None)
    );

    // Genuinely different semantics still refuse: inverted literal bounds…
    let err = with_window(CapacityWindowSpec::Range {
        lo: BoundSpec::Lit(5),
        hi: BoundSpec::Lit(2),
    })
    .descriptor()
    .expect_err("inverted refuses");
    assert!(err.issues().iter().any(|issue| matches!(
        issue,
        SpecIssue::CapacityInverted {
            statement: 0,
            lo: 5,
            hi: 2
        }
    )));
    // …and a dependent bound in the floor slot.
    let err = with_window(CapacityWindowSpec::Floor(BoundSpec::Field("id".into())))
        .descriptor()
        .expect_err("dependent floor refuses");
    assert!(
        err.issues()
            .iter()
            .any(|issue| matches!(issue, SpecIssue::CapacityDependentFloor { statement: 0 }))
    );
}

/// The sealed-field cap (finding 059): a relation past the u16 field-id
/// space is a typed issue at lowering — never a panic on the wire-facing
/// path, even when a statement addresses a field past the id space.
#[test]
fn wide_relation_is_a_typed_issue_not_a_panic() {
    let count = usize::from(u16::MAX) + 2;
    let spec = SchemaSpec {
        relations: vec![RelationSpec {
            name: "Wide".into(),
            fields: (0..count).map(|i| field(&format!("f{i}"), None)).collect(),
            closed: None,
        }],
        statements: vec![StatementSpec::Fd {
            relation: "Wide".into(),
            projection: vec![format!("f{}", count - 1).into()],
        }],
    };
    let err = spec.descriptor().expect_err("the cap refuses typed");
    assert!(err.issues().iter().any(|issue| matches!(
        issue,
        SpecIssue::RelationTooManyFields {
            relation: 0,
            fields: 65_537,
            ..
        }
    )));
}
