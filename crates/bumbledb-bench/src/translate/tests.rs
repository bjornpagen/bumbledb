use super::*;
use crate::schema::{ids, schema};
use bumbledb::ir::{Atom, CmpOp, Comparison, FindTerm, Term};
use bumbledb::{AggOp, Query, Value};

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

#[test]
fn point_matches_its_hand_written_golden() {
    // Q(amount, at) :- Posting(id = ?0, amount, at).
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, Term::Param(ParamId(0))),
                (ids::posting::AMOUNT, var(0)),
                (ids::posting::AT, var(1)),
            ],
        }],
        predicates: vec![],
    };
    let t = translate(&query, schema()).expect("translates");
    assert_eq!(t.sql, goldens::POINT);
    assert_eq!(t.params, vec![ParamId(0)]);
}

#[test]
fn fk_walk_matches_its_hand_written_golden() {
    // Q(name, amount) :- Posting(account = ?0, amount),
    //                    Account(id = ?0, holder = h),
    //                    Holder(id = h, name).
    // (The account is pinned by the same param on both sides — the
    // join predicate through ?1 twice, param reused.)
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, Term::Param(ParamId(0))),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![
                    (ids::account::ID, Term::Param(ParamId(0))),
                    (ids::account::HOLDER, var(2)),
                ],
            },
            Atom {
                relation: ids::HOLDER,
                bindings: vec![(ids::holder::ID, var(2)), (ids::holder::NAME, var(0))],
            },
        ],
        predicates: vec![],
    };
    let t = translate(&query, schema()).expect("translates");
    assert_eq!(t.sql, goldens::FK_WALK);
    assert_eq!(t.params, vec![ParamId(0)], "one placeholder, reused");
}

#[test]
fn balance_matches_its_hand_written_golden() {
    // Q(a, Sum(amount)) :- Posting(id, account = a, amount),
    //                      Account(id = a, holder = ?0).
    let query = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ID, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![
                    (ids::account::ID, var(0)),
                    (ids::account::HOLDER, Term::Param(ParamId(0))),
                ],
            },
        ],
        predicates: vec![],
    };
    let t = translate(&query, schema()).expect("translates");
    assert_eq!(t.sql, goldens::BALANCE);
}

#[test]
fn every_construct_translates() {
    // Gate atom → EXISTS; repeated in-atom var; same-atom and
    // cross-atom comparisons; every operator; literal escaping.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::AMOUNT, var(0)),
                    (ids::posting::AT, var(1)),
                    (
                        ids::posting::MEMO,
                        Term::Literal(Value::String(b"it's a 'quote'".to_vec().into())),
                    ),
                ],
            },
            Atom {
                relation: ids::TAG,
                bindings: vec![],
            },
        ],
        predicates: vec![
            Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: var(1),
            },
            Comparison {
                op: CmpOp::Ge,
                lhs: var(1),
                rhs: Term::Literal(Value::I64(-5)),
            },
            Comparison {
                op: CmpOp::Ne,
                lhs: var(0),
                rhs: Term::Param(ParamId(0)),
            },
        ],
    };
    let t = translate(&query, schema()).expect("translates");
    assert!(
        t.sql.contains("EXISTS (SELECT 1 FROM \"Tag\")"),
        "{}",
        t.sql
    );
    assert!(t.sql.contains("'it''s a ''quote'''"), "{}", t.sql);
    assert!(t.sql.contains("t0.\"amount\" < t0.\"at\""), "{}", t.sql);
    assert!(t.sql.contains(">= -5"), "{}", t.sql);
    assert!(t.sql.contains("<> ?1"), "{}", t.sql);
    assert_eq!(t.params, vec![ParamId(0)]);

    // Repeated in-atom variable equates its two columns.
    let repeated = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::AMOUNT, var(0)), (ids::posting::AT, var(0))],
        }],
        predicates: vec![],
    };
    let t = translate(&repeated, schema()).expect("translates");
    assert!(t.sql.contains("t0.\"amount\" = t0.\"at\""), "{}", t.sql);
}

#[test]
fn global_aggregates_carry_the_having_rule() {
    // Q(Count) :- Posting(amount = x): SQL's NULL-row-over-empty must
    // collapse to the engine's empty set.
    let query = Query {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![(ids::posting::AMOUNT, var(0))],
        }],
        predicates: vec![],
    };
    let t = translate(&query, schema()).expect("translates");
    assert!(t.sql.ends_with("HAVING COUNT(*) > 0"), "{}", t.sql);
    assert!(t.sql.contains("SELECT DISTINCT"), "{}", t.sql);

    // Min/Max over the distinct binding set, grouped.
    let grouped = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Min,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Max,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ACCOUNT, var(0)),
                (ids::posting::AMOUNT, var(1)),
            ],
        }],
        predicates: vec![],
    };
    let t = translate(&grouped, schema()).expect("translates");
    assert!(t.sql.contains("MIN(v1)"), "{}", t.sql);
    assert!(t.sql.contains("MAX(v1)"), "{}", t.sql);
    assert!(t.sql.ends_with("GROUP BY v0"), "{}", t.sql);
}

#[test]
fn errors_name_the_untranslatable_construct() {
    let gates_only = Query {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![Atom {
            relation: ids::TAG,
            bindings: vec![],
        }],
        predicates: vec![],
    };
    let err = translate(&gates_only, schema()).unwrap_err();
    assert!(err.contains("no bound atoms"), "{err}");
}

/// PRD 07 (docs/hardening): a NUL in a string literal would truncate
/// `SQLite`'s tokenizer mid-statement — the translator rejects it by
/// name instead of emitting silently-shortened SQL.
#[test]
fn a_nul_string_literal_is_a_named_error() {
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, Term::Var(VarId(0))),
                (
                    ids::posting::MEMO,
                    Term::Literal(Value::String(b"before\0after".to_vec().into())),
                ),
            ],
        }],
        predicates: vec![],
    };
    let err = translate(&query, schema()).unwrap_err();
    assert!(err.contains("NUL byte in string literal"), "{err}");
}
