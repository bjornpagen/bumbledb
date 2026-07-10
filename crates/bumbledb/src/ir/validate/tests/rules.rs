//! The program-shape roster: empty rule set, the rule cap, head
//! alignment (arity, shape, type), per-rule variable scoping, and
//! query-global params — the rules-IR additions, each with its typed
//! error (`docs/architecture/20-query-ir.md`, the rules shape).

use super::*;
use crate::ir::{HeadTerm, ParamId, Rule, MAX_RULES};

/// A one-atom rule projecting Posting.account (U64) as `Var(var)`.
fn account_rule(var: u16) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(var))],
        atoms: vec![atom(POSTING, vec![(1, Term::Var(VarId(var)))])],
        negated: vec![],
        predicates: vec![],
    }
}

/// A one-atom rule projecting Posting.amount (I64) as `Var(var)`.
fn amount_rule(var: u16) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(var))],
        atoms: vec![atom(POSTING, vec![(2, Term::Var(VarId(var)))])],
        negated: vec![],
        predicates: vec![],
    }
}

#[test]
fn the_empty_rule_set_is_rejected() {
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![],
    };
    assert_eq!(expect_err(&query), ValidationError::EmptyRuleSet);
}

#[test]
fn the_rule_cap_is_rejected_one_past_the_line() {
    let at_cap = Query {
        head: vec![HeadTerm::Var],
        rules: (0..MAX_RULES).map(|_| account_rule(0)).collect(),
    };
    validate(&schema(), &at_cap).expect("MAX_RULES rules validate");
    let over = Query {
        head: vec![HeadTerm::Var],
        rules: (0..=MAX_RULES).map(|_| account_rule(0)).collect(),
    };
    assert_eq!(
        expect_err(&over),
        ValidationError::TooManyRules {
            count: MAX_RULES + 1
        }
    );
}

#[test]
fn head_arity_mismatch_names_the_rule() {
    let wide = Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Var(VarId(0))), (2, Term::Var(VarId(1)))],
        )],
        negated: vec![],
        predicates: vec![],
    };
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![account_rule(0), wide],
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadArityMismatch {
            rule: 1,
            expected: 1,
            found: 2
        }
    );
}

#[test]
fn head_aggregate_mismatch_names_the_position() {
    // A variable where the head names a variable — but the second rule
    // projects an aggregate at that position.
    let counting = Rule {
        finds: vec![FindTerm::Aggregate {
            op: crate::ir::AggOp::Count,
            over: None,
        }],
        atoms: vec![atom(POSTING, vec![(1, Term::Var(VarId(0)))])],
        negated: vec![],
        predicates: vec![],
    };
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![account_rule(0), counting],
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadAggregateMismatch {
            rule: 1,
            position: 0
        }
    );
}

#[test]
fn head_aggregate_op_kind_mismatch_is_the_same_error() {
    let agg = |op| Rule {
        finds: vec![FindTerm::Aggregate {
            op,
            over: Some(VarId(0)),
        }],
        atoms: vec![atom(POSTING, vec![(2, Term::Var(VarId(0)))])],
        negated: vec![],
        predicates: vec![],
    };
    let query = Query {
        head: vec![HeadTerm::Aggregate(crate::ir::HeadOp::Sum)],
        rules: vec![agg(crate::ir::AggOp::Sum), agg(crate::ir::AggOp::Min)],
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadAggregateMismatch {
            rule: 1,
            position: 0
        }
    );
}

#[test]
fn head_type_mismatch_names_rule_and_position() {
    // Rule 0 pins position 0 at U64 (Posting.account); rule 1 projects
    // I64 (Posting.amount) there.
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![account_rule(0), amount_rule(0)],
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadTypeMismatch {
            rule: 1,
            position: 0
        }
    );
}

#[test]
fn variables_are_rule_scoped_so_one_var_id_may_differ_in_type() {
    // VarId(0) is U64 in rule 0 (Posting.account) and I64 in rule 1
    // (Posting.amount, unprojected) — two variables, one id, two scopes.
    // The head stays aligned: both rules project a U64 at position 0.
    let second = Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Var(VarId(1))), (2, Term::Var(VarId(0)))],
        )],
        negated: vec![],
        predicates: vec![],
    };
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![account_rule(0), second],
    };
    let witness = validate(&schema(), &query).expect("per-rule scopes validate");
    assert_eq!(witness.rule(0).var_type(VarId(0)), &ValueType::U64);
    assert_eq!(witness.rule(1).var_type(VarId(0)), &ValueType::I64);
    assert_eq!(witness.head_types(), &[ValueType::U64]);
}

#[test]
fn params_are_query_global_and_unify_across_rules() {
    // The same ParamId anchored U64 in rule 0 and I64 in rule 1: one
    // binding surface, so the conflict is typed.
    let with_param = |field: u16, var: u16| Rule {
        finds: vec![FindTerm::Var(VarId(var))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Var(VarId(var))), (field, Term::Param(ParamId(0)))],
        )],
        negated: vec![],
        predicates: vec![],
    };
    // Agreeing anchors (amount and at are both I64) validate; amount
    // (I64) against flag (Bool) is the typed conflict.
    let agree = Query {
        head: vec![HeadTerm::Var],
        rules: vec![with_param(2, 0), with_param(3, 0)],
    };
    validate(&schema(), &agree).expect("agreeing anchors validate");
    let conflict = Query {
        head: vec![HeadTerm::Var],
        rules: vec![with_param(2, 0), with_param(5, 0)],
    };
    assert_eq!(
        expect_err(&conflict),
        ValidationError::ParamTypeConflict { param: ParamId(0) }
    );
}

#[test]
fn param_density_is_judged_across_the_whole_program() {
    // Rule 0 uses param 0, rule 1 uses param 1: dense jointly, even
    // though neither rule alone sees both ids.
    let with_param = |param: u16| Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Var(VarId(0))), (2, Term::Param(ParamId(param)))],
        )],
        negated: vec![],
        predicates: vec![],
    };
    let dense = Query {
        head: vec![HeadTerm::Var],
        rules: vec![with_param(0), with_param(1)],
    };
    validate(&schema(), &dense).expect("jointly dense param ids validate");
    let gapped = Query {
        head: vec![HeadTerm::Var],
        rules: vec![with_param(0), with_param(2)],
    };
    assert_eq!(
        expect_err(&gapped),
        ValidationError::ParamIdGap { param: ParamId(1) }
    );
}

#[test]
fn the_single_rule_program_is_the_degenerate_case() {
    // `Query::single` derives the head from the rule's own find shape;
    // an explicit head+rules spelling of the same program validates to a
    // byte-identical witness (the artifact equality the port is pinned
    // by).
    let rule = account_rule(0);
    let explicit = Query {
        head: vec![HeadTerm::Var],
        rules: vec![rule.clone()],
    };
    let sugar = Query::single(rule);
    assert_eq!(explicit, sugar);
    let schema = schema();
    let a = validate(&schema, &explicit).expect("valid");
    let b = validate(&schema, &sugar).expect("valid");
    assert_eq!(format!("{a:?}"), format!("{b:?}"), "byte-identical witness");
    assert_eq!(a.rules().count(), 1);
}
