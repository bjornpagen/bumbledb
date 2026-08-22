use super::*;
use crate::ir::FoldOp;
use crate::ir::ParamId;
use bumbledb_theory::schema::{Generation, IntervalElement};

fn pack_schema() -> Schema {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    let fresh_id = || FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Busy".into(),
                fields: vec![
                    fresh_id(),
                    field("person", ValueType::U64),
                    field("cap", ValueType::U64),
                    field(
                        "slot",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Shift".into(),
                fields: vec![
                    fresh_id(),
                    field("person", ValueType::U64),
                    field(
                        "slot",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const BUSY: RelationId = RelationId(0);
const SHIFT: RelationId = RelationId(1);

fn insert_busy(env: &Environment, schema: &Schema, rows: &[(u64, u64, u64, (u64, u64))]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, person, cap, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*person),
                ValueRef::U64(*cap),
                ValueRef::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(*start, *end).expect("nonempty interval"),
                ),
            ],
            schema.relation(BUSY).layout(),
            &mut bytes,
        );
        delta.insert(&view, BUSY, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit").expect("admitted");
}

fn insert_shifts(env: &Environment, schema: &Schema, rows: &[(u64, u64, (i64, i64))]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, person, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*person),
                ValueRef::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
            ],
            schema.relation(SHIFT).layout(),
            &mut bytes,
        );
        delta.insert(&view, SHIFT, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit").expect("admitted");
}

fn pack_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(BUSY),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn packed_u64_answers(out: &Answers) -> Vec<(u64, u64, u64)> {
    let mut answers: Vec<(u64, u64, u64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(person), AnswerValue::IntervalU64(iv)) => {
                (person, iv.start(), iv.end())
            }
            other => panic!("(u64, interval<u64>) answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();
    answers
}

#[test]
fn pack_coalesces_overlap_adjacency_and_duplicates_per_group() {
    let dir = TempDir::new("pack-coalesce");
    let schema = pack_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_busy(
        &env,
        &schema,
        &[
            (1, 10, 0, (1, 3)),
            (2, 10, 0, (2, 5)),
            (3, 10, 0, (5, 7)),
            (4, 10, 0, (2, 4)),
            (5, 10, 0, (9, 10)),
            (6, 20, 0, (4, 6)),
            (7, 20, 0, (4, 6)),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &pack_query()).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(
        packed_u64_answers(&out),
        vec![(10, 1, 7), (10, 9, 10), (20, 4, 6)]
    );
}

/// A ray absorbs everything after its start and the packed ray is a ray (`end
/// == MAX` is the frontier no later claim exceeds) — the I64 element type,
/// negative spans included.
#[test]
fn pack_absorbs_rays_over_i64_spans() {
    let dir = TempDir::new("pack-rays");
    let schema = pack_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_shifts(
        &env,
        &schema,
        &[
            (1, 10, (-5, -2)),
            (2, 10, (-2, 4)),
            (3, 10, (3, i64::MAX)),
            (4, 10, (100, 200)),
            (5, 20, (-10, -9)),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(SHIFT),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(u64, i64, i64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(person), AnswerValue::IntervalI64(iv)) => {
                (person, iv.start(), iv.end())
            }
            other => panic!("(u64, interval<i64>) answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(answers, vec![(10, -5, i64::MAX), (20, -10, -9)]);
}

#[test]
fn pack_groups_exactly_as_sum_does() {
    let dir = TempDir::new("pack-groups-as-sum");
    let schema = pack_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_busy(
        &env,
        &schema,
        &[
            (1, 10, 4, (1, 3)),
            (2, 10, 6, (7, 9)),
            (3, 20, 5, (2, 4)),
            (4, 30, 1, (2, 4)),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let sum_query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(2),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(BUSY),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut sum = prepare(&txn, &cache, &schema, &sum_query).expect("prepare");
    let sum_out = sum
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let mut sum_groups: Vec<u64> = (0..sum_out.len())
        .map(|answer| match sum_out.get(answer, 0) {
            AnswerValue::U64(person) => person,
            other => panic!("u64 group key: {other:?}"),
        })
        .collect();
    sum_groups.sort_unstable();

    let mut pack = prepare(&txn, &cache, &schema, &pack_query()).expect("prepare");
    let pack_out = pack
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    let mut pack_groups: Vec<u64> = packed_u64_answers(&pack_out)
        .into_iter()
        .map(|(person, _, _)| person)
        .collect();
    pack_groups.dedup();
    assert_eq!(pack_groups, sum_groups);

    assert_eq!(pack_out.len(), 4);
}

#[test]
fn multi_rule_pack_folds_the_union() {
    let dir = TempDir::new("pack-union");
    let schema = pack_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_busy(
        &env,
        &schema,
        &[(1, 10, 1, (1, 3)), (2, 10, 5, (5, 6)), (3, 10, 9, (6, 8))],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let rule = |op: CmpOp, param: u16| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(BUSY),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(2))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(2)),
            rhs: Term::Param(ParamId(param)),
        })],
    };
    let query = Query {
        interiors: vec![],
        head: vec![
            crate::ir::HeadTerm::Var,
            crate::ir::HeadTerm::Aggregate(crate::ir::HeadOp::Pack),
        ],
        rules: vec![rule(CmpOp::Ge, 0), rule(CmpOp::Le, 1)],
        rec: None,
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(5), BindValue::U64(5)])
        .expect("execute");

    assert_eq!(packed_u64_answers(&out), vec![(10, 1, 3), (10, 5, 8)]);
}
