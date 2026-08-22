use super::*;
use crate::error::AtomIndex;
use crate::ir::{AtomSource, HeadTerm, Interior, InteriorId, ProjectionRule};

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

fn query_with_interior(interior: Interior, main: Rule) -> Query {
    Query {
        interiors: vec![interior],
        head: main.head(),
        rules: vec![main],
        rec: None,
    }
}

#[test]
fn more_than_sixteen_interiors_still_validate() {
    let interiors = (0..17)
        .map(|_| Interior {
            rules: vec![proj(vec![VarId(0)], vec![atom(ACCOUNT, vec![(0, var(0))])])],
        })
        .collect();
    let query = Query {
        interiors,
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(16, vec![(0, var(0))])],
        )],
        rec: None,
    };
    validate(&schema(), &query).expect("interior count is uncapped");
}

#[test]
fn rejects_empty_interior() {
    let query = query_with_interior(
        Interior { rules: vec![] },
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        ),
    );
    assert_eq!(
        expect_err(&query),
        ValidationError::EmptyInterior {
            interior: InteriorId(0)
        }
    );
}

#[test]
fn rejects_unknown_interior() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![interior_atom(0, vec![(0, var(0))])],
    );
    assert_eq!(
        expect_err(&query),
        ValidationError::UnknownInterior {
            atom: AtomIndex(0),
            interior: InteriorId(0)
        }
    );
}

#[test]
fn rejects_a_negated_phantom_read() {
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(ACCOUNT, vec![(0, var(0))])],
            negated: vec![interior_atom(9, vec![(0, var(0))])],
            conditions: vec![],
        }],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::UnknownInterior {
            atom: AtomIndex(1),
            interior: InteriorId(9)
        }
    );
}

#[test]
fn rejects_interior_column_out_of_range() {
    let query = query_with_interior(
        Interior {
            rules: vec![proj(vec![VarId(0)], vec![atom(ACCOUNT, vec![(0, var(0))])])],
        },
        rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(0, vec![(0, var(0)), (4, var(1))])],
        ),
    );
    assert_eq!(
        expect_err(&query),
        ValidationError::InteriorColumnOutOfRange {
            atom: AtomIndex(0),
            field: FieldId(4)
        }
    );
}

#[test]
fn rejects_interior_not_prior() {
    let query = Query {
        interiors: vec![Interior {
            rules: vec![proj(
                vec![VarId(0)],
                vec![interior_atom(0, vec![(0, var(0))])],
            )],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::InteriorNotPrior {
            interior: InteriorId(0),
            at: InteriorId(0)
        }
    );
}

#[test]
fn rejects_an_interior_reading_a_later_interior() {
    let query = Query {
        interiors: vec![
            Interior {
                rules: vec![proj(
                    vec![VarId(0)],
                    vec![interior_atom(1, vec![(0, var(0))])],
                )],
            },
            Interior {
                rules: vec![proj(vec![VarId(0)], vec![atom(ACCOUNT, vec![(0, var(0))])])],
            },
        ],
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![interior_atom(1, vec![(0, var(0))])],
        )],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::InteriorNotPrior {
            interior: InteriorId(1),
            at: InteriorId(0)
        }
    );
}

#[test]
fn interior_anchors_resolve_against_sealed_columns() {
    let query = Query {
        interiors: vec![Interior {
            rules: vec![proj(vec![VarId(0)], vec![atom(ACCOUNT, vec![(0, var(0))])])],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![
                interior_atom(0, vec![(0, var(0))]),
                atom(POSTING, vec![(2, var(0))]),
            ],
        )],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::VariableTypeConflict { var: VarId(0) }
    );
}

#[test]
fn an_interval_interior_column_reads_bivalently() {
    let query = Query {
        interiors: vec![Interior {
            rules: vec![proj(
                vec![VarId(0)],
                vec![atom(ACCOUNT, vec![(VALIDITY, var(0))])],
            )],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![
                atom(POSTING, vec![(1, var(0))]),
                interior_atom(0, vec![(0, var(0))]),
            ],
        )],
        rec: None,
    };
    validate(&schema(), &query).expect("legal: the membership typing rule");
}

#[test]
fn negation_of_an_interior_in_main_is_legal() {
    let query = Query {
        interiors: vec![Interior {
            rules: vec![proj(vec![VarId(0)], vec![atom(ACCOUNT, vec![(0, var(0))])])],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(POSTING, vec![(1, var(0))])],
            negated: vec![interior_atom(0, vec![(0, var(0))])],
            conditions: vec![],
        }],
        rec: None,
    };
    validate(&schema(), &query).expect("main may anti-join a finished interior");
}

#[test]
fn query_global_params_unify_across_interiors() {
    let query = Query {
        interiors: vec![Interior {
            rules: vec![proj(
                vec![VarId(0)],
                vec![atom(
                    ACCOUNT,
                    vec![(0, var(0)), (2, Term::Param(ParamId(0)))],
                )],
            )],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![
                interior_atom(0, vec![(0, var(0))]),
                atom(POSTING, vec![(2, Term::Param(ParamId(0))), (1, var(1))]),
            ],
        )],
        rec: None,
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::ParamTypeConflict { param: ParamId(0) }
    );
}

#[test]
fn a_plain_query_still_validates() {
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(ACCOUNT, vec![(0, var(0))])],
    );
    assert!(query.interiors().is_empty());
    assert!(matches!(query, Query { rec: None, .. }));
    validate(&schema(), &query).expect("plain query validates");
}
