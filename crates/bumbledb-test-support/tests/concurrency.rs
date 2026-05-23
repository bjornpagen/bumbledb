#![allow(clippy::result_large_err)]

use std::sync::{Arc, Barrier};
use std::thread;

use bumbledb_core::query_builder::QueryBuilder;
use bumbledb_lmdb::{Environment, InputBindings, StorageSchema};
use bumbledb_test_support::assertions::assert_invariants;
use bumbledb_test_support::facts::{account, holder, seeded_ledger_facts};
use bumbledb_test_support::schemas::ledger_schema;

#[test]
fn readers_see_stable_snapshots_while_writer_commits() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let env = Arc::new(Environment::open(dir.path())?);
    let schema = Arc::new(StorageSchema::new(ledger_schema(), env.max_key_size())?);
    env.bulk_load(&schema, seeded_ledger_facts())?;
    let barrier = Arc::new(Barrier::new(2));
    let query = Arc::new(
        QueryBuilder::new(schema.descriptor())
            .rel("Account")?
            .var("id", "account")?
            .done()
            .find_var("account")?
            .finish()?,
    );

    let reader_env = env.clone();
    let reader_schema = schema.clone();
    let reader_query = query.clone();
    let reader_barrier = barrier.clone();
    let reader = thread::spawn(move || {
        reader_env.read(|txn| {
            let before = txn
                .execute_query(&reader_schema, &reader_query, &InputBindings::new())?
                .result
                .facts
                .len();
            reader_barrier.wait();
            reader_barrier.wait();
            let after = txn
                .execute_query(&reader_schema, &reader_query, &InputBindings::new())?
                .result
                .facts
                .len();
            assert_eq!(before, after);
            Ok::<(), bumbledb_lmdb::Error>(())
        })
    });

    barrier.wait();
    env.write(|txn| {
        txn.insert(&schema, holder(99, "late-holder"))?;
        txn.insert(&schema, account(99, 99, 1))?;
        Ok::<(), bumbledb_lmdb::Error>(())
    })?;
    barrier.wait();
    reader
        .join()
        .map_err(|_| std::io::Error::other("reader thread panicked"))??;

    let latest = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
    assert_eq!(latest.result.facts.len(), 4);
    assert_invariants(&env, &schema)?;
    Ok(())
}
