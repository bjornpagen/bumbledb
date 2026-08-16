//! Rec roster: one linear SCC. Empty/missing-self/negation/measure
//! shapes are unrepresentable on [`Rec`]; remaining checks are DNF
//! emptiness, self-in-base, nonlinearity, head alignment, and the pool cap.

use super::*;
use crate::ir::{
    AtomSource, ConditionTree, HeadTerm, Interior, InteriorId, NonEmpty, ProjectionRule, Rec,
    RecRule, RecStep,
};

fn interior_atom(id: u32, bindings: Vec<(u16, Term)>) -> crate::ir::Atom {
    crate::ir::Atom {
        source: AtomSource::Interior(InteriorId(id)),
        bindings: bindings.into_iter().map(|(f, t)| (FieldId(f), t)).collect(),
    }
}

fn proj(finds: Vec<VarId>, atoms: Vec<crate::ir::Atom>) -> ProjectionRule {
    ProjectionRule {
        finds,
        atoms,
        negated: vec![],
        conditions: vec![],
    }
}

fn rule(finds: Vec<FindTerm>, atoms: Vec<crate::ir::Atom>) -> Rule {
    Rule {
        finds,
        atoms,
        negated: vec![],
        conditions: vec![],
    }
}

fn rec_rule(finds: Vec<VarId>, atoms: Vec<crate::ir::Atom>) -> RecRule {
    RecRule {
        finds,
        atoms,
        conditions: vec![],
    }
}

fn rec_step(
    finds: Vec<VarId>,
    self_bindings: Vec<(u16, Term)>,
    atoms: Vec<crate::ir::Atom>,
) -> RecStep {
    RecStep {
        finds,
        self_bindings: self_bindings
            .into_iter()
            .map(|(f, t)| (FieldId(f), t))
            .collect(),
        atoms,
        conditions: vec![],
    }
}

fn nonempty<T>(items: Vec<T>) -> NonEmpty<T> {
    NonEmpty::from_vec(items).expect("nonempty fixture")
}

fn reach_query(base: Vec<RecRule>, rec: Vec<RecStep>, main: Rule) -> Query {
    Query::Reach {
        interiors: vec![],
        rec: Rec {
            base: nonempty(base),
            rec: nonempty(rec),
        },
        head: main.head(),
        rules: vec![main],
    }
}

fn linear_reach() -> Query {
    reach_query(
        vec![rec_rule(
            vec![VarId(0)],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![rec_step(vec![VarId(0)], vec![(0, var(0))], vec![])],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    )
}

#[test]
fn a_linear_reach_validates() {
    validate(&schema(), &linear_reach()).expect("linear rec is executable");
}

#[test]
fn a_rec_step_whose_dnf_is_empty_is_empty_recursive_step() {
    let query = reach_query(
        vec![rec_rule(
            vec![VarId(0)],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![RecStep {
            finds: vec![VarId(0)],
            self_bindings: vec![(FieldId(0), var(0))],
            atoms: vec![],
            conditions: vec![ConditionTree::Or(vec![])],
        }],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(expect_err(&query), ValidationError::EmptyRecursiveStep);
}

#[test]
fn rejects_self_in_base() {
    let query = reach_query(
        vec![rec_rule(
            vec![VarId(0)],
            vec![interior_atom(0, vec![(0, var(0))])],
        )],
        vec![rec_step(vec![VarId(0)], vec![(0, var(0))], vec![])],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(expect_err(&query), ValidationError::SelfInBase);
}

#[test]
fn rejects_nonlinear_rec_arm() {
    let query = reach_query(
        vec![rec_rule(
            vec![VarId(0)],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![rec_step(
            vec![VarId(0)],
            vec![(0, var(0))],
            vec![interior_atom(0, vec![(0, var(1))])],
        )],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(expect_err(&query), ValidationError::NonlinearRecArm);
}

#[test]
fn a_measure_on_main_over_finished_rec_is_legal() {
    let query = Query::Reach {
        interiors: vec![],
        rec: Rec {
            base: NonEmpty::one(rec_rule(
                vec![VarId(0)],
                vec![atom(ACCOUNT, vec![(VALIDITY, var(0))])],
            )),
            rec: NonEmpty::one(rec_step(vec![VarId(0)], vec![(0, var(0))], vec![])),
        },
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Measure(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        )],
    };
    validate(&schema(), &query).expect("main may measure a finished rec");
}

#[test]
fn negation_of_finished_rec_in_main_is_legal() {
    let query = Query::Reach {
        interiors: vec![],
        rec: Rec {
            base: NonEmpty::one(rec_rule(
                vec![VarId(0)],
                vec![atom(ACCOUNT, vec![(0, var(0))])],
            )),
            rec: NonEmpty::one(rec_step(vec![VarId(0)], vec![(0, var(0))], vec![])),
        },
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(POSTING, vec![(1, var(0))])],
            negated: vec![interior_atom(0, vec![(0, var(0))])],
            conditions: vec![],
        }],
    };
    validate(&schema(), &query).expect("main may anti-join a finished rec");
}

#[test]
fn recursive_arms_align_against_the_base_row() {
    let query = reach_query(
        vec![rec_rule(
            vec![VarId(0)],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![rec_step(
            vec![VarId(0)],
            vec![(0, var(1))],
            vec![atom(POSTING, vec![(2, var(0)), (1, var(1))])],
        )],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(
        expect_err(&query),
        ValidationError::HeadTypeMismatch {
            rule: 0,
            position: 0
        }
    );
}

#[test]
fn rec_pool_caps_base_plus_rec() {
    let base: Vec<RecRule> = (0..10)
        .map(|_| rec_rule(vec![VarId(0)], vec![atom(ACCOUNT, vec![(0, var(0))])]))
        .collect();
    let rec: Vec<RecStep> = (0..7)
        .map(|_| rec_step(vec![VarId(0)], vec![(0, var(0))], vec![]))
        .collect();
    let query = reach_query(
        base,
        rec,
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(
        expect_err(&query),
        ValidationError::TooManyRules { count: 17 }
    );
}

#[test]
fn an_interior_reading_the_rec_is_not_prior() {
    let query = Query::Reach {
        interiors: vec![Interior {
            rules: vec![proj(
                vec![VarId(0)],
                vec![interior_atom(1, vec![(0, var(0))])],
            )],
        }],
        rec: Rec {
            base: NonEmpty::one(rec_rule(
                vec![VarId(0)],
                vec![atom(ACCOUNT, vec![(0, var(0))])],
            )),
            rec: NonEmpty::one(rec_step(vec![VarId(0)], vec![(0, var(0))], vec![])),
        },
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(1, vec![(0, var(0))])],
        )],
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::InteriorNotPrior {
            interior: InteriorId(1),
            at: InteriorId(0)
        }
    );
}
