use std::collections::BTreeSet;

use bumbledb::ir::{
    Atom, AtomSource, FindTerm, HeadTerm, Interior, InteriorId, Query, Rec, RecRule, RecStep, Rule,
    Term, Value, VarId,
};
use bumbledb::schema::FieldId;
use bumbledb::{AnswerValue, Answers, Db, Fact, Interval, NonEmpty, ProjectionRule};

mod common;

bumbledb::schema! {
    pub Hunt;

    relation Edge {
        src: u64,
        dst: u64,
    }
    relation Link {
        src: u64,
        dst: u64,
    }
    relation Item {
        id: u64,
        score: i64,
        flag: bool,
        name: str,
        tag: str,
        span: interval<u64>,
        payload: bytes<12>,
    }
}

fn v(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn edge_atom(src: u16, dst: u16) -> Atom {
    Atom {
        source: AtomSource::Edb(Edge::RELATION),
        bindings: vec![(FieldId(0), v(src)), (FieldId(1), v(dst))],
    }
}

fn link_atom(src: u16, dst: u16) -> Atom {
    Atom {
        source: AtomSource::Edb(Link::RELATION),
        bindings: vec![(FieldId(0), v(src)), (FieldId(1), v(dst))],
    }
}

fn interior_atom(id: u32, src: u16, dst: u16) -> Atom {
    Atom {
        source: AtomSource::Interior(InteriorId(id)),
        bindings: vec![(FieldId(0), v(src)), (FieldId(1), v(dst))],
    }
}

fn identity_pair_main() -> Rule {
    pair_rule((0, 1), vec![interior_atom(0, 0, 1)])
}

fn pair_rule(finds: (u16, u16), atoms: Vec<Atom>) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(finds.0)), FindTerm::Var(VarId(finds.1))],
        atoms,
        negated: vec![],
        conditions: vec![],
    }
}

fn closure_query() -> Query {
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![edge_atom(0, 1)],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![edge_atom(0, 1)],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![identity_pair_main()],
    }
}

fn naive_closure(edges: &BTreeSet<(u64, u64)>) -> BTreeSet<(u64, u64)> {
    let mut closed: BTreeSet<(u64, u64)> = edges.clone();
    loop {
        let mut next = closed.clone();
        for &(x, y) in edges {
            for &(a, z) in closed.iter().filter(|(a, _)| *a == y) {
                debug_assert_eq!(a, y);
                next.insert((x, z));
            }
        }
        if next == closed {
            return closed;
        }
        closed = next;
    }
}

fn answer_pairs(answers: &Answers) -> BTreeSet<(u64, u64)> {
    answers
        .answers()
        .map(|answer| {
            let (AnswerValue::U64(x), AnswerValue::U64(z)) = (answer.get(0), answer.get(1)) else {
                panic!("pair columns are u64")
            };
            (x, z)
        })
        .collect()
}

/// A deep chain (diameter ~48) plus a cycle and a self-loop: dozens of fixpoint
/// rounds against one prepared handle, executed repeatedly on one snapshot
/// (warm pools: the append floor must reset per execution), then re-executed
/// after a commit that grows the seen-set past the retained capacity (the
/// append's rebuild-whole arm) — every answer set compared against the naive
/// closure.
#[test]
fn deep_chain_closure_matches_naive_across_repeat_executions_and_commits() {
    const CHAIN: u64 = 48;
    let dir = common::TempDir::new("hunt-deep-chain");
    let db = Db::create(dir.path(), Hunt)
        .expect("create")
        .expect("accepted");
    let mut edges: BTreeSet<(u64, u64)> = (0..CHAIN).map(|n| (n, n + 1)).collect();
    edges.insert((10, 3)); // a back edge: a cycle inside the chain
    edges.insert((7, 7)); 
    edges.insert((5, 30)); 
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert([&Edge { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();

    let expected = naive_closure(&edges);
    let mut prepared = db.prepare(&closure_query()).expect("prepare");
    db.read(|snap| {
        for run in 0..3 {
            let got =
                answer_pairs(&snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?);
            assert_eq!(
                got, expected,
                "closure differs from the naive fixpoint on warm run {run}"
            );
        }
        Ok(())
    })
    .expect("read");

    // Grow the graph: the same prepared handle re-executes against a new

    let mut more = edges.clone();
    for n in CHAIN..(CHAIN + 16) {
        more.insert((n, n + 1));
    }
    more.insert((CHAIN + 16, 0)); 
    db.write(|tx| {
        for &(src, dst) in more.difference(&edges) {
            tx.insert([&Edge { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();
    let expected = naive_closure(&more);
    db.read(|snap| {
        for run in 0..2 {
            let got =
                answer_pairs(&snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?);
            assert_eq!(
                got, expected,
                "post-commit closure differs from the naive fixpoint on run {run}"
            );
        }
        Ok(())
    })
    .expect("read");
}

#[test]
fn a_finished_interior_feeds_a_linear_rec() {
    let dir = common::TempDir::new("hunt-interior-then-rec");
    let db = Db::create(dir.path(), Hunt)
        .expect("create")
        .expect("accepted");
    let edges: BTreeSet<(u64, u64)> = (0..12)
        .map(|n| (n, n + 1))
        .chain([(12, 4), (2, 9)])
        .collect();
    let links: BTreeSet<(u64, u64)> = [(100, 0), (101, 6), (3, 3), (200, 100)].into();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert([&Edge { src, dst }])?;
        }
        for &(src, dst) in &links {
            tx.insert([&Link { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();

    let mut expected: BTreeSet<(u64, u64)> = links.clone();
    loop {
        let mut next = expected.clone();
        for &(x, y) in &edges {
            for &(a, z) in expected.iter().filter(|(a, _)| *a == y) {
                debug_assert_eq!(a, y);
                next.insert((x, z));
            }
        }
        if next == expected {
            break;
        }
        expected = next;
    }

    let query = Query {
        interiors: vec![Interior {
            rules: vec![ProjectionRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![edge_atom(0, 1)],
                negated: vec![],
                conditions: vec![],
            }],
        }],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![link_atom(0, 1)],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![interior_atom(0, 0, 1)],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![pair_rule((0, 1), vec![interior_atom(1, 0, 1)])],
    };
    let mut prepared = db.prepare(&query).expect("prepare");
    db.read(|snap| {
        for run in 0..3 {
            let got =
                answer_pairs(&snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?);
            assert_eq!(
                got, expected,
                "interior-then-rec differs from the naive fixpoint on run {run}"
            );
        }
        Ok(())
    })
    .expect("read");
}

#[test]
fn a_fold_over_the_finished_closure_matches_naive_counts() {
    let dir = common::TempDir::new("hunt-fold");
    let db = Db::create(dir.path(), Hunt)
        .expect("create")
        .expect("accepted");
    let edges: BTreeSet<(u64, u64)> = [(1, 0), (2, 1), (3, 1), (4, 2), (4, 3), (0, 5)].into();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert([&Edge { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();
    let closed = naive_closure(&edges);
    let mut expected: std::collections::BTreeMap<u64, u64> = std::collections::BTreeMap::new();
    for &(x, _) in &closed {
        *expected.entry(x).or_insert(0) += 1;
    }

    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![edge_atom(0, 1)],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![edge_atom(0, 1)],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(bumbledb::HeadOp::Count)],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Count],
            atoms: vec![interior_atom(0, 0, 1)],
            negated: vec![],
            conditions: vec![],
        }],
    };
    let mut prepared = db.prepare(&query).expect("prepare");
    db.read(|snap| {
        for run in 0..2 {
            let answers = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
            let got: std::collections::BTreeMap<u64, u64> = answers
                .answers()
                .map(|answer| {
                    let (AnswerValue::U64(x), AnswerValue::U64(count)) =
                        (answer.get(0), answer.get(1))
                    else {
                        panic!("count columns are u64")
                    };
                    (x, count)
                })
                .collect();
            assert_eq!(got.len(), answers.len(), "one group per source");
            assert_eq!(got, expected, "fold over closure differs on run {run}");
        }
        Ok(())
    })
    .expect("read");
}

/// Typed payload THROUGH the accumulator: the Reach query itself is recursive
/// and its head carries `(u64, str, bool, interval<u64>)` — the transient
/// delta/accumulated images must transpose an intern word, a bool (stored as a
/// BYTE column and read back as 0/1), and a two-word interval per row, round
/// after round, and finalize then resolves the same seen-set.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one four-column reach query spelled whole, clearer kept together"
)]
fn typed_payload_propagates_through_the_recursive_accumulator() {
    let dir = common::TempDir::new("hunt-typed-payload");
    let db = Db::create(dir.path(), Hunt)
        .expect("create")
        .expect("accepted");
    let rows = item_rows();
    // A path 1 → 2 → 3 → 4 plus a shortcut: rows blend at shared nodes.
    let edges: BTreeSet<(u64, u64)> = [(1, 2), (2, 3), (3, 4), (1, 3)].into();
    db.write(|tx| {
        for row in &rows {
            tx.insert([row])?;
        }
        for &(src, dst) in &edges {
            tx.insert([&Edge { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();

    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1), VarId(2), VarId(3)],
                atoms: vec![Atom {
                    source: AtomSource::Edb(Item::RELATION),
                    bindings: vec![
                        (FieldId(0), v(0)), 
                        (FieldId(3), v(1)), 
                        (FieldId(2), v(2)), 
                        (FieldId(5), v(3)), 
                    ],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(4), VarId(1), VarId(2), VarId(3)],
                self_bindings: vec![
                    (FieldId(0), v(0)),
                    (FieldId(1), v(1)),
                    (FieldId(2), v(2)),
                    (FieldId(3), v(3)),
                ],
                atoms: vec![edge_atom(0, 4)],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var, HeadTerm::Var, HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Var(VarId(1)),
                FindTerm::Var(VarId(2)),
                FindTerm::Var(VarId(3)),
            ],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), v(0)),
                    (FieldId(1), v(1)),
                    (FieldId(2), v(2)),
                    (FieldId(3), v(3)),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    };

    let closed = naive_closure(&edges);
    let expected: BTreeSet<(u64, String, bool, (u64, u64))> = rows
        .iter()
        .flat_map(|row| {
            std::iter::once(row.id)
                .chain(
                    closed
                        .iter()
                        .filter(move |(x, _)| *x == row.id)
                        .map(|&(_, z)| z),
                )
                .map(move |node| {
                    (
                        node,
                        row.name.to_owned(),
                        row.flag,
                        (row.span.start(), row.span.end()),
                    )
                })
        })
        .collect();

    let mut prepared = db.prepare(&query).expect("prepare");
    db.read(|snap| {
        for run in 0..3 {
            let answers = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
            let got: BTreeSet<(u64, String, bool, (u64, u64))> = answers
                .answers()
                .map(|answer| {
                    let AnswerValue::U64(node) = answer.get(0) else {
                        panic!("column 0 is u64")
                    };
                    let AnswerValue::String(name) = answer.get(1) else {
                        panic!("column 1 is a string")
                    };
                    let AnswerValue::Bool(flag) = answer.get(2) else {
                        panic!("column 2 is bool")
                    };
                    let AnswerValue::IntervalU64(span) = answer.get(3) else {
                        panic!("column 3 is an interval<u64>")
                    };
                    (node, name.to_owned(), flag, (span.start(), span.end()))
                })
                .collect();
            assert_eq!(
                got, expected,
                "typed payload propagation differs from naive on run {run}"
            );
        }
        Ok(())
    })
    .expect("read");
}

#[test]
fn a_budget_abort_leaves_the_prepared_handle_correct() {
    const CHAIN: u64 = 66_000;
    let dir = common::TempDir::new("hunt-budget-abort");
    let db = Db::create(dir.path(), Hunt)
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        for n in 0..CHAIN {
            tx.insert([&Edge { src: n, dst: n + 1 }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();
    let mut prepared = db.prepare(&single_source_chain_query()).expect("prepare");
    for run in 0..2 {
        db.read(|snap| {
            let err = snap
                .execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])
                .expect_err("66k hops exceed the default 2^16-round budget");
            assert!(
                matches!(err, bumbledb::Error::DerivedBudgetExceeded { rounds, .. } if rounds > 0),
                "typed budget error on run {run}, got {err:?}"
            );
            Ok(())
        })
        .expect("read");
    }
}

fn single_source_chain_query() -> Query {
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![Atom {
                    source: AtomSource::Edb(Edge::RELATION),
                    bindings: vec![
                        (FieldId(0), Term::Literal(Value::U64(0))),
                        (FieldId(1), Term::Var(VarId(0))),
                    ],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(1)],
                self_bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                atoms: vec![Atom {
                    source: AtomSource::Edb(Edge::RELATION),
                    bindings: vec![
                        (FieldId(0), Term::Var(VarId(0))),
                        (FieldId(1), Term::Var(VarId(1))),
                    ],
                }],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

#[test]
fn alternating_param_envelopes_reuse_the_pools_correctly() {
    use bumbledb::ir::ParamId;
    let dir = common::TempDir::new("hunt-param-envelopes");
    let db = Db::create(dir.path(), Hunt)
        .expect("create")
        .expect("accepted");

    let edges: BTreeSet<(u64, u64)> = (0..30)
        .map(|n| (n, n + 1))
        .chain([(50, 51), (51, 52)])
        .collect();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert([&Edge { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();
    let closed = naive_closure(&edges);
    let reach = |src: u64| -> BTreeSet<u64> {
        closed
            .iter()
            .filter(|(x, _)| *x == src)
            .map(|&(_, z)| z)
            .collect()
    };

    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![Atom {
                    source: AtomSource::Edb(Edge::RELATION),
                    bindings: vec![(FieldId(0), Term::Param(ParamId(0))), (FieldId(1), v(0))],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(1)],
                self_bindings: vec![(FieldId(0), v(0))],
                atoms: vec![edge_atom(0, 1)],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![(FieldId(0), v(0))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    };
    let mut prepared = db.prepare(&query).expect("prepare");
    db.read(|snap| {
        // big → small → empty → big → small: every transition where a

        for &src in &[0u64, 50, 90, 0, 50, 90, 0] {
            let answers = snap.execute_collect(&mut prepared, &[bumbledb::BindValue::U64(src)])?;
            let got: BTreeSet<u64> = answers
                .answers()
                .map(|answer| {
                    let AnswerValue::U64(z) = answer.get(0) else {
                        panic!("closure column is u64")
                    };
                    z
                })
                .collect();
            assert_eq!(
                got,
                reach(src),
                "anchored closure from {src} differs from naive reachability"
            );
        }
        Ok(())
    })
    .expect("read");
}

fn item_rows() -> Vec<Item<'static>> {
    let pad = |s: &[u8]| -> [u8; 12] {
        let mut out = [0u8; 12];
        out[..s.len()].copy_from_slice(s);
        out
    };
    vec![
        Item {
            id: 1,
            score: -7,
            flag: true,
            name: "alpha",
            tag: "x",
            span: Interval::<u64>::new(1, 4).expect("nonempty"),
            payload: pad(b"one"),
        },
        Item {
            id: 2,
            score: 0,
            flag: false,
            name: "beta",
            tag: "alpha", 
            span: Interval::<u64>::new(0, 9).expect("nonempty"),
            payload: pad(b"two-two-two!"),
        },
        Item {
            id: 3,
            score: 42,
            flag: true,
            name: "alpha", 
            tag: "y",
            span: Interval::<u64>::new(100, 101).expect("nonempty"),
            payload: pad(b""),
        },
        Item {
            id: 4,
            score: i64::MIN,
            flag: false,
            name: "delta",
            tag: "delta", 
            span: Interval::<u64>::new(7, u64::MAX >> 2).expect("nonempty"),
            payload: pad(b"\xff\x00\xfe123"),
        },
    ]
}

type ResolvedRow = (String, (u64, u64), i64, String, bool, Vec<u8>, u64);

#[test]
fn resolving_columnar_finalize_reproduces_every_cell() {
    let dir = common::TempDir::new("hunt-finalize-resolved");
    let db = Db::create(dir.path(), Hunt)
        .expect("create")
        .expect("accepted");
    let rows = item_rows();
    db.write(|tx| {
        for row in &rows {
            tx.insert([row])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(3)), 
            FindTerm::Var(VarId(5)), 
            FindTerm::Var(VarId(1)), 
            FindTerm::Var(VarId(4)), 
            FindTerm::Var(VarId(2)), 
            FindTerm::Var(VarId(6)), 
            FindTerm::Var(VarId(0)), 
        ],
        atoms: vec![Atom {
            source: AtomSource::Edb(Item::RELATION),
            bindings: (0..7).map(|f| (FieldId(f), v(f))).collect(),
        }],
        negated: vec![],
        conditions: vec![],
    });
    let expected: BTreeSet<ResolvedRow> = rows
        .iter()
        .map(|row| {
            (
                row.name.to_owned(),
                (row.span.start(), row.span.end()),
                row.score,
                row.tag.to_owned(),
                row.flag,
                row.payload.to_vec(),
                row.id,
            )
        })
        .collect();
    let mut prepared = db.prepare(&query).expect("prepare");
    db.read(|snap| {
        for run in 0..2 {
            let answers = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
            let got: BTreeSet<ResolvedRow> = answers
                .answers()
                .map(|answer| {
                    let AnswerValue::String(name) = answer.get(0) else {
                        panic!("column 0 is a string")
                    };
                    let AnswerValue::IntervalU64(span) = answer.get(1) else {
                        panic!("column 1 is an interval<u64>")
                    };
                    let AnswerValue::I64(score) = answer.get(2) else {
                        panic!("column 2 is i64")
                    };
                    let AnswerValue::String(tag) = answer.get(3) else {
                        panic!("column 3 is a string")
                    };
                    let AnswerValue::Bool(flag) = answer.get(4) else {
                        panic!("column 4 is bool")
                    };
                    let AnswerValue::FixedBytes(payload) = answer.get(5) else {
                        panic!("column 5 is bytes<12>")
                    };
                    let AnswerValue::U64(id) = answer.get(6) else {
                        panic!("column 6 is u64")
                    };
                    (
                        name.to_owned(),
                        (span.start(), span.end()),
                        score,
                        tag.to_owned(),
                        flag,
                        payload.to_vec(),
                        id,
                    )
                })
                .collect();
            assert_eq!(got, expected, "resolved cells differ on run {run}");
        }
        Ok(())
    })
    .expect("read");
}

#[test]
fn word_columnar_finalize_reproduces_every_cell() {
    let dir = common::TempDir::new("hunt-finalize-words");
    let db = Db::create(dir.path(), Hunt)
        .expect("create")
        .expect("accepted");
    let rows = item_rows();
    db.write(|tx| {
        for row in &rows {
            tx.insert([row])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(5)),
            FindTerm::Var(VarId(2)),
            FindTerm::Var(VarId(1)),
        ],
        atoms: vec![Atom {
            source: AtomSource::Edb(Item::RELATION),
            bindings: (0..7).map(|f| (FieldId(f), v(f))).collect(),
        }],
        negated: vec![],
        conditions: vec![],
    });
    let expected: BTreeSet<(u64, (u64, u64), bool, i64)> = rows
        .iter()
        .map(|row| {
            (
                row.id,
                (row.span.start(), row.span.end()),
                row.flag,
                row.score,
            )
        })
        .collect();
    let mut prepared = db.prepare(&query).expect("prepare");
    db.read(|snap| {
        let answers = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
        let got: BTreeSet<(u64, (u64, u64), bool, i64)> = answers
            .answers()
            .map(|answer| {
                let AnswerValue::U64(id) = answer.get(0) else {
                    panic!("column 0 is u64")
                };
                let AnswerValue::IntervalU64(span) = answer.get(1) else {
                    panic!("column 1 is an interval<u64>")
                };
                let AnswerValue::Bool(flag) = answer.get(2) else {
                    panic!("column 2 is bool")
                };
                let AnswerValue::I64(score) = answer.get(3) else {
                    panic!("column 3 is i64")
                };
                (id, (span.start(), span.end()), flag, score)
            })
            .collect();
        assert_eq!(got, expected, "word cells differ");
        Ok(())
    })
    .expect("read");
}
