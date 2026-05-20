use std::sync::{Mutex, OnceLock};

use bumbledb_lmdb::failpoints::{self, Failpoint};
use bumbledb_lmdb::{Environment, Error, StorageSchema, TestError};
use bumbledb_test_support::assertions::assert_invariants;
use bumbledb_test_support::rows::{account, holder, posting, seeded_ledger_rows};
use bumbledb_test_support::schemas::ledger_schema;

#[test]
fn failpoints_abort_insert_replace_delete_and_bulk_load() -> Result<(), Box<dyn std::error::Error>>
{
    let _guard = lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for failpoint in [
        Failpoint::BeforeDictionaryPut,
        Failpoint::AfterDictionaryPut,
        Failpoint::AfterCurrentIndexPut,
        Failpoint::AfterUniqueGuardPut,
        Failpoint::AfterStatsUpdate,
        Failpoint::AfterHistoryAppend,
        Failpoint::BeforeCommit,
    ] {
        failpoint_insert_is_atomic(failpoint)?;
    }
    failpoint_replace_delete_and_bulk_are_atomic()?;
    Ok(())
}

fn failpoint_insert_is_atomic(failpoint: Failpoint) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    failpoints::set(failpoint);
    let result = env.write(|txn| txn.insert(&schema, holder(1, "x")));
    failpoints::clear();
    assert!(matches!(
        result,
        Err(Error::Test(TestError::InjectedFailpoint { .. }))
    ));
    let diagnostics = env.storage_diagnostics(&schema)?;
    assert!(
        diagnostics
            .relations
            .iter()
            .all(|relation| relation.row_count == 0)
    );
    assert_eq!(diagnostics.dictionary_entries, 0);
    Ok(())
}

fn failpoint_replace_delete_and_bulk_are_atomic() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    env.bulk_load(&schema, seeded_ledger_rows())?;
    assert_invariants(&env, &schema)?;

    failpoints::set(Failpoint::AfterCurrentIndexPut);
    assert!(matches!(
        env.write(|txn| txn.replace(&schema, account(1, 1, 999))),
        Err(Error::Test(TestError::InjectedFailpoint { .. }))
    ));
    failpoints::clear();
    assert_invariants(&env, &schema)?;

    failpoints::set(Failpoint::BeforeCommit);
    assert!(matches!(
        env.write(|txn| txn.delete(&schema, bumbledb_test_support::rows::account(3, 2, 840))),
        Err(Error::Test(TestError::InjectedFailpoint { .. }))
    ));
    failpoints::clear();
    assert_invariants(&env, &schema)?;

    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    failpoints::set(Failpoint::AfterHistoryAppend);
    assert!(matches!(
        env.bulk_load(
            &schema,
            vec![holder(1, "x"), account(1, 1, 840), posting(1, 1, 10, 1)]
        ),
        Err(Error::Test(TestError::InjectedFailpoint { .. }))
    ));
    failpoints::clear();
    let diagnostics = env.storage_diagnostics(&schema)?;
    assert!(
        diagnostics
            .relations
            .iter()
            .all(|relation| relation.row_count == 0)
    );
    Ok(())
}

fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
