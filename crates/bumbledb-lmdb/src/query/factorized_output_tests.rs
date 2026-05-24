use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb_core::query_ir::{
    TypedClause, TypedFieldBinding, TypedFindTerm, TypedQuery, TypedRelationAtom, TypedTerm,
    TypedVariable,
};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::execute_query_with_output_mode_for_test;
use crate::query::sink::OutputMode;
use crate::{Environment, Fact, QueryResultSet, Result, StorageSchema, Value};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn factorized_output_equals_materialized_for_clover() -> Result<()> {
    let (env, schema) = env_and_schema("clover")?;
    insert_clover(&env, &schema)?;
    let query = clover_query(&[0, 1, 2, 3]);

    assert_modes_equal(&env, &schema, &query)?;
    Ok(())
}

#[test]
fn factorized_output_equals_materialized_for_triangle() -> Result<()> {
    let (env, schema) = env_and_schema("triangle")?;
    env.write(|txn| {
        for fact in [
            pair("R", 1, 2),
            pair("R", 1, 3),
            pair("S", 2, 4),
            pair("S", 3, 4),
            pair("T", 4, 1),
        ] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "y", "z"],
        &[0, 1, 2],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 1), (1, "right", 2)]),
            atom(2, "T", [(0, "left", 2), (1, "right", 0)]),
        ],
    );

    assert_modes_equal(&env, &schema, &query)?;
    Ok(())
}

#[test]
fn factorized_output_suppresses_duplicate_projection_witnesses() -> Result<()> {
    let (env, schema) = env_and_schema("duplicates")?;
    env.write(|txn| {
        for fact in [
            pair("R", 1, 10),
            pair("R", 1, 11),
            pair("S", 1, 20),
            pair("S", 1, 21),
        ] {
            txn.insert(&schema, &fact)?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "a", "b"],
        &[0],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
        ],
    );

    let (result, stats) = run_factorized(&env, &schema, &query)?;

    assert_eq!(result.facts, vec![row([1])]);
    assert_eq!(stats.materialized_facts, 1);
    assert!(stats.duplicate_witnesses_suppressed > 0);
    Ok(())
}

#[test]
fn factorized_output_records_cartesian_projection_compression() -> Result<()> {
    let (env, schema) = env_and_schema("cartesian")?;
    env.write(|txn| {
        for a in 0..5 {
            txn.insert(&schema, &pair("R", 1, a))?;
        }
        for b in 0..5 {
            txn.insert(&schema, &pair("S", 1, b))?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "a", "b"],
        &[0],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
        ],
    );

    let (_result, stats) = run_factorized(&env, &schema, &query)?;

    assert_eq!(stats.materialized_facts, 1);
    assert!(stats.expansions_avoided > 0);
    Ok(())
}

#[test]
fn encoded_set_sink_avoids_reexpanding_seen_projection_prefix() -> Result<()> {
    let (env, schema) = env_and_schema("prefix-avoidance")?;
    env.write(|txn| {
        for a in 0..10 {
            txn.insert(&schema, &pair("R", 1, a))?;
        }
        for b in 0..10 {
            txn.insert(&schema, &pair("S", 1, b))?;
        }
        Ok::<(), crate::Error>(())
    })?;
    let query = typed_query(
        &["x", "a", "b"],
        &[0],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
        ],
    );

    let (_result, stats) = env.read(|txn| {
        execute_query_with_output_mode_for_test(txn, &schema, &query, OutputMode::Materialized)
    })?;

    assert_eq!(stats.materialized_facts, 1);
    assert!(stats.expansions_avoided > 0);
    Ok(())
}

#[test]
fn factorized_output_decodes_strings_and_bytes_from_lmdb_dictionary() -> Result<()> {
    let (env, schema) = env_and_schema("dictionary")?;
    env.write(|txn| txn.insert(&schema, &text_fact(1, "alice", b"blob")))?;
    let query = TypedQuery {
        variables: vec![
            TypedVariable {
                id: 0,
                name: "s".to_owned(),
                value_type: ValueType::String,
            },
            TypedVariable {
                id: 1,
                name: "b".to_owned(),
                value_type: ValueType::Bytes,
            },
        ],
        inputs: Vec::new(),
        find: vec![
            TypedFindTerm::Variable { variable: 0 },
            TypedFindTerm::Variable { variable: 1 },
        ],
        clauses: vec![TypedClause::Relation(text_atom([
            text_field("name", 1, TypedTerm::Variable(0), ValueType::String),
            text_field("blob", 2, TypedTerm::Variable(1), ValueType::Bytes),
        ]))],
    };

    let (result, _stats) = run_factorized(&env, &schema, &query)?;

    assert_eq!(
        result.facts,
        vec![vec![
            Value::String("alice".to_owned()),
            Value::Bytes(b"blob".to_vec())
        ]]
    );
    Ok(())
}

#[test]
fn factorized_output_handles_empty_output() -> Result<()> {
    let (env, schema) = env_and_schema("empty")?;
    env.write(|txn| txn.insert(&schema, &pair("R", 1, 10)))?;
    let query = typed_query(
        &["x", "a", "b"],
        &[0],
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
        ],
    );

    let (result, stats) = run_factorized(&env, &schema, &query)?;

    assert!(result.facts.is_empty());
    assert_eq!(stats.materialized_facts, 0);
    Ok(())
}

fn assert_modes_equal(env: &Environment, schema: &StorageSchema, query: &TypedQuery) -> Result<()> {
    let materialized = env.read(|txn| {
        execute_query_with_output_mode_for_test(txn, schema, query, OutputMode::Materialized)
    })?;
    let factorized = env.read(|txn| {
        execute_query_with_output_mode_for_test(txn, schema, query, OutputMode::Factorized)
    })?;
    assert_eq!(materialized.0, factorized.0);
    assert_eq!(
        materialized.1.materialized_facts,
        factorized.1.materialized_facts
    );
    Ok(())
}

fn run_factorized(
    env: &Environment,
    schema: &StorageSchema,
    query: &TypedQuery,
) -> Result<(QueryResultSet, crate::query::sink::OutputStats)> {
    env.read(|txn| {
        execute_query_with_output_mode_for_test(txn, schema, query, OutputMode::Factorized)
    })
}

fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("bumbledb-prd17-{name}-{}-{id}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "Factorized",
        vec![
            pair_relation("R"),
            pair_relation("S"),
            pair_relation("T"),
            text_relation(),
        ],
    )
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

fn text_relation() -> RelationDescriptor {
    RelationDescriptor::new(
        "Text",
        vec![
            FieldDescriptor::new("id", ValueType::U64),
            FieldDescriptor::new("name", ValueType::String),
            FieldDescriptor::new("blob", ValueType::Bytes),
        ],
    )
}

fn insert_clover(env: &Environment, schema: &StorageSchema) -> Result<()> {
    env.write(|txn| {
        for fact in [
            pair("R", 1, 10),
            pair("R", 2, 20),
            pair("S", 1, 30),
            pair("S", 3, 40),
            pair("T", 1, 50),
            pair("T", 4, 60),
        ] {
            txn.insert(schema, &fact)?;
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

fn text_fact(id: u64, name: &str, blob: &[u8]) -> Fact {
    Fact::new(
        "Text",
        [
            ("id", Value::U64(id)),
            ("name", Value::String(name.to_owned())),
            ("blob", Value::Bytes(blob.to_vec())),
        ],
    )
}

fn clover_query(find: &[usize]) -> TypedQuery {
    typed_query(
        &["x", "a", "b", "c"],
        find,
        vec![
            atom(0, "R", [(0, "left", 0), (1, "right", 1)]),
            atom(1, "S", [(0, "left", 0), (1, "right", 2)]),
            atom(2, "T", [(0, "left", 0), (1, "right", 3)]),
        ],
    )
}

fn typed_query(variables: &[&str], find: &[usize], atoms: Vec<TypedRelationAtom>) -> TypedQuery {
    TypedQuery {
        variables: variables
            .iter()
            .enumerate()
            .map(|(id, name)| TypedVariable {
                id,
                name: (*name).to_owned(),
                value_type: ValueType::U64,
            })
            .collect(),
        inputs: Vec::new(),
        find: find
            .iter()
            .copied()
            .map(|variable| TypedFindTerm::Variable { variable })
            .collect(),
        clauses: atoms.into_iter().map(TypedClause::Relation).collect(),
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
            .map(|(field_id, field, variable)| {
                text_field(
                    field,
                    field_id,
                    TypedTerm::Variable(variable),
                    ValueType::U64,
                )
            })
            .collect(),
    }
}

fn text_atom<const N: usize>(fields: [TypedFieldBinding; N]) -> TypedRelationAtom {
    TypedRelationAtom {
        relation_id: 3,
        relation: "Text".to_owned(),
        fields: fields.into_iter().collect(),
    }
}

fn text_field(
    field: &str,
    field_id: usize,
    term: TypedTerm,
    value_type: ValueType,
) -> TypedFieldBinding {
    TypedFieldBinding {
        field_id,
        field: field.to_owned(),
        value_type,
        term,
    }
}

fn row<const N: usize>(values: [u64; N]) -> Vec<Value> {
    values.into_iter().map(Value::U64).collect()
}
