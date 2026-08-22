use std::path::Path;

use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, Row, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};
use bumbledb::{
    CmpOp, Comparison, ConditionTree, Db, FindTerm, Query, RelationId, Rule, Term, Value, VarId,
    with_grounding_disabled,
};

use crate::corpus_gen::{GenConfig, Rng, Scale};
use crate::differential::{Answers, engine_query};
use crate::fixture::{TempDir, atom, field, var};
use crate::naive::query::{ParamValue, QueryError};
use crate::naive::{Delta, NaiveDb};
use crate::querygen::target;
use crate::querygen::{params_for, random_query};

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
    let db = Db::create(dir, descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
    let mut naive = NaiveDb::new(descriptor);
    let delta = Delta {
        deletes: vec![],
        inserts,
    };
    naive.apply(&delta).expect("the corpus commits");
    db.write(|tx| {
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, [fact])?;
        }
        Ok(())
    })
    .expect("the corpus commits")
    .unwrap();
    (db, naive)
}

fn three_way(db: &Db<SchemaDescriptor>, naive: &NaiveDb, query: &Query, _marks: usize, tag: &str) {
    let on = engine_query(db, query, &[]);
    let off = with_grounding_disabled(|| engine_query(db, query, &[]));
    let model = Answers::Ok(naive.query(query, &[]).expect("the model executes"));
    assert_eq!(on, off, "folded and unfolded disagree ({tag})");
    assert_eq!(on, model, "engine and model disagree ({tag})");
}

fn selected(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(READING, &[(0, var(0)), (1, var(1)), (2, var(2))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(rank)))]),
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn selected_count(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Count],
        atoms: vec![
            atom(READING, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(rank)))]),
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn dead_payload() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(READING, &[(0, var(0)), (1, var(1)), (2, var(2))]),
            atom(KIND, &[(0, var(1)), (1, var(3))]),
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn double_closed() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(READING, &[(0, var(0)), (1, var(1)), (2, var(2))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(20)))]),
            atom(KIND, &[(0, var(1)), (1, var(3))]),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(3),
            rhs: Term::Literal(Value::U64(20)),
        })],
    })
}

fn negated_subset(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![atom(READING, &[(0, var(0)), (1, var(1)), (2, var(2))])],
        negated: vec![atom(
            KIND,
            &[(0, var(1)), (1, Term::Literal(Value::U64(rank)))],
        )],
        conditions: vec![],
    })
}

fn negated_whole() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(READING, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(KIND, &[(0, var(1))])],
        conditions: vec![],
    })
}

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

        three_way(&db, &naive, &double_closed(), 2, "double closed");

        three_way(&db, &naive, &selected(99), 0, "S = ∅ (dead rule)");

        three_way(&db, &naive, &negated_subset(99), 1, "negated S = ∅");

        three_way(
            &db,
            &naive,
            &negated_whole(),
            0,
            "complement = ∅ (dead rule)",
        );
    }
}

#[test]
fn the_fold_fixtures_produce_answers() {
    let descriptor = descriptor();
    let dir = TempDir::new("fold-nonempty");
    let mut rng = Rng::new(7);
    let (db, _) = stores(dir.path(), &descriptor, corpus(&mut rng, 16));
    let Answers::Ok(rows) = engine_query(&db, &selected(20), &[]) else {
        unreachable!("fixture queries never overflow")
    };
    assert!(!rows.is_empty(), "rank-20 readings exist by construction");
    let Answers::Ok(rows) = engine_query(&db, &negated_subset(20), &[]) else {
        unreachable!("fixture queries never overflow")
    };
    assert!(
        !rows.is_empty(),
        "non-rank-20 readings exist by construction"
    );
}

#[test]
fn randomized_generator_queries_agree_folded_and_unfolded() {
    const CFG: GenConfig = GenConfig {
        seed: 0x0700_0002,
        scale: Scale::S,
    };
    let dir = TempDir::new("fold-generator");
    let db = target::publish_admitted(dir.path());
    let mut naive = NaiveDb::new(&target::descriptor());
    let delta = super::closed::base_delta();
    naive.apply(&delta).expect("the seed commits");
    db.write(|tx| {
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, [fact])?;
        }
        Ok(())
    })
    .expect("the seed commits")
    .unwrap();

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
            let off = with_grounding_disabled(|| engine_query(&db, &query, &params));
            let model = match naive.query(&query, &params) {
                Ok(rows) => Answers::Ok(rows),
                Err(QueryError::Overflow { .. }) => Answers::Overflow,
            };
            assert_eq!(on, off, "folded and unfolded disagree: {query:?}");
            assert_eq!(on, model, "engine and model disagree: {query:?}");
            compared += 1;
        }
    }
    assert_eq!(compared, 120, "30 queries x 4 draws");
}
