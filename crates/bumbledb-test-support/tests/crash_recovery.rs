#![allow(clippy::result_large_err)]

use std::process::Command;

use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::assertions::assert_invariants;
use bumbledb_test_support::rows::holder;
use bumbledb_test_support::schemas::ledger_schema;

#[test]
#[ignore]
fn subprocess_crash_before_commit_leaves_no_rows() {
    assert!(crash_parent("subprocess_crash_before_commit_leaves_no_rows", "precommit").is_ok());
}

#[test]
#[ignore]
fn subprocess_crash_after_commit_leaves_committed_rows() {
    assert!(
        crash_parent(
            "subprocess_crash_after_commit_leaves_committed_rows",
            "postcommit",
        )
        .is_ok()
    );
}

fn crash_parent(test_name: &str, mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("BUMBLEDB_CRASH_CHILD").ok().as_deref() == Some(mode) {
        crash_child(mode);
    }

    let dir = tempfile::tempdir()?;
    let status = Command::new(std::env::current_exe()?)
        .arg("--ignored")
        .arg("--exact")
        .arg(test_name)
        .env("BUMBLEDB_CRASH_CHILD", mode)
        .env("BUMBLEDB_CRASH_PATH", dir.path())
        .status()?;
    assert!(!status.success());

    let env = Environment::open(dir.path())?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    assert_invariants(&env, &schema)?;
    let diagnostics = env.storage_diagnostics(&schema)?;
    if mode == "precommit" {
        assert!(
            diagnostics
                .relations
                .iter()
                .all(|relation| relation.row_count == 0)
        );
    } else {
        assert!(
            diagnostics
                .relations
                .iter()
                .any(|relation| relation.relation == "Holder" && relation.row_count == 1)
        );
    }
    Ok(())
}

fn crash_child(mode: &str) -> ! {
    let Ok(path) = std::env::var("BUMBLEDB_CRASH_PATH") else {
        std::process::abort();
    };
    let Ok(env) = Environment::open(path) else {
        std::process::abort();
    };
    let Ok(schema) = StorageSchema::new(ledger_schema(), env.max_key_size()) else {
        std::process::abort();
    };
    if mode == "precommit" {
        let _ = env.write(|txn| -> bumbledb_lmdb::Result<()> {
            txn.insert(&schema, holder(1, "crash"))?;
            std::process::abort();
        });
        std::process::abort();
    } else {
        if env
            .write(|txn| {
                txn.insert(&schema, holder(1, "crash"))?;
                Ok::<(), bumbledb_lmdb::Error>(())
            })
            .is_err()
        {
            std::process::abort();
        }
        std::process::abort();
    }
}
