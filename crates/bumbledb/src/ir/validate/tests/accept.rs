use super::*;
use crate::ir::{AggOp, CmpOp, Comparison, MaskTerm, Value};

// --- Accepting shapes ---

#[test]
fn accepts_the_containment_walk_join_with_predicates() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            atom(POSTING, vec![(1, var(0)), (2, var(1)), (3, var(2))]),
            atom(ACCOUNT, vec![(0, var(0))]),
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
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
        predicates: vec![PredicateTree::Leaf(Comparison {
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
    // An unanchored param is unwritable: a param in an atom binding is
    // typed by its field; a param in a comparison is typed by the
    // variable side (a variable-free comparison is already
    // `ConstantComparison`). This pins the anchored case; the roster
    // item is discharged by representation, not by a check.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
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
    // Empty group key, one global group — legal per the doc.
    let query = simple(
        vec![
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(0)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
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
        vec![
            atom(POSTING, vec![(0, var(0))]),
            atom(HOLDER, vec![]), // nonemptiness gate
        ],
    );
    validate(&schema(), &query).expect("valid");
}

#[test]
fn accepts_repeated_variable_within_one_atom() {
    // Same-fact equality: amount == at (both I64).
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(POSTING, vec![(2, var(0)), (3, var(0))])],
    );
    validate(&schema(), &query).expect("valid");
}

// --- The four accept cases pinning the bivalent-anchor typing rule ---

#[test]
fn accepts_membership_bound_variable_with_a_scalar_binding_elsewhere() {
    // (a) t ∈ Posting.span, t = Account.id: the scalar field is the
    // monovalent anchor — t is element-typed, the span binding is
    // membership, and Account.id is the enumerable domain.
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
    // (b) v in Account.validity and Posting.span: every anchor is
    // bivalent, so v resolves to the interval type — a value-equality
    // join, not membership.
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
    // (c) 7 ∈ Account.validity: an element-typed literal in an interval
    // field is a membership filter.
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
    // The point-domain law's legal side: `[5, MAX)` is the ray `[5, ∞)` —
    // an honest interval value — and `MAX−1` is the last point.
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(
            ACCOUNT,
            vec![
                (0, var(0)),
                (VALIDITY, Term::Literal(Value::IntervalU64(5, u64::MAX))),
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
    // ?0 meets Posting.span (membership — element-anchored by
    // Account.id) and is a point param; ?1 meets Account.validity with
    // only bivalent anchors, resolves interval-typed (value equality),
    // and is not.
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
fn accepts_allen_between_interval_variables_from_different_atoms() {
    // (d) Allen(v1, v3, INTERSECTS): the interval-pair comparison needs
    // no shared point variable — both vars stay bivalent and resolve to
    // intervals. Both mask forms are exercised: literal and param.
    let masks = [
        MaskTerm::Literal(crate::allen::AllenMask::INTERSECTS),
        MaskTerm::Param(ParamId(0)),
    ];
    for mask in masks {
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))]),
                atom(POSTING, vec![(0, var(2)), (SPAN, var(3))]),
            ],
            negated: vec![],
            predicates: vec![PredicateTree::Leaf(Comparison {
                op: CmpOp::Allen { mask },
                lhs: var(1),
                rhs: var(3),
            })],
        });
        let witness = validate(&schema(), &query).expect("valid");
        let interval = ValueType::Interval {
            element: IntervalElement::U64,
        };
        assert_eq!(witness.rule(0).var_type(VarId(1)), &interval);
        assert_eq!(witness.rule(0).var_type(VarId(3)), &interval);
        if let MaskTerm::Param(param) = mask {
            assert!(witness.mask_params().contains(&param));
        }
    }
}

// --- Negation, param sets, and the new aggregates ---

#[test]
fn accepts_a_zero_binding_negated_atom_as_an_emptiness_gate() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(HOLDER, vec![(0, var(0))])],
        negated: vec![atom(POSTING, vec![])],
        predicates: vec![],
    });
    validate(&schema(), &query).expect("valid");
}

#[test]
fn accepts_literals_params_and_sets_inside_negated_atoms() {
    // ¬Posting(account = a, span = ?0, memo ∈ ?set1): the negated atom's
    // interval-field param has only bivalent anchors, so it resolves to
    // the interval type (value equality); the set anchors at Bytes.
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
        predicates: vec![],
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
    assert_eq!(params[1], (ParamId(1), &ValueType::Bytes));
    assert!(witness.set_params().contains(&ParamId(1)));
    assert!(!witness.set_params().contains(&ParamId(0)));
}

#[test]
fn accepts_param_sets_in_bindings_and_under_eq() {
    // Account(holder ∈ ?set0, id = x), Eq(x, ?set1): both legal set
    // positions; each set's type is its element type.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            ACCOUNT,
            vec![(0, var(0)), (1, Term::ParamSet(ParamId(0)))],
        )],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
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
fn accepts_count_distinct_over_every_type() {
    // CountDistinct over a String variable — equality is all it needs.
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::CountDistinct,
                over: Some(VarId(1)),
            },
        ],
        vec![atom(HOLDER, vec![(0, var(0)), (1, var(1))])],
    );
    validate(&schema(), &query).expect("valid");
}

#[test]
fn accepts_arg_restriction_with_a_projected_key() {
    // finds [at, ArgMax_{at}(memo)]: the key variable may itself be
    // projected; the carry rides with the attaining bindings.
    let query = simple(
        vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::ArgMax { key: VarId(0) },
                over: Some(VarId(1)),
            },
        ],
        vec![atom(POSTING, vec![(3, var(0)), (4, var(1))])],
    );
    validate(&schema(), &query).expect("valid");
}

#[test]
fn accepts_an_arg_carry_equal_to_its_key() {
    // over = the carry, and it may equal the key: ArgMax_{at}(at).
    let query = simple(
        vec![FindTerm::Aggregate {
            op: AggOp::ArgMax { key: VarId(0) },
            over: Some(VarId(0)),
        }],
        vec![atom(POSTING, vec![(3, var(0)), (1, var(1))])],
    );
    validate(&schema(), &query).expect("valid");
}
