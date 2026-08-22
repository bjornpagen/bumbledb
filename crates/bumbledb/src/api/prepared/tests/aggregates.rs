//! (validate → plan → execute → result buffer), including interval finds.
use super::*;
use crate::ir::FoldOp;
use bumbledb_theory::schema::IntervalElement;

fn interval_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Payroll".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "emp".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const PAYROLL: RelationId = RelationId(0);

fn insert_payroll(env: &Environment, schema: &Schema, rows: &[(u64, u64, (i64, i64))]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, emp, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*emp),
                ValueRef::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
            ],
            schema.relation(PAYROLL).layout(),
            &mut bytes,
        );
        delta.insert(&view, PAYROLL, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit").expect("admitted");
}

#[test]
fn interval_find_round_trips_through_answers() {
    let dir = TempDir::new("prepared-interval-roundtrip");
    let schema = interval_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let stored = [
        (1u64, 10u64, (5i64, 9i64)),
        (2, 10, (-3, 4)),
        (3, 11, (i64::MIN, i64::MAX)),
    ];
    insert_payroll(&env, &schema, &stored);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let types: Vec<ValueType> = prepared
        .signature()
        .columns
        .iter()
        .map(|column| *column.ty())
        .collect();
    assert_eq!(
        types,
        vec![
            ValueType::U64,
            ValueType::Interval {
                element: IntervalElement::I64
            },
        ],
        "the signature reports the interval type"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(u64, i64, i64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(emp), AnswerValue::IntervalI64(iv)) => (emp, iv.start(), iv.end()),
            other => panic!("(u64, interval) answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();
    let mut expected: Vec<(u64, i64, i64)> = stored
        .iter()
        .map(|(_, emp, (start, end))| (*emp, *start, *end))
        .collect();
    expected.sort_unstable();
    assert_eq!(answers, expected, "stored bounds round-trip exactly");
}

#[test]
fn a_closed_group_key_takes_the_dense_table() {
    let dir = TempDir::new("agg-dense-groups");
    let schema = super::folded::closed_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    super::folded::insert_readings(&env, &schema, super::folded::READINGS);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
            FindTerm::Count,
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let EitherSink::Aggregate(sink) = &prepared.sink else {
        panic!("folds build the aggregate sink");
    };
    assert!(
        sink.dense_group_table(),
        "the closed domain (4 rows) proves the dense table"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(u64, i64, u64)> = (0..out.len())
        .map(|answer| {
            let (AnswerValue::U64(kind), AnswerValue::I64(sum), AnswerValue::U64(count)) =
                (out.get(answer, 0), out.get(answer, 1), out.get(answer, 2))
            else {
                panic!("(u64, i64, u64) rows");
            };
            (kind, sum, count)
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(
        answers,
        vec![(0, 100, 1), (1, 421, 2), (2, 220, 1), (3, 300, 1)],
        "per-kind folds over the dense ordinals"
    );

    let open = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let prepared = prepare(&txn, &cache, &schema, &open).expect("prepare");
    let EitherSink::Aggregate(sink) = &prepared.sink else {
        panic!("folds build the aggregate sink");
    };
    assert!(!sink.dense_group_table(), "open domains keep the map");
}

#[test]
fn fold_split_then_gj_split_composes_on_a_grouped_cyclic_body() {
    let dir = TempDir::new("prepared-fold-gj-composition");
    let edge_schema: Schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Edge".into(),
            fields: vec![
                FieldDescriptor {
                    name: "src".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "dst".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &edge_schema).expect("create");
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&edge_schema);
        for (src, dst) in [(1u64, 2u64), (2, 3), (1, 3), (3, 1), (2, 1), (4, 1)] {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(src), ValueRef::U64(dst)],
                edge_schema.relation(RelationId(0)).layout(),
                &mut bytes,
            );
            delta.insert(&view, RelationId(0), &bytes).expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit").expect("admitted");
    }
    let cache = ImageCache::new(&edge_schema);
    let txn = env.read_txn().expect("txn");

    let edge = |a: u16, b: u16| Atom {
        source: crate::ir::AtomSource::Edb(RelationId(0)),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(a))),
            (FieldId(1), Term::Var(VarId(b))),
        ],
    };
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(2)), FindTerm::Count],
        atoms: vec![edge(0, 1), edge(1, 2), edge(0, 2)],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &edge_schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(u64, u64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(z), AnswerValue::U64(n)) => (z, n),
            other => panic!("all-U64 answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();

    assert_eq!(answers, vec![(1, 1), (3, 2)]);
}
