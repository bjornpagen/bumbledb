//! The measure's differential criteria (20-query-ir § the measure): `Duration` in finds, in
//! `Sum`/`Min`/`Max`, and in comparisons — engine vs the naive model
//! (which evaluates the measure **from the definition**, `|[s,e)| =
//! e − s` over logical values), both element types, boundary intervals
//! (`[x, x+1)`, `[MIN, MAX−1)`), the `Sum(Duration)` overflow verdict,
//! and the ray: both oracles raise `MeasureOfRay` on the unguarded query
//! while the `Allen(DISJOINT`-from-rays`)` guard keeps the same query
//! answering rows.

use bumbledb::schema::{IntervalElement, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    AggOp, AllenMask, Atom, CmpOp, Comparison, ConditionTree, Db, FindTerm, MaskTerm, ParamId,
    Query, RelationId, Rule, Term, Value, VarId,
};

use crate::differential::{Op, run};
use crate::fixture::{TempDir, atom, field, var};
use crate::naive::query::ParamValue;
use crate::naive::{Delta, NaiveDb};

/// Stay(room u64, span interval<u64>, cap u64);
/// Shift(tag u64, span interval<i64>);
/// Limit(room u64, cap u64). No statements: every write commits.
fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Stay".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                    field("cap", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Shift".into(),
                fields: vec![
                    field("tag", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Limit".into(),
                fields: vec![field("room", ValueType::U64), field("cap", ValueType::U64)],
            },
        ],
        statements: vec![],
    }
}

const STAY: RelationId = RelationId(0);
const SHIFT: RelationId = RelationId(1);
const LIMIT: RelationId = RelationId(2);

fn stay_atom() -> Atom {
    atom(STAY, &[(0, var(0)), (1, var(1)), (2, var(2))])
}

fn shift_atom() -> Atom {
    atom(SHIFT, &[(0, var(0)), (1, var(1))])
}

fn single(finds: Vec<FindTerm>, atoms: Vec<Atom>, conditions: Vec<ConditionTree>) -> Query {
    Query::single(Rule {
        finds,
        atoms,
        negated: vec![],
        conditions,
    })
}

/// The bounded corpus: boundary intervals on both element types —
/// `[x, x+1)` (measure 1) and near-maximal spans — plus ordinary spans
/// and per-room caps.
fn bounded_corpus() -> Delta {
    Delta {
        deletes: vec![],
        inserts: vec![
            (
                STAY,
                vec![Value::U64(0), Value::IntervalU64(5, 6), Value::U64(1)],
            ),
            (
                STAY,
                vec![Value::U64(1), Value::IntervalU64(10, 25), Value::U64(20)],
            ),
            (
                STAY,
                vec![Value::U64(2), Value::IntervalU64(3, 11), Value::U64(8)],
            ),
            (SHIFT, vec![Value::U64(0), Value::IntervalI64(-4, 4)]),
            (SHIFT, vec![Value::U64(1), Value::IntervalI64(7, 8)]),
            (LIMIT, vec![Value::U64(0), Value::U64(2)]),
            (LIMIT, vec![Value::U64(1), Value::U64(14)]),
            (LIMIT, vec![Value::U64(2), Value::U64(9)]),
        ],
    }
}

/// The fixed measure queries over the bounded corpus.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // a fixed list, one entry per query
fn measure_queries() -> Vec<(Query, Vec<ParamValue>)> {
    let dur = |op: AggOp| FindTerm::AggregateDuration { op, over: VarId(1) };
    vec![
        // The measure in a find position, u64 spans.
        (
            single(
                vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
                vec![stay_atom()],
                vec![],
            ),
            vec![],
        ),
        // The measure in a find position, i64 spans (the sign-flip
        // cancellation, differentially pinned).
        (
            single(
                vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
                vec![shift_atom()],
                vec![],
            ),
            vec![],
        ),
        // Sum/Min/Max over the measure, grouped by room.
        (
            single(
                vec![
                    FindTerm::Var(VarId(0)),
                    dur(AggOp::Sum),
                    dur(AggOp::Min),
                    dur(AggOp::Max),
                ],
                vec![stay_atom()],
                vec![],
            ),
            vec![],
        ),
        // Duration vs a literal.
        (
            single(
                vec![FindTerm::Var(VarId(0))],
                vec![stay_atom()],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Gt,
                    lhs: Term::Duration(VarId(1)),
                    rhs: Term::Literal(Value::U64(7)),
                })],
            ),
            vec![],
        ),
        // Duration vs a param, written value-first (the mirrored form).
        (
            single(
                vec![FindTerm::Var(VarId(0))],
                vec![stay_atom()],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Ge,
                    lhs: Term::Param(ParamId(0)),
                    rhs: Term::Duration(VarId(1)),
                })],
            ),
            vec![ParamValue::Scalar(Value::U64(8))],
        ),
        // Duration vs a same-atom u64 variable (the fact's own cap).
        (
            single(
                vec![FindTerm::Var(VarId(0))],
                vec![stay_atom()],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Lt,
                    lhs: Term::Duration(VarId(1)),
                    rhs: var(2),
                })],
            ),
            vec![],
        ),
        // Duration vs a cross-atom u64 variable (the measure residual).
        (
            single(
                vec![FindTerm::Var(VarId(0))],
                vec![
                    atom(STAY, &[(0, var(0)), (1, var(1))]),
                    atom(LIMIT, &[(0, var(0)), (1, var(3))]),
                ],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Ge,
                    lhs: Term::Duration(VarId(1)),
                    rhs: var(3),
                })],
            ),
            vec![],
        ),
        // The rules union over measure head positions: u64 stay
        // durations ∪ i64 shift durations — one u64 head, the union's
        // dedup over measure values (a stay and a shift sharing one
        // measure collapse).
        (
            Query {
                head: vec![bumbledb::HeadTerm::Var],
                rules: vec![
                    Rule {
                        finds: vec![FindTerm::Duration(VarId(1))],
                        atoms: vec![stay_atom()],
                        negated: vec![],
                        conditions: vec![],
                    },
                    Rule {
                        finds: vec![FindTerm::Duration(VarId(1))],
                        atoms: vec![shift_atom()],
                        negated: vec![],
                        conditions: vec![],
                    },
                ],
            },
            vec![],
        ),
    ]
}

#[test]
fn measure_queries_agree_with_the_naive_model() {
    let descriptor = schema();
    let dir = TempDir::new("differential-measure");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    let mut ops = vec![Op::Write(bounded_corpus())];
    for (query, params) in measure_queries() {
        ops.push(Op::Query { query, params });
    }
    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagree: {divergence:#?}");
    });
    assert_eq!((summary.commits, summary.queries), (1, 8));
}

/// The two runtime-error verdicts, differentially: `Sum(Duration)` past
/// `u64::MAX` is the overflow verdict on both oracles (the boundary
/// interval `[MIN, MAX−1)` twice), and a ray reaching `Duration` is
/// `MeasureOfRay` on both — while the `Allen(DISJOINT`-from-rays`)`
/// guard keeps the same query answering rows on both.
#[test]
fn measure_error_verdicts_agree_with_the_naive_model() {
    let descriptor = schema();
    let dir = TempDir::new("differential-measure-errors");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    // Two near-maximal u64 spans in one room (Sum overflows u64), one
    // i64 boundary span, and one ray.
    let corpus = Delta {
        deletes: vec![],
        inserts: vec![
            (
                STAY,
                vec![
                    Value::U64(0),
                    Value::IntervalU64(0, u64::MAX - 1),
                    Value::U64(0),
                ],
            ),
            (
                STAY,
                vec![
                    Value::U64(0),
                    Value::IntervalU64(1, u64::MAX - 1),
                    Value::U64(0),
                ],
            ),
            (
                SHIFT,
                vec![Value::U64(0), Value::IntervalI64(i64::MIN, i64::MAX - 1)],
            ),
            (
                STAY,
                vec![
                    Value::U64(7),
                    Value::IntervalU64(3, u64::MAX), // the ray [3, ∞)
                    Value::U64(0),
                ],
            ),
        ],
    };
    let overflow_query = single(
        vec![FindTerm::AggregateDuration {
            op: AggOp::Sum,
            over: VarId(1),
        }],
        vec![atom(
            STAY,
            &[(0, Term::Literal(Value::U64(0))), (1, var(1)), (2, var(2))],
        )],
        vec![],
    );
    // The i64 boundary measure, guarded from nothing (no i64 rays here).
    let boundary_query = single(
        vec![FindTerm::Duration(VarId(1))],
        vec![shift_atom()],
        vec![],
    );
    let unguarded = single(
        vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
        vec![stay_atom()],
        vec![],
    );
    let guarded = single(
        vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
        vec![stay_atom()],
        // The ray probe [MAX−1, MAX): an interval intersects it iff its
        // end is MAX — exactly the rays; DISJOINT keeps the bounded.
        vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::DISJOINT),
            },
            lhs: var(1),
            rhs: Term::Literal(Value::IntervalU64(u64::MAX - 1, u64::MAX)),
        })],
    );

    let ops = vec![
        Op::Write(corpus),
        Op::Query {
            query: overflow_query,
            params: vec![],
        },
        Op::Query {
            query: boundary_query,
            params: vec![],
        },
        Op::Query {
            query: unguarded,
            params: vec![],
        },
        Op::Query {
            query: guarded.clone(),
            params: vec![],
        },
    ];
    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagree: {divergence:#?}");
    });
    assert_eq!((summary.commits, summary.queries), (1, 4));

    // `run` proved agreement; pin the verdicts themselves on the model.
    assert_eq!(
        naive
            .query(&guarded, &[])
            .expect("the guarded query answers rows")
            .len(),
        2,
        "the two bounded stays survive the ray guard"
    );
}
