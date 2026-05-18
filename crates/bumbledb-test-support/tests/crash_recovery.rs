use std::process::Command;

use bumbledb_lmdb::{Environment, StorageSchema};
use bumbledb_test_support::assertions::assert_invariants;
use bumbledb_test_support::rows::holder;
use bumbledb_test_support::schemas::ledger_schema;

#[test]
#[ignore]
fn subprocess_crash_before_commit_leaves_no_rows() {
    crash_parent("subprocess_crash_before_commit_leaves_no_rows", "precommit");
}

#[test]
#[ignore]
fn subprocess_crash_after_commit_leaves_committed_rows() {
    crash_parent(
        "subprocess_crash_after_commit_leaves_committed_rows",
        "postcommit",
    );
}

fn crash_parent(test_name: &str, mode: &str) {
    if std::env::var("BUMBLEDB_CRASH_CHILD").ok().as_deref() == Some(mode) {
        crash_child(mode);
    }

    let dir = tempfile::tempdir().unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--ignored")
        .arg("--exact")
        .arg(test_name)
        .env("BUMBLEDB_CRASH_CHILD", mode)
        .env("BUMBLEDB_CRASH_PATH", dir.path())
        .status()
        .unwrap();
    assert!(!status.success());

    let env = Environment::open(dir.path()).unwrap();
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();
    assert_invariants(&env, &schema).unwrap();
    let diagnostics = env.storage_diagnostics(&schema).unwrap();
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
}

fn crash_child(mode: &str) -> ! {
    let path = std::env::var("BUMBLEDB_CRASH_PATH").unwrap();
    let env = Environment::open(path).unwrap();
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();
    if mode == "precommit" {
        env.write(|txn| -> bumbledb_lmdb::Result<()> {
            txn.insert(&schema, holder(1, "crash"))?;
            std::process::abort();
        })
        .unwrap();
    } else {
        env.write(|txn| {
            txn.insert(&schema, holder(1, "crash"))?;
            Ok::<(), bumbledb_lmdb::Error>(())
        })
        .unwrap();
        std::process::abort();
    }
    unreachable!()
}
