//! The seeded differential stream: a fixed 200-op random stream over a
//! two-relation schema with one `==` pair and one pointwise key. Engine
//! and model must agree on every write verdict (including the violating
//! statement) and on every one of 20 fixed queries — plus the dual-run
//! chase differential ([`chase`]).

mod chase;
mod closed;
mod contradiction;
mod fold;
mod identity_bytes;
mod measure;
mod pack;
mod witness;

use bumbledb::schema::{
    FieldId, IntervalElement, RelationDescriptor, SchemaDescriptor, Side, StatementDescriptor,
    ValueType,
};
use bumbledb::{
    AggOp, AllenMask, Atom, CmpOp, Comparison, ConditionTree, Db, FindTerm, HeadOp, HeadTerm,
    MaskTerm, ParamId, Query, RelationId, Rule, Term, Value, VarId,
};

use crate::differential::{Op, Summary, run};
use crate::fixture::{TempDir, atom, field, var};
use crate::naive::query::ParamValue;
use crate::naive::{Delta, NaiveDb};

/// Booking(room, span: interval<u64>, ref) with the pointwise key
/// (room, span) and the scalar key (ref); Marker(id) with key (id); and
/// the `==` pair Booking(ref) == Marker(id), lowered to its two
/// containments. Materialized statement order:
/// 0 Booking(room, span) -> Booking, 1 Booking(ref) -> Booking,
/// 2 Marker(id) -> Marker, 3 Booking(ref) <= Marker(id),
/// 4 Marker(id) <= Booking(ref).
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

/// splitmix64, local by design: the 200-op stream's exact content is
/// this test's identity (the assertions pin its verdict mix), so it is
/// not deduplicated into `corpus_gen::Rng`.
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

/// A random pick from a relation's current facts, from the generator's
/// own mirror of the state.
fn pick(mirror: &NaiveDb, rel: RelationId, rng: &mut Rng) -> Option<Vec<Value>> {
    let facts = mirror.relation(rel);
    if facts.is_empty() {
        return None;
    }
    let index = usize::try_from(rng.below(facts.len() as u64)).expect("index fits");
    facts.iter().nth(index).map(|tuple| tuple.0.clone())
}

/// The 200 write ops: consistent pair inserts and deletes (which commit),
/// lone inserts and lone-fact deletes (which abort on one of the five
/// statements), overlapping spans (pointwise aborts), duplicate
/// references (scalar-key aborts), redundant inserts alongside a delete
/// of their containment target (the net-disposition pattern class — the
/// second count returned; verdicts must classify target-side on both
/// oracles), and no-op deletes. The generator keeps its own model mirror
/// so deletes can name real facts.
fn write_ops(rng: &mut Rng) -> (Vec<Delta>, u64) {
    let mut mirror = NaiveDb::new(&schema());
    let mut deltas = Vec::new();
    let mut pattern_cases = 0u64;
    for _ in 0..200 {
        let delta = match rng.below(11) {
            // A consistent pair: commits unless the reference is taken or
            // the span overlaps an existing booking's.
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
            // A lone insert: aborts on one containment direction unless
            // its counterpart already stands.
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
            // A lone delete of a real fact: strands the counterpart.
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
            // Demolish a whole pair: commits.
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
            // The net-disposition pattern class (the normative rule:
            // "source side" means facts the transaction actually added):
            // a redundant insert of a committed booking alongside the
            // delete of its containment target — in its plain form
            // (re-insert of a committed fact) and its cancellation form
            // (delete + re-insert netting to nothing). Either way the
            // booking was not genuinely added, so the stranding judges
            // target-side on both oracles, Direction included.
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
            // A no-op delete of a (probably) absent fact.
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

/// The 23 fixed queries, each with its parameters.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // a fixed list, one entry per query
fn queries() -> Vec<(Query, Vec<ParamValue>)> {
    let v = |id: u16| FindTerm::Var(VarId(id));
    let agg = |op: AggOp, over: Option<u16>| FindTerm::Aggregate {
        op,
        over: over.map(VarId),
    };
    vec![
        // 1: every booking.
        (plain(vec![v(0), v(1), v(2)], vec![booking_atom()]), vec![]),
        // 2: every marker.
        (
            plain(vec![v(0)], vec![atom(MARKER, &[(0, var(0))])]),
            vec![],
        ),
        // 3: rooms booked at instant 7 (literal membership).
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
        // 4: marker ids inside some booking's span (point-variable
        // membership anchored at Marker.id).
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
        // 5: the reference join.
        (
            plain(
                vec![v(0), v(2)],
                vec![booking_atom(), atom(MARKER, &[(0, var(2))])],
            ),
            vec![],
        ),
        // 6: markers with no booking in room 0 (negation).
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
        // 7: bookings per room.
        (
            plain(vec![v(0), agg(AggOp::Count, None)], vec![booking_atom()]),
            vec![],
        ),
        // 8: global booking count (empty input ⇒ empty set).
        (
            plain(vec![agg(AggOp::Count, None)], vec![booking_atom()]),
            vec![],
        ),
        // 9: sum of references per room.
        (
            plain(vec![v(0), agg(AggOp::Sum, Some(2))], vec![booking_atom()]),
            vec![],
        ),
        // 10: distinct rooms, globally.
        (
            plain(
                vec![agg(AggOp::CountDistinct, Some(0))],
                vec![booking_atom()],
            ),
            vec![],
        ),
        // 11: the room carrying the maximal reference.
        (
            plain(
                vec![agg(AggOp::ArgMax { key: VarId(2) }, Some(0))],
                vec![booking_atom()],
            ),
            vec![],
        ),
        // 12: the span carrying the minimal reference.
        (
            plain(
                vec![agg(AggOp::ArgMin { key: VarId(2) }, Some(1))],
                vec![booking_atom()],
            ),
            vec![],
        ),
        // 13: overlapping spans across distinct bookings.
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
                            mask: MaskTerm::Literal(AllenMask::INTERSECTS),
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
        // 14: spans containing another booking's span.
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
                            mask: MaskTerm::Literal(AllenMask::COVERS),
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
        // 15: markers lying inside a booking's span, as a predicate.
        (
            Query::single(Rule {
                finds: vec![v(2), v(3)],
                atoms: vec![booking_atom(), atom(MARKER, &[(0, var(3))])],
                negated: vec![],
                conditions: vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Contains,
                    lhs: var(1),
                    rhs: var(3),
                })],
            }),
            vec![],
        ),
        // 16: bookings of one room (scalar param).
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
        // 17: bookings whose room is in a set (param set).
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
        // 18: references above a threshold (order predicate).
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
        // 19: markers not referenced from rooms in a set (negation with a
        // param set inside the negated atom).
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
        // 20: markers gated on Booking being nonempty (zero-binding atom).
        (
            plain(
                vec![v(0)],
                vec![atom(MARKER, &[(0, var(0))]), atom(BOOKING, &[])],
            ),
            vec![],
        ),
        // 21: the rules union with overlap — rooms booked at instant 7
        // ∪ rooms of bookings with reference >= 4 (one head, one sink;
        // the spanning seen-set is the union — 40-execution's rule
        // loop, differentially pinned against the model's set union).
        (
            Query {
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
            },
            vec![],
        ),
        // 22: the multi-rule aggregate — Sum(reference) and Count over
        // the union of the two rules' head-projected bindings (a
        // booking matched by both rules folds once — the union fold,
        // 20-query-ir § aggregation).
        (
            Query {
                head: vec![
                    HeadTerm::Aggregate(HeadOp::Sum),
                    HeadTerm::Aggregate(HeadOp::Count),
                ],
                rules: vec![
                    Rule {
                        finds: vec![agg(AggOp::Sum, Some(1)), agg(AggOp::Count, None)],
                        atoms: vec![atom(
                            BOOKING,
                            &[(0, var(0)), (1, Term::Literal(Value::U64(7))), (2, var(1))],
                        )],
                        negated: vec![],
                        conditions: vec![],
                    },
                    Rule {
                        finds: vec![agg(AggOp::Sum, Some(2)), agg(AggOp::Count, None)],
                        atoms: vec![booking_atom()],
                        negated: vec![],
                        conditions: vec![ConditionTree::Leaf(Comparison {
                            op: CmpOp::Ge,
                            lhs: var(2),
                            rhs: Term::Literal(Value::U64(4)),
                        })],
                    },
                ],
            },
            vec![],
        ),
        // 23: one query-global param reaching both rules — references
        // of room ?0 ∪ references >= ?0 (params bind once; every rule
        // reads the shared slot).
        (
            Query {
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
            },
            vec![ParamValue::Scalar(Value::U64(2))],
        ),
    ]
}

/// The net-disposition Direction-divergence regression: `A(x) <= B(y)` standing,
/// `a ∈ A` and its target `b ∈ B` committed; one transaction does
/// `insert(a)` (a storage no-op) and `delete(b)`. The naive model is
/// normative — "source side" means facts the transaction *actually
/// added* — so both oracles must judge the delete target-side: identical
/// verdicts **including `Direction`**. Covered in the plain form and the
/// cancellation form (`delete(a); insert(a)` netting to nothing), which
/// the engine's old last-disposition delta judged source-side.
#[test]
fn a_redundant_insert_beside_its_targets_delete_judges_target_side() {
    use bumbledb::{Direction, StatementId};

    use crate::naive::Violation;

    let descriptor = schema();
    let dir = TempDir::new("differential-net-disposition");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    // Pre-seed {a, b}: a booking and the marker it requires.
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

    // `run` proved the verdicts identical including Direction; pin the
    // label itself to the normative target-side classification, on the
    // Booking(ref) <= Marker(id) statement.
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
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
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
        // last, keeping the engine's per-commit fsync cost sane.
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
    // The stream must actually exercise both verdicts and real data.
    assert!(summary.commits >= 20, "commits: {summary:?}");
    assert!(summary.aborts >= 20, "aborts: {summary:?}");
    assert!(summary.queries >= 800, "queries: {summary:?}");
    assert!(
        !naive.relation(BOOKING).is_empty(),
        "the stream should leave live bookings"
    );
}
