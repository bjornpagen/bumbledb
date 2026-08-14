//! The closure goldens (the shipping law: the oracles landed BEFORE
//! the evaluator — `docs/architecture/60-validation.md` § the two
//! oracles): hand-verified reachability answers over a fixed tree and
//! a fixed cyclic graph, held against every oracle that can run a
//! Query — the naive interiors-then-rec-then-main eval
//! ([`NaiveDb::query`]), the `SQLite` recursive lane
//! ([`translate`] executed against the same facts), and the
//! ENGINE's reach driver. The recursive conformance corpus
//! (`crate::conformance`) carries the Lean side
//! (`lean/Bumbledb/Exec/Reach.lean: evalQueryList`) over the Tiny
//! worlds.

use bumbledb::schema::ValidateDescriptor as _;
use std::collections::BTreeSet;

use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    Atom, AtomSource, FieldId, FindTerm, HeadTerm, Interior, InteriorId, Query, Rec, Rule, Term,
    Value, VarId,
};

use crate::fixture::field;
use crate::naive::{Delta, NaiveDb, Tuple};
use crate::translate::{LaneCase, sqlite_expressible_on, translate};

/// The goldens' graph descriptor: `Node(id)`, `Edge(src, dst)` — no
/// statements (nothing here judges writes; the graphs are fixed data).
fn graph_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Node".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Edge".into(),
                fields: vec![field("src", ValueType::U64), field("dst", ValueType::U64)],
            },
        ],
        statements: vec![],
    }
}

/// The descriptor, validated.
fn graph_schema() -> bumbledb::Schema {
    graph_descriptor()
        .validate()
        .expect("the graph schema validates")
}

const NODE: bumbledb::RelationId = bumbledb::RelationId(0);
const EDGE: bumbledb::RelationId = bumbledb::RelationId(1);

/// The fixed tree, edges child → parent: `0 ← {1, 2}`, `1 ← {3, 4}`,
/// `2 ← {5}` — closure sizes bounded by depth 2.
const TREE: [(u64, u64); 5] = [(1, 0), (2, 0), (3, 1), (4, 1), (5, 2)];

/// The fixed cyclic graph: the 3-cycle `0 → 1 → 2 → 0` with the tail
/// `2 → 3` — the fixpoint must saturate the cycle and stop.
const CYCLE: [(u64, u64); 4] = [(0, 1), (1, 2), (2, 0), (2, 3)];

fn v(id: u16) -> Term {
    Term::Var(VarId(id))
}

/// The transitive closure: rec identity-main, linear, one self-atom.
fn closure_query() -> Query {
    Query {
        interiors: vec![],
        rec: Some(Rec {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            base: vec![Rule {
                finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
                atoms: vec![Atom {
                    source: AtomSource::Edb(EDGE),
                    bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
                }],
                negated: vec![],
                conditions: vec![],
            }],
            rec: vec![Rule {
                finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
                atoms: vec![
                    Atom {
                        source: AtomSource::Edb(EDGE),
                        bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
                    },
                    Atom {
                        source: AtomSource::Interior(InteriorId(0)),
                        bindings: vec![(FieldId(0), v(1)), (FieldId(1), v(2))],
                    },
                ],
                negated: vec![],
                conditions: vec![],
            }],
        }),
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

/// Main anti-joins the finished rec: `Node(id = x), ¬rec(c1 = x)` —
/// nodes that are nobody's reachable target.
fn unreached_query() -> Query {
    let mut query = closure_query();
    query.head = vec![HeadTerm::Var];
    query.rules = vec![Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(NODE),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }],
        negated: vec![Atom {
            source: AtomSource::Interior(InteriorId(0)),
            bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
        }],
        conditions: vec![],
    }];
    query
}

/// The naive model over one fixed graph: nodes 0..n plus the edges.
fn naive_world(nodes: u64, edges: &[(u64, u64)]) -> NaiveDb {
    let descriptor = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Node".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Edge".into(),
                fields: vec![field("src", ValueType::U64), field("dst", ValueType::U64)],
            },
        ],
        statements: vec![],
    };
    let mut naive = NaiveDb::new(&descriptor);
    let mut delta = Delta::default();
    for node in 0..nodes {
        delta.inserts.push((NODE, vec![Value::U64(node)]));
    }
    for (src, dst) in edges {
        delta
            .inserts
            .push((EDGE, vec![Value::U64(*src), Value::U64(*dst)]));
    }
    naive
        .apply(&delta)
        .expect("no statements: every write lands");
    naive
}

/// The `SQLite` oracle over the same graph: DDL from the shared
/// mapping, the fixed facts, the translated `WITH RECURSIVE` executed.
fn sqlite_answers(nodes: u64, edges: &[(u64, u64)], query: &Query) -> BTreeSet<Tuple> {
    let schema = graph_schema();
    let conn = rusqlite::Connection::open_in_memory().expect("open");
    for statement in crate::sqlmap::schema_ddl(&schema) {
        conn.execute(&statement, []).expect("ddl");
    }
    for node in 0..nodes {
        conn.execute(
            "INSERT INTO \"Node\" VALUES (?1)",
            [i64::try_from(node).expect("small")],
        )
        .expect("insert node");
    }
    for (src, dst) in edges {
        conn.execute(
            "INSERT INTO \"Edge\" VALUES (?1, ?2)",
            [
                i64::try_from(*src).expect("small"),
                i64::try_from(*dst).expect("small"),
            ],
        )
        .expect("insert edge");
    }
    let translated = translate(query, &schema, &[]).expect("translates");
    let arity = query.head.len();
    let mut statement = conn.prepare(&translated.sql).expect("prepare");
    let rows = statement
        .query_map([], |row| {
            let mut values = Vec::with_capacity(arity);
            for column in 0..arity {
                let raw: i64 = row.get(column)?;
                values.push(Value::U64(u64::try_from(raw).expect("node ids are small")));
            }
            Ok(Tuple(values))
        })
        .expect("query");
    rows.map(|row| row.expect("row decodes")).collect()
}

/// The engine over the same graph: a real store, the query prepared
/// through `Db::prepare` — [`crate::differential::engine_query`] is
/// the shared leg.
fn engine_answers(nodes: u64, edges: &[(u64, u64)], query: &Query) -> BTreeSet<Tuple> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let descriptor = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Node".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Edge".into(),
                fields: vec![field("src", ValueType::U64), field("dst", ValueType::U64)],
            },
        ],
        statements: vec![],
    };
    let tag = format!(
        "recursive-goldens-{}",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let dir = crate::fixture::TempDir::new(&tag);
    let db = bumbledb::Db::create(dir.path(), descriptor).expect("create engine store");
    db.write(|tx| {
        for node in 0..nodes {
            tx.insert_dyn(NODE, &[Value::U64(node)])?;
        }
        for (src, dst) in edges {
            tx.insert_dyn(EDGE, &[Value::U64(*src), Value::U64(*dst)])?;
        }
        Ok(())
    })
    .expect("no statements: every write lands");
    match crate::differential::engine_query(&db, query, &[]) {
        crate::differential::Answers::Ok(rows) => rows,
        other => panic!("closure goldens execute clean: {other:?}"),
    }
}

/// Every oracle that can answer a query, by name — the goldens below
/// loop over this list, so every oracle answers every case.
fn oracle_answers(
    nodes: u64,
    edges: &[(u64, u64)],
    query: &Query,
) -> Vec<(&'static str, BTreeSet<Tuple>)> {
    assert_eq!(
        sqlite_expressible_on(&LaneCase::Query(query), &graph_schema()),
        Ok(()),
        "the goldens' queries stay inside the SQLite lane"
    );
    vec![
        (
            "naive",
            naive_world(nodes, edges)
                .query(query, &[])
                .expect("closure queries raise no runtime error"),
        ),
        ("sqlite", sqlite_answers(nodes, edges, query)),
        ("engine", engine_answers(nodes, edges, query)),
    ]
}

fn pairs(expected: &[(u64, u64)]) -> BTreeSet<Tuple> {
    expected
        .iter()
        .map(|(a, b)| Tuple(vec![Value::U64(*a), Value::U64(*b)]))
        .collect()
}

fn singletons(expected: &[u64]) -> BTreeSet<Tuple> {
    expected
        .iter()
        .map(|node| Tuple(vec![Value::U64(*node)]))
        .collect()
}

/// The tree closure, verified BY HAND: each node's ancestors up the
/// two-level tree — 8 pairs, no more.
#[test]
fn tree_closure_matches_the_hand_answer_on_every_oracle() {
    let expected = pairs(&[
        (1, 0),
        (2, 0),
        (3, 1),
        (3, 0),
        (4, 1),
        (4, 0),
        (5, 2),
        (5, 0),
    ]);
    for (oracle, answers) in oracle_answers(6, &TREE, &closure_query()) {
        assert_eq!(answers, expected, "{oracle} disagrees with the hand answer");
    }
}

/// The cyclic closure, verified BY HAND: everyone on the 3-cycle
/// reaches everyone (self included) plus the tail; the tail reaches
/// nothing — 12 pairs, and the fixpoint terminates on the cycle.
#[test]
fn cyclic_closure_matches_the_hand_answer_on_every_oracle() {
    let expected = pairs(&[
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (1, 0),
        (1, 1),
        (1, 2),
        (1, 3),
        (2, 0),
        (2, 1),
        (2, 2),
        (2, 3),
    ]);
    for (oracle, answers) in oracle_answers(4, &CYCLE, &closure_query()) {
        assert_eq!(answers, expected, "{oracle} disagrees with the hand answer");
    }
}

/// Empty-store recursion: the fixpoint over zero facts is the empty
/// set on every oracle — round 0 derives nothing, the first Δ is empty,
/// and the driver closes without a round.
#[test]
fn recursion_over_the_empty_store_is_empty_on_every_oracle() {
    for (oracle, answers) in oracle_answers(0, &[], &closure_query()) {
        assert!(answers.is_empty(), "{oracle} answered a fact-free store");
    }
    for (oracle, answers) in oracle_answers(0, &[], &unreached_query()) {
        assert!(answers.is_empty(), "{oracle} answered a fact-free store");
    }
}

/// Negation of the finished rec, verified BY HAND: on the tree the
/// leaves `{3, 4, 5}` are nobody's reachable target; on the cyclic
/// graph every node is reached (the cycle reaches itself and the
/// tail), so the answer is empty.
#[test]
fn stratified_negation_matches_the_hand_answers_on_every_oracle() {
    let expected = singletons(&[3, 4, 5]);
    for (oracle, answers) in oracle_answers(6, &TREE, &unreached_query()) {
        assert_eq!(answers, expected, "{oracle} disagrees with the hand answer");
    }
    let expected = singletons(&[]);
    for (oracle, answers) in oracle_answers(4, &CYCLE, &unreached_query()) {
        assert_eq!(answers, expected, "{oracle} disagrees with the hand answer");
    }
}

/// Interval-typed interior columns, engine vs naive: a named interior
/// projects a bound interval variable through its head, and main reads
/// the interval-typed interior column both ways the validation doc
/// defines. Engine-vs-naive only: the `SQLite` translator refuses
/// interval-typed derived columns.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one differential scenario, schema to verdict — clearer kept together"
)]
fn interval_typed_interior_columns_agree_engine_vs_naive() {
    const CLAIM: bumbledb::RelationId = bumbledb::RelationId(0);
    const PROBE: bumbledb::RelationId = bumbledb::RelationId(1);
    let descriptor = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Claim".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: bumbledb::schema::IntervalElement::U64,
                            width: None,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Probe".into(),
                fields: vec![field("at", ValueType::U64)],
            },
        ],
        statements: vec![],
    };
    let claims = [
        (1u64, (1u64, 10u64)),
        (1, (3, 12)),
        (2, (3, 12)),
        (2, (20, 30)),
        (3, (40, u64::MAX)), // the ray `[40, ∞)`
    ];
    let probes = [5u64, 25, 45, 100];

    let carrier = Interior {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: AtomSource::Edb(CLAIM),
                bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    };
    let membership = Query {
        interiors: vec![carrier.clone()],
        rec: None,
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![
                Atom {
                    source: AtomSource::Edb(PROBE),
                    bindings: vec![(FieldId(0), v(1))],
                },
                Atom {
                    source: AtomSource::Interior(InteriorId(0)),
                    bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
                },
            ],
            negated: vec![],
            conditions: vec![],
        }],
    };
    let equality = Query {
        interiors: vec![carrier],
        rec: None,
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![
                Atom {
                    source: AtomSource::Interior(InteriorId(0)),
                    bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(2))],
                },
                Atom {
                    source: AtomSource::Interior(InteriorId(0)),
                    bindings: vec![(FieldId(0), v(1)), (FieldId(1), v(2))],
                },
            ],
            negated: vec![],
            conditions: vec![],
        }],
    };

    let mut naive = NaiveDb::new(&descriptor);
    naive
        .apply(&Delta {
            deletes: vec![],
            inserts: claims
                .iter()
                .map(|(account, (start, end))| {
                    (
                        CLAIM,
                        vec![
                            Value::U64(*account),
                            Value::IntervalU64(
                                bumbledb::Interval::<u64>::new(*start, *end)
                                    .expect("nonempty fixture interval"),
                            ),
                        ],
                    )
                })
                .chain(probes.iter().map(|at| (PROBE, vec![Value::U64(*at)])))
                .collect(),
        })
        .expect("no statements: the fixture commits");

    let dir = crate::fixture::TempDir::new("recursive-interval-interior");
    let db = bumbledb::Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    db.write(|tx| {
        for (account, (start, end)) in &claims {
            tx.insert_dyn(
                CLAIM,
                &[
                    Value::U64(*account),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(*start, *end)
                            .expect("nonempty fixture interval"),
                    ),
                ],
            )?;
        }
        for at in &probes {
            tx.insert_dyn(PROBE, &[Value::U64(*at)])?;
        }
        Ok(())
    })
    .expect("no statements: every write lands");

    let schema = descriptor.validate().expect("validates");
    for (name, query, expected) in [
        (
            "membership",
            &membership,
            pairs(&[(1, 5), (2, 5), (2, 25), (3, 45), (3, 100)]),
        ),
        (
            "equality",
            &equality,
            pairs(&[(1, 1), (2, 2), (3, 3), (1, 2), (2, 1)]),
        ),
    ] {
        let model = naive
            .query(query, &[])
            .expect("the fixture raises no runtime error");
        assert_eq!(
            model, expected,
            "naive {name} disagrees with the hand answer"
        );
        let engine = crate::differential::engine_query(&db, query, &[]);
        assert_eq!(
            engine,
            crate::differential::Answers::Ok(expected),
            "TROPHY (engine vs naive) on the interval-interior {name} face"
        );
        assert_eq!(
            sqlite_expressible_on(&LaneCase::Query(query), &schema),
            Err(crate::translate::Inexpressible::IntervalDerivedColumn),
            "interval derived columns remain the translator limit"
        );
    }
}
