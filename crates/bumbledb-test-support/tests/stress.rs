#![allow(clippy::result_large_err)]

use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::assertions::assert_invariants;
use bumbledb_test_support::facts::generated_ledger_rows;
use bumbledb_test_support::schemas::ledger_schema;

#[test]
#[ignore]
fn stress_large_bulk_load_and_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    env.bulk_load(&schema, generated_ledger_rows(1_000))?;
    assert_invariants(&env, &schema)?;
    drop(env);
    let env = Environment::open(dir.path())?;
    assert_invariants(&env, &schema)?;
    Ok(())
}
