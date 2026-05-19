use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::schemas::ledger_schema;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::open("/tmp/bumbledb-trybuild-pass")?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    env.read(|txn| {
        let _rows = txn.scan_relation(&schema, "Holder")?.collect::<bumbledb_lmdb::Result<Vec<_>>>()?;
        Ok::<(), bumbledb_lmdb::Error>(())
    })?;
    Ok(())
}
