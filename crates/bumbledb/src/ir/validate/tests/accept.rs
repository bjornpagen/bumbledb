use super::*;
use crate::ir::{AggOp, CmpOp, Comparison, Value};

// --- Accepting shapes ---

#[test]
fn accepts_the_fk_walk_join_with_predicates() {
    let query = Query {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            atom(POSTING, vec![(1, var(0)), (2, var(1)), (3, var(2))]),
            atom(ACCOUNT, vec![(0, var(0))]),
        ],
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Ge,
            lhs: var(2),
            rhs: Term::Literal(Value::I64(100)),
        }],
    };
    let witness = validate(&schema(), &query).expect("valid");
    assert_eq!(witness.var_type(VarId(0)), &ValueType::U64);
    assert_eq!(witness.var_type(VarId(2)), &ValueType::I64);
    assert_eq!(witness.group_key().len(), 1);
}

#[test]
fn accepts_params_anchored_by_fields_and_comparisons() {
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Param(ParamId(0))), (0, var(0)), (3, var(1))],
        )],
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(1),
            rhs: Term::Param(ParamId(1)),
        }],
    };
    let witness = validate(&schema(), &query).expect("valid");
    let params: Vec<_> = witness.param_types().collect();
    assert_eq!(params[0], (ParamId(0), &ValueType::U64));
    assert_eq!(params[1], (ParamId(1), &ValueType::I64));
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
    assert!(witness.group_key().is_empty());
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
