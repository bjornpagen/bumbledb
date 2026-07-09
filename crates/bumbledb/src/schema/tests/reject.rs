use super::*;
use crate::error::SchemaError;

// Field-level checks first, then the statement reject corpus: one test per
// line of the validation roster (docs/architecture/30-dependencies.md
// § validation roster), each asserting the exact error variant. "FD with
// selection" and "non-key FD form" have no tests: both are unrepresentable
// under `StatementDescriptor::Functionality` (PRD 03 § Conflict).

#[test]
fn rejects_duplicate_relation_name() {
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "R".into(),
                fields: vec![],
            },
            RelationDescriptor {
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

#[test]
fn rejects_enum_without_variants() {
    let decl = one_relation(vec![field("e", enum_type(&[]))]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::EnumWithoutVariants {
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

#[test]
fn rejects_enum_with_more_than_256_variants() {
    let names: Vec<String> = (0..257).map(|i| format!("V{i}")).collect();
    let decl = one_relation(vec![field(
        "e",
        enum_type(&names.iter().map(String::as_str).collect::<Vec<_>>()),
    )]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::EnumTooManyVariants {
            relation: RelationId(0),
            field: FieldId(0),
            count: 257
        }
    );
}

#[test]
fn rejects_duplicate_enum_variant() {
    let decl = one_relation(vec![field("e", enum_type(&["A", "A"]))]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateEnumVariant {
            relation: RelationId(0),
            field: FieldId(0),
            variant: "A".into()
        }
    );
}

#[test]
fn rejects_serial_on_non_u64() {
    let decl = one_relation(vec![FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::I64,
        generation: Generation::Serial,
    }]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::SerialOnNonU64 {
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

// --- The statement roster ---

/// Two relations with no serials: statement ids equal declaration order.
fn two_relations(
    source_fields: Vec<FieldDescriptor>,
    target_fields: Vec<FieldDescriptor>,
    statements: Vec<StatementDescriptor>,
) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "S".into(),
                fields: source_fields,
            },
            RelationDescriptor {
                name: "T".into(),
                fields: target_fields,
            },
        ],
        statements,
    }
}

/// Roster "unknown relation … ids".
#[test]
fn rejects_statement_unknown_relation() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements.push(fd(RelationId(7), &[FieldId(0)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::StatementUnknownRelation {
            statement: StatementId(0),
            relation: RelationId(7)
        }
    );
}

/// Roster "unknown … field ids".
#[test]
fn rejects_statement_unknown_field() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements.push(fd(RelationId(0), &[FieldId(9)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::StatementUnknownField {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(9)
        }
    );
}

/// Roster "empty … projections".
#[test]
fn rejects_empty_projection() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements.push(fd(RelationId(0), &[]));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::EmptyProjection {
            statement: StatementId(0),
            relation: RelationId(0)
        }
    );
}

/// Roster "duplicate-carrying projections".
#[test]
fn rejects_duplicate_projection_field() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements
        .push(fd(RelationId(0), &[FieldId(0), FieldId(0)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateProjectionField {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

/// Roster "duplicate-carrying projections", selection sibling: σ is a set.
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
        SchemaError::DuplicateSelectionField {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(1)
        }
    );
}

/// Roster ">1 interval position": 2-D exclusion, which the ordered guard
/// cannot answer.
#[test]
fn rejects_functionality_with_two_intervals() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
    };
    let mut decl = one_relation(vec![field("a", iv.clone()), field("b", iv)]);
    decl.statements
        .push(fd(RelationId(0), &[FieldId(0), FieldId(1)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::FunctionalityMultipleIntervals {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(1)
        }
    );
}

/// Roster "interval not in final position": the neighbor probe needs the
/// scalar prefix as its group.
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
        SchemaError::FunctionalityIntervalNotLast {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

/// Roster "duplicate statements", FD form: an identical ordered projection.
#[test]
fn rejects_duplicate_functionality() {
    let mut decl = one_relation(vec![field("a", ValueType::U64)]);
    decl.statements.push(fd(RelationId(0), &[FieldId(0)]));
    decl.statements.push(fd(RelationId(0), &[FieldId(0)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateFunctionality {
            statement: StatementId(1),
            earlier: StatementId(0)
        }
    );
}

/// The FD duplicate rule is a *set* rule: a permuted projection asserts the
/// same judgment (its guard is pure write amplification), and rejecting it
/// is what keeps target-key resolution — a permutation match — unambiguous.
#[test]
fn rejects_permuted_duplicate_functionality() {
    let mut decl = one_relation(vec![field("a", ValueType::U64), field("b", ValueType::I64)]);
    decl.statements
        .push(fd(RelationId(0), &[FieldId(0), FieldId(1)]));
    decl.statements
        .push(fd(RelationId(0), &[FieldId(1), FieldId(0)]));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateFunctionality {
            statement: StatementId(1),
            earlier: StatementId(0)
        }
    );
}

/// Roster "guard width overflow": one u64 field more than
/// `MAX_GUARD_WIDTH` (the storage-side constant, imported — never
/// duplicated) admits.
#[test]
fn rejects_guard_overflow() {
    let count = crate::storage::keys::MAX_GUARD_WIDTH / 8 + 1;
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
        SchemaError::GuardKeyTooWide {
            statement: StatementId(0),
            width: count * 8
        }
    );
}

/// Roster "arity mismatch between sides".
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
        SchemaError::ContainmentArityMismatch {
            statement: StatementId(0),
            source: 2,
            target: 1
        }
    );
}

/// Roster "positional structural-type mismatch".
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
        SchemaError::ContainmentTypeMismatch {
            statement: StatementId(0),
            position: 0
        }
    );
}

/// Roster's called-out type-mismatch instance: an interval position against
/// a scalar position — the same variant, pinned separately because it is
/// the one migration authors will hit.
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
        SchemaError::ContainmentTypeMismatch {
            statement: StatementId(0),
            position: 0
        }
    );
}

/// Roster "a selected field also projected": a constant column — write the
/// statement you mean.
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
        SchemaError::SelectedFieldProjected {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

/// Roster "selection literal type mismatch".
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
        SchemaError::SelectionLiteralTypeMismatch {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(1)
        }
    );
}

/// Roster "… out-of-range enum ordinals".
#[test]
fn rejects_out_of_range_enum_selection_literal() {
    let decl = two_relations(
        vec![
            field("a", ValueType::U64),
            field("kind", enum_type(&["A", "B"])),
        ],
        vec![field("x", ValueType::U64)],
        vec![containment(
            side_where(
                RelationId(0),
                &[FieldId(0)],
                vec![(FieldId(1), Value::Enum(2))],
            ),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::SelectionEnumOrdinalOutOfRange {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(1),
            ordinal: 2
        }
    );
}

/// Roster "… non-UTF-8 string literals".
#[test]
fn rejects_non_utf8_string_selection_literal() {
    let decl = two_relations(
        vec![field("a", ValueType::U64), field("name", ValueType::String)],
        vec![field("x", ValueType::U64)],
        vec![containment(
            side_where(
                RelationId(0),
                &[FieldId(0)],
                vec![(FieldId(1), Value::String(Box::new([0xFF])))],
            ),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::SelectionLiteralNotUtf8 {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(1)
        }
    );
}

/// The interval literal bound rule: `start >= end` denotes no points, and a
/// fact never denotes nothing.
#[test]
fn rejects_empty_interval_selection_literal() {
    let decl = two_relations(
        vec![
            field("a", ValueType::U64),
            field(
                "span",
                ValueType::Interval {
                    element: IntervalElement::U64,
                },
            ),
        ],
        vec![field("x", ValueType::U64)],
        vec![containment(
            side_where(
                RelationId(0),
                &[FieldId(0)],
                vec![(FieldId(1), Value::IntervalU64(5, 5))],
            ),
            side(RelationId(1), &[FieldId(0)]),
        )],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::SelectionIntervalEmpty {
            statement: StatementId(0),
            relation: RelationId(0),
            field: FieldId(1)
        }
    );
}

/// Roster "IND whose target projection matches no key of the target".
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
        SchemaError::NoMatchingTargetKey {
            statement: StatementId(0),
            relation: RelationId(1)
        }
    );
}

/// Roster "… (or, with an interval position, no pointwise key carrying
/// it)": the target's only key is scalar.
#[test]
fn rejects_interval_containment_without_pointwise_key() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
    };
    let decl = two_relations(
        vec![field("who", ValueType::U64), field("span", iv.clone())],
        vec![field("who", ValueType::U64), field("during", iv)],
        vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
            ),
        ],
    );
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::NoPointwiseTargetKey {
            statement: StatementId(1),
            relation: RelationId(1)
        }
    );
}

/// Roster "duplicate statements (identical normalized sides and form)".
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
        SchemaError::DuplicateStatement {
            statement: StatementId(2),
            earlier: StatementId(1)
        }
    );
}

/// Normalization sorts selections by field id: two containments whose
/// selections differ only in written order are the same statement.
#[test]
fn rejects_duplicate_statement_up_to_selection_order() {
    // Same bindings, opposite written order.
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
        SchemaError::DuplicateStatement {
            statement: StatementId(2),
            earlier: StatementId(1)
        }
    );
}

// `Schema` is unconstructible outside this module: its fields and
// `Relation`'s fields are private, and no public constructor exists —
// the only path in is `SchemaDescriptor::validate`. (Compile-time
// property; recorded here as the sealing contract.)
