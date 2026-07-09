//! Render goldens (PRD 20): the exact macro notation back out — an FD, a
//! one-way containment with selection, a bidirectional pair rendering
//! `==` once from either id, an interval selection literal, and enum
//! ordinals resolved to variant names.

use super::*;
use crate::schema::{
    FieldDescriptor, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor,
};

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn serial_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Serial,
    }
}

fn side_of(relation: u32, projection: &[u16]) -> Side {
    Side {
        relation: RelationId(relation),
        projection: projection.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::new([]),
    }
}

fn side_where(relation: u32, projection: &[u16], selection: Vec<(u16, Value)>) -> Side {
    Side {
        selection: selection
            .into_iter()
            .map(|(f, value)| (FieldId(f), value))
            .collect(),
        ..side_of(relation, projection)
    }
}

/// The `docs/architecture/30-dependencies.md` example schema plus an
/// interval-selected containment (Shift/Roster). Materialized ids: 0/1
/// the serial auto-FDs, 2.. the declared statements below in order.
fn example() -> SchemaDescriptor {
    let savings = Value::Enum(1); // ["Checking", "Savings"]
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![serial_field("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    serial_field("id"),
                    field("holder", ValueType::U64),
                    field(
                        "kind",
                        ValueType::Enum {
                            variants: ["Checking", "Savings"]
                                .iter()
                                .map(|v| Box::from(*v))
                                .collect(),
                        },
                    ),
                    field(
                        "active",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                name: "SavingsTerms".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field("rate_bps", ValueType::I64),
                ],
            },
            RelationDescriptor {
                name: "Roster".into(),
                fields: vec![field("worker", ValueType::U64)],
            },
            RelationDescriptor {
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
            // id 2: Account(holder) <= Holder(id)
            StatementDescriptor::Containment {
                source: side_of(1, &[1]),
                target: side_of(0, &[0]),
            },
            // ids 3 and 4: Account(id | kind == Savings) == SavingsTerms(account)
            StatementDescriptor::Containment {
                source: side_where(1, &[0], vec![(2, savings.clone())]),
                target: side_of(2, &[0]),
            },
            StatementDescriptor::Containment {
                source: side_of(2, &[0]),
                target: side_where(1, &[0], vec![(2, savings)]),
            },
            // id 5: SavingsTerms(account) -> SavingsTerms
            StatementDescriptor::Functionality {
                relation: RelationId(2),
                projection: Box::new([FieldId(0)]),
            },
            // id 6: Roster(worker) -> Roster
            StatementDescriptor::Functionality {
                relation: RelationId(3),
                projection: Box::new([FieldId(0)]),
            },
            // id 7: Shift(worker | span == 0..86400) <= Roster(worker)
            StatementDescriptor::Containment {
                source: side_where(4, &[0], vec![(1, Value::IntervalU64(0, 86_400))]),
                target: side_of(3, &[0]),
            },
        ],
    }
}

#[test]
fn goldens_render_the_exact_macro_notation() {
    let schema = example().validate().expect("the example schema is valid");
    // A serial auto-FD renders like any declared FD.
    assert_eq!(render(&schema, StatementId(0)), "Holder(id) -> Holder");
    // A one-way containment.
    assert_eq!(
        render(&schema, StatementId(2)),
        "Account(holder) <= Holder(id)"
    );
    // An FD, key form.
    assert_eq!(
        render(&schema, StatementId(5)),
        "SavingsTerms(account) -> SavingsTerms"
    );
    // A one-way containment with a selection whose literal is an
    // interval: the macro form `start..end`.
    assert_eq!(
        render(&schema, StatementId(7)),
        "Shift(worker | span == 0..86400) <= Roster(worker)"
    );
}

#[test]
fn a_bidirectional_pair_renders_as_double_equals_once_from_either_id() {
    let schema = example().validate().expect("valid");
    // Both lowered ids render the pair's one written form — `==` exactly
    // once, the enum ordinal resolved to its variant name.
    let expected = "Account(id | kind == Savings) == SavingsTerms(account)";
    assert_eq!(render(&schema, StatementId(3)), expected);
    assert_eq!(render(&schema, StatementId(4)), expected);
    assert_eq!(expected.matches("==").count(), 2, "one selection, one pair");
}

#[test]
fn a_non_adjacent_mirrored_pair_renders_as_double_equals() {
    // The closed gap, pinned: the pairing is a sealed fact computed over
    // *all* statements, so a hand-built descriptor separating the lowered
    // pair with an unrelated statement still renders `==` once from
    // either id (adjacency-based detection rendered it as two `<=`
    // lines).
    let declaration = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "P".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Q".into(),
                fields: vec![field("pid", ValueType::U64)],
            },
            RelationDescriptor {
                name: "R".into(),
                fields: vec![field("x", ValueType::U64)],
            },
        ],
        statements: vec![
            // id 0: P(id) -> P
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            },
            // id 1: Q(pid) -> Q
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            // id 2: the pair's first half.
            StatementDescriptor::Containment {
                source: side_of(0, &[0]),
                target: side_of(1, &[0]),
            },
            // id 3: an unrelated statement between the halves.
            StatementDescriptor::Functionality {
                relation: RelationId(2),
                projection: Box::new([FieldId(0)]),
            },
            // id 4: the pair's second half — exactly id 2's sides swapped.
            StatementDescriptor::Containment {
                source: side_of(1, &[0]),
                target: side_of(0, &[0]),
            },
        ],
    };
    let schema = declaration.clone().validate().expect("valid");
    // The links seal symmetric across the gap.
    assert_eq!(
        schema.statement(StatementId(2)).mirror,
        Some(StatementId(4))
    );
    assert_eq!(
        schema.statement(StatementId(4)).mirror,
        Some(StatementId(2))
    );
    // Both halves render the pair once, in the lower id's orientation.
    let expected = "P(id) == Q(pid)";
    assert_eq!(render(&schema, StatementId(2)), expected);
    assert_eq!(render(&schema, StatementId(4)), expected);
    // The declared (diagnostic) path agrees.
    assert_eq!(render_declared(&declaration, StatementId(2)), expected);
    assert_eq!(render_declared(&declaration, StatementId(4)), expected);
}

#[test]
fn declared_rendering_matches_sealed_rendering() {
    let declaration = example();
    let schema = declaration.clone().validate().expect("valid");
    for id in 0..u16::try_from(schema.statements().len()).expect("small") {
        assert_eq!(
            render_declared(&declaration, StatementId(id)),
            render(&schema, StatementId(id)),
            "statement {id}"
        );
    }
}

#[test]
fn schema_error_diagnostics_render_the_offending_statement() {
    // Reject: the containment's target projection matches no key of
    // Roster (no FD declared) — the diagnostic renders the statement.
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
}

#[test]
fn unresolvable_names_fall_back_to_id_placeholders() {
    // A statement naming a relation outside the declaration renders with
    // placeholders instead of panicking — that unknown id IS the error
    // being diagnosed.
    let declaration = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Only".into(),
            fields: vec![field("x", ValueType::U64)],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(9),
            projection: Box::new([FieldId(3)]),
        }],
    };
    assert_eq!(
        render_declared(&declaration, StatementId(0)),
        "relation#9(field#3) -> relation#9"
    );
}
