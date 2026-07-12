use super::super::{normalize, OccId, Role};
use super::*;
use crate::encoding::encode_i64;
use crate::ir::validate::validate;
use crate::ir::{
    Atom, Comparison, FindTerm, MaskTerm, ParamId, PredicateTree, Query, Rule, Term, VarId,
};
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
};
use crate::storage::dict::SENTINEL_ID;

/// R(id u64 fresh, a i64, k u64) + P(emp u64, during interval<i64>,
/// review interval<i64>, at i64) — the normalize fixture family, trimmed
/// to what the fold reads.
fn schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
    };
    let interval_i64 = ValueType::Interval {
        element: IntervalElement::I64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    field("a", ValueType::I64),
                    field("k", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "P".into(),
                fields: vec![
                    field("emp", ValueType::U64),
                    field("during", interval_i64.clone()),
                    field("review", interval_i64),
                    field("at", ValueType::I64),
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);
const R_A: FieldId = FieldId(1);

/// The biased I64 column word.
fn w(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

fn summary(bounds: &[(CmpOp, u64)]) -> RangeSummary {
    let mut summary = RangeSummary::new();
    for (op, word) in bounds {
        summary.narrow(*op, *word);
    }
    summary
}

// Rule (a) — the empty range summary.

#[test]
fn an_empty_range_summary_is_statically_empty() {
    assert!(range_is_empty(&summary(&[(CmpOp::Gt, 5), (CmpOp::Lt, 3)])));
    // The domain edges: strictly-above-MAX and strictly-below-0 admit
    // no word at all.
    assert!(range_is_empty(&summary(&[(CmpOp::Gt, u64::MAX)])));
    assert!(range_is_empty(&summary(&[(CmpOp::Lt, 0)])));
}

#[test]
fn a_single_point_range_survives() {
    // The near miss: `x > 5 ∧ x <= 6` admits exactly the word 6.
    let summary = summary(&[(CmpOp::Gt, 5), (CmpOp::Le, 6)]);
    assert!(!range_is_empty(&summary));
    assert_eq!((summary.lo, summary.hi), (6, 6));
}

// Rule (b) — Eq to two distinct constants on one slot.

#[test]
fn two_distinct_eq_constants_are_statically_empty() {
    assert!(eq_conflicts(&Const::Word(3), &Const::Word(5)));
    assert!(eq_conflicts(
        &Const::Interval { start: 2, end: 5 },
        &Const::Interval { start: 2, end: 6 },
    ));
    // Distinct pending `str` literals are distinct values — the
    // dictionary is injective, so the byte comparison is the value
    // comparison.
    assert!(eq_conflicts(
        &Const::PendingIntern {
            bytes: Box::from(&b"a"[..])
        },
        &Const::PendingIntern {
            bytes: Box::from(&b"b"[..])
        },
    ));
}

#[test]
fn a_repeated_eq_constant_survives() {
    assert!(!eq_conflicts(&Const::Word(3), &Const::Word(3)));
    // Mixed shapes stay conservative (they never arise from one field's
    // lowering).
    assert!(!eq_conflicts(&Const::Word(1), &Const::Byte(1)));
}

// Rule (c) — an Eq constant outside the range summary.

#[test]
fn an_eq_constant_outside_the_range_is_statically_empty() {
    let summary = summary(&[(CmpOp::Ge, 8), (CmpOp::Le, 19)]);
    assert!(eq_outside_range(3, &summary));
    assert!(eq_outside_range(20, &summary));
}

#[test]
fn an_eq_constant_on_the_range_edge_survives() {
    let summary = summary(&[(CmpOp::Ge, 8), (CmpOp::Le, 19)]);
    assert!(!eq_outside_range(8, &summary));
    assert!(!eq_outside_range(19, &summary));
}

// Rule (d) — the membership set after sentinel-trim.

#[test]
fn an_eq_constant_missing_from_the_set_is_statically_empty() {
    assert!(set_refutes_eq(&[1, 2, 5], Some(7)));
    // Empty after sentinel-trim: the never-minted id matches nothing,
    // with or without an Eq companion.
    assert!(set_refutes_eq(&[SENTINEL_ID], None));
    assert!(set_refutes_eq(&[], None));
}

#[test]
fn an_eq_constant_in_the_trimmed_set_survives() {
    assert!(!set_refutes_eq(&[SENTINEL_ID, 7], Some(7)));
    assert!(!set_refutes_eq(&[1, 2], None));
}

// Rule (e) — a literal-vs-literal Allen predicate classify refutes.

#[test]
fn a_refuted_literal_allen_pair_is_statically_empty() {
    // classify([2,5), [7,9)) = BEFORE, not in AFTER.
    assert!(allen_refuted((2, 5), AllenMask::AFTER, (7, 9)));
    assert!(allen_refuted((2, 5), AllenMask::EQUALS, (2, 6)));
}

#[test]
fn an_admitted_literal_allen_pair_survives() {
    // The near miss: the same pair under the mask that names it.
    assert!(!allen_refuted((2, 5), AllenMask::BEFORE, (7, 9)));
    // Degenerate encoded pairs refute nothing — conservative, never
    // wrong (unconstructible from validated literals).
    assert!(!allen_refuted((5, 5), AllenMask::AFTER, (7, 9)));
}

// Rule (f) — a constant point in a constant interval.

#[test]
fn a_point_outside_the_constant_interval_is_statically_empty() {
    assert!(point_outside((2, 5), 7));
    // The half-open law: the end is not a member.
    assert!(point_outside((2, 5), 5));
}

#[test]
fn a_point_at_the_interval_start_survives() {
    assert!(!point_outside((2, 5), 2));
    assert!(!point_outside((2, 5), 4));
}

// The fold end to end: lowering through `normalize`, emission shapes,
// the verdict, and the off switch.

fn one_rule(schema: &Schema, query: &Query) -> super::super::NormalizedQuery {
    let mut rules = normalize(schema, &validate(schema, query).expect("valid"));
    assert_eq!(rules.len(), 1, "one-rule fixtures");
    rules.remove(0)
}

/// R(a: v0, id: v1) with the given comparisons on v0.
fn range_query(predicates: Vec<Comparison>) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: R,
            bindings: vec![(R_A, Term::Var(VarId(0)))],
        }],
        negated: vec![],
        predicates: predicates.into_iter().map(PredicateTree::Leaf).collect(),
    })
}

fn cmp(op: CmpOp, rhs: Term) -> Comparison {
    Comparison {
        op,
        lhs: Term::Var(VarId(0)),
        rhs,
    }
}

#[test]
fn an_order_conjunction_folds_to_one_summary() {
    // a > 5 ∧ a >= 7 ∧ a < 20 → [7, 19] as exactly two bounds.
    let schema = schema();
    let normalized = one_rule(
        &schema,
        &range_query(vec![
            cmp(CmpOp::Gt, Term::Literal(Value::I64(5))),
            cmp(CmpOp::Ge, Term::Literal(Value::I64(7))),
            cmp(CmpOp::Lt, Term::Literal(Value::I64(20))),
        ]),
    );
    assert_eq!(normalized.dead, None);
    assert_eq!(
        normalized.occurrences[0].filters,
        vec![
            FilterPredicate::Compare {
                field: R_A,
                op: CmpOp::Ge,
                value: Const::Word(w(7)),
            },
            FilterPredicate::Compare {
                field: R_A,
                op: CmpOp::Le,
                value: Const::Word(w(19)),
            },
        ],
    );
}

#[test]
fn an_eq_pin_subsumes_its_folded_bounds() {
    // a == 5 ∧ a >= 1 ∧ a < 9 → the Eq alone (a point implies every
    // bound it survived).
    let schema = schema();
    let normalized = one_rule(
        &schema,
        &range_query(vec![
            cmp(CmpOp::Eq, Term::Literal(Value::I64(5))),
            cmp(CmpOp::Ge, Term::Literal(Value::I64(1))),
            cmp(CmpOp::Lt, Term::Literal(Value::I64(9))),
        ]),
    );
    assert_eq!(normalized.dead, None);
    assert_eq!(
        normalized.occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: R_A,
            op: CmpOp::Eq,
            value: Const::Word(w(5)),
        }],
    );
}

#[test]
fn param_and_ne_predicates_never_fold() {
    // a >= ?0 ∧ a < ?1 ∧ a != 3: params are stage-3, Ne prunes nothing
    // statically — all three filters survive verbatim.
    let schema = schema();
    let normalized = one_rule(
        &schema,
        &range_query(vec![
            cmp(CmpOp::Ge, Term::Param(ParamId(0))),
            cmp(CmpOp::Lt, Term::Param(ParamId(1))),
            cmp(CmpOp::Ne, Term::Literal(Value::I64(3))),
        ]),
    );
    assert_eq!(normalized.dead, None);
    assert_eq!(normalized.occurrences[0].filters.len(), 3);
}

#[test]
fn contradictory_order_filters_kill_the_rule() {
    let schema = schema();
    let normalized = one_rule(
        &schema,
        &range_query(vec![
            cmp(CmpOp::Gt, Term::Literal(Value::I64(5))),
            cmp(CmpOp::Lt, Term::Literal(Value::I64(3))),
        ]),
    );
    let reason = normalized.dead.expect("statically empty");
    assert_eq!(reason, "R: a > 5 ∧ a < 3");
}

#[test]
fn an_eq_outside_the_summary_kills_the_rule_with_the_prd_picture() {
    // The PRD's own example: x ∈ [8, 19] ∧ x == 3.
    let schema = schema();
    let normalized = one_rule(
        &schema,
        &range_query(vec![
            cmp(CmpOp::Gt, Term::Literal(Value::I64(7))),
            cmp(CmpOp::Lt, Term::Literal(Value::I64(20))),
            cmp(CmpOp::Eq, Term::Literal(Value::I64(3))),
        ]),
    );
    let reason = normalized.dead.expect("statically empty");
    assert_eq!(reason, "R: a ∈ [8, 19] ∧ a == 3");
}

#[test]
fn an_allen_equals_pin_refutes_a_sibling_literal_mask() {
    // P(during: v0), v0 == [2, 5) ∧ Allen(v0, AFTER, [7, 9)): the
    // interval Eq canonicalizes to Allen(EQUALS) — the pin — and
    // classify([2,5), [7,9)) = BEFORE refutes AFTER.
    let schema = schema();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: RelationId(1),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(1))),
                (FieldId(1), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Eq,
                lhs: Term::Var(VarId(0)),
                rhs: Term::Literal(Value::IntervalI64(2, 5)),
            }),
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: MaskTerm::Literal(AllenMask::AFTER),
                },
                lhs: Term::Var(VarId(0)),
                rhs: Term::Literal(Value::IntervalI64(7, 9)),
            }),
        ],
    });
    let normalized = one_rule(&schema, &query);
    let reason = normalized.dead.expect("statically empty");
    assert_eq!(reason, "P: during == 2..5 ∧ Allen(during, AFTER, 7..9)");
}

#[test]
fn a_pinned_point_outside_a_constant_interval_kills_the_rule() {
    // R(a: v0), v0 == 7 ∧ Contains([2, 5), v0): the reversed membership
    // (`FieldWithin`) against the Eq pin — rule (f).
    let schema = schema();
    let normalized = one_rule(
        &schema,
        &range_query(vec![
            cmp(CmpOp::Eq, Term::Literal(Value::I64(7))),
            Comparison {
                op: CmpOp::Contains,
                lhs: Term::Literal(Value::IntervalI64(2, 5)),
                rhs: Term::Var(VarId(0)),
            },
        ]),
    );
    let reason = normalized.dead.expect("statically empty");
    assert_eq!(reason, "R: a == 7 ∧ a in 2..5");
}

#[test]
fn a_negated_occurrence_contradiction_is_no_rule_verdict() {
    // A negated occurrence with a refuted filter list matches nothing —
    // its anti-probe never rejects, so the RULE is not empty; the fold
    // must neither kill nor rewrite it (module doc).
    let schema = schema();
    let filters = vec![
        FilterPredicate::Compare {
            field: R_A,
            op: CmpOp::Gt,
            value: Const::Word(w(5)),
        },
        FilterPredicate::Compare {
            field: R_A,
            op: CmpOp::Lt,
            value: Const::Word(w(3)),
        },
    ];
    let mut occurrences = vec![Occurrence {
        occ_id: OccId(0),
        relation: R,
        role: Role::Negated,
        vars: vec![],
        filters: filters.clone(),
    }];
    assert_eq!(fold(&schema, &mut occurrences), None);
    assert_eq!(occurrences[0].filters, filters, "untouched");
}

#[test]
fn the_off_switch_keeps_constituents_and_verdicts_away() {
    // The fold-preservation differential's switch (the chase-off
    // precedent): the same contradictory query lowers verbatim.
    let schema = schema();
    let query = range_query(vec![
        cmp(CmpOp::Gt, Term::Literal(Value::I64(5))),
        cmp(CmpOp::Lt, Term::Literal(Value::I64(3))),
    ]);
    let normalized = with_fold_disabled(|| one_rule(&schema, &query));
    assert_eq!(normalized.dead, None);
    assert_eq!(normalized.occurrences[0].filters.len(), 2);
}

#[test]
fn an_empty_word_set_kills_and_a_word_set_eq_intersection_kills() {
    // The (d) accumulator arms over hand-built filters (no stage-2
    // lowering emits a resolved `WordSet` today — the templates carry
    // `ParamSet` markers, which never fold; the arms serve the rule's
    // future literal-set producers).
    let schema = schema();
    let occurrence = |filters| Occurrence {
        occ_id: OccId(0),
        relation: R,
        role: Role::Positive,
        vars: vec![],
        filters,
    };
    let mut empty_set = vec![occurrence(vec![FilterPredicate::Compare {
        field: FieldId(2),
        op: CmpOp::Eq,
        value: Const::WordSet(vec![SENTINEL_ID]),
    }])];
    assert_eq!(fold(&schema, &mut empty_set).as_deref(), Some("R: k ∈ {}"));

    let mut disjoint = vec![occurrence(vec![
        FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Eq,
            value: Const::WordSet(vec![1, 2]),
        },
        FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Eq,
            value: Const::Word(7),
        },
    ])];
    assert_eq!(
        fold(&schema, &mut disjoint).as_deref(),
        Some("R: k ∈ {1, 2} ∧ k == 7")
    );

    let mut member = vec![occurrence(vec![
        FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Eq,
            value: Const::WordSet(vec![1, 7]),
        },
        FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Eq,
            value: Const::Word(7),
        },
    ])];
    assert_eq!(fold(&schema, &mut member), None);
}
