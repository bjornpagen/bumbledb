//! Rec roster: one linear SCC, empty/self/nonlinear/negation/measure.

use super::*;
use crate::ir::{AtomSource, HeadTerm, Interior, InteriorId, Rec};

fn interior_atom(id: u32, bindings: Vec<(u16, Term)>) -> crate::ir::Atom {
    crate::ir::Atom {
        source: AtomSource::Interior(InteriorId(id)),
        bindings: bindings.into_iter().map(|(f, t)| (FieldId(f), t)).collect(),
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

fn reach_query(base: Vec<Rule>, rec: Vec<Rule>, main: Rule) -> Query {
    Query {
        interiors: vec![],
        rec: Some(Rec {
            head: vec![HeadTerm::Var],
            base,
            rec,
        }),
        head: main.head(),
        rules: vec![main],
    }
}

fn linear_reach() -> Query {
    reach_query(
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        )],
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
fn rejects_empty_recursive_base() {
    let query = reach_query(
        vec![],
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        )],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(expect_err(&query), ValidationError::EmptyRecursiveBase);
}

#[test]
fn rejects_empty_recursive_step() {
    let query = reach_query(
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![],
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
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        )],
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        )],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(expect_err(&query), ValidationError::SelfInBase);
}

#[test]
fn rejects_rec_arm_missing_self() {
    let query = reach_query(
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(expect_err(&query), ValidationError::RecArmMissingSelf);
}

#[test]
fn rejects_nonlinear_rec_arm() {
    let query = reach_query(
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![
                interior_atom(0, vec![(0, var(0))]),
                interior_atom(0, vec![(0, var(1))]),
            ],
        )],
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        ),
    );
    assert_eq!(expect_err(&query), ValidationError::NonlinearRecArm);
}

#[test]
fn rejects_negation_in_rec() {
    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            head: vec![HeadTerm::Var],
            base: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![atom(ACCOUNT, vec![(0, var(0))])],
            )],
            rec: vec![Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![
                    atom(ACCOUNT, vec![(0, var(0))]),
                    interior_atom(0, vec![(0, var(1))]),
                ],
                negated: vec![atom(POSTING, vec![(1, var(0))])],
                conditions: vec![],
            }],
        }),
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        )],
    };
    assert_eq!(expect_err(&query), ValidationError::NegationInRec);
}

#[test]
fn rejects_measure_in_interior_on_rec_head() {
    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            head: vec![HeadTerm::Var],
            base: vec![rule(
                vec![FindTerm::Measure(VarId(0))],
                vec![atom(ACCOUNT, vec![(VALIDITY, var(0))])],
            )],
            rec: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![interior_atom(0, vec![(0, var(0))])],
            )],
        }),
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0))])],
        )],
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::MeasureInInterior {
            interior: InteriorId(0)
        }
    );
}

#[test]
fn rejects_aggregate_in_interior_on_rec_head() {
    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            head: vec![HeadTerm::Var, HeadTerm::Aggregate(crate::ir::HeadOp::Count)],
            base: vec![rule(
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: crate::ir::AggOp::Count,
                        over: None,
                    },
                ],
                vec![atom(ACCOUNT, vec![(0, var(0))])],
            )],
            rec: vec![rule(
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: crate::ir::AggOp::Count,
                        over: None,
                    },
                ],
                vec![interior_atom(0, vec![(0, var(0))])],
            )],
        }),
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::AggregateInInterior {
            interior: InteriorId(0)
        }
    );
}

#[test]
fn a_measure_on_main_over_finished_rec_is_legal() {
    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            head: vec![HeadTerm::Var],
            base: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![atom(ACCOUNT, vec![(VALIDITY, var(0))])],
            )],
            rec: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![interior_atom(0, vec![(0, var(0))])],
            )],
        }),
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
    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            head: vec![HeadTerm::Var],
            base: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![atom(ACCOUNT, vec![(0, var(0))])],
            )],
            rec: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![interior_atom(0, vec![(0, var(0))])],
            )],
        }),
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
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![
                interior_atom(0, vec![(0, var(1))]),
                atom(POSTING, vec![(2, var(0)), (1, var(1))]),
            ],
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
    let base: Vec<Rule> = (0..10)
        .map(|_| {
            rule(
                vec![FindTerm::Var(VarId(0))],
                vec![atom(ACCOUNT, vec![(0, var(0))])],
            )
        })
        .collect();
    let rec: Vec<Rule> = (0..7)
        .map(|_| {
            rule(
                vec![FindTerm::Var(VarId(0))],
                vec![interior_atom(0, vec![(0, var(0))])],
            )
        })
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
    let query = Query {
        interiors: vec![Interior {
            head: vec![HeadTerm::Var],
            rules: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![interior_atom(1, vec![(0, var(0))])],
            )],
        }],
        rec: Some(Rec {
            head: vec![HeadTerm::Var],
            base: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![atom(ACCOUNT, vec![(0, var(0))])],
            )],
            rec: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![interior_atom(1, vec![(0, var(0))])],
            )],
        }),
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
