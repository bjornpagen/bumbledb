//! against the honest pipeline (validate → normalize → grounding) over a
//! condition's refusal shape is easier to pin in isolation.
use super::*;
use crate::image::view::{Const, FilterPredicate, IntervalConst, SetConst, ViewWordSource};
use crate::ir::normalize::{FoldedMark, NormalizedQuery, normalize_rules};
use crate::ir::validate::validate;
use crate::ir::{Atom, Comparison, ConditionTree, FindTerm, Query, Rule, Term, Value};
use crate::ir::{CmpOp, WordCmp};
use crate::plan::ground::{ground, with_grounding_disabled};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, IntervalElement, RelationDescriptor, Row, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn fresh(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    }
}

const ITEM: u32 = 0;
const LOOSE: u32 = 1;
const SCHED: u32 = 2;
const KIND: u32 = 3;
const CAL: u32 = 4;

fn theory() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![
                    fresh("id"),
                    field("kind", ValueType::U64),
                    field("score", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Loose".into(),
                fields: vec![fresh("id"), field("k", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Sched".into(),
                fields: vec![fresh("id"), field("cal", ValueType::U64)],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "A".into(),
                        values: Box::new([Value::U64(10)]),
                    },
                    Row {
                        handle: "B".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "C".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "D".into(),
                        values: Box::new([Value::U64(30)]),
                    },
                ])),
                name: "Kind".into(),
                fields: vec![field("rank", ValueType::U64)],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "X".into(),
                        values: Box::new([Value::IntervalU64(
                            bumbledb_theory::Interval::<u64>::new(2, 5).expect("nonempty interval"),
                        )]),
                    },
                    Row {
                        handle: "Y".into(),
                        values: Box::new([Value::IntervalU64(
                            bumbledb_theory::Interval::<u64>::new(5, 9).expect("nonempty interval"),
                        )]),
                    },
                ])),
                name: "Cal".into(),
                fields: vec![field(
                    "span",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                )],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(ITEM),
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(KIND),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
    .validate()
    .expect("valid fixture")
}

fn atom(relation: u32, bindings: &[(u16, Term)]) -> Atom {
    Atom {
        source: crate::ir::AtomSource::Edb(RelationId(relation)),
        bindings: bindings
            .iter()
            .map(|(f, t)| (FieldId(*f), t.clone()))
            .collect(),
    }
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn grounded(schema: &Schema, query: &Query) -> NormalizedQuery {
    let witness = validate(schema, query).expect("valid fixture query");
    let mut normalized = normalize_rules(schema, &[], witness.rules()).remove(0);
    ground(&mut normalized, schema, &query.rules()[0].finds);
    normalized
}

fn roles(normalized: &NormalizedQuery) -> Vec<Role> {
    normalized
        .occurrences
        .iter()
        .map(|o| o.role.clone())
        .collect()
}

fn attached_sets(normalized: &NormalizedQuery, idx: usize) -> Vec<Vec<u64>> {
    normalized.occurrences[idx]
        .filters
        .iter()
        .filter_map(|filter| match filter {
            FilterPredicate::Compare {
                op: WordCmp::Eq,
                value: Const::WordSet(words),
                ..
            } => Some(words.clone()),
            _ => None,
        })
        .collect()
}

fn folded_pos(relation: u32, survivors: &[u64]) -> Role {
    Role::Folded(FoldedMark::Positive {
        relation: bumbledb_theory::schema::RelationId(relation),
        survivors: survivors.to_vec().into_boxed_slice(),
    })
}

fn folded_neg(relation: u32, survivors: &[u64]) -> Role {
    Role::Folded(FoldedMark::Negated {
        relation: bumbledb_theory::schema::RelationId(relation),
        survivors: survivors.to_vec().into_boxed_slice(),
    })
}

fn selected_fold_query(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1)), (2, var(2))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(rank)))]),
        ],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn a_filtered_closed_atom_folds_to_a_membership_set() {
    let schema = theory();
    let normalized = grounded(&schema, &selected_fold_query(20));
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded_pos(KIND, &[1, 2])],
        "the Kind occurrence folded with |S| = 2"
    );
    assert_eq!(
        attached_sets(&normalized, 0),
        vec![vec![1, 2]],
        "the sibling's kind field carries exactly the σ-surviving ids"
    );
    assert!(normalized.dead.is_none());
}

#[test]
fn the_off_switch_bypasses_the_evaluator() {
    let schema = theory();
    let query = selected_fold_query(20);
    let witness = validate(&schema, &query).expect("valid fixture query");
    let mut normalized = normalize_rules(&schema, &[], witness.rules()).remove(0);
    with_grounding_disabled(|| ground(&mut normalized, &schema, &query.rules()[0].finds));
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
    assert!(attached_sets(&normalized, 0).is_empty());
}

#[test]
fn a_live_payload_variable_blocks_the_fold() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
    assert!(attached_sets(&normalized, 0).is_empty());
}

#[test]
fn a_dead_payload_variable_folds() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded_pos(KIND, &[0, 1, 2, 3])]
    );
    assert_eq!(attached_sets(&normalized, 0), vec![vec![0, 1, 2, 3]]);
}

/// Condition 2 negative — a param-bearing filter defers to bind time, which is
/// REFUSED v0: the fold must not judge stage-3 values.
#[test]
fn a_param_filter_blocks_the_fold() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(
                KIND,
                &[(0, var(1)), (1, Term::Param(crate::ir::ParamId(0)))],
            ),
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
    assert!(attached_sets(&normalized, 0).is_empty());
}

#[test]
fn resolvable_parser_is_total_over_the_filter_vocabulary() {
    assert_word_and_field_compares_parse();
    assert_wide_compares_parse();
    assert_structured_filters_parse();
    assert_compare_refusals();
    assert_other_refusals();
}

fn assert_parse(filter: &FilterPredicate, resolvable: bool) {
    assert_eq!(
        crate::image::view::is_prepare_resolvable(filter),
        resolvable
    );
}

fn assert_word_and_field_compares_parse() {
    let f = FieldId(1);
    for op in [
        WordCmp::Eq,
        WordCmp::Ne,
        WordCmp::Lt,
        WordCmp::Le,
        WordCmp::Gt,
        WordCmp::Ge,
    ] {
        assert_parse(
            &FilterPredicate::Compare {
                field: f.into(),
                op,
                value: Const::Word(7),
            },
            true,
        );
        assert_parse(
            &FilterPredicate::FieldsCompare {
                left: FieldId(0).into(),
                right: f.into(),
                op,
            },
            true,
        );
    }
}

fn assert_wide_compares_parse() {
    let f = FieldId(1);
    for op in [WordCmp::Eq, WordCmp::Ne] {
        assert_parse(
            &FilterPredicate::Compare {
                field: f.into(),
                op,
                value: Const::Byte(1),
            },
            true,
        );
        let words = Box::new([3u64, 5]);
        assert_parse(
            &FilterPredicate::Compare {
                field: f.into(),
                op,
                value: Const::Words(words),
            },
            true,
        );
        assert_parse(
            &FilterPredicate::Compare {
                field: f.into(),
                op,
                value: Const::Interval { start: 2, end: 9 },
            },
            true,
        );
    }
    assert_parse(
        &FilterPredicate::Compare {
            field: f.into(),
            op: WordCmp::Eq,
            value: Const::WordSet(vec![1, 2]),
        },
        true,
    );
}

fn assert_structured_filters_parse() {
    let f = FieldId(1);
    assert_parse(
        &FilterPredicate::PointIn {
            field: f.into(),
            point: ViewWordSource::Word(4),
        },
        true,
    );
    assert_parse(
        &FilterPredicate::FieldsPointIn {
            interval: f.into(),
            point: FieldId(2).into(),
        },
        true,
    );
    assert_parse(
        &FilterPredicate::FieldWithin {
            field: f.into(),
            outer: IntervalConst::Interval { start: 2, end: 9 },
        },
        true,
    );
    assert_parse(
        &FilterPredicate::FieldsAllen {
            left: f.into(),
            right: FieldId(2).into(),
            mask: AllenMask::BEFORE,
        },
        true,
    );
    assert_parse(
        &FilterPredicate::FieldAllen {
            field: f.into(),
            other: IntervalConst::Interval { start: 2, end: 9 },
            mask: AllenMask::BEFORE,
        },
        true,
    );
}

fn assert_compare_refusals() {
    let f = FieldId(1);
    for filter in [
        FilterPredicate::Compare {
            field: f.into(),
            op: WordCmp::Ne,
            value: Const::WordSet(vec![1, 2]),
        },
        FilterPredicate::Compare {
            field: f.into(),
            op: WordCmp::Lt,
            value: Const::Byte(1),
        },
        FilterPredicate::Compare {
            field: f.into(),
            op: WordCmp::Lt,
            value: Const::Words(Box::new([1, 2])),
        },
        FilterPredicate::Compare {
            field: f.into(),
            op: WordCmp::Lt,
            value: Const::Interval { start: 2, end: 9 },
        },
        FilterPredicate::Compare {
            field: f.into(),
            op: WordCmp::Eq,
            value: Const::Param(crate::ir::ParamId(0)),
        },
        FilterPredicate::Compare {
            field: f.into(),
            op: WordCmp::Eq,
            value: Const::ParamSet(crate::ir::ParamId(0)),
        },
        FilterPredicate::Compare {
            field: f.into(),
            op: WordCmp::Eq,
            value: Const::PendingIntern {
                bytes: Box::from(&b"x"[..]),
            },
        },
    ] {
        assert_parse(&filter, false);
    }
}

fn assert_other_refusals() {
    let f = FieldId(1);
    for filter in [
        FilterPredicate::PointIn {
            field: f.into(),
            point: ViewWordSource::Param(crate::ir::ParamId(0)),
        },
        FilterPredicate::AnyPointIn {
            field: f.into(),
            set: SetConst::ParamSet(crate::ir::ParamId(0)),
        },
        FilterPredicate::FieldAllen {
            field: f.into(),
            other: IntervalConst::Param(crate::ir::ParamId(0)),
            mask: AllenMask::BEFORE,
        },
        FilterPredicate::FieldWithin {
            field: f.into(),
            outer: IntervalConst::Param(crate::ir::ParamId(0)),
        },
    ] {
        assert_parse(&filter, false);
    }
}

#[test]
fn parsed_evaluator_agrees_with_the_pinned_extension_id_sets() {
    let schema = theory();
    let cases = [
        (
            RelationId(KIND),
            vec![FilterPredicate::Compare {
                field: FieldId(1).into(),
                op: WordCmp::Eq,
                value: Const::Word(20),
            }],
            vec![1, 2],
        ),
        (
            RelationId(KIND),
            vec![FilterPredicate::Compare {
                field: FieldId(1).into(),
                op: WordCmp::Ge,
                value: Const::Word(20),
            }],
            vec![1, 2, 3],
        ),
        (
            RelationId(KIND),
            vec![FilterPredicate::Compare {
                field: FieldId(0).into(),
                op: WordCmp::Eq,
                value: Const::WordSet(vec![0, 3]),
            }],
            vec![0, 3],
        ),
        (
            RelationId(CAL),
            vec![FilterPredicate::PointIn {
                field: FieldId(1).into(),
                point: ViewWordSource::Word(3),
            }],
            vec![0],
        ),
        (
            RelationId(CAL),
            vec![FilterPredicate::FieldAllen {
                field: FieldId(1).into(),
                other: IntervalConst::Interval { start: 6, end: 8 },
                mask: AllenMask::BEFORE,
            }],
            vec![0],
        ),
    ];
    for (relation, original, expected) in cases {
        assert!(
            original
                .iter()
                .all(crate::image::view::is_prepare_resolvable)
        );
        assert_eq!(
            surviving_ids(schema.relation(relation), &original),
            expected
        );
    }
}

#[test]
fn a_negated_closed_atom_folds_to_the_complement() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ITEM, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(
            KIND,
            &[(0, var(1)), (1, Term::Literal(Value::U64(20)))],
        )],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded_neg(KIND, &[1, 2])]
    );
    assert_eq!(
        attached_sets(&normalized, 0),
        vec![vec![0, 3]],
        "the complement of {{1, 2}} in the 4-row extension"
    );
    assert!(
        normalized.anti_probes.is_empty(),
        "the folded probe's descriptor is deleted"
    );
}

#[test]
fn a_negated_fold_without_the_domain_guarantee_refuses() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(LOOSE, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(
            KIND,
            &[(0, var(1)), (1, Term::Literal(Value::U64(20)))],
        )],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Negated]);
    assert!(attached_sets(&normalized, 0).is_empty());
    assert_eq!(normalized.anti_probes.len(), 1, "the probe stays");
}

#[test]
fn a_negated_atom_over_an_empty_set_deletes_and_rejects_nothing() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(LOOSE, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(
            KIND,
            &[(0, var(1)), (1, Term::Literal(Value::U64(99)))],
        )],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded_neg(KIND, &[])]
    );
    assert!(attached_sets(&normalized, 0).is_empty());
    assert!(normalized.anti_probes.is_empty());
    assert!(normalized.dead.is_none(), "the rule is NOT empty");
}

#[test]
fn an_empty_complement_kills_the_rule() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ITEM, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(KIND, &[(0, var(1))])],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        normalized.dead.as_deref(),
        Some("folded: !Kind{} rejects every binding"),
        "S = the whole extension ∧ k ∈ ids ⇒ every binding rejected"
    );
}

#[test]
fn a_satisfied_var_less_gate_deletes_outright() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0))]),
            atom(KIND, &[(1, Term::Literal(Value::U64(20)))]),
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded_pos(KIND, &[1, 2])]
    );
    assert!(attached_sets(&normalized, 0).is_empty());
    assert!(normalized.dead.is_none());
}

#[test]
fn a_var_binding_gate_refuses() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(20)))]),
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

#[test]
fn an_empty_surviving_set_kills_the_rule() {
    let schema = theory();
    let normalized = grounded(&schema, &selected_fold_query(99));
    assert_eq!(
        normalized.dead.as_deref(),
        Some("folded to ∅: Kind{rank == 99}"),
        "the rendered reason names the refuting atom"
    );
}

#[test]
fn an_unsatisfiable_gate_kills_the_rule() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0))]),
            atom(KIND, &[(1, Term::Literal(Value::U64(99)))]),
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        normalized.dead.as_deref(),
        Some("folded to ∅: Kind{rank == 99}")
    );
}

#[test]
fn a_fold_with_no_membership_home_refuses() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            KIND,
            &[(0, var(0)), (1, Term::Literal(Value::U64(20)))],
        )],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive]);
    assert!(normalized.dead.is_none());
}

#[test]
fn multi_rule_queries_fold_per_rule_independently() {
    let schema = theory();
    let fold_rule = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(20)))]),
        ],
        negated: vec![],
        conditions: vec![],
    };
    let refusing_rule = Rule {
        finds: vec![FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        conditions: vec![],
    };

    let query = Query {
        interiors: vec![],
        head: fold_rule.head(),
        rules: vec![fold_rule, refusing_rule],
        rec: None,
    };
    let witness = validate(&schema, &query).expect("valid fixture query");
    let mut rules = normalize_rules(&schema, &[], witness.rules());
    for (idx, rule) in rules.iter_mut().enumerate() {
        ground(rule, &schema, &witness.rule(idx).rule().finds);
    }
    assert_eq!(
        roles(&rules[0]),
        vec![Role::Positive, folded_pos(KIND, &[1, 2])]
    );
    assert_eq!(attached_sets(&rules[0], 0), vec![vec![1, 2]]);
    assert_eq!(
        roles(&rules[1]),
        vec![Role::Positive, Role::Positive],
        "rule 1 projects the payload — its own refusal, untouched by rule 0's fold"
    );
    assert!(attached_sets(&rules[1], 0).is_empty());
}

#[test]
fn interval_filters_evaluate_against_the_sealed_extension() {
    let schema = theory();

    let membership = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(SCHED, &[(0, var(0)), (1, var(1))]),
            atom(CAL, &[(0, var(1)), (1, Term::Literal(Value::U64(3)))]),
        ],
        negated: vec![],
        conditions: vec![],
    });
    let normalized = grounded(&schema, &membership);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded_pos(CAL, &[0])]
    );
    assert_eq!(attached_sets(&normalized, 0), vec![vec![0]]);

    let allen = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(SCHED, &[(0, var(0)), (1, var(1))]),
            atom(CAL, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::BEFORE,
            },
            lhs: var(2),
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(6, 8).expect("nonempty interval"),
            )),
        })],
    });
    let normalized = grounded(&schema, &allen);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded_pos(CAL, &[0])]
    );
    assert_eq!(attached_sets(&normalized, 0), vec![vec![0]]);
}

#[test]
fn a_second_closed_atom_folds_over_the_first_folds_set() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(20)))]),
            atom(KIND, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(2),
            rhs: Term::Literal(Value::U64(20)),
        })],
    });
    let normalized = grounded(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![
            Role::Positive,
            folded_pos(KIND, &[1, 2]),
            folded_pos(KIND, &[1, 2])
        ],
        "both closed occurrences fold (the second sees the first's set as a filter)"
    );

    let sets = attached_sets(&normalized, 0);
    assert_eq!(sets.len(), 2);
    assert!(
        sets.contains(&vec![1, 2]),
        "rank == 20 → {{1, 2}}: {sets:?}"
    );
}

#[test]
fn the_folded_picture_prints_handles_at_the_id_position() {
    let schema = theory();
    let relation = RelationId(KIND);
    let eq_id = |value: Const| FilterPredicate::Compare {
        field: FieldId(0).into(),
        op: WordCmp::Eq,
        value,
    };
    assert_eq!(
        folded_picture(&schema, relation, &[eq_id(Const::Word(1))]),
        "Kind{id == B}"
    );
    assert_eq!(
        folded_picture(&schema, relation, &[eq_id(Const::Word(9))]),
        "Kind{id == Kind(9?)}"
    );
    assert_eq!(
        folded_picture(&schema, relation, &[eq_id(Const::WordSet(vec![0, 2]))]),
        "Kind{id ∈ {A, C}}"
    );

    assert_eq!(
        folded_picture(
            &schema,
            relation,
            &[FilterPredicate::Compare {
                field: FieldId(1).into(),
                op: WordCmp::Eq,
                value: Const::Word(20),
            }]
        ),
        "Kind{rank == 20}"
    );
}
