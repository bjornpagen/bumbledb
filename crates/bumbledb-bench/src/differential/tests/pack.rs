//! `Pack`'s differential criteria (20-query-ir § aggregation): the
//! coalescing fold, engine vs the naive model — which packs **from the
//! point-set definition** (union of point sets → maximal segments,
//! sort-and-merge over logical endpoints), independent of the engine's
//! word sweep. Randomized claim sets per group — overlapping, adjacent,
//! nested, duplicate, ray-bearing — over both element types; the
//! calendar golden (per-person busy in, coalesced busy out,
//! hand-checked with adjacency and a triple overlap); and the
//! multi-rule union fold. `SQLite` cannot express `Pack`
//! (`translate::Inexpressible::PackAggregate`), so the differential here
//! is naive-only by decision.

use bumbledb::schema::{IntervalElement, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{AggOp, Atom, Db, FieldId, FindTerm, Query, RelationId, Rule, Term, Value, VarId};

use crate::differential::{Op, run};
use crate::fixture::{TempDir, field};
use crate::naive::{Delta, NaiveDb, Tuple};

/// Busy(id u64, person u64, slot interval<u64>);
/// Shift(id u64, person u64, slot interval<i64>). No statements: every
/// write commits (ids are plain — the generator numbers them itself).
fn schema() -> SchemaDescriptor {
    let slot = |element: IntervalElement| ValueType::Interval { element };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Busy".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("person", ValueType::U64),
                    field("slot", slot(IntervalElement::U64)),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Shift".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("person", ValueType::U64),
                    field("slot", slot(IntervalElement::I64)),
                ],
            },
        ],
        statements: vec![],
    }
}

const BUSY: RelationId = RelationId(0);
const SHIFT: RelationId = RelationId(1);

/// Q(person, Pack(slot)) over one relation.
fn pack_query(relation: RelationId) -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation,
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// A deterministic LCG (Knuth's MMIX line, the sweep tests' twin) so the
/// randomized corpus is reproducible; draws discard the weak low half.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

/// One random corpus: per person, a claim mix forcing the boundary
/// cases — overlaps and containments from the dense start domain,
/// every third claim on its predecessor's adjacency boundary or one
/// point past it (the minimal gap), every fourth a duplicate of the
/// last, every fifth a ray. Both element types (the i64 lane shifts
/// starts negative).
fn random_corpus(rng: &mut Lcg) -> Delta {
    let mut inserts = Vec::new();
    let mut id = 0u64;
    for person in 0..4u64 {
        let count = rng.next() % 9;
        let mut last: Option<(u64, u64)> = None;
        for ordinal in 0..count {
            let claim = match (ordinal % 5, last) {
                (4, _) => {
                    let start = rng.next() % 24;
                    (start, u64::MAX) // the ray [start, ∞)
                }
                (3, Some(previous)) => previous, // duplicate claim
                (n, Some((_, end))) if n % 3 == 2 && end <= 24 => {
                    let start = end + rng.next() % 2; // adjacent, or gap 1
                    (start, start + 1 + rng.next() % 4)
                }
                _ => {
                    let start = rng.next() % 24;
                    (start, start + 1 + rng.next() % 6)
                }
            };
            last = Some(claim);
            let (start, end) = claim;
            inserts.push((
                BUSY,
                vec![
                    Value::U64(id),
                    Value::U64(person),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
                    ),
                ],
            ));
            // The i64 lane: the same shape shifted below zero (rays
            // become i64 rays).
            let shift = |word: u64| {
                if word == u64::MAX {
                    i64::MAX
                } else {
                    i64::try_from(word).expect("small") - 12
                }
            };
            inserts.push((
                SHIFT,
                vec![
                    Value::U64(id),
                    Value::U64(person),
                    Value::IntervalI64(
                        bumbledb::Interval::<i64>::new(shift(start), shift(end))
                            .expect("generated segment is nonempty"),
                    ),
                ],
            ));
            id += 1;
        }
    }
    Delta {
        deletes: vec![],
        inserts,
    }
}

#[test]
fn randomized_claim_sets_agree_with_the_naive_model() {
    let descriptor = schema();
    for round in 0..40u64 {
        let dir = TempDir::new(&format!("differential-{round}"));
        let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
        let mut naive = NaiveDb::new(&descriptor);
        let mut rng = Lcg(0x5EED_0012 ^ (round << 8));
        let ops = vec![
            Op::Write(random_corpus(&mut rng)),
            Op::Query {
                query: pack_query(BUSY),
                params: vec![],
            },
            Op::Query {
                query: pack_query(SHIFT),
                params: vec![],
            },
        ];
        let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
            panic!("engine and model disagree (round {round}): {divergence:#?}");
        });
        assert_eq!((summary.commits, summary.queries), (1, 2));
    }
}

/// The calendar golden: per-person busy claims in, coalesced busy out —
/// hand-checked. Person 1 carries the adjacency chain and a **triple
/// overlap** (9–12, 10–14, 11–13 → one segment [9, 14)); person 2 the
/// duplicate and the gap; person 3 a lone ray.
#[test]
fn the_calendar_golden_coalesces_by_hand() {
    let descriptor = schema();
    let dir = TempDir::new("calendar-golden");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    let busy = |id: u64, person: u64, start: u64, end: u64| {
        (
            BUSY,
            vec![
                Value::U64(id),
                Value::U64(person),
                Value::IntervalU64(
                    bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
                ),
            ],
        )
    };
    let corpus = Delta {
        deletes: vec![],
        inserts: vec![
            // Person 1: 8–9 meets 9–10 (adjacency), then the triple
            // overlap 9–12 / 10–14 / 11–13 — all one busy block [8, 14).
            busy(0, 1, 8, 9),
            busy(1, 1, 9, 10),
            busy(2, 1, 9, 12),
            busy(3, 1, 10, 14),
            busy(4, 1, 11, 13),
            busy(5, 1, 16, 17), // after the gap
            // Person 2: a duplicate meeting and a contained one.
            busy(6, 2, 9, 11),
            busy(7, 2, 9, 11),
            busy(8, 2, 10, 11),
            busy(9, 2, 13, 15),
            // Person 3: on call from 20 on — the packed ray is a ray.
            busy(10, 3, 20, u64::MAX),
            busy(11, 3, 22, 23),
        ],
    };
    let query = pack_query(BUSY);
    let summary = run(
        &db,
        &mut naive,
        &[
            Op::Write(corpus),
            Op::Query {
                query: query.clone(),
                params: vec![],
            },
        ],
    )
    .unwrap_or_else(|divergence| panic!("engine and model disagree: {divergence:#?}"));
    assert_eq!((summary.commits, summary.queries), (1, 1));

    // `run` proved engine == model; pin the hand-checked rows themselves.
    let coalesced = |person: u64, start: u64, end: u64| {
        Tuple(vec![
            Value::U64(person),
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ])
    };
    let expected: std::collections::BTreeSet<Tuple> = [
        coalesced(1, 8, 14),
        coalesced(1, 16, 17),
        coalesced(2, 9, 11),
        coalesced(2, 13, 15),
        coalesced(3, 20, u64::MAX),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        naive.query(&query, &[]).expect("the golden answers rows"),
        expected
    );
}

/// The multi-rule union fold: Busy(person, slot) ∪ Shift'(person, slot)
/// over one u64 head — a claim derived by both rules folds once, and
/// the coalesce runs over the union (the spanning seen-set semantics).
#[test]
fn multi_rule_pack_folds_the_union_differentially() {
    let descriptor = schema();
    let dir = TempDir::new("union");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    // Two u64-slot rules over the SAME relation, split by id parity with
    // an overlap (id 2 satisfies both): union without double-fold.
    let rule = |op: bumbledb::CmpOp, bound: u64| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![bumbledb::ConditionTree::Leaf(bumbledb::Comparison {
            op,
            lhs: Term::Var(VarId(2)),
            rhs: Term::Literal(Value::U64(bound)),
        })],
    };
    let query = Query {
        head: vec![
            bumbledb::HeadTerm::Var,
            bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Pack),
        ],
        rules: vec![rule(bumbledb::CmpOp::Le, 2), rule(bumbledb::CmpOp::Ge, 2)],
    };
    let corpus = Delta {
        deletes: vec![],
        inserts: vec![
            (
                BUSY,
                vec![
                    Value::U64(0),
                    Value::U64(1),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(1, 3).expect("nonempty interval"),
                    ),
                ],
            ),
            (
                BUSY,
                vec![
                    Value::U64(2),
                    Value::U64(1),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(3, 5).expect("nonempty interval"),
                    ),
                ],
            ),
            (
                BUSY,
                vec![
                    Value::U64(4),
                    Value::U64(1),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(8, 9).expect("nonempty interval"),
                    ),
                ],
            ),
        ],
    };
    let summary = run(
        &db,
        &mut naive,
        &[
            Op::Write(corpus),
            Op::Query {
                query,
                params: vec![],
            },
        ],
    )
    .unwrap_or_else(|divergence| panic!("engine and model disagree: {divergence:#?}"));
    assert_eq!((summary.commits, summary.queries), (1, 1));
}
