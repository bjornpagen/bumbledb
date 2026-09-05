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

fn eq_pin(field: u16, value: Const) -> FilterPredicate {
    FilterPredicate::Compare {
        field: OperandAddr::from(FieldId(field)),
        op: WordCmp::Eq,
        value,
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
        OccBind::RecAcc(crate::ir::InteriorId(0)),
    ] {
        let mut occ = occurrence(0, 0, &[(0, X), (1, A), (2, B)]);
        occ.bind = bind;
        let query = normalized(vec![occ], vec![]);
        assert!(provably_distinct(&query, &keyed_schema(&[])).is_none());
    }
}
