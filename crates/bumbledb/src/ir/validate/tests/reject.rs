use super::*;
use crate::ir::{AggOp, CmpOp, Comparison, MaskTerm, Value};

// --- Rejecting shapes, one per roster item ---

#[test]
fn rejects_unknown_relation() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(RelationId(9), vec![(0, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::UnknownRelation { atom: 0, .. }
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
            atom: 0,
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
            atom: 0,
            field: FieldId(0)
        }
    ));
}

#[test]
fn rejects_variable_type_conflict() {
    // Var 0 bound to a U64 field and an I64 field.
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
            vec![(0, var(0)), (2, Term::Literal(Value::U64(5)))], // I64 field
        )],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::LiteralTypeMismatch {
            atom: 0,
            field: FieldId(2)
        }
    ));
}

#[test]
fn rejects_conflicting_param_anchors() {
    // Param 0 anchored at U64 (Posting.account) and I64 (Posting.amount).
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
    // Holder.name is a String: both written orders get the dedicated
    // equality-only refusal before generic classification.
    for literal_on_left in [false, true] {
        let literal = Term::Literal(Value::String(Box::from(&b"x"[..])));
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
    // x < x is constant-valued: write the query you mean.
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
fn rejects_order_comparison_on_bool_in_both_written_orders() {
    // Posting.flag is Bool (field 5): both written orders get the dedicated
    // equality-only refusal before generic classification.
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
        assert_eq!(
            expect_err(&query),
            ValidationError::OrderComparisonOnBool { index: 0 }
        );
    }
}

#[test]
fn rejects_cross_type_comparison() {
    // U64 var vs I64 var: no silent coercion, ever.
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
            lhs: var(9), // appears in no atom
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

    // Aggregate terms collide under the same structural equality: two
    // nullary Counts are one find written twice.
    let count = || FindTerm::Aggregate {
        op: AggOp::Count,
        over: None,
    };
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
    // Negated atoms alone bind nothing: not a query.
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
            op: AggOp::Sum,
            over: Some(VarId(0)),
        }],
        vec![atom(HOLDER, vec![(1, var(0))])], // String
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateInputType { find: 0 }
    ));
}

#[test]
fn rejects_min_and_max_over_str() {
    // The str-extrema roster refusal (the README's recorded ruling):
    // intern words are not order-preserving, so a str extreme would be
    // a dictionary-id extreme — meaningless. Min/Max fold U64/I64 only.
    for op in [AggOp::Min, AggOp::Max] {
        let query = simple(
            vec![FindTerm::Aggregate {
                op,
                over: Some(VarId(0)),
            }],
            vec![atom(HOLDER, vec![(1, var(0))])], // String
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::AggregateInputType { find: 0 }
        ));
    }
}

#[test]
fn rejects_count_with_a_variable() {
    let query = simple(
        vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: Some(VarId(0)),
        }],
        vec![atom(POSTING, vec![(2, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::CountWithVariable { find: 0 }
    ));
}

#[test]
fn rejects_sum_without_a_variable() {
    let query = simple(
        vec![FindTerm::Aggregate {
            op: AggOp::Sum,
            over: None,
        }],
        vec![atom(POSTING, vec![(2, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateWithoutVariable { find: 0 }
    ));
}

#[test]
fn rejects_aggregate_over_group_key() {
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(0)),
            },
        ],
        vec![atom(POSTING, vec![(2, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateOverGroupKey { find: 1 }
    ));
}

#[test]
fn rejects_sparse_param_ids() {
    // ?1 without ?0: the gap would be an unchecked positional slot.
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
    // One 129-field relation binds 129 fresh variables in a single
    // atom — past the executor's 128-bit variable bitsets.
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
            relation: RelationId(0),
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
    // MAX_OCCURRENCES positive atoms alone pass; one negated atom tips
    // the occurrence count over — anti-probes consume plan-time work.
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

// --- The PRD 12 reject corpus: the new roster lines ---

#[test]
fn order_operator_on_an_interval_gets_the_dedicated_diagnostic() {
    // Lt over Account.validity — the predictable mistake gets the good
    // error, not a generic IllegalComparison.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(
                crate::Interval::<u64>::new(1, 5).expect("nonempty interval"),
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
    // Both sides bound only in interval fields: the bivalent anchors
    // resolve to the interval type, and the order op is rejected with
    // the dedicated diagnostic.
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
    // Lt over Posting.memo (bytes<32>): a digest's lexicographic order
    // is an encoding artifact — identity only, refused typed
    // (docs/architecture/10-data-model.md, the order-on-bytes refusal).
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
    // Min/Max fold an order that bytes<N> refuses to have.
    for op in [AggOp::Min, AggOp::Max] {
        let query = simple(
            vec![FindTerm::Aggregate {
                op,
                over: Some(VarId(0)),
            }],
            vec![atom(POSTING, vec![(4, var(0)), (0, var(1))])], // bytes<32>
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::AggregateInputType { find: 0 }
        ));
    }
}

#[test]
fn rejects_a_wrong_width_fixed_bytes_literal() {
    // The length is the type: a 16-byte literal against bytes<32> is a
    // type mismatch, exactly like a wrong variant.
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
        ValidationError::LiteralTypeMismatch { atom: 0, .. }
    ));
}

#[test]
fn rejects_param_set_under_ne() {
    // Ne(x, set) reads as ambiguous quantification: a param set is legal
    // only in atom bindings and under Eq.
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
    // ?0 as a set in Posting.account and as a scalar in Holder.id.
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
    // The comparison collapses t to the element type (U64), so its one
    // atom binding is membership — no enumerable domain.
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
    // A negated atom binds nothing; y comes from nowhere.
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
                op: AggOp::Sum,
                over: Some(VarId(1)),
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
fn rejects_mixed_arg_and_fold_aggregates() {
    // ArgMax + Sum in one find list: "sum of the latest" is two queries.
    let query = simple(
        vec![
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(0) },
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(2)),
            },
        ],
        vec![atom(POSTING, vec![(3, var(0)), (1, var(1)), (2, var(2))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::MixedArgAndFold { find: 1 }
    ));
}

#[test]
fn rejects_arg_terms_with_differing_keys() {
    let query = simple(
        vec![
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(0) },
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(2) },
                over: Some(VarId(3)),
            },
        ],
        vec![atom(
            POSTING,
            vec![(3, var(0)), (1, var(1)), (2, var(2)), (0, var(3))],
        )],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::ArgKeyMismatch { find: 1 }
    ));
}

#[test]
fn rejects_arg_terms_with_differing_directions() {
    // One key, two directions: ArgMax and ArgMin may not mix either.
    let query = simple(
        vec![
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(0) },
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::ArgMin { key: VarId(0) },
                over: Some(VarId(2)),
            },
        ],
        vec![atom(POSTING, vec![(3, var(0)), (1, var(1)), (2, var(2))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::ArgKeyMismatch { find: 1 }
    ));
}

#[test]
fn rejects_a_non_orderable_arg_key() {
    // Holder.name (String) as the Arg key: no extreme to attain.
    let query = simple(
        vec![FindTerm::Aggregate {
            op: AggOp::ArgMax { key: VarId(0) },
            over: Some(VarId(1)),
        }],
        vec![atom(HOLDER, vec![(1, var(0)), (0, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::NonOrderableArgKey { find: 0 }
    ));
}

#[test]
fn rejects_a_point_literal_at_the_ceiling_in_a_membership_binding() {
    // The point-domain law: points are MIN..=MAX-1, and MAX is the ray's
    // ∞ — inside no interval, so the membership is typed out, never
    // silently unmatchable.
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
            atom: 0,
            field: FieldId(VALIDITY)
        }
    ));
}

#[test]
fn rejects_a_point_literal_at_the_ceiling_under_point_in() {
    // The comparison-site sibling: a PointIn right side is an interval
    // position, so the ceiling is equally not a point there.
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
    // v resolves to the interval type (its only anchors are bivalent), so
    // Eq(v, ?set0) would make ?set0 a set of intervals — not a thing.
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

// --- The Allen mask roster lines (PRD ALG-03) ---

#[test]
fn rejects_the_empty_allen_mask() {
    // Allen(v, [1,5), ∅): no basic can hold — "never"; write no query.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(crate::allen::AllenMask::EMPTY),
            },
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(
                crate::Interval::<u64>::new(1, 5).expect("nonempty interval"),
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
    // Allen(v, w, all 13): every pair satisfies it — "always"; write no
    // condition.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))]),
            atom(POSTING, vec![(0, var(2)), (SPAN, var(3))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(crate::allen::AllenMask::FULL),
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
    // Allen over two scalar variables: the interval-pair comparison
    // types over intervals only.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(0)), (2, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(crate::allen::AllenMask::INTERSECTS),
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
    // The interval⊇interval PointIn overload is gone — that predicate is
    // Allen(COVERS); an interval-typed right side is illegal.
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
    // The literal form of the same mistake.
    let literal = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::PointIn,
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(
                crate::Interval::<u64>::new(1, 5).expect("nonempty interval"),
            )),
        })],
    });
    assert!(matches!(
        expect_err(&literal),
        ValidationError::IllegalComparison { index: 0 }
    ));
}

#[test]
fn rejects_a_mask_param_with_a_value_anchor() {
    // ?0 is both the Allen mask and a field binding: a mask is not a
    // data-model type, so the anchors conflict.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(
                ACCOUNT,
                vec![
                    (0, var(0)),
                    (1, Term::Param(ParamId(0))),
                    (VALIDITY, var(1)),
                ],
            ),
            atom(POSTING, vec![(0, var(2)), (SPAN, var(3))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Param(ParamId(0)),
            },
            lhs: var(1),
            rhs: var(3),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::ParamTypeConflict { param: ParamId(0) }
    ));
}

// --- The measure's typed rejections (20-query-ir § the measure: every other position) ---

/// A `Duration` rule with the interval variable bound on `Posting.span`
/// and the given predicate — the measure rejection fixtures' one shape.
fn duration_condition(comparison: Comparison) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(0)), (SPAN, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(comparison)],
    })
}

#[test]
fn rejects_duration_in_a_binding() {
    // The measure is a computation, not a bindable value.
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            POSTING,
            vec![(0, var(0)), (1, Term::Measure(VarId(1))), (SPAN, var(1))],
        )],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::DurationInBinding {
            atom: 0,
            field: FieldId(1)
        }
    ));
}

#[test]
fn rejects_duration_over_a_non_interval_variable() {
    // Var 1 is I64 (Posting.amount): the measure is defined by the
    // interval denotation and by nothing else.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(0)), (2, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Gt,
            lhs: Term::Measure(VarId(1)),
            rhs: Term::Literal(Value::U64(5)),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::DurationOverNonInterval { var: VarId(1) }
    ));
}

#[test]
fn rejects_a_duration_find_over_a_non_interval_variable() {
    let query = simple(
        vec![FindTerm::Measure(VarId(0))],
        vec![atom(POSTING, vec![(1, var(0))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::DurationOverNonInterval { var: VarId(0) }
    ));
}

#[test]
fn rejects_a_duration_aggregate_outside_sum_min_max() {
    // Count is nullary; CountDistinct and the Arg ops are refused too.
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::AggregateMeasure {
                op: AggOp::CountDistinct,
                over: VarId(1),
            },
        ],
        vec![atom(POSTING, vec![(0, var(0)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::DurationAggregateOp { find: 1 }
    ));
}

#[test]
fn rejects_duration_under_equality() {
    // Only the order comparisons take a measure side.
    let query = duration_condition(Comparison {
        op: CmpOp::Eq,
        lhs: Term::Measure(VarId(1)),
        rhs: Term::Literal(Value::U64(5)),
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::DurationComparisonOperator { index: 0 }
    ));
}

#[test]
fn rejects_duration_on_both_sides() {
    let query = duration_condition(Comparison {
        op: CmpOp::Lt,
        lhs: Term::Measure(VarId(1)),
        rhs: Term::Measure(VarId(1)),
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::DurationBothSides { index: 0 }
    ));
}

#[test]
fn rejects_duration_against_a_non_u64_side() {
    // Posting.amount is I64: the measure's value side is u64.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            vec![(0, var(0)), (2, var(2)), (SPAN, var(1))],
        )],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Gt,
            lhs: Term::Measure(VarId(1)),
            rhs: var(2),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::IllegalComparison { index: 0 }
    ));
}

#[test]
fn rejects_a_duration_fold_over_a_group_key_variable() {
    // Duration(v) projected makes v a group-key variable, so a fold over
    // its measure is constant per group.
    let query = simple(
        vec![
            FindTerm::Measure(VarId(1)),
            FindTerm::AggregateMeasure {
                op: AggOp::Sum,
                over: VarId(1),
            },
        ],
        vec![atom(POSTING, vec![(0, var(0)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateOverGroupKey { find: 1 }
    ));
}

#[test]
fn rejects_a_comparison_only_duration_variable() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(0))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Gt,
            lhs: Term::Measure(VarId(9)),
            rhs: Term::Literal(Value::U64(5)),
        })],
    });
    assert!(matches!(
        expect_err(&query),
        ValidationError::ComparisonOnlyVariable { var: VarId(9) }
    ));
}

#[test]
fn rejects_a_second_pack_term() {
    // The multi-Pack product has no sighting — refused with its trigger
    // recorded on the error variant.
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(2)),
            },
        ],
        vec![
            atom(POSTING, vec![(1, var(0)), (SPAN, var(1))]),
            atom(ACCOUNT, vec![(1, var(0)), (VALIDITY, var(2))]),
        ],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::MultiplePackTerms { find: 2 }
    ));
}

#[test]
fn rejects_pack_beside_a_fold_aggregate() {
    // Pack is relation-shaped: a fold column repeated per segment row is
    // refused — coalesced-time accounting is two queries or a host fold.
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        vec![atom(POSTING, vec![(1, var(0)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::MixedPackAndFold { find: 2 }
    ));
}

#[test]
fn rejects_pack_beside_a_measure_fold() {
    // The AggregateMeasure form is a fold too — Sum∘Duration∘Pack in
    // one head stays two queries (the composition refusal).
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::AggregateMeasure {
                op: AggOp::Sum,
                over: VarId(1),
            },
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        vec![atom(POSTING, vec![(1, var(0)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::MixedPackAndFold { find: 2 }
    ));
}

#[test]
fn rejects_pack_beside_arg_terms() {
    let query = simple(
        vec![
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(2) },
                over: Some(VarId(2)),
            },
        ],
        vec![atom(POSTING, vec![(3, var(2)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::MixedPackAndArg { find: 1 }
    ));
}

#[test]
fn rejects_pack_over_a_non_interval_variable() {
    // Posting.amount is I64: the coalesce is defined by the interval
    // point-set denotation and by nothing else.
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        vec![atom(POSTING, vec![(1, var(0)), (2, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::PackInputType { find: 1 }
    ));
}

#[test]
fn rejects_pack_without_a_variable() {
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: None,
            },
        ],
        vec![atom(POSTING, vec![(1, var(0)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateWithoutVariable { find: 1 }
    ));
}

#[test]
fn rejects_pack_over_a_group_key_variable() {
    // The packed variable projected plain makes it a group key; packing
    // it too would coalesce a constant per group.
    let query = simple(
        vec![
            FindTerm::Var(VarId(1)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        vec![atom(POSTING, vec![(1, var(0)), (SPAN, var(1))])],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::AggregateOverGroupKey { find: 1 }
    ));
}
