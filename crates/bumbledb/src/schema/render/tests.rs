//! Render goldens: the exact macro notation back out — an FD, a one-way
//! containment with selection, a bidirectional pair rendering `==` once
//! from either id, and an interval selection literal.

use super::*;
use crate::schema::ContainmentId;
use crate::schema::ValidateDescriptor as _;
use crate::schema::tests::{containment, fd, field, fresh_field, side, side_where};
use bumbledb_theory::schema::{IntervalElement, LiteralSet, RelationDescriptor};

/// The `docs/architecture/30-dependencies.md` example schema plus an
/// interval-selected containment (Shift/Roster). Materialized ids: 0/1
/// the fresh auto-FDs, 2.. the declared statements below in order.
fn example() -> SchemaDescriptor {
    let savings = Value::U64(1); // kind 1 = Savings
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
                            width: None,
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
                            width: None,
                        },
                    ),
                ],
            },
        ],
        statements: vec![
            // id 2: Account(holder) <= Holder(id)
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
            // ids 3 and 4: Account(id | kind == 1) == SavingsTerms(account)
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
            // id 5: SavingsTerms(account) -> SavingsTerms
            fd(RelationId(2), &[FieldId(0)]),
            // id 6: Roster(worker) -> Roster
            fd(RelationId(3), &[FieldId(0)]),
            // id 7: Shift(worker | span == 0..86400) <= Roster(worker)
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
    // A fresh auto-FD renders like any declared FD.
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
    // once, the selection literal in its macro spelling.
    let expected = "Account(id | kind == 1) == SavingsTerms(account)";
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
            // id 0: P(id) -> P
            fd(RelationId(0), &[FieldId(0)]),
            // id 1: Q(pid) -> Q
            fd(RelationId(1), &[FieldId(0)]),
            // id 2: the pair's first half.
            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
            // id 3: an unrelated statement between the halves.
            fd(RelationId(2), &[FieldId(0)]),
            // id 4: the pair's second half — exactly id 2's sides swapped.
            containment(
                side(RelationId(1), &[FieldId(0)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    };
    let schema = declaration.clone().validate().expect("valid");
    // The links seal symmetric across the gap.
    assert_eq!(
        schema.containment(ContainmentId(0)).mirror,
        Some(StatementId(4))
    );
    assert_eq!(
        schema.containment(ContainmentId(1)).mirror,
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
fn a_respelled_literal_set_still_seals_the_mirror_pair() {
    // Mirror pairing compares the NORMALIZED statement identity, never
    // the raw spelling: the two halves spell one literal set in opposite
    // orders, and the sides seal canonical — exact swapped mirrors — so
    // the links must seal (raw side equality rendered this pair as two
    // `<=` lines while a canonically-spelled twin with the SAME
    // fingerprint rendered `==`).
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
            // id 0: A(x) -> A; id 1: B(y) -> B — both targets probe-able.
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            // id 2: A(x) <= B(y | f == {1, 2}).
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
            // id 3: B(y | f == {2, 1}) <= A(x) — id 2's mirror, the set
            // respelled.
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
        schema.containment(ContainmentId(0)).mirror,
        Some(StatementId(3))
    );
    assert_eq!(
        schema.containment(ContainmentId(1)).mirror,
        Some(StatementId(2))
    );
    // Both halves render the pair once, in the lower id's orientation —
    // the sealed sides carry the canonical set.
    let expected = "A(x) == B(y | f == {1, 2})";
    assert_eq!(render(&schema, StatementId(2)), expected);
    assert_eq!(render(&schema, StatementId(3)), expected);
    // The declared (diagnostic) path pairs over the same identity.
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

/// A selection word at a closed-reference position renders its handle —
/// the macro's own bare-handle spelling back out — on the source's
/// referencing field and the closed relation's id alike; an out-of-range
/// word renders visibly wrong as `Status(9?)` (the `ir/render` fallback).
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
            // Submission(status) <= Status(id) — the closed reference.
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
            // FrozenNote(submission) <= Submission(id | status == <word>).
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
    // Materialized ids: 0 the fresh auto-FD, 1 the closed auto-key,
    // 2..3 the declared containments above.
    let schema = declaration(1).validate().expect("valid");
    assert_eq!(
        render(&schema, StatementId(2)),
        "Submission(status) <= Status(id)"
    );
    assert_eq!(
        render(&schema, StatementId(3)),
        "FrozenNote(submission) <= Submission(id | status == Frozen)"
    );
    // The declared (diagnostic) path agrees, and an out-of-range word —
    // no tenth row exists — keeps the number with the `?` that marks it
    // wrong, under the relation's name (the engine never learns host
    // newtype names).
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
    // A statement naming a relation outside the declaration renders with
    // placeholders instead of panicking — that unknown id IS the error
    // being diagnosed.
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

/// The extension forms render back in the exact grammar — B-family,
/// target-left, and every legal window spelling round-trips canonically
/// (the canonical-utterance law, `docs/architecture/70-api.md`): the
/// `{lo..hi}` range, the `{lo..*}` floor (lo ≥ 2), the `{0..hi}`
/// ceiling, the `{n}` exact, and the `{0}` exclusion — plus the set
/// selection in braces (canonical order). Validation refuses the banned
/// bound shapes (`{0..*}`, `{1..*}`, inverted — the reject suite), so
/// no sealed statement can render a banned spelling.
#[test]
fn extension_forms_render_in_the_grammar() {
    use crate::schema::tests::{cardinality, side_where_sets};
    use crate::schema::{StatementView, WindowId};

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
            // `{1..3}` — the two-bound range, over a set selection.
            cardinality(
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
            // `{2..*}` — the floor (lo ≥ 2; `{1..*}` is the banned
            // containment respelling).
            cardinality(
                side(RelationId(1), &[FieldId(0)]),
                2,
                None,
                side(RelationId(0), &[FieldId(0)]),
            ),
            // `{0..4}` — the ceiling.
            cardinality(
                side(RelationId(1), &[FieldId(0)]),
                0,
                Some(4),
                side(RelationId(0), &[FieldId(0)]),
            ),
            // `{3}` — the exact count (`lo = hi`).
            cardinality(
                side(RelationId(1), &[FieldId(0)]),
                3,
                Some(3),
                side(RelationId(0), &[FieldId(0)]),
            ),
            // `{0}` — the exclusion.
            cardinality(
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

    // Declared-side rendering (the diagnostic path) — the set as
    // written.
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

    // Sealed-side rendering — the set now canonical (sorted), every
    // survivor in its one spelling.
    let schema = decl.validate().expect("the extension forms validate");
    for (offset, want) in expected.iter().enumerate() {
        let id = StatementId(u16::try_from(2 + offset).expect("small"));
        assert_eq!(render(&schema, id), *want);
        // The spine agrees with the arenas.
        assert!(matches!(
            schema.statement(id),
            StatementView::Cardinality(WindowId(w), _) if usize::from(w) == offset
        ));
    }
}
