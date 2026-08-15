//! The naive linear-reach landmarks
//! (`lean/Bumbledb/Exec/Reach.lean` is the truth): a fold over a
//! finished rec reads the closed lfp from main, and the empty-Δ-at-round-1
//! boundary stops after the base round. The naive-vs-SQLite closure
//! goldens live with the comparison runners (`crate::differential::tests::recursive`
//! — nothing under `naive/` may touch another oracle).

use std::collections::BTreeSet;

use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    AggOp, Atom, AtomSource, FieldId, FindTerm, HeadTerm, InteriorId, Query, Rec, Rule, Term,
    Value, VarId,
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

fn interior_atom(id: u32, bindings: &[(u16, u16)]) -> Atom {
    Atom {
        source: AtomSource::Interior(InteriorId(id)),
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

fn identity_pair_main() -> Rule {
    projection(&[0, 1], vec![interior_atom(0, &[(0, 0), (1, 1)])], vec![])
}

fn rows(values: &[&[u64]]) -> BTreeSet<Tuple> {
    values
        .iter()
        .map(|row| Tuple(row.iter().map(|value| Value::U64(*value)).collect()))
        .collect()
}

/// Linear closure: identity main over the rec.
fn closure_query() -> Query {
    Query::Reach {
        interiors: vec![],
        rec: Rec {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            base: vec![projection(&[0, 1], vec![edge_atom(0, 1)], vec![])],
            rec: vec![projection(
                &[0, 2],
                vec![edge_atom(0, 1), interior_atom(0, &[(0, 1), (1, 2)])],
                vec![],
            )],
        },
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![identity_pair_main()],
    }
}

/// A fold over a finished rec: `Count` per source over the closure.
#[test]
fn a_fold_reads_the_finished_fixpoint() {
    let naive = world(4, &[(1, 0), (2, 1), (3, 1)]);
    let query = Query::Reach {
        interiors: vec![],
        rec: Rec {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            base: vec![projection(&[0, 1], vec![edge_atom(0, 1)], vec![])],
            rec: vec![projection(
                &[0, 2],
                vec![edge_atom(0, 1), interior_atom(0, &[(0, 1), (1, 2)])],
                vec![],
            )],
        },
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(bumbledb::HeadOp::Count)],
        rules: vec![Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: AggOp::Count,
                    over: None,
                },
            ],
            atoms: vec![interior_atom(0, &[(0, 0), (1, 1)])],
            negated: vec![],
            conditions: vec![],
        }],
    };
    // Ancestor counts: 1 → {0}, 2 → {1, 0}, 3 → {1, 0}.
    assert_eq!(
        naive.query(&query, &[]).expect("no runtime error"),
        rows(&[&[1, 1], &[2, 2], &[3, 2]]),
    );
}

/// The empty-Δ-at-round-1 boundary: on a star graph the rec arm derives
/// nothing in round one, and the fixpoint is exactly the base round.
#[test]
fn an_empty_first_delta_stops_at_the_base_round() {
    let naive = world(4, &[(1, 0), (2, 0), (3, 0)]);
    assert_eq!(
        naive
            .query(&closure_query(), &[])
            .expect("no runtime error"),
        rows(&[&[1, 0], &[2, 0], &[3, 0]]),
        "the closure of a star IS its edge set",
    );
}
