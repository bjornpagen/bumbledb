mod capacity_ray;
mod capacity_witness;
mod closed;
mod contradiction;
mod fixed_width;
mod fold;
mod ground;
mod identity_bytes;
mod marks;
mod pack;
mod recursive;
mod witness;

use bumbledb::schema::{
    FieldId, IntervalElement, RelationDescriptor, SchemaDescriptor, Side, StatementDescriptor,
    ValueType,
};
use bumbledb::{
    AllenMask, Atom, CmpOp, Comparison, ConditionTree, Db, FindTerm, FoldOp, HeadOp, HeadTerm,
    ParamId, Query, RelationId, Rule, Term, Value, VarId,
};

use crate::differential::{Op, Summary, run};
use crate::fixture::{TempDir, atom, field, var};
use crate::naive::query::ParamValue;
use crate::naive::{Delta, NaiveDb};

fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                    field("reference", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Marker".into(),
                fields: vec![field("id", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: BOOKING,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Functionality {
                relation: BOOKING,
                projection: Box::new([FieldId(2)]),
            },
            StatementDescriptor::Functionality {
                relation: MARKER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: BOOKING,
                    projection: Box::new([FieldId(2)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: MARKER,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: MARKER,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: BOOKING,
                    projection: Box::new([FieldId(2)]),
                    selection: Box::new([]),
                },
            },
        ],
    }
}

const BOOKING: RelationId = RelationId(0);
const MARKER: RelationId = RelationId(1);

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn booking(rng: &mut Rng, reference: u64) -> Vec<Value> {
    let start = rng.below(20);
    let end = start + 1 + rng.below(5);
    vec![
        Value::U64(rng.below(3)),
        Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval")),
        Value::U64(reference),
    ]
}

fn pick(mirror: &NaiveDb, rel: RelationId, rng: &mut Rng) -> Option<Vec<Value>> {
    let facts = mirror.relation(rel);
    if facts.is_empty() {
        return None;
    }
    let index = usize::try_from(rng.below(facts.len() as u64)).expect("index fits");
    facts.iter().nth(index).map(|tuple| tuple.0.clone())
}

fn write_ops(rng: &mut Rng) -> (Vec<Delta>, u64) {
    let mut mirror = NaiveDb::new(&schema());
    let mut deltas = Vec::new();
    let mut pattern_cases = 0u64;
    for _ in 0..200 {
        let delta = match rng.below(11) {
            0..=3 => {
                let reference = rng.below(8);
                Delta {
                    deletes: vec![],
                    inserts: vec![
                        (BOOKING, booking(rng, reference)),
                        (MARKER, vec![Value::U64(reference)]),
                    ],
                }
            }

            4 | 5 => {
                if rng.below(2) == 0 {
                    let reference = rng.below(8);
                    Delta {
                        deletes: vec![],
                        inserts: vec![(BOOKING, booking(rng, reference))],
                    }
                } else {
                    Delta {
                        deletes: vec![],
                        inserts: vec![(MARKER, vec![Value::U64(rng.below(8))])],
                    }
                }
            }

            6 | 7 => {
                let rel = if rng.below(2) == 0 { BOOKING } else { MARKER };
                match pick(&mirror, rel, rng) {
                    Some(fact) => Delta {
                        deletes: vec![(rel, fact)],
                        inserts: vec![],
                    },
                    None => Delta::default(),
                }
            }

            8 => match pick(&mirror, BOOKING, rng) {
                Some(fact) => {
                    let reference = fact[2].clone();
                    Delta {
                        deletes: vec![(BOOKING, fact), (MARKER, vec![reference])],
                        inserts: vec![],
                    }
                }
                None => Delta::default(),
            },

            9 => match pick(&mirror, BOOKING, rng) {
                Some(fact) => {
                    let reference = fact[2].clone();
                    let mut deletes = vec![(MARKER, vec![reference])];
                    if rng.below(2) == 0 {
                        deletes.insert(0, (BOOKING, fact.clone()));
                    }
                    pattern_cases += 1;
                    Delta {
                        deletes,
                        inserts: vec![(BOOKING, fact)],
                    }
                }
                None => Delta::default(),
            },

            _ => Delta {
                deletes: vec![(MARKER, vec![Value::U64(100 + rng.below(8))])],
                inserts: vec![],
            },
        };
        let _ = mirror.apply(&delta);
        deltas.push(delta);
    }
    (deltas, pattern_cases)
}

fn plain(finds: Vec<FindTerm>, atoms: Vec<Atom>) -> Query {
    Query::single(Rule {
        finds,
        atoms,
        negated: vec![],
        conditions: vec![],
    })
}

fn booking_atom() -> Atom {
    atom(BOOKING, &[(0, var(0)), (1, var(1)), (2, var(2))])
}

#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
fn queries() -> Vec<(Query, Vec<ParamValue>)> {
    let v = |id: u16| FindTerm::Var(VarId(id));
    let fold = |op: FoldOp, over: u16| FindTerm::Aggregate {
        op,
        over: VarId(over),
    };
    vec![
        (plain(vec![v(0), v(1), v(2)], vec![booking_atom()]), vec![]),
        (
            plain(vec![v(0)], vec![atom(MARKER, &[(0, var(0))])]),
            vec![],
        ),
        (
            plain(
                vec![v(0)],
                vec![atom(
                    BOOKING,
                    &[(0, var(0)), (1, Term::Literal(Value::U64(7))), (2, var(1))],
                )],
            ),
            vec![],
        ),
        (
            plain(
                vec![v(0), v(1)],
                vec![
                    atom(MARKER, &[(0, var(1))]),
                    atom(BOOKING, &[(0, var(0)), (1, var(1)), (2, var(2))]),
                ],
            ),
            vec![],
        ),
        (
            plain(
                vec![v(0), v(2)],
                vec![booking_atom(), atom(MARKER, &[(0, var(2))])],
            ),
            vec![],
        ),
        (
            Query::single(Rule {
                finds: vec![v(0)],
                atoms: vec![atom(MARKER, &[(0, var(0))])],
                negated: vec![atom(
                    BOOKING,
                    &[(0, Term::Literal(Value::U64(0))), (2, var(0))],
                )],
                conditions: vec![],
            }),
            vec![],
        ),
        (
            plain(vec![v(0), FindTerm::Count], vec![booking_atom()]),
            vec![],
        ),
        (plain(vec![FindTerm::Count], vec![booking_atom()]), vec![]),
        (
            plain(vec![v(0), fold(FoldOp::Sum, 2)], vec![booking_atom()]),
            vec![],
        ),
        (
            plain(vec![fold(FoldOp::Max, 2)], vec![booking_atom()]),
            vec![],
        ),
        (
            plain(vec![v(0), fold(FoldOp::Max, 2)], vec![booking_atom()]),
            vec![],
        ),
        (
            plain(vec![fold(FoldOp::Min, 2)], vec![booking_atom()]),
            vec![],
        ),
        (
            Query::single(Rule {
                finds: vec![v(2), v(5)],
                atoms: vec![
                    booking_atom(),
                    atom(BOOKING, &[(0, var(3)), (1, var(4)), (2, var(5))]),
                ],
                negated: vec![],
                conditions: vec![
                    ConditionTree::Leaf(Comparison {
                        op: CmpOp::Allen {
                            mask: AllenMask::INTERSECTS,
                        },
                        lhs: var(1),
                        rhs: var(4),
                    }),
                    ConditionTree::Leaf(Comparison {
                        op: CmpOp::Lt,
                        lhs: var(2),
                        rhs: var(5),
                    }),
                ],
            }),
            vec![],
        ),
        (
            Query::single(Rule {
                finds: vec![v(2), v(5)],
                atoms: vec![
                    booking_atom(),
                    atom(BOOKING, &[(0, var(3)), (1, var(4)), (2, var(5))]),
                ],
                negated: vec![],
                conditions: vec![
                    ConditionTree::Leaf(Comparison {
                        op: CmpOp::Allen {
                            mask: AllenMask::COVERS,
                        },
                        lhs: var(1),
                        rhs: var(4),
                    }),
                    ConditionTree::Leaf(Comparison {
                        op: CmpOp::Ne,
                        lhs: var(2),
                        rhs: var(5),
                    }),
                ],
            }),
            vec![],
        ),
        (
            Query::single(Rule {
                finds: vec![v(2), v(3)],
                atoms: vec![booking_atom(), atom(MARKER, &[(0, var(3))])],
                negated: vec![],
                conditions: vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::PointIn,
                    lhs: var(1),
                    rhs: var(3),
                })],
            }),
            vec![],
        ),
        (
            plain(
                vec![v(0), v(1)],
                vec![atom(
                    BOOKING,
                    &[(0, Term::Param(ParamId(0))), (1, var(0)), (2, var(1))],
                )],
            ),
            vec![ParamValue::Scalar(Value::U64(1))],
        ),
        (
            plain(
                vec![v(0), v(1)],
                vec![atom(
                    BOOKING,
                    &[(0, Term::ParamSet(ParamId(0))), (1, var(0)), (2, var(1))],
                )],
            ),
            vec![ParamValue::Set(vec![Value::U64(0), Value::U64(2)])],
        ),
        (
            Query::single(Rule {
                finds: vec![v(2)],
                atoms: vec![booking_atom()],
                negated: vec![],
                conditions: vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Ge,
                    lhs: var(2),
                    rhs: Term::Literal(Value::U64(4)),
                })],
            }),
            vec![],
        ),
        (
            Query::single(Rule {
                finds: vec![v(0)],
                atoms: vec![atom(MARKER, &[(0, var(0))])],
                negated: vec![atom(
                    BOOKING,
                    &[(0, Term::ParamSet(ParamId(0))), (2, var(0))],
                )],
                conditions: vec![],
            }),
            vec![ParamValue::Set(vec![Value::U64(1), Value::U64(2)])],
        ),
        (
            plain(
                vec![v(0)],
                vec![atom(MARKER, &[(0, var(0))]), atom(BOOKING, &[])],
            ),
            vec![],
        ),
        (
            Query {
                interiors: vec![],
                head: vec![HeadTerm::Var],
                rules: vec![
                    Rule {
                        finds: vec![v(0)],
                        atoms: vec![atom(
                            BOOKING,
                            &[(0, var(0)), (1, Term::Literal(Value::U64(7))), (2, var(1))],
                        )],
                        negated: vec![],
                        conditions: vec![],
                    },
                    Rule {
                        finds: vec![v(0)],
                        atoms: vec![booking_atom()],
                        negated: vec![],
                        conditions: vec![ConditionTree::Leaf(Comparison {
                            op: CmpOp::Ge,
                            lhs: var(2),
                            rhs: Term::Literal(Value::U64(4)),
                        })],
                    },
                ],
                rec: None,
            },
            vec![],
        ),
        (
            Query {
                interiors: vec![],
                head: vec![
                    HeadTerm::Aggregate(HeadOp::Sum),
                    HeadTerm::Aggregate(HeadOp::Count),
                ],
                rules: vec![
                    Rule {
                        finds: vec![fold(FoldOp::Sum, 1), FindTerm::Count],
                        atoms: vec![atom(
                            BOOKING,
                            &[(0, var(0)), (1, Term::Literal(Value::U64(7))), (2, var(1))],
                        )],
                        negated: vec![],
                        conditions: vec![],
                    },
                    Rule {
                        finds: vec![fold(FoldOp::Sum, 2), FindTerm::Count],
                        atoms: vec![booking_atom()],
                        negated: vec![],
                        conditions: vec![ConditionTree::Leaf(Comparison {
                            op: CmpOp::Ge,
                            lhs: var(2),
                            rhs: Term::Literal(Value::U64(4)),
                        })],
                    },
                ],
                rec: None,
            },
            vec![],
        ),
        (
            Query {
                interiors: vec![],
                head: vec![HeadTerm::Var],
                rules: vec![
                    Rule {
                        finds: vec![v(1)],
                        atoms: vec![atom(BOOKING, &[(0, Term::Param(ParamId(0))), (2, var(1))])],
                        negated: vec![],
                        conditions: vec![],
                    },
                    Rule {
                        finds: vec![v(2)],
                        atoms: vec![booking_atom()],
                        negated: vec![],
                        conditions: vec![ConditionTree::Leaf(Comparison {
                            op: CmpOp::Ge,
                            lhs: var(2),
                            rhs: Term::Param(ParamId(0)),
                        })],
                    },
                ],
                rec: None,
            },
            vec![ParamValue::Scalar(Value::U64(2))],
        ),
    ]
}

#[test]
fn a_redundant_insert_beside_its_targets_delete_judges_target_side() {
    use bumbledb::{Direction, StatementId};

    use crate::naive::Violation;

    let descriptor = schema();
    let dir = TempDir::new("differential-net-disposition");
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
    let mut naive = NaiveDb::new(&descriptor);

    let a = vec![
        Value::U64(0),
        Value::IntervalU64(bumbledb::Interval::<u64>::new(1, 4).expect("nonempty interval")),
        Value::U64(3),
    ];
    let b = vec![Value::U64(3)];
    let seed = Delta {
        deletes: vec![],
        inserts: vec![(BOOKING, a.clone()), (MARKER, b.clone())],
    };
    let redundant = Delta {
        deletes: vec![(MARKER, b.clone())],
        inserts: vec![(BOOKING, a.clone())],
    };
    let cancelled = Delta {
        deletes: vec![(BOOKING, a.clone()), (MARKER, b.clone())],
        inserts: vec![(BOOKING, a.clone())],
    };
    let ops = vec![
        Op::Write(seed),
        Op::Write(redundant.clone()),
        Op::Write(cancelled.clone()),
    ];
    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagree: {divergence:#?}");
    });
    assert_eq!((summary.commits, summary.aborts), (1, 2));

    for delta in [redundant, cancelled] {
        let violations = naive
            .apply(&delta)
            .expect_err("the stranded booking aborts");
        assert_eq!(
            violations,
            vec![Violation::Containment {
                statement: StatementId(3),
                direction: Direction::TargetRequired,
            }]
        );
    }
}

#[test]
fn fixed_200_op_stream_agrees_with_the_engine() {
    let descriptor = schema();
    let dir = TempDir::new("differential-200");
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
    let mut naive = NaiveDb::new(&descriptor);

    let mut rng = Rng(0x0021_0001);
    let fixed_queries = queries();
    let mut ops = Vec::new();
    let (deltas, pattern_cases) = write_ops(&mut rng);
    assert!(
        pattern_cases >= 5,
        "the stream must emit the net-disposition pattern class: {pattern_cases}"
    );
    for (index, delta) in deltas.into_iter().enumerate() {
        ops.push(Op::Write(delta));
        // The full query battery after every 5th write and after the

        if (index + 1) % 5 == 0 || index == 199 {
            for (query, params) in &fixed_queries {
                ops.push(Op::Query {
                    query: query.clone(),
                    params: params.clone(),
                });
            }
        }
    }

    let summary: Summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagree: {divergence:#?}");
    });

    assert!(summary.commits >= 20, "commits: {summary:?}");
    assert!(summary.aborts >= 20, "aborts: {summary:?}");
    assert!(summary.queries >= 800, "queries: {summary:?}");
    assert!(
        !naive.relation(BOOKING).is_empty(),
        "the stream should leave live bookings"
    );
}
