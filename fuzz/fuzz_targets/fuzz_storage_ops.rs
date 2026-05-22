#![no_main]

use std::collections::BTreeSet;

use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb_lmdb::{Environment, Row, StorageSchema, Value};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some(dir) = tempfile::tempdir().ok() else {
        return;
    };
    let Some(env) = Environment::open(dir.path()).ok() else {
        return;
    };
    let schema = SchemaDescriptor::new(
        "FuzzStorageOps",
        vec![RelationDescriptor::new(
            "Item",
            vec![FieldDescriptor::new("id", ValueType::U64)],
        )],
    );
    let Some(schema) = StorageSchema::new(schema, env.max_key_size()).ok() else {
        return;
    };
    let mut expected = BTreeSet::new();
    for byte in data.iter().copied().take(128) {
        let id = u64::from(byte & 0x0f);
        let row = Row::new("Item", [("id", Value::U64(id))]);
        let result = if byte & 0x80 == 0 {
            expected.insert(row.clone());
            env.write(|txn| txn.insert(&schema, row).map(|_| ()))
        } else {
            expected.remove(&row);
            env.write(|txn| txn.delete(&schema, row).map(|_| ()))
        };
        if result.is_err() {
            return;
        }
        let Ok(count) = env.read(|txn| txn.relation_row_count(&schema, "Item")) else {
            return;
        };
        if count as usize != expected.len() {
            std::process::abort();
        }
    }
});
