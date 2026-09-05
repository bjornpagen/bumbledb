use super::*;
use crate::error::{Mismatch, RowIndex, SchemaError, StatementErrorKind, TargetKeyCandidate};

fn target_key(key: u16, projection: &[FieldId], names: &[&str]) -> TargetKeyCandidate {
    TargetKeyCandidate {
        key: KeyId(key),
        projection: projection.into(),
        projection_names: names.iter().map(|name| Box::from(*name)).collect(),
    }
}

fn names(names: &[&str]) -> Box<[Box<str>]> {
    names.iter().map(|name| Box::from(*name)).collect()
}

#[test]
fn rejects_duplicate_relation_name() {
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![],
            },
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![],
            },
        ],
        statements: vec![],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateRelationName { name: "R".into() }
    );
}

#[test]
fn rejects_duplicate_field_name() {
    let decl = one_relation(vec![field("x", ValueType::U64), field("x", ValueType::I64)]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateFieldName {
            relation: RelationId(0),
            name: "x".into()
        }
    );
}

// The fresh-generation refusals are deleted with the mechanism itself:
// `FieldDescriptor` carries no generation attribute, so `fresh` on the
// wrong type or on a closed relation is UNREPRESENTABLE, not rejected
// (E-NO-RESERVE: no database-issued identity capability survives).

#[test]
fn rejects_fixed_bytes_widths_outside_the_range() {
    for len in [0u16, 65] {
        let decl = one_relation(vec![field("hash", ValueType::FixedBytes { len })]);
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::FixedBytesWidthOutOfRange {
                relation: RelationId(0),
                field: FieldId(0),
                len,
            }
        );
    }
    for len in [1u16, 7, 8, 9, 16, 32, 63, 64] {
        let decl = one_relation(vec![field("hash", ValueType::FixedBytes { len })]);
        assert!(decl.validate().is_ok(), "bytes<{len}> validates");
    }
}

#[test]
fn rejects_interval_widths_outside_the_range() {
    let fixed = |width: u64| ValueType::FixedInterval {
        element: FixedIntervalElement::U64,
        width,
    };
    for width in [0u64, u64::MAX] {
        let decl = one_relation(vec![field("span", fixed(width))]);
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::IntervalWidthOutOfRange {
                relation: RelationId(0),
                field: FieldId(0),
                width,
            }
        );
    }
    for width in [1u64, 2, 1 << 40, u64::MAX - 1] {
        let decl = one_relation(vec![field("span", fixed(width))]);
        assert!(decl.validate().is_ok(), "interval<u64, {width}> validates");
    }
}

#[test]
fn rejects_a_relation_whose_derived_column_count_overflows_u16() {
    let wide = |name: String, count: usize, value_type: ValueType, columns: usize| {
        let decl = one_relation(
            (0..count)
                .map(|i| field(&format!("{name}{i}"), value_type))
                .collect(),
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::RelationTooManyColumns {
                relation: RelationId(0),
                columns,
            }
        );
    };

    wide(
        "hash".into(),
        9_000,
        ValueType::FixedBytes { len: 64 },
        72_000,
    );

    wide(
        "span".into(),
        33_000,
        ValueType::Interval {
            element: IntervalElement::U64,
        },
        66_000,
    );

    wide("x".into(), 70_000, ValueType::U64, 70_000);
}

#[test]
fn the_column_cap_fires_before_any_u16_field_id_is_minted() {
    // The cap must fire before per-field checks that mint u16 ids (an
    // invalid bytes<0> width rejection cannot run until after the ids are
    // minted), so an over-wide roster is one typed refusal, never a panic.
    for filler in [ValueType::U64, ValueType::FixedBytes { len: 0 }] {
        let mut fields: Vec<FieldDescriptor> = (0..66_000)
            .map(|i| field(&format!("c{i}"), filler))
            .collect();
        fields.push(field("id", ValueType::U64));
        assert_eq!(
            one_relation(fields).validate().unwrap_err(),
            SchemaError::RelationTooManyColumns {
                relation: RelationId(0),
                columns: 66_001,
            }
        );
    }
}

#[test]
fn rejects_a_statement_roster_past_the_u16_id_space() {
    // count gate must fire before any per-statement validation walks

    let statement = StatementDescriptor::Containment {
        source: Side {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
            selection: Box::new([]),
        },
        target: Side {
            relation: RelationId(1),
            projection: Box::new([FieldId(0)]),
            selection: Box::new([]),
        },
    };
    let decl = two_relations(
        vec![field("x", ValueType::U64)],
        vec![field("y", ValueType::U64)],
        vec![statement; 65_537],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::TooManyStatements { count: 65_537 }
    );
}

#[test]
fn the_column_count_boundary_is_exact() {
    let mut fields: Vec<FieldDescriptor> = (0..8_191)
        .map(|i| field(&format!("hash{i}"), ValueType::FixedBytes { len: 64 }))
        .collect();
    fields.extend((0..7).map(|i| field(&format!("x{i}"), ValueType::U64)));
    assert!(
        one_relation(fields.clone()).validate().is_ok(),
        "65,535 columns validate"
    );
    fields.push(field("one_too_many", ValueType::U64));
    assert_eq!(
        one_relation(fields).validate().unwrap_err(),
        SchemaError::RelationTooManyColumns {
            relation: RelationId(0),
            columns: 65_536,
        }
    );
}

fn two_relations(
    source_fields: Vec<FieldDescriptor>,
    target_fields: Vec<FieldDescriptor>,
    statements: Vec<StatementDescriptor>,
) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "S".into(),
                fields: source_fields,
            },
            RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: target_fields,
            },
        ],
        statements,
    }
}

#[test]
fn equality_rejects_a_singleton_reverse_projection_without_a_left_key() {
    // `S(a) == T(x)` lowers to statements 1 and 2 after T's key. The

    let decl = two_relations(
        vec![field("a", ValueType::U64)],
        vec![field("x", ValueType::U64)],
        vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
            containment(
                side(RelationId(1), &[FieldId(0)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    );
    let StatementDescriptor::Containment { target, .. } = &decl.statements[2] else {
        panic!("the cited reverse half is a containment");
    };
    assert_eq!(target.relation, RelationId(0));
    assert_eq!(&*target.projection, &[FieldId(0)]);
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::NoMatchingTargetKey {
            target: RelationId(0),
            target_name: "S".into(),
            projection: Box::new([FieldId(0)]),
            projection_names: names(&["a"]),
            available: Box::new([])
        }
        .at(StatementId(2))
    );
}

#[test]
fn equality_rejects_a_composite_reverse_projection_without_a_left_key() {
    let decl = two_relations(
        vec![field("a", ValueType::U64), field("b", ValueType::I64)],
        vec![field("x", ValueType::U64), field("y", ValueType::I64)],
        vec![
            fd(RelationId(1), &[FieldId(1), FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
            ),
            containment(
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
            ),
        ],
    );
    let StatementDescriptor::Containment { target, .. } = &decl.statements[2] else {
        panic!("the cited reverse half is a containment");
    };
    assert_eq!(target.relation, RelationId(0));
    assert_eq!(&*target.projection, &[FieldId(0), FieldId(1)]);
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::NoMatchingTargetKey {
            target: RelationId(0),
            target_name: "S".into(),
            projection: Box::new([FieldId(0), FieldId(1)]),
            projection_names: names(&["a", "b"]),
            available: Box::new([])
        }
        .at(StatementId(2))
    );
}

#[test]
fn rejects_statement_unknown_relation() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements.push(fd(RelationId(7), &[FieldId(0)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::UnknownRelation {
            relation: RelationId(7)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_statement_unknown_field() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements.push(fd(RelationId(0), &[FieldId(9)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::UnknownField {
            relation: RelationId(0),
            field: FieldId(9)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_empty_projection() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements.push(fd(RelationId(0), &[]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::EmptyProjection {
            relation: RelationId(0)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_duplicate_projection_field() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements
        .push(fd(RelationId(0), &[FieldId(0), FieldId(0)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DuplicateProjectionField {
            relation: RelationId(0),
            field: FieldId(0)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_duplicate_selection_field() {
    let decl = two_relations(
        vec![field("a", ValueType::U64), field("flag", ValueType::Bool)],
        vec![field("x", ValueType::U64)],
        vec![containment(
            side_where(
                RelationId(0),
                &[FieldId(0)],
                vec![
                    (FieldId(1), Value::Bool(true)),
                    (FieldId(1), Value::Bool(true)),
                ],
            ),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DuplicateSelectionField {
            relation: RelationId(0),
            field: FieldId(1)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_functionality_with_two_intervals() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
    };
    let mut decl = one_relation(vec![field("a", iv), field("b", iv)]);
    decl.statements
        .push(fd(RelationId(0), &[FieldId(0), FieldId(1)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::FunctionalityMultipleIntervals {
            relation: RelationId(0),
            field: FieldId(1)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_functionality_interval_not_last() {
    let mut decl = one_relation(vec![
        field(
            "during",
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        ),
        field("room", ValueType::U64),
    ]);
    decl.statements
        .push(fd(RelationId(0), &[FieldId(0), FieldId(1)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::FunctionalityIntervalNotLast {
            relation: RelationId(0),
            field: FieldId(0)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_duplicate_functionality() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements.push(fd(RelationId(0), &[FieldId(0)]));
    decl.statements.push(fd(RelationId(0), &[FieldId(0)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DuplicateFunctionality {
            earlier: StatementId(0)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_permuted_duplicate_functionality() {
    let mut decl = one_relation(vec![field("a", ValueType::U64), field("b", ValueType::I64)]);
    decl.statements
        .push(fd(RelationId(0), &[FieldId(0), FieldId(1)]));
    decl.statements
        .push(fd(RelationId(0), &[FieldId(1), FieldId(0)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DuplicateFunctionality {
            earlier: StatementId(0)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_determinant_overflow() {
    let count = crate::schema::validate::MAX_DETERMINANT_WIDTH / 8 + 1;
    let fields: Vec<FieldDescriptor> = (0..count)
        .map(|i| field(&format!("f{i}"), ValueType::U64))
        .collect();
    let projection: Vec<FieldId> = (0..count)
        .map(|i| FieldId(u16::try_from(i).expect("field count fits u16")))
        .collect();
    let mut decl = one_relation(fields);
    decl.statements.push(fd(RelationId(0), &projection));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DeterminantKeyTooWide { width: count * 8 }.at(StatementId(0))
    );
}

#[test]
fn rejects_containment_arity_mismatch() {
    let decl = two_relations(
        vec![field("a", ValueType::U64), field("b", ValueType::U64)],
        vec![field("x", ValueType::U64)],
        vec![containment(
            side(RelationId(0), &[FieldId(0), FieldId(1)]),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ContainmentArityMismatch {
            mismatch: Mismatch {
                witnessed: 2,
                required: 1,
            },
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_containment_positional_type_mismatch() {
    let decl = two_relations(
        vec![field("a", ValueType::U64)],
        vec![field("x", ValueType::I64)],
        vec![containment(
            side(RelationId(0), &[FieldId(0)]),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ContainmentTypeMismatch { position: 0 }.at(StatementId(0))
    );
}

#[test]
fn rejects_interval_position_against_scalar() {
    let decl = two_relations(
        vec![field(
            "span",
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        )],
        vec![field("x", ValueType::I64)],
        vec![containment(
            side(RelationId(0), &[FieldId(0)]),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ContainmentTypeMismatch { position: 0 }.at(StatementId(0))
    );
}

#[test]
fn rejects_selected_field_also_projected() {
    let decl = two_relations(
        vec![field("a", ValueType::U64)],
        vec![field("x", ValueType::U64)],
        vec![containment(
            side_where(
                RelationId(0),
                &[FieldId(0)],
                vec![(FieldId(0), Value::U64(1))],
            ),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::SelectedFieldProjected {
            relation: RelationId(0),
            field: FieldId(0)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_selection_literal_type_mismatch() {
    let decl = two_relations(
        vec![field("a", ValueType::U64), field("flag", ValueType::Bool)],
        vec![field("x", ValueType::U64)],
        vec![containment(
            side_where(
                RelationId(0),
                &[FieldId(0)],
                vec![(FieldId(1), Value::U64(1))],
            ),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::SelectionLiteralTypeMismatch {
            relation: RelationId(0),
            field: FieldId(1)
        }
        .at(StatementId(0))
    );
}

#[test]
fn rejects_no_matching_target_key() {
    let decl = two_relations(
        vec![field("a", ValueType::U64)],
        vec![field("x", ValueType::U64)],
        vec![containment(
            side(RelationId(0), &[FieldId(0)]),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::NoMatchingTargetKey {
            target: RelationId(1),
            target_name: "T".into(),
            projection: Box::new([FieldId(0)]),
            projection_names: names(&["x"]),
            available: Box::new([])
        }
        .at(StatementId(0))
    );
}

#[test]
fn target_key_diagnostic_lists_the_requested_projection_and_every_available_key() {
    let decl = two_relations(
        vec![field("a", ValueType::U64), field("b", ValueType::U64)],
        vec![
            field("x", ValueType::U64),
            field("y", ValueType::U64),
            field("z", ValueType::U64),
        ],
        vec![
            fd(RelationId(1), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(1), FieldId(2)]),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
            ),
        ],
    );
    let error = decl.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "statement 2: target relation T (1) projection {x (0), y (1)} matches no \
         declared key; available keys: key 0 {x (0)}; key 1 {y (1), z (2)}"
    );
}

#[test]
fn rejects_interval_containment_without_pointwise_key() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
    };
    let decl = two_relations(
        vec![field("who", ValueType::U64), field("span", iv)],
        vec![field("who", ValueType::U64), field("during", iv)],
        vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
            ),
        ],
    );
    let error = decl.validate().unwrap_err();
    assert_eq!(
        error,
        StatementErrorKind::NoPointwiseTargetKey {
            target: RelationId(1),
            target_name: "T".into(),
            projection: Box::new([FieldId(0), FieldId(1)]),
            projection_names: names(&["who", "during"]),
            available: Box::new([target_key(0, &[FieldId(0)], &["who"])])
        }
        .at(StatementId(1))
    );
    assert_eq!(
        error.to_string(),
        "statement 1: target relation T (1) projection {who (0), during (1)} \
         matches no declared key; available keys: key 0 {who (0)}; hint: declare \
         the exact pointwise key `R(prefix…, interval) -> R`"
    );
}

#[test]
fn rejects_duplicate_statement() {
    let c = containment(
        side(RelationId(0), &[FieldId(0)]),
        side(RelationId(1), &[FieldId(0)]),
    );
    let decl = two_relations(
        vec![field("a", ValueType::U64)],
        vec![field("x", ValueType::U64)],
        vec![fd(RelationId(1), &[FieldId(0)]), c.clone(), c],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DuplicateStatement {
            earlier: StatementId(1)
        }
        .at(StatementId(2))
    );
}

#[test]
fn rejects_duplicate_statement_up_to_selection_order() {
    let a = side_where(
        RelationId(0),
        &[FieldId(0)],
        vec![
            (FieldId(1), Value::Bool(true)),
            (FieldId(2), Value::Bool(false)),
        ],
    );
    let b = side_where(
        RelationId(0),
        &[FieldId(0)],
        vec![
            (FieldId(2), Value::Bool(false)),
            (FieldId(1), Value::Bool(true)),
        ],
    );
    let decl = two_relations(
        vec![
            field("a", ValueType::U64),
            field("f1", ValueType::Bool),
            field("f2", ValueType::Bool),
        ],
        vec![field("x", ValueType::U64)],
        vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(a, side(RelationId(1), &[FieldId(0)])),
            containment(b, side(RelationId(1), &[FieldId(0)])),
        ],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DuplicateStatement {
            earlier: StatementId(1)
        }
        .at(StatementId(2))
    );
}

fn closed_currency(rows: Vec<Row>) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![closed(
            "Currency",
            vec![field("minor_units", ValueType::U64)],
            rows,
        )],
        statements: vec![],
    }
}

#[test]
fn rejects_an_empty_extension() {
    assert_eq!(
        closed_currency(vec![]).validate().unwrap_err(),
        SchemaError::EmptyExtension {
            relation: RelationId(0)
        }
    );
}

#[test]
fn rejects_an_extension_beyond_256_rows() {
    let rows: Vec<Row> = (0..257)
        .map(|i| row(&format!("H{i}"), vec![Value::U64(i)]))
        .collect();
    assert_eq!(
        closed_currency(rows).validate().unwrap_err(),
        SchemaError::ExtensionTooManyRows {
            relation: RelationId(0),
            count: 257
        }
    );
}

#[test]
fn rejects_a_duplicate_handle() {
    assert_eq!(
        closed_currency(vec![
            row("Usd", vec![Value::U64(2)]),
            row("Usd", vec![Value::U64(0)]),
        ])
        .validate()
        .unwrap_err(),
        SchemaError::DuplicateExtensionHandle {
            relation: RelationId(0),
            handle: "Usd".into()
        }
    );
}

#[test]
fn rejects_an_extension_arity_mismatch() {
    assert_eq!(
        closed_currency(vec![row("Usd", vec![Value::U64(2), Value::U64(9)])])
            .validate()
            .unwrap_err(),
        SchemaError::ExtensionArityMismatch {
            relation: RelationId(0),
            row: RowIndex(0),
            mismatch: Mismatch {
                witnessed: 2,
                required: 1,
            },
        }
    );
}

#[test]
fn rejects_an_extension_value_type_mismatch() {
    // after the synthetic id.
    assert_eq!(
        closed_currency(vec![row("Usd", vec![Value::Bool(true)])])
            .validate()
            .unwrap_err(),
        SchemaError::ExtensionValueTypeMismatch {
            relation: RelationId(0),
            row: RowIndex(0),
            field: FieldId(1)
        }
    );
}

#[test]
fn rejects_a_ray_axiom() {
    let of_element = |element, value| SchemaDescriptor {
        relations: vec![closed(
            "Quarter",
            vec![field("span", ValueType::Interval { element })],
            vec![row("Q1", vec![value])],
        )],
        statements: vec![],
    };
    let expected = SchemaError::ExtensionIntervalRay {
        relation: RelationId(0),
        row: RowIndex(0),
        field: FieldId(1),
    };
    assert_eq!(
        of_element(
            IntervalElement::U64,
            Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(5, u64::MAX).expect("nonempty interval")
            )
        )
        .validate()
        .unwrap_err(),
        expected
    );
    assert_eq!(
        of_element(
            IntervalElement::I64,
            Value::IntervalI64(
                bumbledb_theory::Interval::<i64>::new(5, i64::MAX).expect("nonempty interval")
            )
        )
        .validate()
        .unwrap_err(),
        expected
    );
}

#[test]
fn rejects_str_on_a_closed_relation() {
    let decl = SchemaDescriptor {
        relations: vec![closed(
            "Currency",
            vec![field("label", ValueType::String)],
            vec![row("Usd", vec![Value::String("dollar".into())])],
        )],
        statements: vec![],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::StrOnClosedRelation {
            relation: RelationId(0),
            field: FieldId(1)
        }
    );
}

#[test]
fn rejects_a_user_declared_id_on_a_closed_relation() {
    let decl = SchemaDescriptor {
        relations: vec![closed(
            "Currency",
            vec![field("id", ValueType::U64)],
            vec![row("Usd", vec![Value::U64(0)])],
        )],
        statements: vec![],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateFieldName {
            relation: RelationId(0),
            name: "id".into()
        }
    );
}

/// An interval-typed field on a closed relation, for the refusal tests.
fn closed_window() -> RelationDescriptor {
    closed(
        "Window",
        vec![field(
            "during",
            ValueType::Interval {
                element: IntervalElement::U64,
            },
        )],
        vec![row(
            "Morning",
            vec![Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(6, 12).expect("nonempty interval"),
            )],
        )],
    )
}

#[test]
fn rejects_an_interval_position_into_a_closed_target() {
    // walk with virtual storage — refused v0, trigger recorded.
    let decl = SchemaDescriptor {
        relations: vec![
            closed_window(),
            RelationDescriptor {
                extension: None,
                name: "Meeting".into(),
                fields: vec![field(
                    "span",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                )],
            },
        ],
        statements: vec![containment(
            side(RelationId(1), &[FieldId(0)]),
            side(RelationId(0), &[FieldId(1)]),
        )],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedContainmentInterval {
            relation: RelationId(0)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_an_interval_position_from_a_closed_source() {
    // — the same v0 refusal, source arm.
    let decl = SchemaDescriptor {
        relations: vec![
            closed_window(),
            RelationDescriptor {
                extension: None,
                name: "Shift".into(),
                fields: vec![field(
                    "span",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                )],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(1)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
        ],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedContainmentInterval {
            relation: RelationId(0)
        }
        .at(StatementId(2))
    );
}

#[test]
fn rejects_a_closed_target_projection_that_is_not_the_id() {
    // a payload-column target is refused by the closedness rule itself —

    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Currency",
                vec![field("minor_units", ValueType::U64)],
                vec![row("Usd", vec![Value::U64(2)])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Price".into(),
                fields: vec![field("units", ValueType::U64)],
            },
        ],
        statements: vec![containment(
            side(RelationId(1), &[FieldId(0)]),
            side(RelationId(0), &[FieldId(1)]),
        )],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedTargetNotHandle {
            target: RelationId(0),
            target_name: "Currency".into(),
            projection: Box::new([FieldId(1)]),
            projection_names: names(&["minor_units"])
        }
        .at(StatementId(1))
    );
}

#[test]
fn a_declared_key_on_the_closed_target_does_not_soften_the_handle_rule() {
    // point-read-served key whose field set equals the refused

    // available. The refusal names closedness, the actual rule.
    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![field("weight", ValueType::U64)],
                vec![
                    row("Light", vec![Value::U64(1)]),
                    row("Heavy", vec![Value::U64(2)]),
                ],
            ),
            RelationDescriptor {
                extension: None,
                name: "Task".into(),
                fields: vec![field("weight_ref", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(1)]),
            containment(
                side(RelationId(1), &[FieldId(0)]),
                side(RelationId(0), &[FieldId(1)]),
            ),
        ],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedTargetNotHandle {
            target: RelationId(0),
            target_name: "Kind".into(),
            projection: Box::new([FieldId(1)]),
            projection_names: names(&["weight"])
        }
        .at(StatementId(2))
    );
}

#[test]
fn rejects_a_closed_to_closed_containment_the_axioms_refute() {
    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![field("severity", ValueType::U64)],
                vec![
                    row("Soft", vec![Value::U64(0)]),
                    row("Hard", vec![Value::U64(7)]),
                ],
            ),
            closed(
                "Severity",
                vec![],
                vec![row("Low", vec![]), row("High", vec![])],
            ),
        ],
        statements: vec![containment(
            side(RelationId(0), &[FieldId(1)]),
            side(RelationId(1), &[FieldId(0)]),
        )],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedStatementRefuted {
            relation: RelationId(0),
            row: RowIndex(1)
        }
        .at(StatementId(2))
    );
}

#[test]
fn rejects_a_closed_to_closed_containment_whose_value_exceeds_the_index_range() {
    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![field("severity", ValueType::U64)],
                vec![
                    row("Soft", vec![Value::U64(0)]),
                    row("Hard", vec![Value::U64(70_000)]),
                ],
            ),
            closed(
                "Severity",
                vec![],
                vec![row("Low", vec![]), row("High", vec![])],
            ),
        ],
        statements: vec![containment(
            side(RelationId(0), &[FieldId(1)]),
            side(RelationId(1), &[FieldId(0)]),
        )],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedStatementRefuted {
            relation: RelationId(0),
            row: RowIndex(1)
        }
        .at(StatementId(2))
    );
}

#[test]
fn rejects_a_declared_key_the_axioms_refute() {
    let decl = SchemaDescriptor {
        relations: vec![closed(
            "Currency",
            vec![field("minor_units", ValueType::U64)],
            vec![
                row("Usd", vec![Value::U64(2)]),
                row("Eur", vec![Value::U64(2)]),
            ],
        )],
        statements: vec![fd(RelationId(0), &[FieldId(1)])],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedStatementRefuted {
            relation: RelationId(0),
            row: RowIndex(1)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_declared_pointwise_key_the_axioms_refute() {
    let decl = SchemaDescriptor {
        relations: vec![closed(
            "Window",
            vec![field(
                "during",
                ValueType::Interval {
                    element: IntervalElement::U64,
                },
            )],
            vec![
                row(
                    "Morning",
                    vec![Value::IntervalU64(
                        bumbledb_theory::Interval::<u64>::new(6, 12).expect("nonempty interval"),
                    )],
                ),
                row(
                    "Brunch",
                    vec![Value::IntervalU64(
                        bumbledb_theory::Interval::<u64>::new(10, 14).expect("nonempty interval"),
                    )],
                ),
            ],
        )],
        statements: vec![fd(RelationId(0), &[FieldId(1)])],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedStatementRefuted {
            relation: RelationId(0),
            row: RowIndex(1)
        }
        .at(StatementId(1))
    );
}

fn extension_tree() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Parent".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Task".into(),
                fields: vec![
                    field("parent", ValueType::U64),
                    field("pos", ValueType::U64),
                    field("prio", ValueType::U64),
                    field("flag", ValueType::Bool),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![fd(RelationId(0), &[FieldId(0)])],
    }
}

#[test]
fn rejects_an_empty_literal_set() {
    let mut decl = extension_tree();
    decl.statements.push(containment(
        side_where_sets(
            RelationId(1),
            &[FieldId(0)],
            vec![(FieldId(2), LiteralSet::Many(Box::new([])))],
        ),
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DegenerateSelectionSet {
            relation: RelationId(1),
            field: FieldId(2),
            len: 0
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_singleton_spelled_as_a_set() {
    let mut decl = extension_tree();
    decl.statements.push(containment(
        side_where_sets(
            RelationId(1),
            &[FieldId(0)],
            vec![(FieldId(2), LiteralSet::Many(Box::new([Value::U64(1)])))],
        ),
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DegenerateSelectionSet {
            relation: RelationId(1),
            field: FieldId(2),
            len: 1
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_duplicate_literal_within_a_set() {
    let mut decl = extension_tree();
    decl.statements.push(containment(
        side_where_sets(
            RelationId(1),
            &[FieldId(0)],
            vec![(
                FieldId(2),
                LiteralSet::Many(Box::new([Value::U64(1), Value::U64(1)])),
            )],
        ),
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::DuplicateSelectionLiteral {
            relation: RelationId(1),
            field: FieldId(2)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_set_literal_of_the_wrong_type() {
    let mut decl = extension_tree();
    decl.statements.push(containment(
        side_where_sets(
            RelationId(1),
            &[FieldId(0)],
            vec![(
                FieldId(2),
                LiteralSet::Many(Box::new([Value::U64(1), Value::Bool(true)])),
            )],
        ),
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::SelectionLiteralTypeMismatch {
            relation: RelationId(1),
            field: FieldId(2)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_capacity_with_an_interval_position() {
    // The v0 refusal: a window counts FACTS per parent; an interval

    // (`lean/Bumbledb/Capacity.lean` § v0 refusals; trigger: a sighted

    let mut decl = extension_tree();
    // A pointwise key on Task(span) so only the interval refusal fires.
    decl.relations[0].fields.push(field(
        "active",
        ValueType::Interval {
            element: IntervalElement::U64,
        },
    ));
    decl.statements = vec![
        fd(RelationId(0), &[FieldId(0), FieldId(1)]),
        capacity(
            side(RelationId(1), &[FieldId(0), FieldId(4)]),
            1,
            Some(3),
            side(RelationId(0), &[FieldId(0), FieldId(1)]),
        ),
    ];
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityIntervalPosition {
            relation: RelationId(1),
            field: FieldId(4)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_signed_weight() {
    // the illegal weight is a typed refusal, never a checked runtime

    let mut decl = extension_tree();
    decl.relations[1]
        .fields
        .push(field("delta", ValueType::I64));
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::Field(FieldId(5)),
        0,
        Some(Bound::Lit(3)),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityWeightNotU64 {
            relation: RelationId(1),
            field: FieldId(5)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_non_u64_weight() {
    let mut decl = extension_tree();
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::Field(FieldId(3)),
        0,
        Some(Bound::Lit(3)),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityWeightNotU64 {
            relation: RelationId(1),
            field: FieldId(3)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_duration_weight_over_a_scalar() {
    let mut decl = extension_tree();
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::DurationOf(FieldId(1)),
        0,
        Some(Bound::Lit(3)),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityWeightNotDuration {
            relation: RelationId(1),
            field: FieldId(1)
        }
        .at(StatementId(1))
    );
}

/// Float capacity is refused at schema validation, never judged with
/// rounding (chapter 11): a dense `interval<f64>` field is an interval,
/// but not an exact integral duration — neither a `[Duration(field)]`
/// weight nor a `{0..Duration(field)}` bound may read it. A
/// `FixedInterval<F64>` cannot even be spelled (its element enum is
/// discrete), so only the general dense interval needs a refusal.
#[test]
fn rejects_float_interval_duration_weights_and_bounds() {
    let dense = ValueType::Interval {
        element: IntervalElement::F64,
    };
    // Weight over a dense source interval.
    let mut decl = extension_tree();
    decl.relations[1].fields.push(field("window", dense));
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::DurationOf(FieldId(5)),
        0,
        Some(Bound::Lit(3600)),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityWeightNotDuration {
            relation: RelationId(1),
            field: FieldId(5)
        }
        .at(StatementId(1))
    );
    // Dependent bound over a dense target interval.
    let mut decl = extension_tree();
    decl.relations[0].fields.push(field("window", dense));
    decl.relations[1].fields.push(field(
        "busy",
        ValueType::Interval {
            element: IntervalElement::U64,
        },
    ));
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::DurationOf(FieldId(5)),
        0,
        Some(Bound::TargetDuration(FieldId(1))),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityBoundNotDuration {
            relation: RelationId(0),
            field: FieldId(1)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_an_unknown_weight_field() {
    let mut decl = extension_tree();
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::Field(FieldId(9)),
        0,
        Some(Bound::Lit(3)),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::UnknownField {
            relation: RelationId(1),
            field: FieldId(9)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_signed_dependent_bound() {
    let mut decl = extension_tree();
    decl.relations[0].fields.push(field("cap", ValueType::I64));
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::Field(FieldId(1)),
        0,
        Some(Bound::TargetField(FieldId(1))),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityBoundNotU64 {
            relation: RelationId(0),
            field: FieldId(1)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_duration_bound_over_a_scalar() {
    let mut decl = extension_tree();
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::Field(FieldId(1)),
        0,
        Some(Bound::TargetDuration(FieldId(0))),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityBoundNotDuration {
            relation: RelationId(0),
            field: FieldId(0)
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_unit_window_against_a_duration_bound() {
    // Dimension mixing (ruled 2026-07-24, C18): a count of facts

    let mut decl = extension_tree();
    decl.relations[0].fields.push(field(
        "span",
        ValueType::Interval {
            element: IntervalElement::U64,
        },
    ));
    decl.statements.push(capacity(
        side(RelationId(1), &[FieldId(0)]),
        0,
        None,
        side(RelationId(0), &[FieldId(0)]),
    ));

    let Some(StatementDescriptor::Capacity { hi, lo, .. }) = decl.statements.last_mut() else {
        unreachable!("just pushed a capacity statement");
    };
    *lo = 0;
    *hi = Some(Bound::TargetDuration(FieldId(1)));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityDimensionMixing { field: FieldId(1) }.at(StatementId(1))
    );
}

#[test]
fn rejects_a_u64_weight_against_a_duration_bound() {
    let mut decl = extension_tree();
    decl.relations[0].fields.push(field(
        "span",
        ValueType::Interval {
            element: IntervalElement::U64,
        },
    ));
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::Field(FieldId(1)),
        0,
        Some(Bound::TargetDuration(FieldId(1))),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityDimensionMixing { field: FieldId(1) }.at(StatementId(1))
    );
}

#[test]
fn rejects_a_duration_weight_against_a_u64_field_bound() {
    let mut decl = extension_tree();
    decl.relations[0].fields.push(field("cap", ValueType::U64));
    decl.statements.push(capacity_weighted(
        side(RelationId(0), &[FieldId(0)]),
        Weight::DurationOf(FieldId(4)),
        0,
        Some(Bound::TargetField(FieldId(1))),
        side(RelationId(1), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityDimensionMixing { field: FieldId(1) }.at(StatementId(1))
    );
}

#[test]
fn rejects_a_weighted_closed_pair_the_axioms_refute_under_a_dependent_bound() {
    // resolved window (`lean/Bumbledb/Schema.lean: den_closed_constant`).

    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Pool",
                vec![field("cap", ValueType::U64)],
                vec![row("P0", vec![Value::U64(5)])],
            ),
            closed(
                "Dev",
                vec![
                    field("pool", ValueType::U64),
                    field("watts", ValueType::U64),
                ],
                vec![
                    row("D0", vec![Value::U64(0), Value::U64(3)]),
                    row("D1", vec![Value::U64(0), Value::U64(4)]),
                ],
            ),
        ],
        statements: vec![capacity_weighted(
            side(RelationId(0), &[FieldId(0)]),
            Weight::Field(FieldId(2)),
            0,
            Some(Bound::TargetField(FieldId(1))),
            side(RelationId(1), &[FieldId(1)]),
        )],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedStatementRefuted {
            relation: RelationId(0),
            row: RowIndex(0)
        }
        .at(StatementId(2))
    );
}

#[test]
fn rejects_an_inverted_window() {
    let mut decl = extension_tree();
    decl.statements.push(capacity(
        side(RelationId(1), &[FieldId(0)]),
        3,
        Some(1),
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::CapacityInvertedWindow { lo: 3, hi: 1 }.at(StatementId(1))
    );
}

/// The spelling-ban table is deleted: the vacuous `{0..*}` window and the
/// unit existence window `{1..*}` are accepted canonical grouped-measure
/// laws (chapter 10's normalization decision), preserving each statement's
/// authored attribution instead of policing its utterance. The vacuous
/// window is trivially satisfied; the existence window is judged like any
/// floor. Inverted literal bounds remain a genuine semantic refusal.
#[test]
fn accepts_vacuous_and_existence_windows_as_canonical_laws() {
    for lo in [0u64, 1, 4] {
        let mut decl = extension_tree();
        decl.statements.push(capacity(
            side(RelationId(1), &[FieldId(0)]),
            lo,
            None,
            side(RelationId(0), &[FieldId(0)]),
        ));
        let schema = decl
            .validate()
            .unwrap_or_else(|error| panic!("floor {{{lo}..*}} is a law: {error:?}"));
        let statement = &schema.capacities()[0];
        assert_eq!(statement.lo, lo);
        assert_eq!(statement.hi, crate::schema::SealedBound::Unbounded);
    }
}

#[test]
fn rejects_a_capacity_whose_target_is_no_key() {
    let mut decl = extension_tree();
    decl.statements = vec![capacity(
        side(RelationId(1), &[FieldId(0)]),
        1,
        Some(3),
        side(RelationId(0), &[FieldId(0)]),
    )];
    assert!(matches!(
        decl.validate().unwrap_err(),
        SchemaError::Statement {
            statement: StatementId(0),
            kind: StatementErrorKind::NoMatchingTargetKey {
                target: RelationId(0),
                ..
            },
        }
    ));
}

#[test]
fn rejects_a_window_arity_mismatch() {
    let mut decl = extension_tree();
    decl.statements.push(capacity(
        side(RelationId(1), &[FieldId(0), FieldId(2)]),
        1,
        Some(3),
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ContainmentArityMismatch {
            mismatch: Mismatch {
                witnessed: 2,
                required: 1,
            },
        }
        .at(StatementId(1))
    );
}

#[test]
fn rejects_a_closed_to_closed_window_the_axioms_refute() {
    // (`lean/Bumbledb/Schema.lean: den_closed_constant`). The cited row

    let decl = SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![field("severity", ValueType::U64)],
                vec![
                    row("Soft", vec![Value::U64(0)]),
                    row("Hard", vec![Value::U64(0)]),
                ],
            ),
            closed(
                "Severity",
                vec![],
                vec![row("Low", vec![]), row("High", vec![])],
            ),
        ],
        statements: vec![capacity(
            side(RelationId(0), &[FieldId(1)]),
            1,
            Some(1),
            side(RelationId(1), &[FieldId(0)]),
        )],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ClosedStatementRefuted {
            relation: RelationId(1),
            row: RowIndex(0)
        }
        .at(StatementId(2))
    );
}

#[test]
fn rejects_interval_positions_across_element_domains_whatever_the_widths() {
    let decl = two_relations(
        vec![field(
            "slot",
            ValueType::FixedInterval {
                element: FixedIntervalElement::U64,
                width: 1,
            },
        )],
        vec![field(
            "span",
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        )],
        vec![containment(
            side(RelationId(0), &[FieldId(0)]),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        StatementErrorKind::ContainmentTypeMismatch { position: 0 }.at(StatementId(0))
    );
}

/// The declaration counts here are host-supplied data at the public
/// `Db::create` trust boundary, and the query boundary's own caps are all typed
/// refusals (`ValidationError::TooManyRules` / `TooManyAtoms` /
/// `TooManyVariables`) — the schema boundary now matches the engine's
/// typed-refusal law (`lean/Bumbledb/Admission.lean`: acceptance and refusal
/// are a typed gate verdict, never a crash). The caps landed as
/// `SchemaError::TooManyStatements` (the materialized statement roster past
/// 2^16) and `SchemaError::RelationTooManyColumns` (a relation's field-id mint
/// past 2^16), both computed before any u16 id is minted; `validate`'s `#
/// Panics` contract now names only the unreachable 2^32-relations case.
#[test]
fn the_id_width_caps_refuse_typed_rather_than_panicking() {
    let count = u32::from(u16::MAX) + 1;
    let relations: Vec<RelationDescriptor> = (0..=count)
        .map(|idx| RelationDescriptor {
            extension: None,
            name: format!("R{idx}").into(),
            fields: vec![field("id", ValueType::U64)],
        })
        .collect();
    let statements = (0..=count)
        .map(|idx| StatementDescriptor::Functionality {
            relation: RelationId(idx),
            projection: Box::new([FieldId(0)]),
        })
        .collect();
    let decl = SchemaDescriptor {
        relations,
        statements,
    };
    assert!(
        decl.validate().is_err(),
        "past-2^16 statement declarations are hostile input: a typed SchemaError, never a panic"
    );

    let decl = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Wide".into(),
            fields: (0..=count)
                .map(|idx| field(&format!("f{idx}"), ValueType::U64))
                .collect(),
        }],
        statements: vec![],
    };
    assert!(
        decl.validate().is_err(),
        "past-2^16 field declarations are hostile input: a typed SchemaError, never a panic"
    );
}
