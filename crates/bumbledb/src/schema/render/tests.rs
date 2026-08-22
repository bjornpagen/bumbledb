use super::*;
use crate::schema::ContainmentId;
use crate::schema::tests::{containment, fd, field, fresh_field, side, side_where};
use bumbledb_theory::schema::{IntervalElement, LiteralSet, RelationDescriptor};

fn example() -> SchemaDescriptor {
    let savings = Value::U64(1); 
    SchemaDescriptor {
        relations: vec![
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
            RelationDescriptor {
                extension: None,
                name: "Roster".into(),
                fields: vec![field("worker", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Shift".into(),
                fields: vec![
                    field("worker", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![

            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),

            containment(
                side_where(
                    RelationId(1),
                    &[FieldId(0)],
                    vec![(FieldId(2), savings.clone())],
                ),
                side(RelationId(2), &[FieldId(0)]),
            ),
            containment(
                side(RelationId(2), &[FieldId(0)]),
                side_where(RelationId(1), &[FieldId(0)], vec![(FieldId(2), savings)]),
            ),

            fd(RelationId(2), &[FieldId(0)]),

            fd(RelationId(3), &[FieldId(0)]),

            containment(
                side_where(
                    RelationId(4),
                    &[FieldId(0)],
                    vec![(
                        FieldId(1),
                        Value::IntervalU64(
                            bumbledb_theory::Interval::<u64>::new(0, 86_400)
                                .expect("nonempty interval"),
                        ),
                    )],
                ),
                side(RelationId(3), &[FieldId(0)]),
            ),
        ],
    }
}

#[test]
fn goldens_render_the_exact_macro_notation() {
    let schema = example().validate().expect("the example schema is valid");

    assert_eq!(render(&schema, StatementId(0)), "Holder(id) -> Holder");

    assert_eq!(
        render(&schema, StatementId(2)),
        "Account(holder) <= Holder(id)"
    );

    assert_eq!(
        render(&schema, StatementId(5)),
        "SavingsTerms(account) -> SavingsTerms"
    );

    assert_eq!(
        render(&schema, StatementId(7)),
        "Shift(worker | span == 0..86400) <= Roster(worker)"
    );
}

#[test]
fn a_bidirectional_pair_renders_as_double_equals_once_from_either_id() {
    let schema = example().validate().expect("valid");

    let expected = "Account(id | kind == 1) == SavingsTerms(account)";
    assert_eq!(render(&schema, StatementId(3)), expected);
    assert_eq!(render(&schema, StatementId(4)), expected);
    assert_eq!(expected.matches("==").count(), 2, "one selection, one pair");
}

#[test]
fn a_non_adjacent_mirrored_pair_renders_as_double_equals() {

    let declaration = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "P".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Q".into(),
                fields: vec![field("pid", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![field("x", ValueType::U64)],
            },
        ],
        statements: vec![

            fd(RelationId(0), &[FieldId(0)]),

            fd(RelationId(1), &[FieldId(0)]),

            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),

            fd(RelationId(2), &[FieldId(0)]),

            containment(
                side(RelationId(1), &[FieldId(0)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    };
    let schema = declaration.clone().validate().expect("valid");

    assert_eq!(
        schema.containment(ContainmentId(0)).mirror_id(&schema),
        Some(StatementId(4))
    );
    assert_eq!(
        schema.containment(ContainmentId(1)).mirror_id(&schema),
        Some(StatementId(2))
    );

    let expected = "P(id) == Q(pid)";
    assert_eq!(render(&schema, StatementId(2)), expected);
    assert_eq!(render(&schema, StatementId(4)), expected);

    assert_eq!(render_declared(&declaration, StatementId(2)), expected);
    assert_eq!(render_declared(&declaration, StatementId(4)), expected);
}

#[test]
fn a_respelled_literal_set_still_seals_the_mirror_pair() {

    use crate::schema::tests::side_where_sets;
    let declaration = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "A".into(),
                fields: vec![field("x", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "B".into(),
                fields: vec![field("y", ValueType::U64), field("f", ValueType::U64)],
            },
        ],
        statements: vec![

            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),

            containment(
                side(RelationId(0), &[FieldId(0)]),
                side_where_sets(
                    RelationId(1),
                    &[FieldId(0)],
                    vec![(
                        FieldId(1),
                        LiteralSet::Many(Box::new([Value::U64(1), Value::U64(2)])),
                    )],
                ),
            ),

            containment(
                side_where_sets(
                    RelationId(1),
                    &[FieldId(0)],
                    vec![(
                        FieldId(1),
                        LiteralSet::Many(Box::new([Value::U64(2), Value::U64(1)])),
                    )],
                ),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    };
    let schema = declaration.clone().validate().expect("valid");
    assert_eq!(
        schema.containment(ContainmentId(0)).mirror_id(&schema),
        Some(StatementId(3))
    );
    assert_eq!(
        schema.containment(ContainmentId(1)).mirror_id(&schema),
        Some(StatementId(2))
    );

    let expected = "A(x) == B(y | f == {1, 2})";
    assert_eq!(render(&schema, StatementId(2)), expected);
    assert_eq!(render(&schema, StatementId(3)), expected);

    assert!(render_declared(&declaration, StatementId(2)).contains(" == "));
    assert!(render_declared(&declaration, StatementId(3)).contains(" == "));
}

#[test]
fn declared_rendering_matches_sealed_rendering() {
    let declaration = example();
    let schema = declaration.clone().validate().expect("valid");
    for id in 0..u16::try_from(declaration.materialized_statements().len()).expect("small") {
        assert_eq!(
            render_declared(&declaration, StatementId(id)),
            render(&schema, StatementId(id)),
            "statement {id}"
        );
    }
}

#[test]
fn schema_error_diagnostics_render_the_offending_statement() {

    let mut declaration = example();
    declaration.statements.remove(4); // drop `Roster(worker) -> Roster`
    let err = declaration
        .clone()
        .validate()
        .expect_err("no matching target key");
    let rendered = format!("{}", err.display_with(&declaration));
    assert!(
        rendered.contains("Shift(worker | span == 0..86400) <= Roster(worker)"),
        "{rendered}"
    );
    assert!(rendered.starts_with("statement "), "{rendered}");
}

#[test]
fn declaration_scoped_errors_render_without_a_statement_citation() {

    let declaration = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![field("x", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![field("y", ValueType::U64)],
            },
        ],
        statements: vec![],
    };
    let err = declaration
        .clone()
        .validate()
        .expect_err("duplicate relation name");
    let rendered = format!("{}", err.display_with(&declaration));
    assert!(!rendered.contains(" — in `"), "{rendered}");
}

#[test]
fn closed_reference_selections_render_handles() {
    let declaration = |status_word: u64| SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(Box::new([
                    bumbledb_theory::schema::Row {
                        handle: "Open".into(),
                        values: Box::new([]),
                    },
                    bumbledb_theory::schema::Row {
                        handle: "Frozen".into(),
                        values: Box::new([]),
                    },
                ])),
                name: "Status".into(),
                fields: vec![],
            },
            RelationDescriptor {
                extension: None,
                name: "Submission".into(),
                fields: vec![fresh_field("id"), field("status", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "FrozenNote".into(),
                fields: vec![field("submission", ValueType::U64)],
            },
        ],
        statements: vec![

            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),

            containment(
                side(RelationId(2), &[FieldId(0)]),
                side_where(
                    RelationId(1),
                    &[FieldId(0)],
                    vec![(FieldId(1), Value::U64(status_word))],
                ),
            ),
        ],
    };

    let schema = declaration(1).validate().expect("valid");
    assert_eq!(
        render(&schema, StatementId(2)),
        "Submission(status) <= Status(id)"
    );
    assert_eq!(
        render(&schema, StatementId(3)),
        "FrozenNote(submission) <= Submission(id | status == Frozen)"
    );

    assert_eq!(
        render_declared(&declaration(1), StatementId(3)),
        "FrozenNote(submission) <= Submission(id | status == Frozen)"
    );
    assert_eq!(
        render_declared(&declaration(9), StatementId(3)),
        "FrozenNote(submission) <= Submission(id | status == Status(9?))"
    );
}

#[test]
fn unresolvable_names_fall_back_to_id_placeholders() {

    let declaration = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Only".into(),
            fields: vec![field("x", ValueType::U64)],
        }],
        statements: vec![fd(RelationId(9), &[FieldId(3)])],
    };
    assert_eq!(
        render_declared(&declaration, StatementId(0)),
        "relation#9(field#3) -> relation#9"
    );
}

#[test]
fn extension_forms_render_in_the_grammar() {
    use crate::schema::tests::{capacity, side_where_sets};
    use crate::schema::{CapacityId, StatementView};

    let decl = SchemaDescriptor {
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
                    field("state", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Priority".into(),
                fields: vec![field("id", ValueType::U64), field("weight", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(2), &[FieldId(0)]),

            capacity(
                side_where_sets(
                    RelationId(1),
                    &[FieldId(0)],
                    vec![(
                        FieldId(3),
                        LiteralSet::Many(Box::new([Value::U64(2), Value::U64(1)])),
                    )],
                ),
                1,
                Some(3),
                side(RelationId(0), &[FieldId(0)]),
            ),

            capacity(
                side(RelationId(1), &[FieldId(0)]),
                2,
                None,
                side(RelationId(0), &[FieldId(0)]),
            ),

            capacity(
                side(RelationId(1), &[FieldId(0)]),
                0,
                Some(4),
                side(RelationId(0), &[FieldId(0)]),
            ),

            capacity(
                side(RelationId(1), &[FieldId(0)]),
                3,
                Some(3),
                side(RelationId(0), &[FieldId(0)]),
            ),

            capacity(
                side_where_sets(
                    RelationId(1),
                    &[FieldId(0)],
                    vec![(FieldId(3), LiteralSet::One(Value::U64(9)))],
                ),
                0,
                Some(0),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    };

    let expected = [
        "Parent(id) <={1..3} Task(parent | state == {1, 2})",
        "Parent(id) <={2..*} Task(parent)",
        "Parent(id) <={0..4} Task(parent)",
        "Parent(id) <={3} Task(parent)",
        "Parent(id) <={0} Task(parent | state == 9)",
    ];

    assert_eq!(
        render_declared(&decl, StatementId(2)),
        "Parent(id) <={1..3} Task(parent | state == {2, 1})"
    );
    for (offset, want) in expected.iter().enumerate().skip(1) {
        assert_eq!(
            render_declared(
                &decl,
                StatementId(u16::try_from(2 + offset).expect("small"))
            ),
            *want
        );
    }

    let schema = decl.validate().expect("the extension forms validate");
    for (offset, want) in expected.iter().enumerate() {
        let id = StatementId(u16::try_from(2 + offset).expect("small"));
        assert_eq!(render(&schema, id), *want);

        assert!(matches!(
            schema.statement(id),
            StatementView::Capacity(CapacityId(w), _) if usize::from(w) == offset
        ));
    }
}

#[test]
fn weighted_capacity_forms_render_in_the_grammar() {
    use crate::schema::tests::capacity_weighted;
    use crate::schema::{Bound, Weight};

    let interval = ValueType::Interval {
        element: IntervalElement::U64,
    };
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Pool".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("supply", ValueType::U64),
                    field("span", interval),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![
                    field("pool", ValueType::U64),
                    field("watts", ValueType::U64),
                    field("busy", interval),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),

            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::Field(FieldId(1)),
                0,
                Some(Bound::TargetField(FieldId(1))),
                side(RelationId(1), &[FieldId(0)]),
            ),

            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::DurationOf(FieldId(2)),
                0,
                Some(Bound::TargetDuration(FieldId(2))),
                side(RelationId(1), &[FieldId(0)]),
            ),

            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::Field(FieldId(1)),
                1,
                None,
                side(RelationId(1), &[FieldId(0)]),
            ),

            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::DurationOf(FieldId(2)),
                0,
                Some(Bound::Lit(720)),
                side(RelationId(1), &[FieldId(0)]),
            ),

            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::Field(FieldId(1)),
                3,
                Some(Bound::Lit(3)),
                side(RelationId(1), &[FieldId(0)]),
            ),
        ],
    };

    let expected = [
        "Pool(id) <=[watts]{0..supply} Device(pool)",
        "Pool(id) <=[Duration(busy)]{0..Duration(span)} Device(pool)",
        "Pool(id) <=[watts]{1..*} Device(pool)",
        "Pool(id) <=[Duration(busy)]{0..720} Device(pool)",
        "Pool(id) <=[watts]{3} Device(pool)",
    ];

    for (offset, want) in expected.iter().enumerate() {
        let id = StatementId(u16::try_from(1 + offset).expect("small"));
        assert_eq!(render_declared(&decl, id), *want);
    }
    let schema = decl.validate().expect("the weighted forms validate");
    for (offset, want) in expected.iter().enumerate() {
        let id = StatementId(u16::try_from(1 + offset).expect("small"));
        assert_eq!(render(&schema, id), *want);
    }
}
