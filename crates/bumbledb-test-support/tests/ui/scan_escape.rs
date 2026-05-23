use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::schemas::ledger_schema;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join("bumbledb-trybuild-fail");
    let _ = std::fs::remove_dir_all(&path);
    let env = Environment::open(&path)?;
    let _schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    let _txn = env.read(|txn| Ok::<_, bumbledb_lmdb::Error>(txn))?;
    Ok(())
}
