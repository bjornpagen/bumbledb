//! The dual-run fold differential (PRD 07; the chase.rs dual-run
//! precedent): the chase-evaluator's folds — positive membership,
//! payload-dead, ψ-selected, the |S| == 0 rule death, and the negated
//! COMPLEMENT fold — run through the engine twice (rewrite on and off
//! via `with_chase_disabled`, which covers the evaluator inside the
//! same fixpoint) and three-way compare with the naive model, across
//! randomized corpus draws. The profile surface proves the runs are not
//! vacuously equal: the fold-on plan carries `Role::Folded` marks, the
//! unfolded plan none. A randomized generator slice over the target
//! theory (PRD 06's fold-shaped family among every other shape) closes
//! the loop broad-spectrum.

use std::path::Path;

use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, Row, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};
use bumbledb::{
    AggOp, CmpOp, Comparison, Db, FindTerm, PredicateTree, Query, RelationId, Rule, Term, Value,
    VarId, with_chase_disabled,
};

use crate::corpus_gen::{GenConfig, Rng, Scale};
use crate::differential::{Rows, engine_query};
use crate::fixture::{TempDir, atom, field, var};
use crate::naive::query::{ParamValue, QueryError};
use crate::naive::{Delta, NaiveDb};
use crate::querygen::target;
use crate::querygen::{params_for, random_query};

/// Reading(id fresh, kind u64, value i64) referencing the closed
/// Kind(rank u64; ranks 10/20/20/30) through Reading(kind) <= Kind(id)
/// — the containment doubles as the complement fold's domain witness.
fn descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Reading".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    field("kind", ValueType::U64),
                    field("value", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "A".into(),
                        values: Box::new([Value::U64(10)]),
                    },
                    Row {
                        handle: "B".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "C".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "D".into(),
                        values: Box::new([Value::U64(30)]),
                    },
                ])),
                name: "Kind".into(),
                fields: vec![field("rank", ValueType::U64)],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: READING,
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: KIND,
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
}

const READING: RelationId = RelationId(0);
const KIND: RelationId = RelationId(1);

/// One randomized corpus: every kind witnessed at least once, then
/// random draws — the fold's set must matter on every round.
fn corpus(rng: &mut Rng, rows: u64) -> Vec<(RelationId, Vec<Value>)> {
    (0..rows)
        .map(|id| {
            let kind = if id < 4 { id } else { rng.range(4) };
            let value = i64::try_from(rng.range(1000)).expect("small") - 500;
            (
                READING,
                vec![Value::U64(id), Value::U64(kind), Value::I64(value)],
            )
        })
        .collect()
}

fn stores(
    dir: &Path,
    descriptor: &SchemaDescriptor,
    inserts: Vec<(RelationId, Vec<Value>)>,
) -> (Db<SchemaDescriptor>, NaiveDb) {
    let db = Db::create(dir, descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(descriptor);
    let delta = Delta {
        deletes: vec![],
        inserts,
    };
    naive.apply(&delta).expect("the corpus commits");
    db.write(|tx| {
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, fact)?;
        }
        Ok(())
    })
    .expect("the corpus commits");
    (db, naive)
}

/// The folded occurrences of the query's prepared plan, through the
/// public profile surface.
fn folded(db: &Db<SchemaDescriptor>, query: &Query) -> Vec<bumbledb::FoldedOccurrence> {
    let mut prepared = db.prepare(query).expect("fixture queries validate");
    let (_, mut stats) = db
        .read(|snap| snap.profile(&mut prepared, &[]))
        .expect("profile executes");
    stats.rules.swap_remove(0).folded
}

/// The dual run: folded, unfolded, and the model must produce one
/// result set — with the marks asserted so neither equality is vacuous.
fn three_way(db: &Db<SchemaDescriptor>, naive: &NaiveDb, query: &Query, marks: usize, tag: &str) {
    let on = engine_query(db, query, &[]);
    let off = with_chase_disabled(|| engine_query(db, query, &[]));
    let model = Rows::Ok(naive.query(query, &[]).expect("the model executes"));
    assert_eq!(on, off, "folded and unfolded disagree ({tag})");
    assert_eq!(on, model, "engine and model disagree ({tag})");
    assert_eq!(folded(db, query).len(), marks, "fold marks ({tag})");
    assert!(
        with_chase_disabled(|| folded(db, query)).is_empty(),
        "the off switch keeps every occurrence joining ({tag})"
    );
}

/// `Q(id, value) :- Reading(id, kind = x, value), Kind(id = x, rank == r)`.
fn selected(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(READING, &[(0, var(0)), (1, var(1)), (2, var(2))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(rank)))]),
        ],
        negated: vec![],
        predicates: vec![],
    })
}

/// The aggregate twin: `Q(x, Count) :- Reading(id, kind = x),
/// Kind(id = x, rank == r)` — the fold is sink-independent.
fn selected_count(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(1)),
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![
            atom(READING, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(rank)))]),
        ],
        negated: vec![],
        predicates: vec![],
    })
}

/// The dead-payload shape (PRD 06's pattern class (c) verbatim): the
/// closed atom binds `rank` but nothing reads it.
fn dead_payload() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(READING, &[(0, var(0)), (1, var(1)), (2, var(2))]),
            atom(KIND, &[(0, var(1)), (1, var(3))]),
        ],
        negated: vec![],
        predicates: vec![],
    })
}

/// Two closed atoms over one join variable: both fold, and the sibling
/// ends up carrying TWO plan-constant sets on the same field — two
/// set-bound selection levels composing by intersection at execution.
fn double_closed() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(READING, &[(0, var(0)), (1, var(1)), (2, var(2))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(20)))]),
            atom(KIND, &[(0, var(1)), (1, var(3))]),
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(3),
            rhs: Term::Literal(Value::U64(20)),
        })],
    })
}

/// The complement shape: `Q(id, value) :- Reading(id, kind = x, value),
/// !Kind(id = x, rank == r)` — the negated subset atom.
fn negated_subset(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![atom(READING, &[(0, var(0)), (1, var(1)), (2, var(2))])],
        negated: vec![atom(
            KIND,
            &[(0, var(1)), (1, Term::Literal(Value::U64(rank)))],
        )],
        predicates: vec![],
    })
}

/// The complement's degenerate: `!Kind(id = x)` — S is the whole
/// extension, the complement is empty, the rule is statically dead
/// (every reference is a real handle by the containment).
fn negated_whole() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(READING, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(KIND, &[(0, var(1))])],
        predicates: vec![],
    })
}

/// The fold family across randomized corpus draws, three-way with the
/// marks pinned: ψ-selected folds (both sinks), the dead-payload class,
/// the |S| == 0 rule death, the negated complement — byte-identical
/// folded vs unfolded on every round (the fold is never semantic).
#[test]
fn the_fold_family_agrees_three_ways_across_randomized_draws() {
    let descriptor = descriptor();
    let mut rng = Rng::new(0x0700_0001);
    for round in 0..6 {
        let dir = TempDir::new(&format!("fold-round-{round}"));
        let rows = 24 + rng.range(24);
        let (db, naive) = stores(dir.path(), &descriptor, corpus(&mut rng, rows));
        for rank in [10, 20, 30] {
            three_way(&db, &naive, &selected(rank), 1, &format!("selected {rank}"));
            three_way(
                &db,
                &naive,
                &selected_count(rank),
                1,
                &format!("count {rank}"),
            );
            three_way(
                &db,
                &naive,
                &negated_subset(rank),
                1,
                &format!("negated {rank}"),
            );
        }
        three_way(&db, &naive, &dead_payload(), 1, "dead payload");
        // Two folds over one join variable: two plan-constant sets on
        // one field, intersecting at the sibling's selection levels.
        three_way(&db, &naive, &double_closed(), 2, "double closed");
        // |S| == 0, positive: the rule dies at prepare on the fold-on
        // side and scans to nothing on the unfolded side — no marks (a
        // dead rule plans nothing), same empty result three ways.
        three_way(&db, &naive, &selected(99), 0, "S = ∅ (dead rule)");
        // |S| == 0, negated: the anti-probe never rejects — the atom
        // deletes and every reading survives.
        three_way(&db, &naive, &negated_subset(99), 1, "negated S = ∅");
        // Complement = ∅: the rule dies; the model agrees (every
        // reference is a real handle, so the probe always rejects).
        three_way(
            &db,
            &naive,
            &negated_whole(),
            0,
            "complement = ∅ (dead rule)",
        );
    }
}

/// The non-vacuousness spot check the loop above relies on: the
/// ψ-selected corpus actually produces rows (kinds 0..=3 are all
/// witnessed), so the byte-identity is over non-empty sets.
#[test]
fn the_fold_fixtures_produce_rows() {
    let descriptor = descriptor();
    let dir = TempDir::new("fold-nonempty");
    let mut rng = Rng::new(7);
    let (db, _) = stores(dir.path(), &descriptor, corpus(&mut rng, 16));
    let Rows::Ok(rows) = engine_query(&db, &selected(20), &[]) else {
        unreachable!("fixture queries never overflow")
    };
    assert!(!rows.is_empty(), "rank-20 readings exist by construction");
    let Rows::Ok(rows) = engine_query(&db, &negated_subset(20), &[]) else {
        unreachable!("fixture queries never overflow")
    };
    assert!(
        !rows.is_empty(),
        "non-rank-20 readings exist by construction"
    );
}

/// The broad-spectrum slice: the randomized generator over the target
/// theory (PRD 06's fold-shaped family drawn among every other shape),
/// each draw dual-run against the evaluator's off switch and compared
/// with the model — the fold composes with dressing, negation, sets,
/// and every sink without changing a byte.
#[test]
fn randomized_generator_queries_agree_folded_and_unfolded() {
    const CFG: GenConfig = GenConfig {
        seed: 0x0700_0002,
        scale: Scale::S,
    };
    let dir = TempDir::new("fold-generator");
    let db = Db::create(dir.path(), target::Target).expect("create target store");
    let mut naive = NaiveDb::new(&target::descriptor());
    let delta = super::closed::base_delta();
    naive.apply(&delta).expect("the seed commits");
    db.write(|tx| {
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, fact)?;
        }
        Ok(())
    })
    .expect("the seed commits");

    let mut rng = Rng::new(CFG.seed);
    let mut compared = 0u64;
    for _ in 0..30 {
        let query = random_query(&mut rng, CFG);
        for draw in params_for(&query, &mut rng, CFG) {
            let mut params: Vec<ParamValue> =
                vec![ParamValue::Scalar(Value::Bool(false)); draw.scalars.len() + draw.sets.len()];
            for (param, value) in &draw.scalars {
                params[usize::from(param.0)] = ParamValue::Scalar(value.clone());
            }
            for (param, values) in &draw.sets {
                params[usize::from(param.0)] = ParamValue::Set(values.clone());
            }
            let on = engine_query(&db, &query, &params);
            let off = with_chase_disabled(|| engine_query(&db, &query, &params));
            let model = match naive.query(&query, &params) {
                Ok(rows) => Rows::Ok(rows),
                Err(QueryError::Overflow { .. }) => Rows::Overflow,
                Err(QueryError::MeasureOfRay) => Rows::MeasureOfRay,
            };
            assert_eq!(on, off, "folded and unfolded disagree: {query:?}");
            assert_eq!(on, model, "engine and model disagree: {query:?}");
            compared += 1;
        }
    }
    assert_eq!(compared, 120, "30 queries x 4 draws");
}
