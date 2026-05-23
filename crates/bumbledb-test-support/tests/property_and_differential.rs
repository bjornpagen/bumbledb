#![allow(clippy::result_large_err)]

use bumbledb_core::encoding::TimestampMicros;
use bumbledb_core::query_builder::QueryBuilder;
use bumbledb_lmdb::{Environment, Fact, InputBindings, StorageSchema, Value};
use bumbledb_test_support::assertions::{
    assert_invariants, assert_same_facts, execute_sorted_facts,
};
use bumbledb_test_support::facts::seeded_ledger_facts;
use bumbledb_test_support::operations::{
    duplicate_holder_facts, valid_ledger_facts_strategy, wrong_type_holder_fact,
};
use bumbledb_test_support::reference::ReferenceDb;
use bumbledb_test_support::schemas::ledger_schema;
use bumbledb_test_support::workloads::ledger_queries;
use proptest::prelude::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
enum HolderOp {
    Insert(u64),
    Delete(u64),
}

fn holder_ops_strategy() -> impl Strategy<Value = Vec<HolderOp>> {
    prop::collection::vec(
        prop_oneof![
            (1u64..8).prop_map(HolderOp::Insert),
            (1u64..8).prop_map(HolderOp::Delete),
        ],
        1..64,
    )
}

proptest! {
    #[test]
    fn valid_bulk_loads_match_reference(facts in valid_ledger_facts_strategy()) {
        let dir = prop(tempfile::tempdir())?;
        let env = prop(Environment::open(dir.path()))?;
        let schema = prop(StorageSchema::new(ledger_schema(), env.max_key_size()))?;

        prop(env.bulk_load(&schema, facts.clone()))?;
        prop(assert_invariants(&env, &schema))?;

        let reference = ReferenceDb::from_facts(facts);
        for query in prop(ledger_queries(schema.descriptor()))? {
            let inputs = default_inputs();
            let lmdb_facts = prop(execute_sorted_facts(&env, &schema, &query, &inputs))?;
            let reference_facts = prop(reference.execute(&query, &inputs))?;
            assert_same_facts(lmdb_facts, reference_facts);
        }
    }

    #[test]
    fn insert_delete_sequences_match_holder_set(ops in holder_ops_strategy()) {
        let dir = prop(tempfile::tempdir())?;
        let env = prop(Environment::open(dir.path()))?;
        let schema = prop(StorageSchema::new(ledger_schema(), env.max_key_size()))?;
        let mut expected = BTreeSet::<Fact>::new();

        for op in ops {
            match op {
                HolderOp::Insert(id) => {
                    let fact = holder_fact(id);
                    let _ = prop(env.write(|txn| txn.insert(&schema, fact.clone())))?;
                    expected.insert(fact);
                }
                HolderOp::Delete(id) => {
                    let fact = holder_fact(id);
                    let _ = prop(env.write(|txn| txn.delete(&schema, fact.clone())))?;
                    expected.remove(&fact);
                }
            }
            prop(assert_invariants(&env, &schema))?;
            let holder_query = prop(holder_projection_query(schema.descriptor()))?;
            let actual = prop(execute_sorted_facts(
                &env,
                &schema,
                &holder_query,
                &InputBindings::new(),
            ))?
            .into_iter()
            .map(|fact| holder_fact_from_projection(&fact))
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(TestCaseError::fail)?;
            assert_eq!(actual, expected);
            let count = prop(env.read(|txn| txn.relation_fact_count(&schema, "Holder")))?;
            assert_eq!(count as usize, expected.len());
        }
    }
}

#[test]
fn invalid_bulk_loads_fail_without_partial_state() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;

    assert!(env.bulk_load(&schema, duplicate_holder_facts()).is_err());
    let diagnostics = env.storage_diagnostics(&schema)?;
    assert!(
        diagnostics
            .relations
            .iter()
            .all(|relation| relation.fact_count == 0)
    );
    assert_eq!(diagnostics.dictionary_entries, 0);

    assert!(
        env.bulk_load(&schema, vec![wrong_type_holder_fact()])
            .is_err()
    );
    let diagnostics = env.storage_diagnostics(&schema)?;
    assert!(
        diagnostics
            .relations
            .iter()
            .all(|relation| relation.fact_count == 0)
    );
    Ok(())
}

#[test]
fn representative_queries_match_reference() -> Result<(), Box<dyn std::error::Error>> {
    let facts = seeded_ledger_facts();
    let reference = ReferenceDb::from_facts(facts.clone());
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    env.bulk_load(&schema, facts)?;

    for query in ledger_queries(schema.descriptor())? {
        let inputs = default_inputs();
        assert_same_facts(
            execute_sorted_facts(&env, &schema, &query, &inputs)?,
            reference.execute(&query, &inputs)?,
        );
    }
    Ok(())
}

fn prop<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> Result<T, TestCaseError> {
    result.map_err(|error| TestCaseError::fail(error.to_string()))
}

fn default_inputs() -> InputBindings {
    InputBindings::from_values([
        ("holder", Value::Serial(1)),
        ("start", Value::Timestamp(TimestampMicros(0))),
        ("end", Value::Timestamp(TimestampMicros(1_000_000))),
    ])
}

fn holder_fact(id: u64) -> Fact {
    bumbledb_test_support::facts::holder(id, format!("holder-{id}"))
}

fn holder_projection_query(
    schema: &bumbledb_core::schema::SchemaDescriptor,
) -> Result<bumbledb_core::query_ir::TypedQuery, bumbledb_core::query_builder::QueryBuildError> {
    QueryBuilder::new(schema)
        .rel("Holder")?
        .var("id", "id")?
        .var("name", "name")?
        .done()
        .find_var("id")?
        .find_var("name")?
        .finish()
}

fn holder_fact_from_projection(fact: &[Value]) -> Result<Fact, String> {
    let [Value::Serial(id), Value::String(name)] = fact else {
        return Err(format!("unexpected Holder projection: {fact:?}"));
    };
    Ok(bumbledb_test_support::facts::holder(*id, name.clone()))
}
