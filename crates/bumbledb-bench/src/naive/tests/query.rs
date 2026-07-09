//! Query goldens: the `20-query-ir.md` semantics landmarks — duplicate
//! witnesses collapse, the aggregation footgun triples the sum, the
//! empty-input global aggregate is empty, an Arg tie yields every
//! attaining row, membership boundaries, and negation against
//! multiplicities — plus the membership/equality bivalence and param
//! sets.

use std::collections::BTreeSet;

use bumbledb::schema::{
    FieldDescriptor, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor, ValueType,
};
use bumbledb::{
    AggOp, Atom, CmpOp, Comparison, FieldId, FindTerm, ParamId, Query, RelationId, Term, Value,
    VarId,
};

use crate::naive::query::ParamValue;
use crate::naive::{Delta, NaiveDb, Tuple};

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

/// The fixture schema: Posting(id, account, amount), PostingTag(posting,
/// tag), Mandate(account, active: interval<u64>) — no statements; query
/// evaluation never consults them.
fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Posting".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("account", ValueType::U64),
                    field("amount", ValueType::I64),
                ],
            },
            RelationDescriptor {
                name: "PostingTag".into(),
                fields: vec![
                    field("posting", ValueType::U64),
                    field("tag", ValueType::U64),
                ],
            },
            RelationDescriptor {
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
        vec![Value::U64(account), Value::IntervalU64(start, end)],
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

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn atom(relation: RelationId, bindings: Vec<(u16, Term)>) -> Atom {
    Atom {
        relation,
        bindings: bindings
            .into_iter()
            .map(|(field, term)| (FieldId(field), term))
            .collect(),
    }
}

fn rows(raw: Vec<Vec<Value>>) -> BTreeSet<Tuple> {
    raw.into_iter().map(Tuple).collect()
}

#[test]
fn duplicate_witnesses_collapse() {
    // Two postings on account 7: projecting the account yields ONE row —
    // existential variables never multiply projection output.
    let db = db(vec![
        posting(1, 7, 100),
        posting(2, 7, 100),
        posting(3, 8, 5),
    ]);
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        predicates: vec![],
    };
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(7)], vec![Value::U64(8)]])
    );
}

#[test]
fn aggregation_footgun_triples_the_sum() {
    // Joining the multiplicity-adding PostingTag into the aggregate
    // multiplies the binding set: 3 tags on one posting of 100 ⇒ 300.
    let db = db(vec![posting(1, 7, 100), tag(1, 0), tag(1, 1), tag(1, 2)]);
    let plain = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(2)),
            },
        ],
        atoms: vec![atom(POSTING, vec![(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        predicates: vec![],
    };
    assert_eq!(
        db.query(&plain, &[]).unwrap(),
        rows(vec![vec![Value::U64(7), Value::I64(100)]])
    );
    let joined = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(2)),
            },
        ],
        atoms: vec![
            atom(POSTING, vec![(0, var(1)), (1, var(0)), (2, var(2))]),
            atom(TAG, vec![(0, var(1)), (1, var(3))]),
        ],
        negated: vec![],
        predicates: vec![],
    };
    assert_eq!(
        db.query(&joined, &[]).unwrap(),
        rows(vec![vec![Value::U64(7), Value::I64(300)]])
    );
}

#[test]
fn empty_input_global_aggregate_is_the_empty_set() {
    // Over empty input the result is the empty set — not a 0 or NULL row.
    let db = db(vec![]);
    let query = Query {
        finds: vec![
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(2)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![atom(POSTING, vec![(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        predicates: vec![],
    };
    assert_eq!(db.query(&query, &[]).unwrap(), rows(vec![]));
}

#[test]
fn arg_tie_yields_every_attaining_row() {
    // Two postings share the maximal amount 100: ArgMax carries both ids
    // — the answer is a set, and a tie survives on every carried column.
    let db = db(vec![
        posting(1, 7, 100),
        posting(2, 7, 100),
        posting(3, 7, 99),
    ]);
    let query = Query {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::ArgMax { key: VarId(2) },
            over: Some(VarId(1)),
        }],
        atoms: vec![atom(POSTING, vec![(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        predicates: vec![],
    };
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(1)], vec![Value::U64(2)]])
    );
}

#[test]
fn membership_boundaries_are_half_open() {
    // Mandate active over [10, 20): the start is in, the end is out.
    let db = db(vec![mandate(1, 10, 20)]);
    for (point, expect_hit) in [(9u64, false), (10, true), (19, true), (20, false)] {
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(
                MANDATE,
                vec![(0, var(0)), (1, Term::Literal(Value::U64(point)))],
            )],
            negated: vec![],
            predicates: vec![],
        };
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
    // The point variable is anchored by a scalar field (Posting.account)
    // and tested by membership against Mandate.active — only accounts
    // whose value lies inside some mandate interval survive.
    let db = db(vec![
        posting(1, 12, 5),
        posting(2, 25, 5),
        mandate(9, 10, 20),
    ]);
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(POSTING, vec![(0, var(1)), (1, var(0)), (2, var(2))]),
            atom(MANDATE, vec![(0, var(3)), (1, var(0))]),
        ],
        negated: vec![],
        predicates: vec![],
    };
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(12)]])
    );
}

#[test]
fn interval_variable_on_interval_fields_is_value_equality() {
    // A variable occurring ONLY on interval fields is interval-typed:
    // binding it in two atoms joins on interval identity, not overlap.
    let db = db(vec![
        mandate(1, 10, 20),
        mandate(2, 10, 20),
        mandate(3, 10, 21),
    ]);
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(MANDATE, vec![(0, var(0)), (1, var(1))]),
            atom(MANDATE, vec![(0, var(2)), (1, var(1))]),
        ],
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(2),
        }],
    };
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(1), Value::U64(2)]])
    );
}

#[test]
fn negation_rejects_once_regardless_of_multiplicities() {
    // Posting 1 carries two tags, posting 3 none: the negated atom
    // rejects the tagged posting exactly once — no multiplicity effects,
    // plain anti-join over sets.
    let db = db(vec![
        posting(1, 7, 100),
        posting(3, 8, 5),
        tag(1, 0),
        tag(1, 1),
    ]);
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(0)), (1, var(1)), (2, var(2))])],
        negated: vec![atom(TAG, vec![(0, var(0))])],
        predicates: vec![],
    };
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(3)]])
    );
}

#[test]
fn negated_zero_binding_atom_is_an_emptiness_gate() {
    let db = db(vec![posting(1, 7, 100), tag(1, 0)]);
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(POSTING, vec![(0, var(0)), (1, var(1)), (2, var(2))])],
        negated: vec![atom(TAG, vec![])],
        predicates: vec![],
    };
    assert_eq!(db.query(&query, &[]).unwrap(), rows(vec![]));
}

#[test]
fn count_distinct_folds_values_not_bindings() {
    let db = db(vec![
        posting(1, 7, 100),
        posting(2, 7, 100),
        posting(3, 8, 5),
    ]);
    let query = Query {
        finds: vec![
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
            FindTerm::Aggregate {
                op: AggOp::CountDistinct,
                over: Some(VarId(0)),
            },
        ],
        atoms: vec![atom(POSTING, vec![(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        predicates: vec![],
    };
    // 3 distinct bindings, 2 distinct accounts.
    assert_eq!(
        db.query(&query, &[]).unwrap(),
        rows(vec![vec![Value::U64(3), Value::U64(2)]])
    );
}

#[test]
fn param_set_membership_and_the_empty_set() {
    let db = db(vec![
        posting(1, 7, 100),
        posting(2, 8, 50),
        posting(3, 9, 25),
    ]);
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            POSTING,
            vec![(0, var(0)), (1, Term::ParamSet(ParamId(0))), (2, var(1))],
        )],
        negated: vec![],
        predicates: vec![],
    };
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
fn overlaps_and_contains_use_the_endpoint_formulas() {
    let db = db(vec![
        mandate(1, 10, 20),
        mandate(2, 15, 25),
        mandate(3, 20, 30),
    ]);
    let overlapping = Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(MANDATE, vec![(0, var(0)), (1, var(1))]),
            atom(MANDATE, vec![(0, var(2)), (1, var(3))]),
        ],
        negated: vec![],
        predicates: vec![
            Comparison {
                op: CmpOp::Overlaps,
                lhs: var(1),
                rhs: var(3),
            },
            Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: var(2),
            },
        ],
    };
    // [10,20) and [20,30) are adjacent, not overlapping.
    assert_eq!(
        db.query(&overlapping, &[]).unwrap(),
        rows(vec![
            vec![Value::U64(1), Value::U64(2)],
            vec![Value::U64(2), Value::U64(3)],
        ])
    );
    let containing = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(MANDATE, vec![(0, var(0)), (1, var(1))])],
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Contains,
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(16, 22)),
        }],
    };
    assert_eq!(
        db.query(&containing, &[]).unwrap(),
        rows(vec![vec![Value::U64(2)]])
    );
}

#[test]
fn sum_overflow_is_the_one_runtime_error() {
    let db = db(vec![posting(1, 7, i64::MAX), posting(2, 7, 1)]);
    let query = Query {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Sum,
            over: Some(VarId(2)),
        }],
        atoms: vec![atom(POSTING, vec![(0, var(1)), (1, var(0)), (2, var(2))])],
        negated: vec![],
        predicates: vec![],
    };
    assert!(db.query(&query, &[]).is_err());
}
