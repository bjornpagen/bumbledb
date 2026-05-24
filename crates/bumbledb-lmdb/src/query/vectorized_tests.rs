use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb_core::query_ir::{
    TypedClause, TypedFieldBinding, TypedFindTerm, TypedQuery, TypedRelationAtom, TypedTerm,
    TypedVariable,
};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::execute_query_with_mode_for_test;
use crate::query::cover::ExecutionMode;
use crate::{Environment, Fact, QueryResultSet, Result, StorageSchema, Value};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn vectorized_batch_size_one_matches_scalar() -> Result<()> {
    let (env, schema) = env_and_schema("batch-one")?;
    insert_pairs(
        &env,
        &schema,
        &[("R", 1, 10), ("R", 2, 20), ("S", 1, 30), ("S", 2, 40)],
    )?;
    let query = join_query(&[0]);

    let (scalar, _) = run(&env, &schema, &query, ExecutionMode::Scalar)?;
    let (vectorized, stats) = run(
        &env,
        &schema,
        &query,
        ExecutionMode::Vectorized { batch_size: 1 },
    )?;

    assert_eq!(scalar, vectorized);
    assert_eq!(stats.vectorized.batch_size, 1);
    Ok(())
}

#[test]
fn vectorized_batch_sizes_return_identical_sets() -> Result<()> {
    let (env, schema) = env_and_schema("batch-sizes")?;
    insert_pairs(
        &env,
        &schema,
        &[
            ("R", 1, 10),
            ("R", 2, 20),
            ("R", 3, 30),
            ("S", 1, 100),
            ("S", 2, 200),
            ("S", 3, 300),
        ],
    )?;
    let query = join_query(&[0, 1, 2]);
    let (expected, _) = run(&env, &schema, &query, ExecutionMode::Scalar)?;

    for batch_size in [1, 4, 16, 100, 1000, 1024] {
        let (actual, _) = run(
            &env,
            &schema,
            &query,
            ExecutionMode::Vectorized { batch_size },
        )?;
        assert_eq!(actual, expected);
    }
    Ok(())
}

#[test]
fn vectorized_all_probes_succeed() -> Result<()> {
    let (env, schema) = env_and_schema("all-succeed")?;
    insert_pairs(
        &env,
        &schema,
        &[("R", 1, 10), ("R", 2, 20), ("S", 1, 30), ("S", 2, 40)],
    )?;
    let query = join_query(&[0]);

    let (result, stats) = run(
        &env,
        &schema,
        &query,
        ExecutionMode::Vectorized { batch_size: 10 },
    )?;

    assert_eq!(result.facts, vec![row([1]), row([2])]);
    assert_eq!(stats.vectorized.failed_tuples, 0);
    assert!(stats.vectorized.survivor_tuples >= 2);
    Ok(())
}

#[test]
fn vectorized_some_and_all_probe_failures_compact_survivors() -> Result<()> {
    let (env, schema) = env_and_schema("some-fail")?;
    insert_pairs(
        &env,
        &schema,
        &[
            ("R", 1, 10),
            ("R", 2, 20),
            ("R", 3, 30),
            ("S", 1, 40),
            ("S", 3, 50),
        ],
    )?;
    let query = join_query(&[0]);

    let (result, stats) = run(
        &env,
        &schema,
        &query,
        ExecutionMode::Vectorized { batch_size: 10 },
    )?;

    assert_eq!(result.facts, vec![row([1]), row([3])]);
    assert!(stats.vectorized.failed_tuples > 0);

    let (empty_env, empty_schema) = env_and_schema("all-fail")?;
    insert_pairs(
        &empty_env,
        &empty_schema,
        &[("R", 1, 10), ("R", 2, 20), ("S", 9, 90)],
    )?;
    let (empty, empty_stats) = run(
        &empty_env,
        &empty_schema,
        &query,
        ExecutionMode::Vectorized { batch_size: 10 },
    )?;
    assert!(empty.facts.is_empty());
    assert_eq!(empty_stats.vectorized.survivor_tuples, 0);
    Ok(())
}

#[test]
fn vectorized_final_partial_batch_and_empty_relation_work() -> Result<()> {
    let (env, schema) = env_and_schema("partial")?;
    insert_pairs(
        &env,
        &schema,
        &[
            ("R", 1, 10),
            ("R", 2, 20),
            ("R", 3, 30),
            ("R", 4, 40),
            ("R", 5, 50),
            ("S", 1, 60),
            ("S", 2, 70),
            ("S", 3, 80),
            ("S", 4, 90),
            ("S", 5, 100),
        ],
    )?;
    let query = join_query(&[0]);

    let (_result, stats) = run(
        &env,
        &schema,
        &query,
        ExecutionMode::Vectorized { batch_size: 3 },
    )?;
    assert!(stats.vectorized.batches >= 2);

    let (empty_env, empty_schema) = env_and_schema("empty-relation")?;
    empty_env.write(|txn| txn.insert(&empty_schema, &pair("S", 1, 10)))?;
    let (empty, _) = run(
        &empty_env,
        &empty_schema,
        &query,
        ExecutionMode::Vectorized { batch_size: 3 },
    )?;
    assert!(empty.facts.is_empty());
    Ok(())
}

#[test]
fn vectorized_duplicate_witnesses_still_deduplicate_output() -> Result<()> {
    let (env, schema) = env_and_schema("duplicates")?;
    insert_pairs(
        &env,
        &schema,
        &[("R", 1, 10), ("R", 1, 11), ("S", 1, 20), ("S", 1, 21)],
    )?;
    let query = join_query(&[0]);

    let (result, stats) = run(
        &env,
        &schema,
        &query,
        ExecutionMode::Vectorized { batch_size: 2 },
    )?;

    assert_eq!(
        result,
        QueryResultSet::new(
            vec![crate::ResultColumn::Variable("x".to_owned())],
            vec![row([1])]
        )
    );
    assert!(stats.vectorized.survivor_tuples >= 2);
    Ok(())
}

fn run(
    env: &Environment,
    schema: &StorageSchema,
    query: &TypedQuery,
    mode: ExecutionMode,
) -> Result<(QueryResultSet, crate::query::cover::ExecutionStats)> {
    env.read(|txn| execute_query_with_mode_for_test(txn, schema, query, mode))
}

fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("bumbledb-prd14-{name}-{}-{id}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new("Vectorized", vec![pair_relation("R"), pair_relation("S")])
}

fn pair_relation(name: &str) -> RelationDescriptor {
    RelationDescriptor::new(
        name,
        vec![
            FieldDescriptor::new("left", ValueType::U64),
            FieldDescriptor::new("right", ValueType::U64),
        ],
    )
}

fn insert_pairs(
    env: &Environment,
    schema: &StorageSchema,
    facts: &[(&str, u64, u64)],
) -> Result<()> {
    env.write(|txn| {
        for (relation, left, right) in facts {
            txn.insert(schema, &pair(relation, *left, *right))?;
        }
        Ok::<(), crate::Error>(())
    })
}

fn pair(relation: &str, left: u64, right: u64) -> Fact {
    Fact::new(
        relation,
        [("left", Value::U64(left)), ("right", Value::U64(right))],
    )
}

fn join_query(find: &[usize]) -> TypedQuery {
    TypedQuery {
        variables: ["x", "a", "b"]
            .into_iter()
            .enumerate()
            .map(|(id, name)| TypedVariable {
                id,
                name: name.to_owned(),
                value_type: ValueType::U64,
            })
            .collect(),
        inputs: Vec::new(),
        find: find
            .iter()
            .copied()
            .map(|variable| TypedFindTerm::Variable { variable })
            .collect(),
        clauses: vec![
            TypedClause::Relation(atom(0, "R", [(0, "left", 0), (1, "right", 1)])),
            TypedClause::Relation(atom(1, "S", [(0, "left", 0), (1, "right", 2)])),
        ],
    }
}

fn atom<const N: usize>(
    relation_id: usize,
    relation: &str,
    fields: [(usize, &str, usize); N],
) -> TypedRelationAtom {
    TypedRelationAtom {
        relation_id,
        relation: relation.to_owned(),
        fields: fields
            .into_iter()
            .map(|(field_id, field, variable)| TypedFieldBinding {
                field_id,
                field: field.to_owned(),
                value_type: ValueType::U64,
                term: TypedTerm::Variable(variable),
            })
            .collect(),
    }
}

fn row<const N: usize>(values: [u64; N]) -> Vec<Value> {
    values.into_iter().map(Value::U64).collect()
}
