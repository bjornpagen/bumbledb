use super::*;
use crate::{F64, FoldOp};

fn float_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Reading".into(),
            fields: [("group", ValueType::U64), ("id", ValueType::U64), ("value", ValueType::F64)]
                .into_iter().map(|(name, value_type)| FieldDescriptor {
                    name: name.into(), value_type, generation: Generation::None,
                }).collect(),
        }],
        statements: vec![],
    }.validate().unwrap()
}

fn insert(env: &Environment, schema: &Schema, rows: &[(u64, u64, F64)]) {
    let view = env.read_txn().unwrap();
    let mut delta = WriteDelta::new(schema);
    for &(group, id, value) in rows {
        let mut bytes = Vec::new();
        encode_fact(&[ValueRef::U64(group), ValueRef::U64(id), ValueRef::F64(value)],
            schema.relation(RelationId(0)).layout(), &mut bytes);
        delta.insert(&view, RelationId(0), &bytes).unwrap();
    }
    drop(view);
    commit(delta, env).unwrap().unwrap();
}

fn reduction_rule(identity: bool) -> Rule {
    let mut bindings = vec![(FieldId(0), Term::Var(VarId(0))), (FieldId(2), Term::Var(VarId(2)))];
    if identity { bindings.push((FieldId(1), Term::Var(VarId(1)))); }
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Count,
            FindTerm::Aggregate { op: FoldOp::Sum, over: VarId(2) },
            FindTerm::Aggregate { op: FoldOp::Mean, over: VarId(2) }],
        atoms: vec![Atom { source: AtomSource::Edb(RelationId(0)), bindings }],
        negated: vec![], conditions: vec![],
    }
}

#[test]
fn real_query_sum_mean_match_all_independent_rational_fixture_bits() {
    let dir = TempDir::new("float-aggregate-rational-fixtures");
    let schema = float_schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let mut rows = Vec::new();
    let mut expected = std::collections::BTreeMap::new();
    for line in include_str!("../../../../tests/fixtures/f64_reference.txt").lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        if words.first() != Some(&"reduce") { continue; }
        let bits = |word: &str| u64::from_str_radix(word, 16).unwrap();
        let group = expected.len() as u64;
        expected.insert(group, ((words.len() - 3) as u64, bits(words[1]), bits(words[2])));
        for (id, word) in words[3..].iter().enumerate() {
            rows.push((group, id as u64, F64::from_bits(bits(word))));
        }
    }
    assert_eq!(expected.len(), 317);
    rows.reverse();
    insert(&env, &schema, &rows);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().unwrap();
    let mut prepared = prepare(&txn, &cache, &schema, &Query::single(reduction_rule(true))).unwrap();
    for _ in 0..2 {
        let answers = prepared.execute_collect(&txn, &cache, &[] as &[BindValue]).unwrap();
        let actual: std::collections::BTreeMap<_, _> = (0..answers.len()).map(|row| {
            let (AnswerValue::U64(group), AnswerValue::U64(count), AnswerValue::F64(sum), AnswerValue::F64(mean)) =
                (answers.get(row, 0), answers.get(row, 1), answers.get(row, 2), answers.get(row, 3)) else {
                    panic!("typed float aggregate answers")
                };
            (group, (count, sum.to_bits(), mean.to_bits()))
        }).collect();
        assert_eq!(actual, expected);
    }
}

#[test]
fn float_folds_keep_full_binding_grain_dnf_transparency_and_no_empty_group() {
    let dir = TempDir::new("float-aggregate-grain");
    let schema = float_schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    insert(&env, &schema, &[(0, 1, F64::from(3.0)), (0, 2, F64::from(3.0))]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().unwrap();
    for (identity, dnf, expected_count) in [(true, false, 2), (true, true, 2), (false, false, 1), (false, true, 1)] {
        let mut rule = reduction_rule(identity);
        if dnf {
            let leaf = ConditionTree::Leaf(Comparison {
                op: CmpOp::Eq, lhs: Term::Var(VarId(0)), rhs: Term::Literal(Value::U64(0)),
            });
            rule.conditions.push(ConditionTree::Or(vec![leaf.clone(), leaf]));
        }
        let mut prepared = prepare(&txn, &cache, &schema, &Query::single(rule)).unwrap();
        let answers = prepared.execute_collect(&txn, &cache, &[] as &[BindValue]).unwrap();
        assert_eq!(answers.len(), 1);
        assert_eq!(answers.get(0, 1), AnswerValue::U64(expected_count));
        assert_eq!(answers.get(0, 2), AnswerValue::F64(F64::from(if identity { 6.0 } else { 3.0 })));
        assert_eq!(answers.get(0, 3), AnswerValue::F64(F64::from(3.0)));
    }
    let mut rule = reduction_rule(true);
    rule.finds.remove(0); // ungrouped reductions also produce no row on empty input
    rule.conditions.push(ConditionTree::Leaf(Comparison {
        op: CmpOp::Eq, lhs: Term::Var(VarId(0)), rhs: Term::Literal(Value::U64(1)),
    }));
    let mut prepared = prepare(&txn, &cache, &schema, &Query::single(rule)).unwrap();
    assert_eq!(prepared.execute_collect(&txn, &cache, &[] as &[BindValue]).unwrap().len(), 0);
}

#[test]
fn mean_requires_explicit_float_input() {
    let dir = TempDir::new("mean-integer-refusal");
    let schema = float_schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().unwrap();
    let mut rule = reduction_rule(true);
    rule.finds = vec![FindTerm::Aggregate { op: FoldOp::Mean, over: VarId(1) }];
    assert!(matches!(prepare(&txn, &cache, &schema, &Query::single(rule)),
        Err(Error::Validation(crate::error::ValidationError::AggregateInputType { .. }))));
}
