//! The end-of-PRD differential unit test: a fixed, seeded 200-op random
//! stream over a two-relation schema with one `==` pair and one pointwise
//! key. Engine and model must agree on every write verdict (including the
//! violating statement) and on every one of 20 fixed queries.

use std::path::{Path, PathBuf};

use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor,
    Side, StatementDescriptor, ValueType,
};
use bumbledb::{
    AggOp, Atom, CmpOp, Comparison, Db, FindTerm, ParamId, Query, RelationId, Term, Value, VarId,
};

use crate::naive::differential::{run, Op, Summary};
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
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
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

/// splitmix64 — the crate's no-dependency randomness discipline.
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
        Value::IntervalU64(start, end),
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
/// references (scalar-key aborts), and no-op deletes. The generator keeps
/// its own model mirror so deletes can name real facts.
fn write_ops(rng: &mut Rng) -> Vec<Delta> {
    let mut mirror = NaiveDb::new(&schema());
    let mut deltas = Vec::new();
    for _ in 0..200 {
        let delta = match rng.below(10) {
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
            // A no-op delete of a (probably) absent fact.
            _ => Delta {
                deletes: vec![(MARKER, vec![Value::U64(100 + rng.below(8))])],
                inserts: vec![],
            },
        };
        let _ = mirror.apply(&delta);
        deltas.push(delta);
    }
    deltas
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

fn plain(finds: Vec<FindTerm>, atoms: Vec<Atom>) -> Query {
    Query {
        finds,
        atoms,
        negated: vec![],
        predicates: vec![],
    }
}

fn booking_atom() -> Atom {
    atom(BOOKING, vec![(0, var(0)), (1, var(1)), (2, var(2))])
}

/// The 20 fixed queries, each with its parameters.
#[allow(clippy::too_many_lines)] // a fixed list, one entry per query
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
            plain(vec![v(0)], vec![atom(MARKER, vec![(0, var(0))])]),
            vec![],
        ),
        // 3: rooms booked at instant 7 (literal membership).
        (
            plain(
                vec![v(0)],
                vec![atom(
                    BOOKING,
                    vec![(0, var(0)), (1, Term::Literal(Value::U64(7))), (2, var(1))],
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
                    atom(MARKER, vec![(0, var(1))]),
                    atom(BOOKING, vec![(0, var(0)), (1, var(1)), (2, var(2))]),
                ],
            ),
            vec![],
        ),
        // 5: the reference join.
        (
            plain(
                vec![v(0), v(2)],
                vec![booking_atom(), atom(MARKER, vec![(0, var(2))])],
            ),
            vec![],
        ),
        // 6: markers with no booking in room 0 (negation).
        (
            Query {
                finds: vec![v(0)],
                atoms: vec![atom(MARKER, vec![(0, var(0))])],
                negated: vec![atom(
                    BOOKING,
                    vec![(0, Term::Literal(Value::U64(0))), (2, var(0))],
                )],
                predicates: vec![],
            },
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
            Query {
                finds: vec![v(2), v(5)],
                atoms: vec![
                    booking_atom(),
                    atom(BOOKING, vec![(0, var(3)), (1, var(4)), (2, var(5))]),
                ],
                negated: vec![],
                predicates: vec![
                    Comparison {
                        op: CmpOp::Overlaps,
                        lhs: var(1),
                        rhs: var(4),
                    },
                    Comparison {
                        op: CmpOp::Lt,
                        lhs: var(2),
                        rhs: var(5),
                    },
                ],
            },
            vec![],
        ),
        // 14: spans containing another booking's span.
        (
            Query {
                finds: vec![v(2), v(5)],
                atoms: vec![
                    booking_atom(),
                    atom(BOOKING, vec![(0, var(3)), (1, var(4)), (2, var(5))]),
                ],
                negated: vec![],
                predicates: vec![
                    Comparison {
                        op: CmpOp::Contains,
                        lhs: var(1),
                        rhs: var(4),
                    },
                    Comparison {
                        op: CmpOp::Ne,
                        lhs: var(2),
                        rhs: var(5),
                    },
                ],
            },
            vec![],
        ),
        // 15: markers lying inside a booking's span, as a predicate.
        (
            Query {
                finds: vec![v(2), v(3)],
                atoms: vec![booking_atom(), atom(MARKER, vec![(0, var(3))])],
                negated: vec![],
                predicates: vec![Comparison {
                    op: CmpOp::Contains,
                    lhs: var(1),
                    rhs: var(3),
                }],
            },
            vec![],
        ),
        // 16: bookings of one room (scalar param).
        (
            plain(
                vec![v(0), v(1)],
                vec![atom(
                    BOOKING,
                    vec![(0, Term::Param(ParamId(0))), (1, var(0)), (2, var(1))],
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
                    vec![(0, Term::ParamSet(ParamId(0))), (1, var(0)), (2, var(1))],
                )],
            ),
            vec![ParamValue::Set(vec![Value::U64(0), Value::U64(2)])],
        ),
        // 18: references above a threshold (order predicate).
        (
            Query {
                finds: vec![v(2)],
                atoms: vec![booking_atom()],
                negated: vec![],
                predicates: vec![Comparison {
                    op: CmpOp::Ge,
                    lhs: var(2),
                    rhs: Term::Literal(Value::U64(4)),
                }],
            },
            vec![],
        ),
        // 19: markers not referenced from rooms in a set (negation with a
        // param set inside the negated atom).
        (
            Query {
                finds: vec![v(0)],
                atoms: vec![atom(MARKER, vec![(0, var(0))])],
                negated: vec![atom(
                    BOOKING,
                    vec![(0, Term::ParamSet(ParamId(0))), (2, var(0))],
                )],
                predicates: vec![],
            },
            vec![ParamValue::Set(vec![Value::U64(1), Value::U64(2)])],
        ),
        // 20: markers gated on Booking being nonempty (zero-binding atom).
        (
            plain(
                vec![v(0)],
                vec![atom(MARKER, vec![(0, var(0))]), atom(BOOKING, vec![])],
            ),
            vec![],
        ),
    ]
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-naive-{tag}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create test dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn seeded_200_op_stream_agrees_with_the_engine() {
    let descriptor = schema();
    let sealed = descriptor.clone().validate().expect("fixture validates");
    let dir = TempDir::new("differential-200");
    let db = Db::create(dir.path(), &sealed).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    let mut rng = Rng(0x0021_0001);
    let fixed_queries = queries();
    let mut ops = Vec::new();
    for (index, delta) in write_ops(&mut rng).into_iter().enumerate() {
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
