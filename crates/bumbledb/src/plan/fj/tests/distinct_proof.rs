//! Proof unit suite for the [`provably_distinct`] witness arms (chapter
//! 12 §2's preserved elided-dedup regime): the declared-key arm, the
//! whole-row implicit-key arm, and — as important — everything that must
//! NOT prove (partial covers, non-equality pins, point-membership probes,
//! derived occurrences).
use super::*;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::WordCmp;
use bumbledb_theory::schema::{IntervalElement, StatementDescriptor};

/// One arity-3 relation with the given key statements (each a field-id
/// projection over relation 0) — the tests.rs `schema` helper declares no
/// statements, and these tests are about statement-vs-implicit coverage.
fn keyed_schema(keys: &[&[u16]]) -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: (0..3)
                .map(|f| FieldDescriptor {
                    name: format!("f{f}").into(),
                    value_type: ValueType::U64,
                })
                .collect(),
        }],
        statements: keys
            .iter()
            .map(|projection| StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: projection.iter().map(|f| FieldId(*f)).collect(),
            })
            .collect(),
    }
    .validate()
    .expect("valid fixture")
}

fn interval_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "P".into(),
            fields: vec![
                FieldDescriptor {
                    name: "emp".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "shift".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn pointwise_schema(element: IntervalElement) -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Booking".into(),
            fields: vec![
                FieldDescriptor {
                    name: "room".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "payload".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval { element },
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0), FieldId(2)]),
        }],
    }
    .validate()
    .expect("pointwise key")
}

fn eq_pin(field: u16, value: Const) -> FilterPredicate {
    FilterPredicate::Compare {
        field: OperandAddr::from(FieldId(field)),
        op: WordCmp::Eq,
        value,
    }
}

#[test]
fn projection_dependencies_close_over_hidden_keys_independently_of_atom_order() {
    let schema = keyed_schema(&[&[0]]);
    let mut chain = vec![
        occurrence(0, 0, &[(0, X), (1, A)]),
        occurrence(1, 0, &[(0, A), (1, B)]),
        occurrence(2, 0, &[(0, B), (1, VarId(9))]),
    ];
    for _ in 0..2 {
        assert!(
            provably_distinct_projection(
                &normalized(chain.clone(), vec![]),
                &schema,
                &[crate::ir::FindTerm::Var(X)],
            )
            .is_some()
        );
        chain.reverse();
    }
    // No seed: knowing a downstream payload does not determine its key.
    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, X), (1, A)]),
            occurrence(1, 0, &[(0, A), (1, B)]),
        ],
        vec![],
    );
    assert!(
        provably_distinct_projection(&query, &schema, &[crate::ir::FindTerm::Var(B)],).is_none()
    );
}

#[test]
fn projection_dependencies_never_cross_nonkeys_or_negative_guards() {
    let schema = keyed_schema(&[&[0]]);
    let first = occurrence(0, 0, &[(0, X), (1, A)]);
    let downstream_nonkey = occurrence(1, 0, &[(0, B), (1, A)]);
    assert!(
        provably_distinct_projection(
            &normalized(vec![first.clone(), downstream_nonkey], vec![]),
            &schema,
            &[crate::ir::FindTerm::Var(X)],
        )
        .is_none()
    );
    // A negative match never exports a row or determines its other fields.
    assert!(
        provably_distinct_projection(
            &normalized(
                vec![
                    first,
                    negated(1, 0, &[(0, A), (1, B)]),
                    occurrence(2, 0, &[(0, B)]),
                ],
                vec![]
            ),
            &schema,
            &[crate::ir::FindTerm::Var(X)],
        )
        .is_none()
    );
}

#[test]
fn a_determined_interval_row_does_not_determine_membership_points() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Span".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    },
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "Point".into(),
                fields: vec![FieldDescriptor {
                    name: "at".into(),
                    value_type: ValueType::I64,
                }],
                extension: None,
            },
        ],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    }
    .validate()
    .unwrap();
    let mut span = occurrence(0, 0, &[(0, X)]);
    span.point_vars.push((FieldId(1), A, false));
    let point = occurrence(1, 1, &[(0, A)]);
    assert!(
        provably_distinct_projection(
            &normalized(vec![span, point], vec![]),
            &schema,
            &[crate::ir::FindTerm::Var(X)],
        )
        .is_none()
    );
}

#[test]
fn output_key_coverage_is_stronger_than_full_binding_distinctness() {
    let query = normalized(vec![occurrence(0, 0, &[(0, X), (1, A), (2, B)])], vec![]);
    let schema = keyed_schema(&[&[0]]);
    let head = |vars: &[VarId]| {
        vars.iter()
            .copied()
            .map(crate::ir::FindTerm::Var)
            .collect::<Vec<_>>()
    };
    // Posting(id, amount, at): projecting id+amount is unique even with
    // unprojected at. Projecting amount alone can repeat across distinct ids.
    assert!(provably_distinct(&query, &schema).is_some());
    assert!(provably_distinct_projection(&query, &schema, &head(&[X, A])).is_some());
    assert!(provably_distinct_projection(&query, &schema, &head(&[A])).is_none());
    assert!(provably_distinct_projection(&query, &schema, &head(&[X, X])).is_some());
    let unkeyed = keyed_schema(&[]);
    assert!(provably_distinct_projection(&query, &unkeyed, &head(&[X, A])).is_none());
    assert!(provably_distinct_projection(&query, &unkeyed, &head(&[X, A, B])).is_some());
    assert!(provably_distinct_projection(&query, &schema, &[]).is_none());
    assert!(provably_distinct_projection(&query, &schema, &[crate::ir::FindTerm::Count]).is_none());
}

#[test]
fn projected_composite_key_accepts_only_equality_single_value_pins() {
    let schema = keyed_schema(&[&[0, 1]]);
    for value in [Const::Word(7), Const::Param(crate::ir::ParamId(0))] {
        let mut occ = occurrence(0, 0, &[(0, X), (2, B)]);
        occ.filters.push(eq_pin(1, value));
        assert!(
            provably_distinct_projection(
                &normalized(vec![occ], vec![]),
                &schema,
                &[crate::ir::FindTerm::Var(X)],
            )
            .is_some()
        );
    }
    for (op, value) in [
        (WordCmp::Lt, Const::Word(7)),
        (WordCmp::Eq, Const::WordSet(vec![7, 8].into())),
        (WordCmp::Eq, Const::ParamSet(crate::ir::ParamId(0))),
    ] {
        let mut occ = occurrence(0, 0, &[(0, X), (2, B)]);
        occ.filters.push(FilterPredicate::Compare {
            field: FieldId(1).into(),
            op,
            value,
        });
        assert!(
            provably_distinct_projection(
                &normalized(vec![occ], vec![]),
                &schema,
                &[crate::ir::FindTerm::Var(X)],
            )
            .is_none()
        );
    }
}

#[test]
fn projection_proof_covers_each_positive_occurrence_but_not_negative_guards() {
    let schema = keyed_schema(&[&[0]]);
    let first = occurrence(0, 0, &[(0, X), (1, A)]);
    // A second row's hidden key can multiply a projected first key.
    let multiplying = occurrence(1, 0, &[(0, B), (1, A)]);
    let query = normalized(vec![first.clone(), multiplying], vec![]);
    assert!(provably_distinct(&query, &schema).is_some());
    assert!(
        provably_distinct_projection(&query, &schema, &[crate::ir::FindTerm::Var(X)]).is_none()
    );
    let unique_lookup = occurrence(1, 0, &[(0, X), (1, A)]);
    assert!(
        provably_distinct_projection(
            &normalized(vec![first.clone(), unique_lookup], vec![]),
            &schema,
            &[crate::ir::FindTerm::Var(X)],
        )
        .is_some()
    );
    assert!(
        provably_distinct_projection(
            &normalized(vec![first, negated(1, 0, &[(1, A)])], vec![]),
            &schema,
            &[crate::ir::FindTerm::Var(X)],
        )
        .is_some()
    );
}

#[test]
fn projection_proof_never_confuses_a_point_with_a_complete_interval_key() {
    let schema = pointwise_schema(IntervalElement::F64);
    let complete = occurrence(0, 0, &[(0, X), (1, A), (2, B)]);
    assert!(
        provably_distinct_projection(
            &normalized(vec![complete.clone()], vec![]),
            &schema,
            &[crate::ir::FindTerm::Var(X), crate::ir::FindTerm::Var(B)],
        )
        .is_some()
    );
    assert!(
        provably_distinct_projection(
            &normalized(vec![complete], vec![]),
            &schema,
            &[crate::ir::FindTerm::Var(X)],
        )
        .is_none()
    );
    let mut point = occurrence(0, 0, &[(0, X), (1, A)]);
    point.point_vars.push((FieldId(2), B, true));
    assert!(
        provably_distinct_projection(
            &normalized(vec![point], vec![]),
            &schema,
            &[crate::ir::FindTerm::Var(X), crate::ir::FindTerm::Var(B)],
        )
        .is_none()
    );
}

#[test]
fn projection_proof_rejects_positive_derived_occurrences() {
    let schema = keyed_schema(&[]);
    for bind in [
        OccBind::Finished(crate::ir::InteriorId(0)),
        OccBind::RecDelta(crate::ir::InteriorId(0)),
    ] {
        let mut occ = occurrence(0, 0, &[(0, X), (1, A), (2, B)]);
        occ.bind = bind;
        assert!(
            provably_distinct_projection(
                &normalized(vec![occ], vec![]),
                &schema,
                &[
                    crate::ir::FindTerm::Var(X),
                    crate::ir::FindTerm::Var(A),
                    crate::ir::FindTerm::Var(B)
                ],
            )
            .is_none()
        );
    }
}

#[test]
fn whole_row_cover_proves_without_declared_keys() {
    // Set semantics: binding every field of a stored relation determines
    // the fact, so the full field set is an implicit key.
    let query = normalized(vec![occurrence(0, 0, &[(0, X), (1, A), (2, B)])], vec![]);
    assert!(provably_distinct(&query, &keyed_schema(&[])).is_some());
}

#[test]
fn partial_cover_without_a_key_does_not_prove() {
    // Two distinct facts can agree on any strict field subset when no key
    // covers it — eliding the seen-set here would double-fold.
    let query = normalized(vec![occurrence(0, 0, &[(0, X), (1, A)])], vec![]);
    assert!(provably_distinct(&query, &keyed_schema(&[])).is_none());
}

#[test]
fn an_equality_pinned_field_completes_the_whole_row_cover() {
    // A field pinned by an equality filter is fixed per execution: vars on
    // f0/f1 plus `f2 = const` still determine the whole row.
    for pin in [
        Const::Word(7),
        Const::Param(crate::ir::ParamId(0)),
        Const::Byte(3),
    ] {
        let mut occ = occurrence(0, 0, &[(0, X), (1, A)]);
        occ.filters.push(eq_pin(2, pin));
        let query = normalized(vec![occ], vec![]);
        assert!(provably_distinct(&query, &keyed_schema(&[])).is_some());
    }
}

#[test]
fn a_non_equality_pin_does_not_bind_its_field() {
    // `f2 < const` admits many f2 values per binding: no cover, no proof.
    for op in [WordCmp::Lt, WordCmp::Ne, WordCmp::Ge] {
        let mut occ = occurrence(0, 0, &[(0, X), (1, A)]);
        occ.filters.push(FilterPredicate::Compare {
            field: OperandAddr::from(FieldId(2)),
            op,
            value: Const::Word(7),
        });
        let query = normalized(vec![occ], vec![]);
        assert!(provably_distinct(&query, &keyed_schema(&[])).is_none());
    }
}

#[test]
fn a_point_membership_probe_does_not_bind_its_interval_field() {
    // A point inside the interval does not determine the interval: two
    // distinct facts can both contain the probe point.
    let schema = interval_schema();
    let mut occ = occurrence(0, 0, &[(0, X), (1, A)]);
    occ.point_vars.push((FieldId(2), Y, false));
    let query = normalized(vec![occ], vec![]);
    assert!(provably_distinct(&query, &schema).is_none());

    // Binding the interval field itself (value equality) does cover it.
    let query = normalized(vec![occurrence(0, 0, &[(0, X), (1, A), (2, B)])], vec![]);
    assert!(provably_distinct(&query, &schema).is_some());
}

#[test]
fn pointwise_keys_require_the_complete_interval_and_scalar_group() {
    use crate::image::view::ViewWordSource;

    for element in [
        IntervalElement::U64,
        IntervalElement::I64,
        IntervalElement::F64,
    ] {
        let schema = pointwise_schema(element);
        let mut covered = normalized(vec![occurrence(0, 0, &[(0, X), (2, B)])], vec![]);
        covered.slot_widths.insert(B, SlotWidth::TWO);
        assert!(
            provably_distinct(&covered, &schema).is_some(),
            "payload need not be bound"
        );
        for bindings in [&[(0, X)][..], &[(2, B)][..]] {
            assert!(
                provably_distinct(
                    &normalized(vec![occurrence(0, 0, bindings)], vec![]),
                    &schema
                )
                .is_none()
            );
        }
        // Membership probes do not supply complete interval equality, even
        // though a pointwise key exists. This change licenses no point probe.
        let mut point_var = occurrence(0, 0, &[(0, X)]);
        point_var
            .point_vars
            .push((FieldId(2), Y, element == IntervalElement::F64));
        assert!(provably_distinct(&normalized(vec![point_var], vec![]), &schema).is_none());
        for point in [
            ViewWordSource::Word(1),
            ViewWordSource::Param(crate::ir::ParamId(0)),
        ] {
            let mut point_filter = occurrence(0, 0, &[(0, X)]);
            point_filter.filters.push(FilterPredicate::PointIn {
                field: FieldId(2).into(),
                point,
                dense: element == IntervalElement::F64,
            });
            assert!(provably_distinct(&normalized(vec![point_filter], vec![]), &schema).is_none());
        }
    }
}

#[test]
fn complete_interval_equality_pins_license_the_pointwise_witness() {
    let schema = pointwise_schema(IntervalElement::U64);
    for value in [
        Const::Interval { start: 1, end: 2 },
        Const::Param(crate::ir::ParamId(0)),
    ] {
        let mut occ = occurrence(0, 0, &[(0, X)]);
        occ.filters.push(eq_pin(2, value));
        assert!(provably_distinct(&normalized(vec![occ], vec![]), &schema).is_some());
    }
    let mut non_equality = occurrence(0, 0, &[(0, X)]);
    non_equality.filters.push(FilterPredicate::Compare {
        field: FieldId(2).into(),
        op: WordCmp::Lt,
        value: Const::Interval { start: 1, end: 2 },
    });
    assert!(provably_distinct(&normalized(vec![non_equality], vec![]), &schema).is_none());
}

#[test]
fn pointwise_law_rejects_distinct_rows_with_equal_complete_keys() {
    use crate::schema::judge::{JudgeBudget, Judgment, MapState, judge_final_state};
    use crate::{F64, Interval, Value};

    for (element, first, adjacent) in [
        (
            IntervalElement::U64,
            Value::IntervalU64(Interval::<u64>::new(1, 2).unwrap()),
            Value::IntervalU64(Interval::<u64>::new(2, 3).unwrap()),
        ),
        (
            IntervalElement::I64,
            Value::IntervalI64(Interval::<i64>::new(1, 2).unwrap()),
            Value::IntervalI64(Interval::<i64>::new(2, 3).unwrap()),
        ),
        (
            IntervalElement::F64,
            Value::IntervalF64(Interval::<F64>::new(F64::from(1.0), F64::from(2.0)).unwrap()),
            Value::IntervalF64(Interval::<F64>::new(F64::from(2.0), F64::from(3.0)).unwrap()),
        ),
    ] {
        let schema = pointwise_schema(element);
        let work = crate::api::db::test_operation();
        let mut equal_keys = MapState::new();
        equal_keys.insert(
            RelationId(0),
            vec![Value::U64(1), Value::U64(10), first.clone()],
        );
        equal_keys.insert(
            RelationId(0),
            vec![Value::U64(1), Value::U64(20), first.clone()],
        );
        assert!(matches!(
            judge_final_state(&schema, &equal_keys, &work, JudgeBudget::default()).expect("judge"),
            Judgment::Rejected(_)
        ));
        let mut lawful = MapState::new();
        let first_row = vec![Value::U64(1), Value::U64(10), first.clone()];
        lawful.insert(RelationId(0), first_row.clone());
        lawful.insert(RelationId(0), first_row); // Exact duplicates remain one set element.
        lawful.insert(RelationId(0), vec![Value::U64(1), Value::U64(20), adjacent]);
        lawful.insert(RelationId(0), vec![Value::U64(2), Value::U64(30), first]);
        assert_eq!(
            judge_final_state(&schema, &lawful, &work, JudgeBudget::default()).expect("judge"),
            Judgment::Admitted
        );
    }
}

#[test]
fn declared_key_cover_still_proves_a_partial_row() {
    // The pre-existing arm: bound fields ⊇ a declared key's projection.
    let schema = keyed_schema(&[&[0]]);
    let query = normalized(vec![occurrence(0, 0, &[(0, X)])], vec![]);
    assert!(provably_distinct(&query, &schema).is_some());

    // A different partial cover that misses every key stays unproven.
    let query = normalized(vec![occurrence(0, 0, &[(1, A)])], vec![]);
    assert!(provably_distinct(&query, &schema).is_none());

    // Composite key: both fields required, one is not enough.
    let schema = keyed_schema(&[&[0, 1]]);
    let query = normalized(vec![occurrence(0, 0, &[(0, X), (1, A)])], vec![]);
    assert!(provably_distinct(&query, &schema).is_some());
    let query = normalized(vec![occurrence(0, 0, &[(0, X)])], vec![]);
    assert!(provably_distinct(&query, &schema).is_none());
}

#[test]
fn every_participating_occurrence_must_be_covered() {
    // One fully covered occurrence does not license the join: the
    // uncovered occurrence can multiply bindings.
    let schema = schema(2, 3);
    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, X), (1, A), (2, B)]),
            occurrence(1, 1, &[(0, X), (1, C)]),
        ],
        vec![],
    );
    assert!(provably_distinct(&query, &schema).is_none());

    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, X), (1, A), (2, B)]),
            occurrence(1, 1, &[(0, X), (1, C), (2, Y)]),
        ],
        vec![],
    );
    assert!(provably_distinct(&query, &schema).is_some());
}

#[test]
fn negated_occurrences_need_no_cover() {
    // Negation filters bindings (0/1 per binding), never multiplies them:
    // only participating (positive) occurrences carry the obligation.
    let schema = schema(2, 3);
    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, X), (1, A), (2, B)]),
            negated(1, 1, &[(0, X)]),
        ],
        vec![],
    );
    assert!(provably_distinct(&query, &schema).is_some());
}

#[test]
fn derived_occurrences_are_never_proven() {
    // Interior/rec outputs are not schema relations; neither arm applies
    // (no declared keys, no schema field roster to cover).
    for bind in [
        OccBind::Finished(crate::ir::InteriorId(0)),
        OccBind::RecDelta(crate::ir::InteriorId(0)),
    ] {
        let mut occ = occurrence(0, 0, &[(0, X), (1, A), (2, B)]);
        occ.bind = bind;
        let query = normalized(vec![occ], vec![]);
        assert!(provably_distinct(&query, &keyed_schema(&[])).is_none());
    }
}
