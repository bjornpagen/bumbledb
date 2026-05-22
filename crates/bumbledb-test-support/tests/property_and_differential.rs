#![allow(clippy::result_large_err)]

use bumbledb_core::encoding::TimestampMicros;
use bumbledb_lmdb::{Environment, InputBindings, Row, StorageSchema, Value};
use bumbledb_test_support::assertions::{assert_invariants, assert_same_rows, execute_sorted};
use bumbledb_test_support::operations::{
    duplicate_holder_rows, valid_ledger_rows_strategy, wrong_type_holder_row,
};
use bumbledb_test_support::reference::ReferenceDb;
use bumbledb_test_support::rows::seeded_ledger_rows;
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
    fn valid_bulk_loads_match_reference(rows in valid_ledger_rows_strategy()) {
        let dir = prop(tempfile::tempdir())?;
        let env = prop(Environment::open(dir.path()))?;
        let schema = prop(StorageSchema::new(ledger_schema(), env.max_key_size()))?;

        prop(env.bulk_load(&schema, rows.clone()))?;
        prop(assert_invariants(&env, &schema))?;

        let reference = ReferenceDb::from_rows(rows);
        for query in prop(ledger_queries(schema.descriptor()))? {
            let inputs = default_inputs();
            let lmdb_rows = prop(execute_sorted(&env, &schema, &query, &inputs))?;
            let reference_rows = prop(reference.execute(&query, &inputs))?;
            assert_same_rows(lmdb_rows, reference_rows);
        }
    }

    #[test]
    fn insert_delete_sequences_match_holder_set(ops in holder_ops_strategy()) {
        let dir = prop(tempfile::tempdir())?;
        let env = prop(Environment::open(dir.path()))?;
        let schema = prop(StorageSchema::new(ledger_schema(), env.max_key_size()))?;
        let mut expected = BTreeSet::<Row>::new();

        for op in ops {
            match op {
                HolderOp::Insert(id) => {
                    let row = holder_row(id);
                    let _ = prop(env.write(|txn| txn.insert(&schema, row.clone())))?;
                    expected.insert(row);
                }
                HolderOp::Delete(id) => {
                    let row = holder_row(id);
                    let _ = prop(env.write(|txn| txn.delete(&schema, row.clone())))?;
                    expected.remove(&row);
                }
            }
            prop(assert_invariants(&env, &schema))?;
            let actual = prop(env.read(|txn| {
                txn.scan_relation(&schema, "Holder")?
                    .map(|item| item.map(|item| item.row))
                    .collect::<bumbledb_lmdb::Result<BTreeSet<_>>>()
            }))?;
            assert_eq!(actual, expected);
            let count = prop(env.read(|txn| txn.relation_row_count(&schema, "Holder")))?;
            assert_eq!(count as usize, expected.len());
        }
    }
}

#[test]
fn invalid_bulk_loads_fail_without_partial_state() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;

    assert!(env.bulk_load(&schema, duplicate_holder_rows()).is_err());
    let diagnostics = env.storage_diagnostics(&schema)?;
    assert!(
        diagnostics
            .relations
            .iter()
            .all(|relation| relation.row_count == 0)
    );
    assert_eq!(diagnostics.dictionary_entries, 0);

    assert!(
        env.bulk_load(&schema, vec![wrong_type_holder_row()])
            .is_err()
    );
    let diagnostics = env.storage_diagnostics(&schema)?;
    assert!(
        diagnostics
            .relations
            .iter()
            .all(|relation| relation.row_count == 0)
    );
    Ok(())
}

#[test]
fn representative_queries_match_reference() -> Result<(), Box<dyn std::error::Error>> {
    let rows = seeded_ledger_rows();
    let reference = ReferenceDb::from_rows(rows.clone());
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    env.bulk_load(&schema, rows)?;

    for query in ledger_queries(schema.descriptor())? {
        let inputs = default_inputs();
        assert_same_rows(
            execute_sorted(&env, &schema, &query, &inputs)?,
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

fn holder_row(id: u64) -> Row {
    bumbledb_test_support::rows::holder(id, format!("holder-{id}"))
}
