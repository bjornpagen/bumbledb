use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::schemas::ledger_schema;

fn main() {
    let env = Environment::open("/tmp/bumbledb-trybuild-fail").unwrap();
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();
    let _scan = env.read(|txn| txn.scan_relation(&schema, "Holder")).unwrap();
}
