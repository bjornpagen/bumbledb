use super::*;
use crate::error::FindIndex;
use crate::ir::FoldOp;
use crate::ir::{CmpOp, Comparison, Value};

// --- Accepting shapes ---

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
    // An unanchored param is unwritable: a param in an atom binding is
    // typed by its field; a param in a comparison is typed by the
    // variable side (a variable-free comparison is already
    // `ConstantComparison`). This pins the anchored case; the roster
    // item is discharged by representation, not by a check.
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
    // Empty group key, one global group — legal per the doc.
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
fn accepts_min_max_over_bool_as_all_and_any() {
    // The quantifiers fall out free (ruled 2026-07-23, R3): bool orders
    // false < true, so `Max(flag)` is Any and `Min(flag)` is All — the
    // documented idiom, true at the validation boundary. Sum over bool
    // stays refused: a quantifier is not an addition.
    for op in [FoldOp::Min, FoldOp::Max] {
        let query = Query::single(Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate { op, over: VarId(1) },
            ],
            atoms: vec![atom(POSTING, vec![(1, var(0)), (5, var(1))])],
            negated: vec![],
            conditions: vec![],
        });
        let witness = validate(&schema(), &query).expect("bool folds under Min/Max");
        assert_eq!(witness.rule(0).var_type(VarId(1)), &ValueType::Bool);
    }
    let sum = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![atom(POSTING, vec![(1, var(0)), (5, var(1))])],
        negated: vec![],
        conditions: vec![],
    });
    assert!(matches!(
        validate(&schema(), &sum).expect_err("Sum over bool refuses"),
        ValidationError::AggregateInputType { find: FindIndex(1) }
    ));
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

// --- Negation, param sets, and the new aggregates ---

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
    // Account(holder ∈ ?set0, id = x), Eq(x, ?set1): both legal set
    // positions; each set's type is its element type.
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
    // finds [account, Pack(span)]: the coalescing fold — the result
    // position is interval-typed (a packed segment shares its input's
    // type), sealed in the signature.
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
    // Pack folds the union (unlike Arg-restriction, whose key is
    // rule-scoped): two rules over one Pack head are legal — the fold
    // domain is the union of the rules' claims projected to the head.
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

// --- Q1: element-domain typing at interval comparison positions ---

/// Zone(id u64, span interval<u64>, lane interval<u64, 5>) — the local
/// mixed-width fixture (the shared fixture carries general intervals
/// only).
fn mixed_width_schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
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
                        element: IntervalElement::U64,
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

/// Q1's Allen rule: a fixed-width term against a general term of ONE
/// element domain classifies — the comparison runs over derived
/// bounds, which carry an element domain and never a width
/// (`docs/architecture/30-dependencies.md` § Q1; the u64-vs-i64 twin
/// still rejects, `reject.rs`).
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
            lhs: var(1), // interval<u64>
            rhs: var(2), // interval<u64, 5>
        })],
    });
    validate(&mixed_width_schema(), &query).expect("mixed widths of one element classify");
}

/// The constant side of Q1's Allen rule: an interval literal spells
/// both bounds and anchors the GENERAL type, so it classifies against
/// a fixed-width variable of the same element domain.
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
            lhs: var(1), // interval<u64, 5>
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(3, 40).expect("nonempty"),
            )),
        })],
    });
    validate(&mixed_width_schema(), &query).expect("a general literal against a fixed var");
}
