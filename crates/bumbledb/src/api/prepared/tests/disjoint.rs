//! different discriminator literals do not collide, and the fold-free
use super::*;
use crate::ir::FoldOp;
use crate::ir::{HeadOp, HeadTerm};

fn du_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Item".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "payload".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const ITEM: RelationId = RelationId(0);

fn insert_items(env: &Environment, schema: &Schema, rows: &[(u64, u8, u64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, kind, payload) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(u64::from(*kind)),
                ValueRef::U64(*payload),
            ],
            schema.relation(ITEM).layout(),
            &mut bytes,
        );
        delta.insert(&view, ITEM, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit").expect("admitted");
}

fn item_rows() -> Vec<(u64, u8, u64)> {
    vec![(1, 0, 10), (2, 0, 20), (3, 1, 20), (4, 1, 40), (5, 2, 50)]
}

fn arm_rule(kind: u8) -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(ITEM),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Literal(Value::U64(u64::from(kind)))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    }
}

fn du_query(rules: Vec<Rule>) -> Query {
    Query {
        interiors: vec![],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules,
        rec: None,
    }
}

/// The fold-free nullary `Count` on this shape is refused instead (R1, pinned
/// below) — the disjointness proof cannot make a constant informative.
#[test]
fn a_fold_over_a_proven_disjoint_union_absorbs_nothing() {
    let dir = TempDir::new("prepared-disjoint-count");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let rule = |kind: u8| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(ITEM),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Literal(Value::U64(u64::from(kind)))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(HeadOp::Sum)],
        rules: vec![rule(0), rule(1)],
        rec: None,
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(!prepared.distinct_bindings(), "unions always retain dedup");
    let EitherSink::Aggregate(sink) = &prepared.sink else {
        panic!("Sum builds the aggregate sink");
    };
    assert!(!sink.seen_elided(), "the spanning seen-set exists");

    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(
        prepared.sink.distinct_seen(),
        Some(4),
        "all four head projections inhabit the spanning set"
    );

    let mut answers: Vec<(u64, u64)> = (0..out.len())
        .map(|answer| {
            let (AnswerValue::U64(id), AnswerValue::U64(sum)) =
                (out.get(answer, 0), out.get(answer, 1))
            else {
                panic!("U64 columns");
            };
            (id, sum)
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(answers, vec![(1, 10), (2, 20), (3, 20), (4, 40)]);

    let count_rule = |kind: u8| {
        let mut rule = rule(kind);
        rule.finds[1] = FindTerm::Count;
        rule
    };
    let refused = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(HeadOp::Count)],
        rules: vec![count_rule(0), count_rule(1)],
        rec: None,
    };
    let Err(err) = prepare(&txn, &cache, &schema, &refused) else {
        panic!("fold-free nullary Count across written rules refuses");
    };
    assert!(
        matches!(
            err,
            Error::Validation(crate::error::ValidationError::CountAcrossRules { rules: 2 })
        ),
        "typed, named, counted: {err:?}"
    );
}

#[test]
fn a_three_arm_union_absorbs_nothing_across_rules() {
    let dir = TempDir::new("prepared-disjoint-spanning");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_items(&env, &schema, &item_rows());
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = du_query(vec![arm_rule(0), arm_rule(1), arm_rule(2)]);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(u64, u64)> = (0..out.len())
        .map(|answer| {
            let (AnswerValue::U64(id), AnswerValue::U64(payload)) =
                (out.get(answer, 0), out.get(answer, 1))
            else {
                panic!("U64 columns");
            };
            (id, payload)
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(
        answers,
        vec![(1, 10), (2, 20), (3, 20), (4, 40), (5, 50)],
        "the whole union, exactly once each — including the equal payloads"
    );
}
