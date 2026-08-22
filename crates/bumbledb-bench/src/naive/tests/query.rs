use std::collections::BTreeSet;

use bumbledb::schema::{IntervalElement, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    AllenMask, CmpOp, Comparison, ConditionTree, FindTerm, FoldOp, ParamId, Query, RelationId,
    Rule, Term, Value, VarId,
};

use crate::fixture::{atom, field, var};
use crate::naive::query::ParamValue;
use crate::naive::{Delta, NaiveDb, Tuple};

fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("account", ValueType::U64),
                    field("amount", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "PostingTag".into(),
                fields: vec![
                    field("posting", ValueType::U64),
                    field("tag", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Mandate".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field(
                        "active",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![],
    }
}

const POSTING: RelationId = RelationId(0);
const TAG: RelationId = RelationId(1);
const MANDATE: RelationId = RelationId(2);

fn posting(id: u64, account: u64, amount: i64) -> (RelationId, Vec<Value>) {
    (
        POSTING,
        vec![Value::U64(id), Value::U64(account), Value::I64(amount)],
    )
}

fn tag(posting: u64, tag: u64) -> (RelationId, Vec<Value>) {
    (TAG, vec![Value::U64(posting), Value::U64(tag)])
}

fn mandate(account: u64, start: u64, end: u64) -> (RelationId, Vec<Value>) {
    (
        MANDATE,
        vec![
            Value::U64(account),
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ],
    )
}

fn db(facts: Vec<(RelationId, Vec<Value>)>) -> NaiveDb {
    let mut db = NaiveDb::new(&schema());
    db.apply(&Delta {
        deletes: vec![],
        inserts: facts,
    })
    .expect("fixture facts commit (no statements declared)");
    db
}

fn rows(raw: Vec<Vec<Value>>) -> BTreeSet<Tuple> {
    raw.into_iter().map(Tuple).collect()
}

#[test]
fn duplicate_witnesses_collapse() {
    let db = db(vec![
        posting(1, 7, 100),
        posting(2, 7, 100),
        posting(3, 8, 5),
    ]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, &[(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        conditions: vec![],
    });
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(7)], vec![Value::U64(8)]])
    );
}

#[test]
fn aggregation_footgun_triples_the_sum() {
    let db = db(vec![posting(1, 7, 100), tag(1, 0), tag(1, 1), tag(1, 2)]);
    let plain = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(2),
            },
        ],
        atoms: vec![atom(POSTING, &[(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        conditions: vec![],
    });
    assert_eq!(
        db.query(&plain, &[]).unwrap(),
        rows(vec![vec![Value::U64(7), Value::I64(100)]])
    );
    let joined = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(2),
            },
        ],
        atoms: vec![
            atom(POSTING, &[(0, var(1)), (1, var(0)), (2, var(2))]),
            atom(TAG, &[(0, var(1)), (1, var(3))]),
        ],
        negated: vec![],
        conditions: vec![],
    });
    assert_eq!(
        db.query(&joined, &[]).unwrap(),
        rows(vec![vec![Value::U64(7), Value::I64(300)]])
    );
}

#[test]
fn empty_input_global_aggregate_is_the_empty_set() {
    let db = db(vec![]);
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(2),
            },
            FindTerm::Count,
        ],
        atoms: vec![atom(POSTING, &[(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        conditions: vec![],
    });
    assert_eq!(db.query(&query, &[]).unwrap(), rows(vec![]));
}

#[test]
fn membership_boundaries_are_half_open() {
    let db = db(vec![mandate(1, 10, 20)]);
    for (point, expect_hit) in [(9u64, false), (10, true), (19, true), (20, false)] {
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(
                MANDATE,
                &[(0, var(0)), (1, Term::Literal(Value::U64(point)))],
            )],
            negated: vec![],
            conditions: vec![],
        });
        let expected = if expect_hit {
            rows(vec![vec![Value::U64(1)]])
        } else {
            rows(vec![])
        };
        assert_eq!(db.query(&query, &[]).unwrap(), expected, "point {point}");
    }
}

#[test]
fn point_variable_membership_uses_the_scalar_anchor() {
    let db = db(vec![
        posting(1, 12, 5),
        posting(2, 25, 5),
        mandate(9, 10, 20),
    ]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(POSTING, &[(0, var(1)), (1, var(0)), (2, var(2))]),
            atom(MANDATE, &[(0, var(3)), (1, var(0))]),
        ],
        negated: vec![],
        conditions: vec![],
    });
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(12)]])
    );
}

#[test]
fn interval_variable_on_interval_fields_is_value_equality() {
    let db = db(vec![
        mandate(1, 10, 20),
        mandate(2, 10, 20),
        mandate(3, 10, 21),
    ]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(MANDATE, &[(0, var(0)), (1, var(1))]),
            atom(MANDATE, &[(0, var(2)), (1, var(1))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(2),
        })],
    });
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(1), Value::U64(2)]])
    );
}

#[test]
fn negation_rejects_once_regardless_of_multiplicities() {
    let db = db(vec![
        posting(1, 7, 100),
        posting(3, 8, 5),
        tag(1, 0),
        tag(1, 1),
    ]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, &[(0, var(0)), (1, var(1)), (2, var(2))])],
        negated: vec![atom(TAG, &[(0, var(0))])],
        conditions: vec![],
    });
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(3)]])
    );
}

#[test]
fn negated_zero_binding_atom_is_an_emptiness_gate() {
    let db = db(vec![posting(1, 7, 100), tag(1, 0)]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, &[(0, var(0)), (1, var(1)), (2, var(2))])],
        negated: vec![atom(TAG, &[])],
        conditions: vec![],
    });
    assert_eq!(db.query(&query, &[]).unwrap(), rows(vec![]));
}

#[test]
fn param_set_membership_and_the_empty_set() {
    let db = db(vec![
        posting(1, 7, 100),
        posting(2, 8, 50),
        posting(3, 9, 25),
    ]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            &[(0, var(0)), (1, Term::ParamSet(ParamId(0))), (2, var(1))],
        )],
        negated: vec![],
        conditions: vec![],
    });
    let hit = db
        .query(
            &query,
            &[ParamValue::Set(vec![Value::U64(7), Value::U64(9)])],
        )
        .unwrap();
    assert_eq!(hit, rows(vec![vec![Value::U64(1)], vec![Value::U64(3)]]));
    let empty = db.query(&query, &[ParamValue::Set(vec![])]).unwrap();
    assert_eq!(empty, rows(vec![]));
}

#[test]
fn allen_masks_use_the_point_set_definitions() {
    let db = db(vec![
        mandate(1, 10, 20),
        mandate(2, 15, 25),
        mandate(3, 20, 30),
    ]);
    let overlapping = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(MANDATE, &[(0, var(0)), (1, var(1))]),
            atom(MANDATE, &[(0, var(2)), (1, var(3))]),
        ],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: AllenMask::INTERSECTS,
                },
                lhs: var(1),
                rhs: var(3),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: var(2),
            }),
        ],
    });

    assert_eq!(
        db.query(&overlapping, &[]).unwrap(),
        rows(vec![
            vec![Value::U64(1), Value::U64(2)],
            vec![Value::U64(2), Value::U64(3)],
        ])
    );

    let covering = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(MANDATE, &[(0, var(0)), (1, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::COVERS,
            },
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(
                bumbledb::Interval::<u64>::new(16, 22).expect("nonempty interval"),
            )),
        })],
    });
    assert_eq!(
        db.query(&covering, &[]).unwrap(),
        rows(vec![vec![Value::U64(2)]])
    );
}

#[test]
fn sum_overflow_is_the_one_runtime_error() {
    let db = db(vec![posting(1, 7, i64::MAX), posting(2, 7, 1)]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: FoldOp::Sum,
            over: VarId(2),
        }],
        atoms: vec![atom(POSTING, &[(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        conditions: vec![],
    });
    assert!(db.query(&query, &[]).is_err());
}

#[test]
fn a_query_denotes_the_set_union_of_its_rules_denotations() {
    let db = db(vec![
        posting(1, 7, 100),
        posting(2, 7, 250),
        posting(3, 8, 100),
        posting(4, 9, 999),
    ]);
    let by_account = |account: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            &[(1, Term::Literal(Value::U64(account))), (2, var(0))],
        )],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![bumbledb::HeadTerm::Var],
        rules: vec![by_account(7), by_account(8)],
        rec: None,
    };
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::I64(100)], vec![Value::I64(250)]]),
        "one union, set semantics: 100 appears once"
    );
}

#[test]
fn variables_are_rule_scoped_in_the_model_too() {
    let db = db(vec![posting(1, 7, 100), posting(2, 8, 250)]);
    let first = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            &[(1, Term::Literal(Value::U64(7))), (2, var(0))],
        )],
        negated: vec![],
        conditions: vec![],
    };
    let second = Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![atom(POSTING, &[(1, var(0)), (2, var(1))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: Term::Literal(Value::U64(8)),
        })],
    };
    let query = Query {
        interiors: vec![],
        head: vec![bumbledb::HeadTerm::Var],
        rules: vec![first, second],
        rec: None,
    };
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::I64(100)], vec![Value::I64(250)]]),
    );
}

#[test]
fn a_multi_rule_aggregate_folds_over_the_union_projected_to_the_head() {
    // and 8 contribute {100, 250} ∪ {100} = {100, 250} → 350 (the

    let db = db(vec![
        posting(1, 7, 100),
        posting(2, 7, 250),
        posting(3, 8, 100),
    ]);
    let sum_of = |account: u64| Rule {
        finds: vec![FindTerm::Aggregate {
            op: FoldOp::Sum,
            over: VarId(0),
        }],
        atoms: vec![atom(
            POSTING,
            &[(1, Term::Literal(Value::U64(account))), (2, var(0))],
        )],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Sum)],
        rules: vec![sum_of(7), sum_of(8)],
        rec: None,
    };
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::I64(350)]]),
    );
}
