//! Targeted differential tests over the freshest recursion/finalize
//! machinery: the fixpoint driver's incremental accumulator
//! (`api/prepared/fixpoint.rs` + `image/build.rs: TransientImage::append`
//! — append floors, watermark drift, ping-pong halves across many
//! rounds, finished-slot reads across strata, repeat executions on one
//! prepared handle) and the column-major finalize
//! (`api/prepared/finalize.rs` — strided cell writes, byte-heap ranges
//! for interned strings across columns, interval two-word cells, the
//! aggregate drain). Every expectation is computed naively in-test
//! (`BTreeSet` fixpoints over the same tiny worlds), so a divergence is a
//! defect, never a golden drift.

use std::collections::BTreeSet;

use bumbledb::ir::{Atom, AtomSource, FindTerm, HeadTerm, Query, Rule, Term, VarId};
use bumbledb::schema::FieldId;
use bumbledb::{AnswerValue, Answers, Db, Fact, Interval};

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

fn idb_atom(pred: u16, src: u16, dst: u16) -> Atom {
    Atom {
        source: AtomSource::Idb(bumbledb::PredId(pred)),
        bindings: vec![(FieldId(0), v(src)), (FieldId(1), v(dst))],
    }
}

fn pair_rule(finds: (u16, u16), atoms: Vec<Atom>) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(finds.0)), FindTerm::Var(VarId(finds.1))],
        atoms,
        negated: vec![],
        conditions: vec![],
    }
}

/// `p0(x, z) | Edge(x, z); p0(x, z) | Edge(x, y), p0(y, z)` — the
/// right-linear closure: on a diameter-`d` graph the driver runs ~`d`
/// rounds, so the accumulator appends and the delta ping-pong flip many
/// times within one execution.
fn closure_program() -> bumbledb::Program {
    bumbledb::Program {
        predicates: vec![bumbledb::PredicateDef {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            rules: vec![
                pair_rule((0, 1), vec![edge_atom(0, 1)]),
                pair_rule((0, 2), vec![edge_atom(0, 1), idb_atom(0, 1, 2)]),
            ],
        }],
        output: bumbledb::PredId(0),
    }
}

/// The naive transitive closure (the in-test oracle).
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

/// A deep chain (diameter ~48) plus a cycle and a self-loop: dozens of
/// fixpoint rounds against one prepared handle, executed repeatedly on
/// one snapshot (warm pools: the append floor must reset per execution),
/// then re-executed after a commit that grows the seen-set past the
/// retained capacity (the append's rebuild-whole arm) — every answer set
/// compared against the naive closure.
#[test]
fn deep_chain_closure_matches_naive_across_repeat_executions_and_commits() {
    const CHAIN: u64 = 48;
    let dir = common::TempDir::new("hunt-deep-chain");
    let db = Db::create(dir.path(), Hunt).expect("create");
    let mut edges: BTreeSet<(u64, u64)> = (0..CHAIN).map(|n| (n, n + 1)).collect();
    edges.insert((10, 3)); // a back edge: a cycle inside the chain
    edges.insert((7, 7)); // a self-loop
    edges.insert((5, 30)); // a shortcut: multiple derivations of one pair
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert(&Edge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");

    let expected = naive_closure(&edges);
    let mut prepared = db.prepare(&closure_program()).expect("prepare");
    db.read(|snap| {
        for run in 0..3 {
            let got = answer_pairs(&snap.execute_collect(&mut prepared, &[])?);
            assert_eq!(
                got, expected,
                "closure differs from the naive fixpoint on warm run {run}"
            );
        }
        Ok(())
    })
    .expect("read");

    // Grow the graph: the same prepared handle re-executes against a new
    // generation whose fixpoint is larger than every retained high-water
    // (the accumulator's rebuild-whole and doubling-headroom arms).
    let mut more = edges.clone();
    for n in CHAIN..(CHAIN + 16) {
        more.insert((n, n + 1));
    }
    more.insert((CHAIN + 16, 0)); // one giant cycle: the closure saturates
    db.write(|tx| {
        for &(src, dst) in more.difference(&edges) {
            tx.insert(&Edge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");
    let expected = naive_closure(&more);
    db.read(|snap| {
        for run in 0..2 {
            let got = answer_pairs(&snap.execute_collect(&mut prepared, &[])?);
            assert_eq!(
                got, expected,
                "post-commit closure differs from the naive fixpoint on run {run}"
            );
        }
        Ok(())
    })
    .expect("read");
}

/// Two recursive strata: `p1` (interior) is the Edge closure; the output
/// is `out(x, z) | Link(x, z); out(x, z) | p1(x, y), out(y, z)` — the
/// output's round loop reads the interior's FINISHED image every round
/// while appending its own accumulator, so a finished-slot alias or a
/// stale append floor at the strata boundary diverges from the naive
/// two-level fixpoint.
#[test]
fn two_recursive_strata_match_the_naive_two_level_fixpoint() {
    let dir = common::TempDir::new("hunt-two-strata");
    let db = Db::create(dir.path(), Hunt).expect("create");
    let edges: BTreeSet<(u64, u64)> = (0..12)
        .map(|n| (n, n + 1))
        .chain([(12, 4), (2, 9)])
        .collect();
    let links: BTreeSet<(u64, u64)> = [(100, 0), (101, 6), (3, 3), (200, 100)].into();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert(&Edge { src, dst })?;
        }
        for &(src, dst) in &links {
            tx.insert(&Link { src, dst })?;
        }
        Ok(())
    })
    .expect("write");

    // out = lfp( Link ∪ closure(Edge) ∘ out )
    let closed = naive_closure(&edges);
    let mut expected: BTreeSet<(u64, u64)> = links.clone();
    loop {
        let mut next = expected.clone();
        for &(x, y) in &closed {
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

    let program = bumbledb::Program {
        predicates: vec![
            // PredId(0): the output, recursive through itself, reading
            // PredId(1) from the stratum below.
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![
                    pair_rule((0, 1), vec![link_atom(0, 1)]),
                    pair_rule((0, 2), vec![idb_atom(1, 0, 1), idb_atom(0, 1, 2)]),
                ],
            },
            // PredId(1): the interior Edge closure (its own recursive
            // stratum, finished before the output's opens).
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![
                    pair_rule((0, 1), vec![edge_atom(0, 1)]),
                    pair_rule((0, 2), vec![edge_atom(0, 1), idb_atom(1, 1, 2)]),
                ],
            },
        ],
        output: bumbledb::PredId(0),
    };
    let mut prepared = db.prepare(&program).expect("prepare");
    db.read(|snap| {
        for run in 0..3 {
            let got = answer_pairs(&snap.execute_collect(&mut prepared, &[])?);
            assert_eq!(
                got, expected,
                "two-strata program differs from the naive fixpoint on run {run}"
            );
        }
        Ok(())
    })
    .expect("read");
}

/// Mutual recursion in ONE stratum (odd/even path parity): two seen-sets
/// grow at different rates under one round loop, so per-member
/// watermarks, per-member accumulator floors, and cross-member
/// delta/accumulated binds all get exercised in the same rounds. The
/// naive parity fixpoint is the oracle; an off-by-one-round frontier or
/// a crossed accumulator half changes the parity sets.
#[test]
fn mutual_recursion_parity_matches_the_naive_fixpoint() {
    let dir = common::TempDir::new("hunt-mutual");
    let db = Db::create(dir.path(), Hunt).expect("create");
    // A chain with an odd cycle: parity sets overlap without being equal.
    let edges: BTreeSet<(u64, u64)> = (0..10)
        .map(|n| (n, n + 1))
        .chain([(10, 8), (8, 6), (6, 10), (4, 0)])
        .collect();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert(&Edge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");

    // odd(x,z)  | Edge(x,z);  odd(x,z) | even(x,y), Edge(y,z)
    // even(x,z) | odd(x,y), Edge(y,z)
    let (mut odd, mut even) = (BTreeSet::<(u64, u64)>::new(), BTreeSet::<(u64, u64)>::new());
    loop {
        let mut next_odd = odd.clone();
        let mut next_even = even.clone();
        next_odd.extend(edges.iter().copied());
        for &(x, y) in &even {
            for &(a, z) in edges.iter().filter(|(a, _)| *a == y) {
                debug_assert_eq!(a, y);
                next_odd.insert((x, z));
            }
        }
        for &(x, y) in &odd {
            for &(a, z) in edges.iter().filter(|(a, _)| *a == y) {
                debug_assert_eq!(a, y);
                next_even.insert((x, z));
            }
        }
        if next_odd == odd && next_even == even {
            break;
        }
        odd = next_odd;
        even = next_even;
    }

    let program_with_output = |output: u16| bumbledb::Program {
        predicates: vec![
            // PredId(0): odd-length reachability.
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![
                    pair_rule((0, 1), vec![edge_atom(0, 1)]),
                    pair_rule((0, 2), vec![idb_atom(1, 0, 1), edge_atom(1, 2)]),
                ],
            },
            // PredId(1): even-length (≥ 2) reachability.
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![pair_rule((0, 2), vec![idb_atom(0, 0, 1), edge_atom(1, 2)])],
            },
        ],
        output: bumbledb::PredId(output),
    };
    let mut odd_prepared = db.prepare(&program_with_output(0)).expect("prepare odd");
    let mut even_prepared = db.prepare(&program_with_output(1)).expect("prepare even");
    db.read(|snap| {
        for run in 0..2 {
            let got_odd = answer_pairs(&snap.execute_collect(&mut odd_prepared, &[])?);
            assert_eq!(got_odd, odd, "odd parity differs from naive on run {run}");
            let got_even = answer_pairs(&snap.execute_collect(&mut even_prepared, &[])?);
            assert_eq!(
                got_even, even,
                "even parity differs from naive on run {run}"
            );
        }
        Ok(())
    })
    .expect("read");
}

/// A fold at the output head over a finished recursive interior: the
/// aggregate drain (`finalize_into` → `push_word_answer`) runs on rows
/// grouped from the interior closure's finished image — per-source
/// reachable-set counts, against the naive closure's group counts.
#[test]
fn a_fold_over_the_finished_closure_matches_naive_counts() {
    let dir = common::TempDir::new("hunt-fold");
    let db = Db::create(dir.path(), Hunt).expect("create");
    let edges: BTreeSet<(u64, u64)> = [(1, 0), (2, 1), (3, 1), (4, 2), (4, 3), (0, 5)].into();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert(&Edge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");
    let closed = naive_closure(&edges);
    let mut expected: std::collections::BTreeMap<u64, u64> = std::collections::BTreeMap::new();
    for &(x, _) in &closed {
        *expected.entry(x).or_insert(0) += 1;
    }

    let program = bumbledb::Program {
        predicates: vec![
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![
                    pair_rule((0, 1), vec![edge_atom(0, 1)]),
                    pair_rule((0, 2), vec![edge_atom(0, 1), idb_atom(0, 1, 2)]),
                ],
            },
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Aggregate(bumbledb::HeadOp::Count)],
                rules: vec![Rule {
                    finds: vec![
                        FindTerm::Var(VarId(0)),
                        FindTerm::Aggregate {
                            op: bumbledb::ir::AggOp::Count,
                            over: None,
                        },
                    ],
                    atoms: vec![idb_atom(0, 0, 1)],
                    negated: vec![],
                    conditions: vec![],
                }],
            },
        ],
        output: bumbledb::PredId(1),
    };
    let mut prepared = db.prepare(&program).expect("prepare");
    db.read(|snap| {
        for run in 0..2 {
            let answers = snap.execute_collect(&mut prepared, &[])?;
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

/// The doubling closure — `p0(x, z) | Edge(x, z); p0(x, z) | p0(x, y),
/// p0(y, z)` — puts the SAME predicate in one rule body twice: two delta
/// variants whose image fill must discriminate the delta occurrence from
/// the accumulated one by `OccId`, not by predicate. A crossed slot
/// (delta bound where accumulated belongs, or both to one half) loses
/// derivations or re-derives stale frontiers; the naive closure catches
/// either.
#[test]
fn doubling_closure_with_two_same_predicate_occurrences_matches_naive() {
    let dir = common::TempDir::new("hunt-doubling");
    let db = Db::create(dir.path(), Hunt).expect("create");
    let edges: BTreeSet<(u64, u64)> = (0..33)
        .map(|n| (n, n + 1))
        .chain([(33, 12), (20, 2)])
        .collect();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert(&Edge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");
    let expected = naive_closure(&edges);
    let program = bumbledb::Program {
        predicates: vec![bumbledb::PredicateDef {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            rules: vec![
                pair_rule((0, 1), vec![edge_atom(0, 1)]),
                pair_rule((0, 2), vec![idb_atom(0, 0, 1), idb_atom(0, 1, 2)]),
            ],
        }],
        output: bumbledb::PredId(0),
    };
    let mut prepared = db.prepare(&program).expect("prepare");
    db.read(|snap| {
        for run in 0..3 {
            let got = answer_pairs(&snap.execute_collect(&mut prepared, &[])?);
            assert_eq!(
                got, expected,
                "doubling closure differs from naive on run {run}"
            );
        }
        Ok(())
    })
    .expect("read");
}

/// Typed payload THROUGH the accumulator: the output predicate itself is
/// recursive and its head carries `(u64, str, bool, interval<u64>)` — the
/// transient delta/accumulated images must transpose an intern word, a
/// bool (stored as a BYTE column and read back as 0/1), and a two-word
/// interval per row, round after round, and finalize then resolves the
/// same seen-set. Item attributes propagate along edges; the oracle is
/// the naive reachability product.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one four-column recursive program spelled whole, clearer kept together"
)]
fn typed_payload_propagates_through_the_recursive_accumulator() {
    let dir = common::TempDir::new("hunt-typed-payload");
    let db = Db::create(dir.path(), Hunt).expect("create");
    let rows = item_rows();
    // A path 1 → 2 → 3 → 4 plus a shortcut: rows blend at shared nodes.
    let edges: BTreeSet<(u64, u64)> = [(1, 2), (2, 3), (3, 4), (1, 3)].into();
    db.write(|tx| {
        for row in &rows {
            tx.insert(row)?;
        }
        for &(src, dst) in &edges {
            tx.insert(&Edge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");

    // out(x, n, f, s) | Item(id: x, name: n, flag: f, span: s)
    // out(y, n, f, s) | out(x, n, f, s), Edge(x, y)
    let program = bumbledb::Program {
        predicates: vec![bumbledb::PredicateDef {
            head: vec![HeadTerm::Var, HeadTerm::Var, HeadTerm::Var, HeadTerm::Var],
            rules: vec![
                Rule {
                    finds: vec![
                        FindTerm::Var(VarId(0)),
                        FindTerm::Var(VarId(1)),
                        FindTerm::Var(VarId(2)),
                        FindTerm::Var(VarId(3)),
                    ],
                    atoms: vec![Atom {
                        source: AtomSource::Edb(Item::RELATION),
                        bindings: vec![
                            (FieldId(0), v(0)), // id
                            (FieldId(3), v(1)), // name
                            (FieldId(2), v(2)), // flag
                            (FieldId(5), v(3)), // span
                        ],
                    }],
                    negated: vec![],
                    conditions: vec![],
                },
                Rule {
                    finds: vec![
                        FindTerm::Var(VarId(4)),
                        FindTerm::Var(VarId(1)),
                        FindTerm::Var(VarId(2)),
                        FindTerm::Var(VarId(3)),
                    ],
                    atoms: vec![
                        Atom {
                            source: AtomSource::Idb(bumbledb::PredId(0)),
                            bindings: vec![
                                (FieldId(0), v(0)),
                                (FieldId(1), v(1)),
                                (FieldId(2), v(2)),
                                (FieldId(3), v(3)),
                            ],
                        },
                        edge_atom(0, 4),
                    ],
                    negated: vec![],
                    conditions: vec![],
                },
            ],
        }],
        output: bumbledb::PredId(0),
    };

    // The naive oracle: each item row lands on its own node and every
    // node reachable from it.
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

    let mut prepared = db.prepare(&program).expect("prepare");
    db.read(|snap| {
        for run in 0..3 {
            let answers = snap.execute_collect(&mut prepared, &[])?;
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

/// A budget abort mid-fixpoint leaves the prepared handle reusable: the
/// deep chain trips a two-round budget (COLT views and pooled halves are
/// mid-flight at the abort), then the SAME handle re-executes under the
/// default budget and must reproduce the naive closure exactly — a
/// poisoned pool (stale append floor, an un-unbound view, a frontier
/// watermark surviving the abort) diverges here.
#[test]
fn a_budget_abort_leaves_the_prepared_handle_correct() {
    const CHAIN: u64 = 24;
    let dir = common::TempDir::new("hunt-budget-abort");
    let db = Db::create(dir.path(), Hunt).expect("create");
    let edges: BTreeSet<(u64, u64)> = (0..CHAIN).map(|n| (n, n + 1)).collect();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert(&Edge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");
    let expected = naive_closure(&edges);
    let mut prepared = db.prepare(&closure_program()).expect("prepare");
    prepared.set_fixpoint_budget(2, u64::MAX);
    db.read(|snap| {
        let err = snap
            .execute_collect(&mut prepared, &[])
            .expect_err("a 24-round closure trips a 2-round budget");
        assert!(
            matches!(err, bumbledb::Error::FixpointBudgetExceeded { .. }),
            "typed budget error, got {err:?}"
        );
        Ok(())
    })
    .expect("read");
    prepared.set_fixpoint_budget(1 << 16, 10_000_000);
    db.read(|snap| {
        for run in 0..2 {
            let got = answer_pairs(&snap.execute_collect(&mut prepared, &[])?);
            assert_eq!(
                got, expected,
                "post-abort closure differs from naive on run {run}"
            );
        }
        Ok(())
    })
    .expect("read");
}

/// One recursive handle, alternating parameter envelopes: the source-
/// anchored closure `p0(z) | Edge(src: ?0, dst: z); p0(z) | p0(y),
/// Edge(y, z)` re-executes with sources whose reachable sets differ
/// wildly in size and round count — every pooled half must be refilled
/// (or appended) to exactly the new execution's rows, never the larger
/// previous envelope's. The oracle is per-source naive reachability.
#[test]
fn alternating_param_envelopes_reuse_the_pools_correctly() {
    use bumbledb::ir::ParamId;
    let dir = common::TempDir::new("hunt-param-envelopes");
    let db = Db::create(dir.path(), Hunt).expect("create");
    // Node 0 heads a 30-chain; node 50 heads a 2-chain; node 90 has no
    // outgoing edge (an empty result between two big ones).
    let edges: BTreeSet<(u64, u64)> = (0..30)
        .map(|n| (n, n + 1))
        .chain([(50, 51), (51, 52)])
        .collect();
    db.write(|tx| {
        for &(src, dst) in &edges {
            tx.insert(&Edge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");
    let closed = naive_closure(&edges);
    let reach = |src: u64| -> BTreeSet<u64> {
        closed
            .iter()
            .filter(|(x, _)| *x == src)
            .map(|&(_, z)| z)
            .collect()
    };

    let program = bumbledb::Program {
        predicates: vec![bumbledb::PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![
                Rule {
                    finds: vec![FindTerm::Var(VarId(0))],
                    atoms: vec![Atom {
                        source: AtomSource::Edb(Edge::RELATION),
                        bindings: vec![(FieldId(0), Term::Param(ParamId(0))), (FieldId(1), v(0))],
                    }],
                    negated: vec![],
                    conditions: vec![],
                },
                Rule {
                    finds: vec![FindTerm::Var(VarId(1))],
                    atoms: vec![
                        Atom {
                            source: AtomSource::Idb(bumbledb::PredId(0)),
                            bindings: vec![(FieldId(0), v(0))],
                        },
                        edge_atom(0, 1),
                    ],
                    negated: vec![],
                    conditions: vec![],
                },
            ],
        }],
        output: bumbledb::PredId(0),
    };
    let mut prepared = db.prepare(&program).expect("prepare");
    db.read(|snap| {
        // big → small → empty → big → small: every transition where a
        // stale pooled suffix or floor could leak rows.
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

/// The item fixtures for the finalize hunts: duplicate strings across
/// rows AND across columns (the resolve memo must hand out one byte
/// range per distinct intern), distinct payloads, intervals whose bounds
/// differ per row.
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
            tag: "alpha", // collides with row 1's NAME across columns
            span: Interval::<u64>::new(0, 9).expect("nonempty"),
            payload: pad(b"two-two-two!"),
        },
        Item {
            id: 3,
            score: 42,
            flag: true,
            name: "alpha", // duplicate within the name column
            tag: "y",
            span: Interval::<u64>::new(100, 101).expect("nonempty"),
            payload: pad(b""),
        },
        Item {
            id: 4,
            score: i64::MIN,
            flag: false,
            name: "delta",
            tag: "delta", // the same intern in two columns of ONE row
            span: Interval::<u64>::new(7, u64::MAX >> 2).expect("nonempty"),
            payload: pad(b"\xff\x00\xfe123"),
        },
    ]
}

type ResolvedRow = (String, (u64, u64), i64, String, bool, Vec<u8>, u64);

/// The resolving column-major fill: two string columns straddling an
/// interval (two-word) column, a bytes<12> (two-word) column, and word
/// columns — a scan rule (no key probe: the sink path) whose answers are
/// checked field by field against the inserted facts. A wrong word
/// cursor, a strided-slot mixup, or a byte-heap range shared across the
/// wrong columns changes some cell.
#[test]
fn resolving_columnar_finalize_reproduces_every_cell() {
    let dir = common::TempDir::new("hunt-finalize-resolved");
    let db = Db::create(dir.path(), Hunt).expect("create");
    let rows = item_rows();
    db.write(|tx| {
        for row in &rows {
            tx.insert(row)?;
        }
        Ok(())
    })
    .expect("write");

    // find(name, span, score, tag, flag, payload, id) — deliberately
    // scrambled against field order, interval mid-tuple.
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(3)), // name
            FindTerm::Var(VarId(5)), // span
            FindTerm::Var(VarId(1)), // score
            FindTerm::Var(VarId(4)), // tag
            FindTerm::Var(VarId(2)), // flag
            FindTerm::Var(VarId(6)), // payload
            FindTerm::Var(VarId(0)), // id
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
            let answers = snap.execute_collect(&mut prepared, &[])?;
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

/// The all-words columnar fill (`fill_word_answers` → strided
/// `fill_fixed_column`): no string/bytes column, interval mid-tuple —
/// the interval column must advance the word cursor by TWO or every
/// column to its right reads the wrong words.
#[test]
fn word_columnar_finalize_reproduces_every_cell() {
    let dir = common::TempDir::new("hunt-finalize-words");
    let db = Db::create(dir.path(), Hunt).expect("create");
    let rows = item_rows();
    db.write(|tx| {
        for row in &rows {
            tx.insert(row)?;
        }
        Ok(())
    })
    .expect("write");

    // find(id, span, flag, score): interval second, word columns after.
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
        let answers = snap.execute_collect(&mut prepared, &[])?;
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
