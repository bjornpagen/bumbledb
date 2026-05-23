use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::schemas::ledger_schema;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!(
        "bumbledb-trybuild-pass-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&path);
    let env = Environment::open(&path)?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    env.read(|txn| {
        let _facts = txn.scan_relation(&schema, "Holder")?.collect::<bumbledb_lmdb::Result<Vec<_>>>()?;
        Ok::<(), bumbledb_lmdb::Error>(())
    })?;
    Ok(())
}
