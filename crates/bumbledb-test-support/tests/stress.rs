use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::assertions::assert_invariants;
use bumbledb_test_support::rows::generated_ledger_rows;
use bumbledb_test_support::schemas::ledger_schema;

#[test]
#[ignore]
fn stress_large_bulk_load_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let env = Environment::open(dir.path()).unwrap();
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();
    env.bulk_load(&schema, generated_ledger_rows(1_000))
        .unwrap();
    assert_invariants(&env, &schema).unwrap();
    drop(env);
    let env = Environment::open(dir.path()).unwrap();
    assert_invariants(&env, &schema).unwrap();
}
