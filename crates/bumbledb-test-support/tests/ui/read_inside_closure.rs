use bumbledb_core::query_builder::QueryBuilder;
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
    let query = QueryBuilder::new(schema.descriptor())
        .rel("Holder")?
        .var("id", "id")?
        .var("name", "name")?
        .done()
        .find_var("id")?
        .find_var("name")?
        .finish()?;
    env.read(|txn| {
        let _output = txn.execute_query(&schema, &query, &bumbledb_lmdb::InputBindings::new())?;
        Ok::<(), bumbledb_lmdb::Error>(())
    })?;
    Ok(())
}
