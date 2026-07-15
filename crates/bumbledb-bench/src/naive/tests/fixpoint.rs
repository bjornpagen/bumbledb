//! The naive stratified fixpoint's own landmarks
//! (`lean/Bumbledb/Exec/Fixpoint.lean` is the truth): the degenerate
//! program IS the query, a mutual pair iterates jointly under one round
//! loop, a fold over a recursive predicate reads the finished fixpoint
//! from a strictly higher stratum, and the empty-Δ-at-round-1 boundary
//! stops after the base round. The naive-vs-SQLite closure goldens live
//! with the comparison runners (`crate::differential::tests::recursive`
//! — nothing under `naive/` may touch another oracle).

use std::collections::BTreeSet;

use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    AggOp, Atom, AtomSource, FieldId, FindTerm, HeadTerm, PredId, PredicateDef, Program, Rule,
    Term, Value, VarId,
};

use crate::fixture::field;
use crate::naive::{Delta, NaiveDb, Tuple};

const NODE: bumbledb::RelationId = bumbledb::RelationId(0);
const EDGE: bumbledb::RelationId = bumbledb::RelationId(1);

fn v(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn world(nodes: u64, edges: &[(u64, u64)]) -> NaiveDb {
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
    naive.apply(&delta).expect("no statements: writes land");
    naive
}

fn edge_atom(src: u16, dst: u16) -> Atom {
    Atom {
        source: AtomSource::Edb(EDGE),
        bindings: vec![(FieldId(0), v(src)), (FieldId(1), v(dst))],
    }
}

fn idb_atom(pred: u16, bindings: &[(u16, u16)]) -> Atom {
    Atom {
        source: AtomSource::Idb(PredId(pred)),
        bindings: bindings
            .iter()
            .map(|(field, var)| (FieldId(*field), v(*var)))
            .collect(),
    }
}

fn projection(finds: &[u16], atoms: Vec<Atom>, negated: Vec<Atom>) -> Rule {
    Rule {
        finds: finds.iter().map(|id| FindTerm::Var(VarId(*id))).collect(),
        atoms,
        negated,
        conditions: vec![],
    }
}

fn rows(values: &[&[u64]]) -> BTreeSet<Tuple> {
    values
        .iter()
        .map(|row| Tuple(row.iter().map(|value| Value::U64(*value)).collect()))
        .collect()
}

/// The transitive closure over `Edge` — the shared fixture.
fn closure_predicate() -> PredicateDef {
    PredicateDef {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![
            projection(&[0, 1], vec![edge_atom(0, 1)], vec![]),
            projection(
                &[0, 2],
                vec![edge_atom(0, 1), idb_atom(0, &[(0, 1), (1, 2)])],
                vec![],
            ),
        ],
    }
}

/// The degenerate embedding: a no-`Idb` one-predicate program denotes
/// exactly the query (`lean/Bumbledb/Exec/Fixpoint.lean:
/// degenerate_embedding`).
#[test]
fn the_degenerate_program_is_the_query() {
    let naive = world(3, &[(1, 0), (2, 1)]);
    let rule = projection(&[0, 1], vec![edge_atom(0, 1)], vec![]);
    let query = bumbledb::Query::single(rule);
    let program = Program::from(query.clone());
    assert_eq!(
        naive.program(&program, &[]).expect("no runtime error"),
        naive.query(&query, &[]).expect("no runtime error"),
    );
}

/// A mutual pair — even/odd path length from the root's edge relation —
/// iterates jointly in ONE stratum: `even(x, z) | Edge(x, y),
/// odd(y, z)` beside `odd(x, y) | Edge(x, y); odd(x, z) | Edge(x, y),
/// even(y, z)`, over the chain `3 → 2 → 1 → 0`. Hand answer: odd hops
/// are the odd-length descents, even the even-length ones.
#[test]
fn a_mutual_pair_iterates_jointly() {
    let naive = world(4, &[(3, 2), (2, 1), (1, 0)]);
    let even = PredicateDef {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![projection(
            &[0, 2],
            vec![edge_atom(0, 1), idb_atom(1, &[(0, 1), (1, 2)])],
            vec![],
        )],
    };
    let odd = PredicateDef {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![
            projection(&[0, 1], vec![edge_atom(0, 1)], vec![]),
            projection(
                &[0, 2],
                vec![edge_atom(0, 1), idb_atom(0, &[(0, 1), (1, 2)])],
                vec![],
            ),
        ],
    };
    let mut program = Program {
        predicates: vec![even, odd],
        output: PredId(1),
    };
    // Odd-length paths down the chain: the three edges and 3 → 0.
    assert_eq!(
        naive.program(&program, &[]).expect("no runtime error"),
        rows(&[&[3, 2], &[2, 1], &[1, 0], &[3, 0]]),
    );
    // Even-length paths: the two 2-hop descents.
    program.output = PredId(0);
    assert_eq!(
        naive.program(&program, &[]).expect("no runtime error"),
        rows(&[&[3, 1], &[2, 0]]),
    );
}

/// A fold over a recursive predicate from a strictly higher stratum:
/// `p1(x, Count) | p0(x, y)` counts each node's reachable set AFTER the
/// closure finishes — the count of a finished set, never of a growing
/// one.
#[test]
fn a_fold_reads_the_finished_fixpoint() {
    let naive = world(4, &[(1, 0), (2, 1), (3, 1)]);
    let program = Program {
        predicates: vec![
            closure_predicate(),
            PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Aggregate(bumbledb::HeadOp::Count)],
                rules: vec![Rule {
                    finds: vec![
                        FindTerm::Var(VarId(0)),
                        FindTerm::Aggregate {
                            op: AggOp::Count,
                            over: None,
                        },
                    ],
                    atoms: vec![idb_atom(0, &[(0, 0), (1, 1)])],
                    negated: vec![],
                    conditions: vec![],
                }],
            },
        ],
        output: PredId(1),
    };
    // Ancestor counts: 1 → {0}, 2 → {1, 0}, 3 → {1, 0}.
    assert_eq!(
        naive.program(&program, &[]).expect("no runtime error"),
        rows(&[&[1, 1], &[2, 2], &[3, 2]]),
    );
}

/// The empty-Δ-at-round-1 boundary: on a star graph (every edge into
/// the hub, no onward edge) the recursive rule derives nothing in round
/// one, and the fixpoint is exactly the base round.
#[test]
fn an_empty_first_delta_stops_at_the_base_round() {
    let naive = world(4, &[(1, 0), (2, 0), (3, 0)]);
    let program = Program {
        predicates: vec![closure_predicate()],
        output: PredId(0),
    };
    assert_eq!(
        naive.program(&program, &[]).expect("no runtime error"),
        rows(&[&[1, 0], &[2, 0], &[3, 0]]),
        "the closure of a star IS its edge set",
    );
}
