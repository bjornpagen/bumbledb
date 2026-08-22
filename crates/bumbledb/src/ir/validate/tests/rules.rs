use super::*;
use crate::error::{FindIndex, Mismatch, RuleIndex};
use crate::ir::{CmpOp, Comparison, HeadTerm, MAX_RULES, ParamId, Rule, Value};

fn account_rule(var: u16) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(var))],
        atoms: vec![atom(POSTING, vec![(1, Term::Var(VarId(var)))])],
        negated: vec![],
        conditions: vec![],
    }
}

fn amount_rule(var: u16) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(var))],
        atoms: vec![atom(POSTING, vec![(2, Term::Var(VarId(var)))])],
        negated: vec![],
        conditions: vec![],
    }
}

#[test]
fn the_empty_rule_set_is_rejected() {
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![],
        rec: None,
    };
    assert_eq!(expect_err(&query), ValidationError::EmptyRuleSet);
}

#[test]
fn the_rule_cap_is_rejected_one_past_the_line() {
    let at_cap = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: (0..MAX_RULES).map(|_| account_rule(0)).collect(),
        rec: None,
    };
    validate(&schema(), &at_cap).expect("MAX_RULES rules validate");
    let over = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: (0..=MAX_RULES).map(|_| account_rule(0)).collect(),
        rec: None,
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
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![account_rule(0), wide],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadArityMismatch {
            rule: RuleIndex(1),
            mismatch: Mismatch {
                witnessed: 2,
                required: 1,
            },
        }
    );
}

#[test]
fn head_aggregate_mismatch_names_the_position() {
    let counting = Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![atom(POSTING, vec![(1, Term::Var(VarId(0)))])],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![account_rule(0), counting],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadAggregateMismatch {
            rule: RuleIndex(1),
            position: FindIndex(0)
        }
    );
}

#[test]
fn head_aggregate_op_kind_mismatch_is_the_same_error() {
    let agg = |op| Rule {
        finds: vec![FindTerm::Aggregate { op, over: VarId(0) }],
        atoms: vec![atom(POSTING, vec![(2, Term::Var(VarId(0)))])],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Aggregate(crate::ir::HeadOp::Sum)],
        rules: vec![agg(crate::ir::FoldOp::Sum), agg(crate::ir::FoldOp::Min)],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadAggregateMismatch {
            rule: RuleIndex(1),
            position: FindIndex(0)
        }
    );
}

#[test]
fn head_type_mismatch_names_rule_and_position() {
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![account_rule(0), amount_rule(0)],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadTypeMismatch {
            rule: RuleIndex(1),
            position: FindIndex(0)
        }
    );
}

#[test]
fn variables_are_rule_scoped_so_one_var_id_may_differ_in_type() {
    let second = Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Var(VarId(1))), (2, Term::Var(VarId(0)))],
        )],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![account_rule(0), second],
        rec: None,
    };
    let witness = validate(&schema(), &query).expect("per-rule scopes validate");
    assert_eq!(witness.rule(0).var_type(VarId(0)), &ValueType::U64);
    assert_eq!(witness.rule(1).var_type(VarId(0)), &ValueType::I64);
    let types: Vec<ValueType> = witness
        .signature()
        .columns
        .iter()
        .map(|column| *column.ty())
        .collect();
    assert_eq!(types, vec![ValueType::U64]);
}

#[test]
fn params_are_query_global_and_unify_across_rules() {
    let with_param = |field: u16, var: u16| Rule {
        finds: vec![FindTerm::Var(VarId(var))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Var(VarId(var))), (field, Term::Param(ParamId(0)))],
        )],
        negated: vec![],
        conditions: vec![],
    };

    let agree = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![with_param(2, 0), with_param(3, 0)],
        rec: None,
    };
    validate(&schema(), &agree).expect("agreeing anchors validate");
    let conflict = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![with_param(2, 0), with_param(5, 0)],
        rec: None,
    };
    assert_eq!(
        expect_err(&conflict),
        ValidationError::ParamTypeConflict { param: ParamId(0) }
    );
}

#[test]
fn param_density_is_judged_across_the_whole_program() {
    let with_param = |param: u16| Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            vec![(1, Term::Var(VarId(0))), (2, Term::Param(ParamId(param)))],
        )],
        negated: vec![],
        conditions: vec![],
    };
    let dense = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![with_param(0), with_param(1)],
        rec: None,
    };
    validate(&schema(), &dense).expect("jointly dense param ids validate");
    let gapped = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![with_param(0), with_param(2)],
        rec: None,
    };
    assert_eq!(
        expect_err(&gapped),
        ValidationError::ParamIdGap { param: ParamId(1) }
    );
}

#[test]
fn the_single_rule_program_is_the_degenerate_case() {
    let rule = account_rule(0);
    let explicit = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![rule.clone()],
        rec: None,
    };
    let sugar = Query::single(rule);
    assert_eq!(explicit, sugar);
    let schema = schema();
    let a = validate(&schema, &explicit).expect("valid");
    let b = validate(&schema, &sugar).expect("valid");
    assert_eq!(format!("{a:?}"), format!("{b:?}"), "byte-identical witness");
    assert_eq!(a.rules().count(), 1);
}

fn amount_leaf(op: CmpOp, literal: i64) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op,
        lhs: Term::Var(VarId(0)),
        rhs: Term::Literal(Value::I64(literal)),
    })
}

fn amount_tree_rule(conditions: Vec<ConditionTree>) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(2, Term::Var(VarId(0)))])],
        negated: vec![],
        conditions,
    }
}

#[test]
fn dnf_distributes_or_pairs_to_four_rules() {
    let query = Query::single(amount_tree_rule(vec![
        ConditionTree::Or(vec![amount_leaf(CmpOp::Gt, 1), amount_leaf(CmpOp::Gt, 2)]),
        ConditionTree::Or(vec![amount_leaf(CmpOp::Lt, 8), amount_leaf(CmpOp::Lt, 9)]),
    ]));
    let witness = validate(&schema(), &query).expect("distributes and validates");
    assert_eq!(witness.rules().count(), 4);
    for rule in witness.rules() {
        assert_eq!(
            rule.rule().conditions.len(),
            2,
            "one leaf from each disjunction"
        );
    }
}

/// Distribution past the cap is the typed error naming the blowup: five two-arm
/// disjunctions produce 2⁵ = 32 rules against the cap of 16 — judged on the
/// structural count, before any disjunct materializes.
#[test]
fn dnf_blowup_past_the_cap_is_typed_with_the_count() {
    let disjunction = |lo: i64| {
        ConditionTree::Or(vec![
            amount_leaf(CmpOp::Gt, lo),
            amount_leaf(CmpOp::Lt, lo + 100),
        ])
    };
    let query = Query::single(amount_tree_rule(
        (0..5).map(|i| disjunction(i64::from(i))).collect(),
    ));
    assert_eq!(
        expect_err(&query),
        ValidationError::DnfExceedsRules {
            exceeded: crate::error::Exceeded {
                observed: 32,
                ceiling: MAX_RULES,
            },
        }
    );
}

/// Duplicate rules after distribution collapse by normalized-form equality: `(a
/// ∨ a)` yields one rule, and `(a ∨ b) ∧ (b ∨ a)` yields three — `[a, b]` and
/// `[b, a]` are one normalized body (a conjunction is a set), while `[a, a]`
/// and `[b, b]` each survive.
#[test]
fn duplicate_rules_after_distribution_collapse() {
    let a = || amount_leaf(CmpOp::Gt, 0);
    let b = || amount_leaf(CmpOp::Lt, 9);
    let same_twice = Query::single(amount_tree_rule(vec![ConditionTree::Or(vec![a(), a()])]));
    let witness = validate(&schema(), &same_twice).expect("valid");
    assert_eq!(witness.rules().count(), 1);

    let permuted = Query::single(amount_tree_rule(vec![
        ConditionTree::Or(vec![a(), b()]),
        ConditionTree::Or(vec![b(), a()]),
    ]));
    let witness = validate(&schema(), &permuted).expect("valid");
    assert_eq!(witness.rules().count(), 3);
}

#[test]
fn empty_and_nested_trees_lower_algebraically() {
    let empty_and = Query::single(amount_tree_rule(vec![ConditionTree::And(vec![])]));
    let witness = validate(&schema(), &empty_and).expect("the empty conjunction is true");
    assert_eq!(witness.rules().count(), 1);
    assert!(witness.rule(0).rule().conditions.is_empty());

    let empty_or = Query::single(amount_tree_rule(vec![ConditionTree::Or(vec![])]));
    assert_eq!(expect_err(&empty_or), ValidationError::EmptyRuleSet);

    let nested = Query::single(amount_tree_rule(vec![ConditionTree::Or(vec![
        ConditionTree::And(vec![amount_leaf(CmpOp::Gt, 0), amount_leaf(CmpOp::Lt, 9)]),
        amount_leaf(CmpOp::Eq, 5),
    ])]));
    let witness = validate(&schema(), &nested).expect("valid");
    let widths: Vec<usize> = witness
        .rules()
        .map(|rule| rule.rule().conditions.len())
        .collect();
    assert_eq!(widths, vec![2, 1]);
}
