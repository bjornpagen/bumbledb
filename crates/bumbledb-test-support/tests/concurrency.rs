use std::sync::{Arc, Barrier};
use std::thread;

use bumbledb_core::datalog::parse_and_typecheck;
use bumbledb_lmdb::{Environment, InputBindings, StorageSchema};
use bumbledb_test_support::assertions::assert_invariants;
use bumbledb_test_support::rows::{account, holder, seeded_ledger_rows};
use bumbledb_test_support::schemas::ledger_schema;

#[test]
fn readers_see_stable_snapshots_while_writer_commits() {
    let dir = tempfile::tempdir().unwrap();
    let env = Arc::new(Environment::open(dir.path()).unwrap());
    let schema = Arc::new(StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap());
    env.bulk_load(&schema, seeded_ledger_rows()).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let query = Arc::new(
        parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account)",
        )
        .unwrap(),
    );

    let reader_env = env.clone();
    let reader_schema = schema.clone();
    let reader_query = query.clone();
    let reader_barrier = barrier.clone();
    let reader = thread::spawn(move || {
        reader_env
            .read(|txn| {
                let before = txn
                    .execute_query(&reader_schema, &reader_query, &InputBindings::new())?
                    .rows
                    .len();
                reader_barrier.wait();
                reader_barrier.wait();
                let after = txn
                    .execute_query(&reader_schema, &reader_query, &InputBindings::new())?
                    .rows
                    .len();
                assert_eq!(before, after);
                Ok::<(), bumbledb_lmdb::Error>(())
            })
            .unwrap();
    });

    barrier.wait();
    env.write(|txn| {
        txn.insert(&schema, holder(99, "late-holder"))?;
        txn.insert(&schema, account(99, 99, 840))?;
        Ok::<(), bumbledb_lmdb::Error>(())
    })
    .unwrap();
    barrier.wait();
    reader.join().unwrap();

    let latest = env
        .read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))
        .unwrap();
    assert_eq!(latest.rows.len(), 4);
    assert_invariants(&env, &schema).unwrap();
}
