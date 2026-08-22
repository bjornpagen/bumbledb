use super::*;
use crate::error::{AtomIndex, FindIndex};
use crate::ir::FoldOp;
use crate::ir::{CmpOp, Comparison, Value};

#[test]
fn rejects_unknown_relation() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(RelationId(9), vec![(0, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::UnknownRelation {
            atom: AtomIndex(0),
            ..
        }
    ));
}

#[test]
fn rejects_unknown_field() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(HOLDER, vec![(9, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::UnknownField {
            atom: AtomIndex(0),
            field: FieldId(9)
        }
    ));
}

#[test]
fn rejects_duplicate_field_binding() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(HOLDER, vec![(0, var(0)), (0, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::DuplicateFieldBinding {
            atom: AtomIndex(0),
            field: FieldId(0)
        }
    ));
}

#[test]
fn rejects_variable_type_conflict() {

    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(POSTING, vec![(1, var(0)), (2, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::VariableTypeConflict { var: VarId(0) }
    ));
}

#[test]
fn rejects_literal_type_mismatch() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            POSTING,
            vec![(0, var(0)), (2, Term::Literal(Value::U64(5)))], 
        )],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::LiteralTypeMismatch {
            atom: AtomIndex(0),
            field: FieldId(2)
        }
    ));
}

#[test]
fn rejects_conflicting_param_anchors() {

    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            POSTING,
            vec![
                (0, var(0)),
                (1, Term::Param(ParamId(0))),
                (2, Term::Param(ParamId(0))),
            ],
        )],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::ParamTypeConflict { param: ParamId(0) }
    ));
}

#[test]
fn rejects_order_comparison_on_string_in_both_written_orders() {

    // equality-only refusal before generic classification.
    for literal_on_left in [false, true] {
        let literal = Term::Literal(Value::String(Box::from("x")));
        let (lhs, rhs) = if literal_on_left {
            (literal, var(0))
        } else {
            (var(0), literal)
        };
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(HOLDER, vec![(0, var(1)), (1, var(0))])],
            negated: vec![],
            conditions: vec![ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs,
                rhs,
            })],
        });
        assert_eq!(
            expect_err(&query),
            ValidationError::OrderComparisonOnString { index: 0 }
        );
    }
}

#[test]
fn rejects_self_comparison() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(0),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::SelfComparison { index: 0 }
    ));
}

#[test]
fn accepts_order_comparison_on_bool_in_both_written_orders() {

    // the strict 0/1 encoding IS the order (ruled 2026-07-23, R3), so

    for literal_on_left in [false, true] {
        let literal = Term::Literal(Value::Bool(true));
        let (lhs, rhs) = if literal_on_left {
            (literal, var(0))
        } else {
            (var(0), literal)
        };
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(1))],
            atoms: vec![atom(POSTING, vec![(5, var(0)), (0, var(1))])],
            negated: vec![],
            conditions: vec![ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs,
                rhs,
            })],
        });
        crate::ir::validate::validate(&schema(), &query).expect("bool orders: false < true");
    }
}

fn closed_schema() -> Schema {
    use bumbledb_theory::schema::{Row, Side, StatementDescriptor};
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Ticket".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("priority", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Low".into(),
                        values: Box::new([]),
                    },
                    Row {
                        handle: "High".into(),
                        values: Box::new([]),
                    },
                ])),
                name: "Priority".into(),
                fields: vec![],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(0),
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
    .validate()
    .expect("valid fixture")
}

fn closed_expect_err(query: &Query) -> ValidationError {
    crate::ir::validate::validate(&closed_schema(), query)
        .expect_err("the closed order wall refuses")
}

#[test]
fn rejects_order_comparison_on_a_closed_reference() {

    // words are declaration indices, so `Lt` on it is refused — the
    // engine-judged wall, identical on every surface (ruled 2026-07-23,

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(RelationId(0), vec![(0, var(0)), (1, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(1),
            rhs: Term::Literal(Value::U64(1)),
        })],
    });
    assert_eq!(
        closed_expect_err(&query),
        ValidationError::OrderComparisonOnClosedReference { index: 0 }
    );
}

#[test]
fn rejects_point_membership_of_a_closed_reference() {

    // a closed-bound point side is refused (R4).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            RelationId(0),
            vec![(0, var(0)), (1, var(1)), (2, var(2))],
        )],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::PointIn,
            lhs: var(2),
            rhs: var(1),
        })],
    });
    assert_eq!(
        closed_expect_err(&query),
        ValidationError::OrderComparisonOnClosedReference { index: 0 }
    );
}

#[test]
fn rejects_cross_type_comparison() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(1, var(0)), (2, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: var(1),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::IllegalComparison { index: 0 }
    ));
}

#[test]
fn rejects_constant_comparison() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: Term::Literal(Value::U64(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::ConstantComparison { index: 0 }
    ));
}

#[test]
fn rejects_unbound_find_variable() {
    let query = simple(
        vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(7))],
        vec![atom(HOLDER, vec![(0, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::UnboundFindVariable { var: VarId(7) }
    ));
}

#[test]
fn rejects_comparison_only_variable() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(9), 
            rhs: var(0),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::ComparisonOnlyVariable { var: VarId(9) }
    ));
}

#[test]
fn rejects_empty_finds() {
    let query = simple(vec![], vec![atom(HOLDER, vec![(0, var(0))])]);
    assert!(matches!(expect_err(&query), ValidationError::EmptyFinds));
}

#[test]
fn rejects_duplicate_find_terms() {
    let query = simple(
        vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(0))],
        vec![atom(HOLDER, vec![(0, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::DuplicateFindTerm { index: 1 }
    ));

    let count = || FindTerm::Count;
    let query = simple(
        vec![count(), count()],
        vec![atom(HOLDER, vec![(0, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::DuplicateFindTerm { index: 1 }
    ));
}

#[test]
fn rejects_no_positive_atoms() {
    let query = simple(vec![FindTerm::Var(VarId(0))], vec![]);
    assert!(matches!(
        expect_err(&query),
        ValidationError::NoPositiveAtoms
    ));
}

#[test]
fn rejects_negated_atoms_without_any_positive_atom() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![],
        negated: vec![atom(POSTING, vec![(1, var(0))])],
        conditions: vec![],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::NoPositiveAtoms
    ));
}

#[test]
fn rejects_sum_over_non_integer() {
    let query = simple(
        vec![FindTerm::Aggregate {
            op: FoldOp::Sum,
            over: VarId(0),
        }],
        vec![atom(HOLDER, vec![(1, var(0))])], 
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateInputType { find: FindIndex(0) }
    ));
}

#[test]
fn rejects_min_and_max_over_str() {
    // The str-extrema roster refusal (the README's recorded ruling):

    for op in [FoldOp::Min, FoldOp::Max] {
        let query = simple(
            vec![FindTerm::Aggregate { op, over: VarId(0) }],
            vec![atom(HOLDER, vec![(1, var(0))])], 
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::AggregateInputType { find: FindIndex(0) }
        ));
    }
}

#[test]
fn rejects_aggregate_over_group_key() {
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(0),
            },
        ],
        vec![atom(POSTING, vec![(2, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateOverGroupKey { find: FindIndex(1) }
    ));
}

#[test]
fn rejects_sparse_param_ids() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            HOLDER,
            vec![(0, var(0)), (1, Term::Param(ParamId(1)))],
        )],
        negated: vec![],
        conditions: vec![],
    });
    assert!(matches!(expect_err(&query), ValidationError::ParamIdGap { param } if param.0 == 0));
}

#[test]
fn rejects_more_atoms_than_the_planner_cap_at_the_boundary() {
    let over = crate::plan::planner::MAX_OCCURRENCES + 1;
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: (0..over).map(|_| atom(HOLDER, vec![(0, var(0))])).collect(),
        negated: vec![],
        conditions: vec![],
    });
    assert!(matches!(expect_err(&query), ValidationError::TooManyAtoms { count } if count == over));
}

#[test]
fn rejects_more_distinct_variables_than_the_bitset_at_the_boundary() {

    let wide = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Wide".into(),
            fields: (0..129)
                .map(|i| FieldDescriptor {
                    name: format!("f{i}").into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                })
                .collect(),
        }],
        statements: vec![],
    }
    .validate()
    .expect("wide fixture");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![crate::ir::Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: (0..129u16).map(|i| (FieldId(i), var(i))).collect(),
        }],
        negated: vec![],
        conditions: vec![],
    });
    let err = validate(&wide, &query).unwrap_err();
    assert!(matches!(
        err,
        ValidationError::TooManyVariables { count: 129 }
    ));
}

#[test]
fn negated_occurrences_count_toward_the_occurrence_cap() {

    let cap = crate::plan::planner::MAX_OCCURRENCES;
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: (0..cap).map(|_| atom(HOLDER, vec![(0, var(0))])).collect(),
        negated: vec![atom(HOLDER, vec![(0, var(0))])],
        conditions: vec![],
    });
    assert!(
        matches!(expect_err(&query), ValidationError::TooManyAtoms { count } if count == cap + 1)
    );
}

#[test]
fn order_operator_on_an_interval_gets_the_dedicated_diagnostic() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(1, 5).expect("nonempty interval"),
            )),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::OrderComparisonOnInterval { index: 0 }
    ));
}

#[test]
fn order_operator_on_two_bivalent_interval_variables() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))]),
            atom(POSTING, vec![(0, var(2)), (SPAN, var(3))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(1),
            rhs: var(3),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::OrderComparisonOnInterval { index: 0 }
    ));
}

#[test]
fn order_operator_on_fixed_bytes_gets_the_dedicated_diagnostic() {

    // is an encoding artifact — identity only, refused typed

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(0)), (4, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(1),
            rhs: Term::Literal(Value::FixedBytes(vec![0u8; 32].into())),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::OrderComparisonOnFixedBytes { index: 0 }
    ));
}

#[test]
fn rejects_min_and_max_over_fixed_bytes() {

    for op in [FoldOp::Min, FoldOp::Max] {
        let query = simple(
            vec![FindTerm::Aggregate { op, over: VarId(0) }],
            vec![atom(POSTING, vec![(4, var(0)), (0, var(1))])], 
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::AggregateInputType { find: FindIndex(0) }
        ));
    }
}

#[test]
fn rejects_a_wrong_width_fixed_bytes_literal() {

    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            POSTING,
            vec![
                (0, var(0)),
                (4, Term::Literal(Value::FixedBytes(vec![0u8; 16].into()))),
            ],
        )],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::LiteralTypeMismatch {
            atom: AtomIndex(0),
            ..
        }
    ));
}

#[test]
fn rejects_param_set_under_ne() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (1, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ne,
            lhs: var(1),
            rhs: Term::ParamSet(ParamId(0)),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::ParamSetComparison { index: 0 }
    ));
}

#[test]
fn rejects_a_param_id_used_both_scalar_and_set() {

    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![
            atom(POSTING, vec![(0, var(0)), (1, Term::ParamSet(ParamId(0)))]),
            atom(HOLDER, vec![(0, Term::Param(ParamId(0)))]),
        ],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::ParamScalarAndSet { param: ParamId(0) }
    ));
}

#[test]
fn rejects_a_membership_only_variable() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(1),
            rhs: Term::Literal(Value::U64(5)),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::MembershipOnlyVariable { var: VarId(1) }
    ));
}

#[test]
fn rejects_a_negated_atom_variable_unbound_by_positive_atoms() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        negated: vec![atom(POSTING, vec![(1, var(1))])],
        conditions: vec![],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::NegatedVariableUnbound { var: VarId(1) }
    ));
}

#[test]
fn a_param_position_does_not_bind_a_negated_variable_even_when_written_after_it() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        // Hostile textual order: the unsafe occurrence is written first.
        negated: vec![atom(POSTING, vec![(1, var(1))])],
        atoms: vec![atom(
            HOLDER,
            vec![(0, var(0)), (1, Term::Param(ParamId(1)))],
        )],
        conditions: vec![],
    });
    assert_eq!(
        expect_err(&query),
        ValidationError::NegatedVariableUnbound { var: VarId(1) }
    );
}

#[test]
fn an_aggregate_output_does_not_bind_a_negated_variable_even_when_written_after_it() {
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        // Hostile textual order: the unsafe occurrence is written first.
        negated: vec![atom(POSTING, vec![(1, var(1))])],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        conditions: vec![],
    });
    assert_eq!(
        expect_err(&query),
        ValidationError::NegatedVariableUnbound { var: VarId(1) }
    );
}

#[test]
fn rejects_a_point_literal_at_the_ceiling_in_a_membership_binding() {

    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            ACCOUNT,
            vec![(0, var(0)), (VALIDITY, Term::Literal(Value::U64(u64::MAX)))],
        )],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::PointLiteralAtCeiling {
            atom: AtomIndex(0),
            field: FieldId(VALIDITY)
        }
    ));
}

#[test]
fn rejects_a_point_literal_at_the_ceiling_under_point_in() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::PointIn,
            lhs: var(1),
            rhs: Term::Literal(Value::U64(u64::MAX)),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::ComparisonPointLiteralAtCeiling { index: 0 }
    ));
}

#[test]
fn rejects_an_interval_typed_param_set_anchor() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(1),
            rhs: Term::ParamSet(ParamId(0)),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::IntervalParamSet { param: ParamId(0) }
    ));
}

#[test]
fn rejects_the_empty_allen_mask() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: bumbledb_theory::allen::AllenMask::EMPTY,
            },
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(1, 5).expect("nonempty interval"),
            )),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::EmptyAllenMask { index: 0 }
    ));
}

#[test]
fn rejects_the_full_allen_mask() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))]),
            atom(POSTING, vec![(0, var(2)), (SPAN, var(3))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: bumbledb_theory::allen::AllenMask::FULL,
            },
            lhs: var(1),
            rhs: var(3),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::FullAllenMask { index: 0 }
    ));
}

#[test]
fn rejects_allen_over_non_interval_sides() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(0)), (2, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: bumbledb_theory::allen::AllenMask::INTERSECTS,
            },
            lhs: var(1),
            rhs: Term::Literal(Value::I64(5)),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::IllegalComparison { index: 0 }
    ));
}

#[test]
fn rejects_point_in_between_two_intervals() {

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))]),
            atom(POSTING, vec![(0, var(2)), (SPAN, var(3))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::PointIn,
            lhs: var(1),
            rhs: var(3),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::IllegalComparison { index: 0 }
    ));

    let literal = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::PointIn,
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(1, 5).expect("nonempty interval"),
            )),
        })],
    });
    assert!(matches!(
        expect_err(&literal),
        ValidationError::IllegalComparison { index: 0 }
    ));
}

#[test]
fn rejects_a_second_pack_term() {
    // The multi-Pack product has no sighting — refused with its trigger

    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Pack { over: VarId(1) },
            FindTerm::Pack { over: VarId(2) },
        ],
        vec![
            atom(POSTING, vec![(1, var(0)), (SPAN, var(1))]),
            atom(ACCOUNT, vec![(1, var(0)), (VALIDITY, var(2))]),
        ],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::MultiplePackTerms { find: FindIndex(2) }
    ));
}

#[test]
fn rejects_pack_beside_a_fold_aggregate() {

    // refused — coalesced-time accounting is two queries or a host fold.
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Pack { over: VarId(1) },
            FindTerm::Count,
        ],
        vec![atom(POSTING, vec![(1, var(0)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::MixedPackAndFold { find: FindIndex(2) }
    ));
}

#[test]
fn rejects_pack_over_a_non_interval_variable() {

    let query = simple(
        vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        vec![atom(POSTING, vec![(1, var(0)), (2, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::PackInputType { find: FindIndex(1) }
    ));
}

#[test]
fn rejects_pack_over_a_group_key_variable() {

    let query = simple(
        vec![FindTerm::Var(VarId(1)), FindTerm::Pack { over: VarId(1) }],
        vec![atom(POSTING, vec![(1, var(0)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateOverGroupKey { find: FindIndex(1) }
    ));
}

fn cross_domain_schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Pair".into(),
            fields: vec![
                field("id", ValueType::U64),
                field(
                    "ulane",
                    ValueType::FixedInterval {
                        element: IntervalElement::U64,
                        width: 5,
                    },
                ),
                field(
                    "iline",
                    ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                ),
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// Q1 relaxes widths, never element domains: Allen between a u64-domain
/// fixed-width term and an i64-domain general term stays an illegal comparison
/// — the two domains share no `Point` tag (`lean/Bumbledb/Schema.lean:
/// Value.points_one_tag_u64`).
#[test]
fn rejects_an_allen_pair_across_element_domains_whatever_the_widths() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            RelationId(0),
            vec![(0, var(0)), (1, var(1)), (2, var(2))],
        )],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: bumbledb_theory::allen::AllenMask::INTERSECTS,
            },
            lhs: var(1), 
            rhs: var(2), 
        })],
    });
    assert!(matches!(
        validate(&cross_domain_schema(), &query).expect_err("cross-domain Allen must reject"),
        ValidationError::IllegalComparison { .. }
    ));
}

#[test]
fn rejects_a_wrong_width_interval_literal_at_a_fixed_width_field() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            RelationId(0),
            vec![
                (0, var(0)),
                (
                    1, 
                    Term::Literal(Value::IntervalU64(
                        bumbledb_theory::Interval::<u64>::new(3, 7).expect("nonempty"), 
                    )),
                ),
            ],
        )],
    );
    assert!(matches!(
        validate(&cross_domain_schema(), &query).expect_err("wrong width must reject"),
        ValidationError::LiteralTypeMismatch {
            atom: AtomIndex(0),
            ..
        }
    ));
}

#[test]
fn rejects_a_width_matched_ray_literal_at_a_fixed_width_field() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            RelationId(0),
            vec![
                (0, var(0)),
                (
                    1, 
                    Term::Literal(Value::IntervalU64(
                        bumbledb_theory::Interval::<u64>::new(u64::MAX - 5, u64::MAX)
                            .expect("a legal general ray"), 
                    )),
                ),
            ],
        )],
    );
    assert!(matches!(
        validate(&cross_domain_schema(), &query).expect_err("the width-matched ray must reject"),
        ValidationError::LiteralTypeMismatch {
            atom: AtomIndex(0),
            ..
        }
    ));
}
