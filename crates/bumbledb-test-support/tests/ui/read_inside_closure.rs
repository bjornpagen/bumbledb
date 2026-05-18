use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::schemas::ledger_schema;

fn main() {
    let env = Environment::open("/tmp/bumbledb-trybuild-pass").unwrap();
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();
    env.read(|txn| {
        let _rows = txn.scan_relation(&schema, "Holder")?.collect::<bumbledb_lmdb::Result<Vec<_>>>()?;
        Ok::<(), bumbledb_lmdb::Error>(())
    })
    .unwrap();
}
