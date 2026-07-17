use super::*;
use crate::error::{SchemaError, TargetKeyCandidate};

fn target_key(key: u16, projection: &[FieldId]) -> TargetKeyCandidate {
    TargetKeyCandidate {
        key: KeyId(key),
        projection: projection.into(),
    }
}

// Field-level checks first, then the statement reject corpus: one test per
// line of the validation roster (docs/architecture/30-dependencies.md
// § validation roster), each asserting the exact error variant. "FD with
// selection" and "non-key FD form" have no tests: both are unrepresentable
// under `StatementDescriptor::Functionality`.

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

#[test]
fn rejects_fresh_on_non_u64() {
    let decl = one_relation(vec![FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::I64,
        generation: Generation::Fresh,
    }]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::FreshOnNonU64 {
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

#[test]
fn rejects_fixed_bytes_widths_outside_the_range() {
    // The bytes<N> width gate: N = 0 denotes nothing and N = 65 crosses
    // the 64-byte (8-word) ceiling — both typed rejections; every width
    // in 1..=64 validates (the pad boundaries included).
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
    // The interval<E, w> width gate: w = 0 denotes nothing, and at
    // w = u64::MAX no start satisfies the Q2 bound in either element
    // domain (an empty type is a relation no fact can inhabit); every
    // other width is a real type.
    let fixed = |width: u64| ValueType::Interval {
        element: IntervalElement::U64,
        width: Some(width),
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
    // The image's column index is u16 (`crate::image::ColumnSpan`): an
    // interval field spans two word columns, a bytes<N> field ⌈N/8⌉.
    // A relation whose fields multiply out past 65,535 columns must be
    // a typed rejection at the declaration boundary — never a
    // query-time panic in `column_spans`.
    let wide = |name: String, count: usize, value_type: ValueType, columns: usize| {
        let decl = one_relation(
            (0..count)
                .map(|i| field(&format!("{name}{i}"), value_type.clone()))
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
    // 9,000 bytes<64> fields: 9,000 × 8 = 72,000 columns.
    wide(
        "hash".into(),
        9_000,
        ValueType::FixedBytes { len: 64 },
        72_000,
    );
    // 33,000 interval fields: 66,000 columns.
    wide(
        "span".into(),
        33_000,
        ValueType::Interval {
            element: IntervalElement::U64,
            width: None,
        },
        66_000,
    );
    // 70,000 scalar fields: past the FieldId width too — the same typed
    // rejection, never the `field count fits u16` expect.
    wide("x".into(), 70_000, ValueType::U64, 70_000);
}

#[test]
fn the_column_cap_fires_before_any_u16_field_id_is_minted() {
    // A fresh field PAST the u16 boundary: `materialized_statements`
    // mints the auto-key's FieldId before relation validation runs, so
    // the cap must be a pre-pass — typed rejection, never the `field
    // count fits u16` expect. The zero-column `bytes<0>` flood counts
    // at its one-column legal minimum for the same reason (its own
    // width rejection cannot run until after the ids are minted).
    for filler in [
        ValueType::U64,
        ValueType::FixedBytes { len: 0 }, // invalid — but the cap fires first
    ] {
        let mut fields: Vec<FieldDescriptor> = (0..66_000)
            .map(|i| field(&format!("c{i}"), filler.clone()))
            .collect();
        fields.push(FieldDescriptor {
            name: "id".into(),
            value_type: ValueType::U64,
            generation: Generation::Fresh,
        });
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
    // 65,537 statements exceed the StatementId space: a typed
    // rejection at the declaration boundary, never the `statement
    // count fits u16` expect. (The statements are duplicates — the
    // count gate must fire before any per-statement validation walks
    // the roster.)
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
    // 8,191 bytes<64> fields (65,528 columns) plus 7 u64 fields land
    // exactly on 65,535 — the widest legal relation validates; one more
    // column is the typed rejection.
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

// --- The statement roster ---

/// Two relations with no fresh ids: statement ids equal declaration order.
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
    // forward half resolves T(x); the reverse half targets unkeyed S(a)
    // and must cite that reverse statement and relation.
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
        SchemaError::NoMatchingTargetKey {
            statement: StatementId(2),
            target: RelationId(0),
            projection: Box::new([FieldId(0)]),
            available: Box::new([]),
        }
    );
}

#[test]
fn equality_rejects_a_composite_reverse_projection_without_a_left_key() {
    // Mixed (u64, i64) product. T's key is declared in reordered (y, x)
    // order, proving the forward half resolves by exact field set and a
    // permutation; only the reverse half targeting unkeyed S(a, b) fails.
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
        SchemaError::NoMatchingTargetKey {
            statement: StatementId(2),
            target: RelationId(0),
            projection: Box::new([FieldId(0), FieldId(1)]),
            available: Box::new([]),
        }
    );
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

/// Roster ">1 interval position": 2-D exclusion, which the ordered determinant
/// cannot answer.
#[test]
fn rejects_functionality_with_two_intervals() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
        width: None,
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
                width: None,
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
/// same judgment (its determinant is pure write amplification), and rejecting it
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

/// Roster "determinant width overflow": one u64 field more than
/// `MAX_DETERMINANT_WIDTH` (the storage-side constant, imported — never
/// duplicated) admits.
#[test]
fn rejects_determinant_overflow() {
    let count = crate::storage::keys::MAX_DETERMINANT_WIDTH / 8 + 1;
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
        SchemaError::DeterminantKeyTooWide {
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
                width: None,
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
            target: RelationId(1),
            projection: Box::new([FieldId(0)]),
            available: Box::new([]),
        }
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
        "statement 2: target relation 1 projection {0, 1} matches no declared key; \
         available keys: key 0 {0}; key 1 {1, 2}"
    );
}

/// Roster "… (or, with an interval position, no pointwise key carrying
/// it)": the target's only key is scalar.
#[test]
fn rejects_interval_containment_without_pointwise_key() {
    let iv = ValueType::Interval {
        element: IntervalElement::I64,
        width: None,
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
    let error = decl.validate().unwrap_err();
    assert_eq!(
        error,
        SchemaError::NoPointwiseTargetKey {
            statement: StatementId(1),
            target: RelationId(1),
            projection: Box::new([FieldId(0), FieldId(1)]),
            available: Box::new([target_key(0, &[FieldId(0)])]),
        }
    );
    assert_eq!(
        error.to_string(),
        "statement 1: target relation 1 projection {0, 1} matches no declared key; \
         available keys: key 0 {0}; hint: declare the exact pointwise key \
         `R(prefix…, interval) -> R`"
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

// --- The closed-relation roster (docs/architecture/10-data-model.md
// § closed relations): one test per variant, fixtures hand-built — the
// macro grammar for closed relations is the emission PRD's.

/// One closed relation over one u64 column with the given rows.
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
    // A closed relation with no rows is a vocabulary of nothing.
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
    // The handle is not a column and neither is the synthetic id: one
    // declared column takes exactly one value.
    assert_eq!(
        closed_currency(vec![row("Usd", vec![Value::U64(2), Value::U64(9)])])
            .validate()
            .unwrap_err(),
        SchemaError::ExtensionArityMismatch {
            relation: RelationId(0),
            row: 0,
            expected: 1,
            supplied: 2
        }
    );
}

#[test]
fn rejects_an_extension_value_type_mismatch() {
    // Field ids are the sealed numbering: the declared column sits at 1,
    // after the synthetic id.
    assert_eq!(
        closed_currency(vec![row("Usd", vec![Value::Bool(true)])])
            .validate()
            .unwrap_err(),
        SchemaError::ExtensionValueTypeMismatch {
            relation: RelationId(0),
            row: 0,
            field: FieldId(1)
        }
    );
}

#[test]
fn rejects_a_ray_axiom() {
    // The ray refusal (`docs/architecture/10-data-model.md`): `[s, ∞)` says the
    // theory's constant is still running, and a still-running span is
    // policy, not an intrinsic property — the write that eventually
    // closes it needs an ordinary relation. Both element domains.
    let of_element = |element, value| SchemaDescriptor {
        relations: vec![closed(
            "Quarter",
            vec![field(
                "span",
                ValueType::Interval {
                    element,
                    width: None,
                },
            )],
            vec![row("Q1", vec![value])],
        )],
        statements: vec![],
    };
    let expected = SchemaError::ExtensionIntervalRay {
        relation: RelationId(0),
        row: 0,
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
    // The handle IS the label; interned columns on a virtual relation
    // would force dictionary writes at open.
    let decl = SchemaDescriptor {
        relations: vec![closed(
            "Currency",
            vec![field("label", ValueType::String)],
            vec![row("Usd", vec![Value::String("dollar".as_bytes().into())])],
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
fn rejects_fresh_on_a_closed_relation() {
    // Identity is the handle; ground axioms are never minted.
    let decl = SchemaDescriptor {
        relations: vec![closed(
            "Currency",
            vec![fresh_field("code")],
            vec![row("Usd", vec![Value::U64(0)])],
        )],
        statements: vec![],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::FreshOnClosedRelation {
            relation: RelationId(0),
            field: FieldId(1)
        }
    );
}

#[test]
fn rejects_a_user_declared_id_on_a_closed_relation() {
    // The synthetic id is validation's own: a declared `id` collides with
    // it — the hand-built-descriptor arm of "the macro never lets the
    // user declare it".
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

// --- Compiled subsets (docs/architecture/30-dependencies.md): the
// closed-containment roster.

/// An interval-typed field on a closed relation, for the refusal tests.
fn closed_window() -> RelationDescriptor {
    closed(
        "Window",
        vec![field(
            "during",
            ValueType::Interval {
                element: IntervalElement::U64,
                width: None,
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
    // A pointwise containment INTO a closed target would mix the coverage
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
                        width: None,
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
        SchemaError::ClosedContainmentInterval {
            statement: StatementId(1),
            relation: RelationId(0)
        }
    );
}

#[test]
fn rejects_an_interval_position_from_a_closed_source() {
    // Coverage FROM a constant source has no delete-time re-judgment path
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
                        width: None,
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
        SchemaError::ClosedContainmentInterval {
            statement: StatementId(2),
            relation: RelationId(0)
        }
    );
}

#[test]
fn rejects_a_closed_target_projection_that_is_not_the_id() {
    // The handle id is the one probe-able identity of a closed relation:
    // a payload-column target matches no key.
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
        SchemaError::NoMatchingTargetKey {
            statement: StatementId(1),
            target: RelationId(0),
            projection: Box::new([FieldId(1)]),
            available: Box::new([target_key(0, &[FieldId(0)])]),
        }
    );
}

#[test]
fn rejects_a_closed_to_closed_containment_the_axioms_refute() {
    // Both sides constant: the judgment is decidable at declaration, and
    // Kind's row 1 (severity 7) escapes Severity's two axioms — a theory
    // whose axioms refute its own statement has no model to commit.
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
        SchemaError::ClosedStatementRefuted {
            statement: StatementId(2),
            relation: RelationId(0),
            row: 1
        }
    );
}

#[test]
fn rejects_a_closed_to_closed_containment_whose_value_exceeds_the_index_range() {
    // A referencing value beyond `u16::MAX` narrows to non-membership —
    // the same miss the commit path takes — so the statement is refuted
    // at validate, never a panic (the F1 lock: the old scan `expect`ed
    // the word to fit the axiom-index width).
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
        SchemaError::ClosedStatementRefuted {
            statement: StatementId(2),
            relation: RelationId(0),
            row: 1
        }
    );
}

#[test]
fn rejects_a_declared_key_the_axioms_refute() {
    // A key on a closed relation is judged at validate — the axioms ARE
    // the final state. Usd and Eur agree on minor_units = 2.
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
        SchemaError::ClosedStatementRefuted {
            statement: StatementId(1),
            relation: RelationId(0),
            row: 1
        }
    );
}

#[test]
fn rejects_a_declared_pointwise_key_the_axioms_refute() {
    // The pointwise arm: two axioms sharing a point collide exactly as
    // the ordered-neighbor probe would judge them at a commit.
    let decl = SchemaDescriptor {
        relations: vec![closed(
            "Window",
            vec![field(
                "during",
                ValueType::Interval {
                    element: IntervalElement::U64,
                    width: None,
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
        SchemaError::ClosedStatementRefuted {
            statement: StatementId(1),
            relation: RelationId(0),
            row: 1
        }
    );
}

// --- The dependency-vocabulary extension's negative corpus: every new
// --- rejection, one pinned typed error each
// --- (`docs/architecture/30-dependencies.md` § validation roster).

/// The extension fixture: Parent(id key) + Task(parent, pos, prio, flag,
/// span) — enough surface for every extension-form rejection.
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
                            width: None,
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
    // A `Many` of zero literals selects nothing — write no statement.
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
        SchemaError::DegenerateSelectionSet {
            statement: StatementId(1),
            relation: RelationId(1),
            field: FieldId(2),
            len: 0,
        }
    );
}

#[test]
fn rejects_a_singleton_spelled_as_a_set() {
    // The one-literal set is the `One` spelling — kept the only singleton
    // by representation, so the equality arm stays zero-cost and
    // unambiguous.
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
        SchemaError::DegenerateSelectionSet {
            statement: StatementId(1),
            relation: RelationId(1),
            field: FieldId(2),
            len: 1,
        }
    );
}

#[test]
fn rejects_a_duplicate_literal_within_a_set() {
    // The set is canonical — sorted, duplicate-free; a repeat is rejected
    // (write it once), never silently collapsed.
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
        SchemaError::DuplicateSelectionLiteral {
            statement: StatementId(1),
            relation: RelationId(1),
            field: FieldId(2),
        }
    );
}

#[test]
fn rejects_a_set_literal_of_the_wrong_type() {
    // Every literal of a set binding type-checks against the selected
    // field — the same shared check as the singleton form.
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
        SchemaError::SelectionLiteralTypeMismatch {
            statement: StatementId(1),
            relation: RelationId(1),
            field: FieldId(2),
        }
    );
}

#[test]
fn rejects_a_window_with_an_interval_position() {
    // The v0 refusal: a window counts FACTS per parent; an interval
    // position would make the count ambiguous between facts and points
    // (`lean/Bumbledb/Cardinality.lean` § v0 refusals; trigger: a sighted
    // counting-over-denotation workload).
    let mut decl = extension_tree();
    // A pointwise key on Task(span) so only the interval refusal fires.
    decl.relations[0].fields.push(field(
        "active",
        ValueType::Interval {
            element: IntervalElement::U64,
            width: None,
        },
    ));
    decl.statements = vec![
        fd(RelationId(0), &[FieldId(0), FieldId(1)]),
        cardinality(
            side(RelationId(1), &[FieldId(0), FieldId(4)]),
            1,
            Some(3),
            side(RelationId(0), &[FieldId(0), FieldId(1)]),
        ),
    ];
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::CardinalityIntervalPosition {
            statement: StatementId(1),
            relation: RelationId(1),
            field: FieldId(4),
        }
    );
}

#[test]
fn rejects_an_inverted_window() {
    // The canonical-utterance law's descriptor face: `hi < lo` is
    // satisfied by no count — unsatisfiable as declared.
    let mut decl = extension_tree();
    decl.statements.push(cardinality(
        side(RelationId(1), &[FieldId(0)]),
        3,
        Some(1),
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::CardinalityInvertedWindow {
            statement: StatementId(1),
            lo: 3,
            hi: 1,
        }
    );
}

#[test]
fn rejects_the_vacuous_window() {
    // `0..*` admits every count — the statement provably says nothing
    // (`lean/Bumbledb/Cardinality.lean: cardinality_zero_star`).
    let mut decl = extension_tree();
    decl.statements.push(cardinality(
        side(RelationId(1), &[FieldId(0)]),
        0,
        None,
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::CardinalityVacuousWindow {
            statement: StatementId(1),
        }
    );
}

#[test]
fn rejects_the_containment_respelled_as_a_window() {
    // `1..*` says exactly what the bare containment says
    // (`lean/Bumbledb/Subsumption.lean: window_floor_containment`) — one
    // meaning, one spelling.
    let mut decl = extension_tree();
    decl.statements.push(cardinality(
        side(RelationId(1), &[FieldId(0)]),
        1,
        None,
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::CardinalityContainmentWindow {
            statement: StatementId(1),
        }
    );
}

#[test]
fn rejects_a_window_whose_target_is_no_key() {
    // Probe-ability, the containment rule reused verbatim: Y must resolve
    // a declared key of B.
    let mut decl = extension_tree();
    decl.statements = vec![cardinality(
        side(RelationId(1), &[FieldId(0)]),
        1,
        Some(3),
        side(RelationId(0), &[FieldId(0)]),
    )];
    assert!(matches!(
        decl.validate().unwrap_err(),
        SchemaError::NoMatchingTargetKey {
            statement: StatementId(0),
            target: RelationId(0),
            ..
        }
    ));
}

#[test]
fn rejects_a_window_arity_mismatch() {
    let mut decl = extension_tree();
    decl.statements.push(cardinality(
        side(RelationId(1), &[FieldId(0), FieldId(2)]),
        1,
        Some(3),
        side(RelationId(0), &[FieldId(0)]),
    ));
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::ContainmentArityMismatch {
            statement: StatementId(1),
            source: 2,
            target: 1,
        }
    );
}

#[test]
fn rejects_a_closed_to_closed_window_the_axioms_refute() {
    // Severity High counts zero Kind axioms against a 1..1 window — a
    // theory whose axioms refute its own statement has no model
    // (`lean/Bumbledb/Schema.lean: den_closed_constant`). The cited row
    // is the parent axiom whose group fails.
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
        statements: vec![cardinality(
            side(RelationId(0), &[FieldId(1)]),
            1,
            Some(1),
            side(RelationId(1), &[FieldId(0)]),
        )],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::ClosedStatementRefuted {
            statement: StatementId(2),
            relation: RelationId(1),
            row: 0,
        }
    );
}

/// Q1's fence at the schema gate: element-domain typing relaxes WIDTHS,
/// never element domains — an `interval<u64, 1>` position against an
/// `interval<i64>` position is still the positional type mismatch (the
/// two domains share no `Point` tag).
#[test]
fn rejects_interval_positions_across_element_domains_whatever_the_widths() {
    let decl = two_relations(
        vec![field(
            "slot",
            ValueType::Interval {
                element: IntervalElement::U64,
                width: Some(1),
            },
        )],
        vec![field(
            "span",
            ValueType::Interval {
                element: IntervalElement::I64,
                width: None,
            },
        )],
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

/// The F1 panic class, CLOSED (2026-07-15): `SchemaDescriptor::validate`
/// refuses the id-width caps typed. The declaration counts here are
/// host-supplied data at the public `Db::create` trust boundary, and
/// the query boundary's own caps are all typed refusals
/// (`ValidationError::TooManyRules` / `TooManyPredicates` /
/// `TooManyAtoms` / `TooManyVariables`) — the schema boundary now
/// matches the engine's typed-refusal law
/// (`lean/Bumbledb/Admission.lean`: acceptance and refusal are a typed
/// gate verdict, never a crash). The caps landed as
/// `SchemaError::TooManyStatements` (the materialized statement roster
/// past 2^16) and `SchemaError::RelationTooManyColumns` (a relation's
/// field-id mint past 2^16), both computed before any u16 id is
/// minted; `validate()`'s `# Panics` contract now names only the
/// unreachable 2^32-relations case.
#[test]
fn the_id_width_caps_refuse_typed_rather_than_panicking() {
    // 2^16 + 1 one-field relations, one Functionality statement each:
    // the statement mint crosses u16.
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

    // One relation with 2^16 + 1 u64 fields: the field-id mint
    // crosses u16.
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
