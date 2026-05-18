use bumbledb_core::datalog::parse_and_typecheck;
use bumbledb_core::encoding::TimestampMicros;
use bumbledb_lmdb::{Environment, InputBindings, StorageSchema, Value};
use bumbledb_test_support::assertions::{assert_invariants, assert_same_rows, execute_sorted};
use bumbledb_test_support::operations::{
    duplicate_holder_rows, valid_ledger_rows_strategy, wrong_type_holder_row,
};
use bumbledb_test_support::reference::ReferenceDb;
use bumbledb_test_support::rows::seeded_ledger_rows;
use bumbledb_test_support::schemas::ledger_schema;
use bumbledb_test_support::workloads::ledger_queries;
use proptest::prelude::*;

proptest! {
    #[test]
    fn valid_bulk_loads_match_reference(rows in valid_ledger_rows_strategy()) {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();
        let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();

        env.bulk_load(&schema, rows.clone()).unwrap();
        assert_invariants(&env, &schema).unwrap();

        let reference = ReferenceDb::from_rows(rows);
        for source in ledger_queries() {
            let query = parse_and_typecheck(schema.descriptor(), source).unwrap();
            let inputs = default_inputs();
            let lmdb_rows = execute_sorted(&env, &schema, &query, &inputs).unwrap();
            let reference_rows = reference.execute(&query, &inputs).unwrap();
            assert_same_rows(lmdb_rows, reference_rows);
        }
    }
}

#[test]
fn invalid_bulk_loads_fail_without_partial_state() {
    let dir = tempfile::tempdir().unwrap();
    let env = Environment::open(dir.path()).unwrap();
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();

    assert!(env.bulk_load(&schema, duplicate_holder_rows()).is_err());
    let diagnostics = env.storage_diagnostics(&schema).unwrap();
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
    let diagnostics = env.storage_diagnostics(&schema).unwrap();
    assert!(
        diagnostics
            .relations
            .iter()
            .all(|relation| relation.row_count == 0)
    );
}

#[test]
fn representative_queries_match_reference() {
    let rows = seeded_ledger_rows();
    let reference = ReferenceDb::from_rows(rows.clone());
    let dir = tempfile::tempdir().unwrap();
    let env = Environment::open(dir.path()).unwrap();
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();
    env.bulk_load(&schema, rows).unwrap();

    for source in ledger_queries() {
        let query = parse_and_typecheck(schema.descriptor(), source).unwrap();
        let inputs = default_inputs();
        assert_same_rows(
            execute_sorted(&env, &schema, &query, &inputs).unwrap(),
            reference.execute(&query, &inputs).unwrap(),
        );
    }
}

fn default_inputs() -> InputBindings {
    InputBindings::from_values([
        ("holder", Value::Ref(1)),
        ("start", Value::Timestamp(TimestampMicros(0))),
        ("end", Value::Timestamp(TimestampMicros(1_000_000))),
    ])
}
