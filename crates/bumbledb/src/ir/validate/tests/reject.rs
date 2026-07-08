use super::*;
use crate::ir::{AggOp, CmpOp, Comparison, Value};

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
fn rejects_enum_ordinal_out_of_range() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            ACCOUNT,
            vec![(0, var(0)), (2, Term::Literal(Value::Enum(2)))], // 2 variants
        )],
    );
    assert!(matches!(
        expect_err(&query),
        ValidationError::EnumOrdinalOutOfRange {
            atom: 0,
            field: FieldId(2),
            ordinal: 2
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
fn rejects_order_comparison_on_non_integer() {
    // Holder.name is a String: Lt is illegal (equality-only type).
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(1)), (1, var(0))])],
        predicates: vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: Term::Literal(Value::String(Box::from(&b"x"[..]))),
        }],
    };
    assert!(matches!(
        expect_err(&query),
        ValidationError::IllegalComparison { index: 0 }
    ));
}

#[test]
fn rejects_self_comparison() {
    // x < x is constant-valued: write the query you mean.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        predicates: vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(0),
        }],
    };
    let err = validate(&schema(), &query).unwrap_err();
    assert!(matches!(err, ValidationError::SelfComparison { index: 0 }));
}

#[test]
fn rejects_order_operators_on_bool_and_enum() {
    // Posting.flag is Bool (field 5); Account.status is Enum (field 2).
    for (rel, field) in [(POSTING, 5u16), (ACCOUNT, 2u16)] {
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                atom(rel, vec![(field, var(0)), (0, var(1))]),
                atom(rel, vec![(field, var(2)), (0, var(3))]),
            ],
            predicates: vec![Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: var(2),
            }],
        };
        let err = validate(&schema(), &query).unwrap_err();
        assert!(
            matches!(err, ValidationError::IllegalComparison { index: 0 }),
            "order ops are integer-only; got {err:?}"
        );
    }
}

#[test]
fn enum_ordinal_in_a_comparison_reports_the_precise_variant() {
    // Account.status has 2 variants; ordinal 9 is out of range.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(2, var(0))])],
        predicates: vec![Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: Term::Literal(Value::Enum(9)),
        }],
    };
    let err = validate(&schema(), &query).unwrap_err();
    assert!(matches!(
        err,
        ValidationError::ComparisonEnumOrdinalOutOfRange {
            index: 0,
            ordinal: 9
        }
    ));
}

#[test]
fn rejects_duplicate_aggregate_find_terms() {
    let query = Query {
        finds: vec![
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        predicates: vec![],
    };
    let err = validate(&schema(), &query).unwrap_err();
    assert!(matches!(
        err,
        ValidationError::DuplicateFindTerm { index: 1 }
    ));
}

#[test]
fn rejects_cross_type_comparison() {
    // U64 var vs I64 var: no silent coercion, ever.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(1, var(0)), (2, var(1))])],
        predicates: vec![Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: var(1),
        }],
    };
    assert!(matches!(
        expect_err(&query),
        ValidationError::IllegalComparison { index: 0 }
    ));
}

#[test]
fn rejects_constant_comparison() {
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        predicates: vec![Comparison {
            op: CmpOp::Eq,
            lhs: Term::Literal(Value::U64(1)),
            rhs: Term::Param(ParamId(0)),
        }],
    };
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
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        predicates: vec![Comparison {
            op: CmpOp::Eq,
            lhs: var(9), // appears in no atom
            rhs: var(0),
        }],
    };
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
}

#[test]
fn rejects_no_atoms() {
    let query = simple(vec![FindTerm::Var(VarId(0))], vec![]);
    assert!(matches!(expect_err(&query), ValidationError::NoAtoms));
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
fn param_anchoring_is_total_by_construction() {
    // An unanchored param is unwritable: a param in an atom binding is
    // typed by its field; a param in a comparison is typed by the
    // variable side (a variable-free comparison is already
    // `ConstantComparison`). This pins the anchored case; the roster
    // item is discharged by representation, not by a check.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        predicates: vec![Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: Term::Param(ParamId(0)),
        }],
    };
    let witness = validate(&schema(), &query).expect("valid");
    assert_eq!(
        witness.param_types().next(),
        Some((ParamId(0), &ValueType::U64))
    );
}

#[test]
fn rejects_sparse_param_ids() {
    // ?1 without ?0: the gap would be an unchecked positional slot.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            HOLDER,
            vec![(0, var(0)), (1, Term::Param(ParamId(1)))],
        )],
        predicates: vec![],
    };
    let err = validate(&schema(), &query).unwrap_err();
    assert!(matches!(err, ValidationError::ParamIdGap { param } if param.0 == 0));
}

#[test]
fn rejects_more_atoms_than_the_planner_cap_at_the_boundary() {
    let over = crate::plan::planner::MAX_OCCURRENCES + 1;
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: (0..over).map(|_| atom(HOLDER, vec![(0, var(0))])).collect(),
        predicates: vec![],
    };
    let err = validate(&schema(), &query).unwrap_err();
    assert!(matches!(err, ValidationError::TooManyAtoms { count } if count == over));
}

#[test]
fn rejects_more_distinct_variables_than_the_bitset_at_the_boundary() {
    // One 129-field relation binds 129 fresh variables in a single
    // atom — past the executor's 128-bit variable bitsets.
    let wide = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Wide".into(),
            fields: (0..129)
                .map(|i| FieldDescriptor {
                    name: format!("f{i}").into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                })
                .collect(),
            constraints: vec![],
        }],
    }
    .validate()
    .expect("wide fixture");
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![crate::ir::Atom {
            relation: RelationId(0),
            bindings: (0..129u16).map(|i| (FieldId(i), var(i))).collect(),
        }],
        predicates: vec![],
    };
    let err = validate(&wide, &query).unwrap_err();
    assert!(matches!(
        err,
        ValidationError::TooManyVariables { count: 129 }
    ));
}
