use super::*;
use crate::ir::FoldOp;
use crate::ir::{CmpOp, Comparison, Value};
use bumbledb_theory::schema::FixedIntervalElement;

#[test]
fn accepts_the_containment_walk_join_with_conditions() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            atom(POSTING, vec![(1, var(0)), (2, var(1)), (3, var(2))]),
            atom(ACCOUNT, vec![(0, var(0))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(2),
            rhs: Term::Literal(Value::I64(100)),
        })],
    });
    let witness = validate(&schema(), &query).expect("valid");
    assert_eq!(witness.rule(0).var_type(VarId(0)), &ValueType::U64);
    assert_eq!(witness.rule(0).var_type(VarId(2)), &ValueType::I64);
    assert_eq!(witness.rule(0).group_key().len(), 1);
}

#[test]
fn accepts_params_anchored_by_fields_and_comparisons() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Param(ParamId(0))), (0, var(0)), (3, var(1))],
        )],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(1),
            rhs: Term::Param(ParamId(1)),
        })],
    });
    let witness = validate(&schema(), &query).expect("valid");
    let params: Vec<_> = witness.param_types().collect();
    assert_eq!(params[0], (ParamId(0), &ValueType::U64));
    assert_eq!(params[1], (ParamId(1), &ValueType::I64));
}

#[test]
fn param_anchoring_is_total_by_construction() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: Term::Param(ParamId(0)),
        })],
    });
    let witness = validate(&schema(), &query).expect("valid");
    assert_eq!(
        witness.param_types().next(),
        Some((ParamId(0), &ValueType::U64))
    );
}

#[test]
fn accepts_all_aggregate_finds() {
    let query = simple(
        vec![
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(0),
            },
            FindTerm::Count,
        ],
        vec![atom(POSTING, vec![(2, var(0))])],
    );
    let witness = validate(&schema(), &query).expect("valid");
    assert!(witness.rule(0).group_key().is_empty());
}

#[test]
fn accepts_zero_binding_atoms() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(POSTING, vec![(0, var(0))]), atom(HOLDER, vec![])],
    );
    validate(&schema(), &query).expect("valid");
}

#[test]
fn accepts_repeated_variable_within_one_atom() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(POSTING, vec![(2, var(0)), (3, var(0))])],
    );
    validate(&schema(), &query).expect("valid");
}

#[test]
fn accepts_membership_bound_variable_with_a_scalar_binding_elsewhere() {
    let query = simple(
        vec![FindTerm::Var(VarId(1))],
        vec![
            atom(POSTING, vec![(0, var(0)), (SPAN, var(1))]),
            atom(ACCOUNT, vec![(0, var(1))]),
        ],
    );
    let witness = validate(&schema(), &query).expect("valid");
    assert_eq!(witness.rule(0).var_type(VarId(1)), &ValueType::U64);
}

#[test]
fn accepts_a_variable_joined_across_two_interval_fields() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![
            atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))]),
            atom(POSTING, vec![(0, var(2)), (SPAN, var(1))]),
        ],
    );
    let witness = validate(&schema(), &query).expect("valid");
    assert_eq!(
        witness.rule(0).var_type(VarId(1)),
        &ValueType::Interval {
            element: IntervalElement::U64
        }
    );
}

#[test]
fn accepts_an_element_literal_in_an_interval_field_position() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            ACCOUNT,
            vec![(0, var(0)), (VALIDITY, Term::Literal(Value::U64(7)))],
        )],
    );
    validate(&schema(), &query).expect("valid");
}

#[test]
fn accepts_a_ray_literal_and_the_last_point() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            ACCOUNT,
            vec![
                (0, var(0)),
                (
                    VALIDITY,
                    Term::Literal(Value::IntervalU64(
                        bumbledb_theory::Interval::<u64>::new(5, u64::MAX)
                            .expect("nonempty interval"),
                    )),
                ),
            ],
        )],
    );
    validate(&schema(), &query).expect("a ray literal is a value");
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            ACCOUNT,
            vec![
                (0, var(0)),
                (VALIDITY, Term::Literal(Value::U64(u64::MAX - 1))),
            ],
        )],
    );
    validate(&schema(), &query).expect("MAX-1 is a point");
}

#[test]
fn point_params_are_the_element_typed_interval_position_params() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![
            atom(POSTING, vec![(0, var(0)), (SPAN, Term::Param(ParamId(0)))]),
            atom(ACCOUNT, vec![(0, Term::Param(ParamId(0)))]),
            atom(
                ACCOUNT,
                vec![(0, var(0)), (VALIDITY, Term::Param(ParamId(1)))],
            ),
        ],
    );
    let witness = validate(&schema(), &query).expect("valid");
    assert!(witness.point_params().contains(&ParamId(0)));
    assert!(!witness.point_params().contains(&ParamId(1)));
}

#[test]
fn accepts_a_zero_binding_negated_atom_as_an_emptiness_gate() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        negated: vec![atom(POSTING, vec![])],
        conditions: vec![],
    });
    validate(&schema(), &query).expect("valid");
}

#[test]
fn accepts_literals_params_and_sets_inside_negated_atoms() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ACCOUNT, vec![(0, var(0))])],
        negated: vec![atom(
            POSTING,
            vec![
                (1, var(0)),
                (SPAN, Term::Param(ParamId(0))),
                (4, Term::ParamSet(ParamId(1))),
            ],
        )],
        conditions: vec![],
    });
    let witness = validate(&schema(), &query).expect("valid");
    let params: Vec<_> = witness.param_types().collect();
    assert_eq!(
        params[0],
        (
            ParamId(0),
            &ValueType::Interval {
                element: IntervalElement::U64
            }
        )
    );
    assert_eq!(params[1], (ParamId(1), &ValueType::FixedBytes { len: 32 }));
    assert!(witness.set_params().contains(&ParamId(1)));
    assert!(!witness.set_params().contains(&ParamId(0)));
}

#[test]
fn accepts_param_sets_in_bindings_and_under_eq() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            ACCOUNT,
            vec![(0, var(0)), (1, Term::ParamSet(ParamId(0)))],
        )],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: Term::ParamSet(ParamId(1)),
        })],
    });
    let witness = validate(&schema(), &query).expect("valid");
    let params: Vec<_> = witness.param_types().collect();
    assert_eq!(params[0], (ParamId(0), &ValueType::U64));
    assert_eq!(params[1], (ParamId(1), &ValueType::U64));
    assert_eq!(witness.set_params().len(), 2);
}

#[test]
fn accepts_pack_and_pins_the_interval_result_type() {
    let query = simple(
        vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        vec![atom(POSTING, vec![(1, var(0)), (SPAN, var(1))])],
    );
    let witness = validate(&schema(), &query).expect("valid");
    let types: Vec<ValueType> = witness
        .signature()
        .columns
        .iter()
        .map(|column| *column.ty())
        .collect();
    assert_eq!(
        types,
        vec![
            ValueType::U64,
            ValueType::Interval {
                element: IntervalElement::U64
            }
        ]
    );
}

#[test]
fn accepts_pack_across_rules() {
    let rule = |atoms: Vec<crate::ir::Atom>| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms,
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![
            crate::ir::HeadTerm::Var,
            crate::ir::HeadTerm::Aggregate(crate::ir::HeadOp::Pack),
        ],
        rules: vec![
            rule(vec![atom(POSTING, vec![(1, var(0)), (SPAN, var(1))])]),
            rule(vec![atom(ACCOUNT, vec![(1, var(0)), (VALIDITY, var(1))])]),
        ],
        rec: None,
    };
    validate(&schema(), &query).expect("valid");
}

fn mixed_width_schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Zone".into(),
            fields: vec![
                field("id", ValueType::U64),
                field(
                    "span",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                ),
                field(
                    "lane",
                    ValueType::FixedInterval {
                        element: FixedIntervalElement::U64,
                        width: 5,
                    },
                ),
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

#[test]
fn accepts_a_mixed_width_allen_pair_of_one_element_domain() {
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
    validate(&mixed_width_schema(), &query).expect("mixed widths of one element classify");
}

#[test]
fn accepts_a_general_interval_literal_allen_against_a_fixed_width_var() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(RelationId(0), vec![(0, var(0)), (2, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: bumbledb_theory::allen::AllenMask::INTERSECTS,
            },
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(3, 40).expect("nonempty"),
            )),
        })],
    });
    validate(&mixed_width_schema(), &query).expect("a general literal against a fixed var");
}
