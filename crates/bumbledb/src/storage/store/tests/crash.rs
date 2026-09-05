//! G06 / E-DURABILITY process-death schedules: the durable commit boundary
//! is exact. A child process killed after `commit` leaves every acknowledged
//! row; killed with a prepared-but-uncommitted candidate it leaves none; and
//! its death releases the kernel directory lock for the next owner.
//!
//! These are process-exit schedules. True power-loss/machine-failure
//! qualification is a separate authorized hardware/filesystem gate and is
//! not claimed here.

use super::*;

const ROLE_ENV: &str = "BUMBLEDB_STORE_CRASH_ROLE";
const PATH_ENV: &str = "BUMBLEDB_STORE_CRASH_PATH";

/// Child-role dispatcher. As a test it is a no-op unless the parent set the
/// role environment; the parent invokes this exact test in a subprocess.
#[test]
fn child_role() {
    let Ok(role) = std::env::var(ROLE_ENV) else {
        return;
    };
    let path = std::path::PathBuf::from(std::env::var(PATH_ENV).expect("crash child path"));
    let store = if path.exists() {
        open_default(&path)
    } else {
        create_default(&path)
    };
    match role.as_str() {
        "commit-then-abort" => {
            commit_changes(
                &store,
                &change_set(&schema(), &[(NOTE, note(1, "durable"))], &[]),
            );
            // Acknowledged durable commit, then abrupt death: no close, no
            // lock release, no drop glue.
            println!("CRASH_CHILD_COMMITTED");
            std::process::abort();
        }
        "prepare-then-abort" => {
            let context = work();
            let mut owner = store.writer(&context).expect("writer");
            let prepared = match owner
                .prepare(
                    &change_set(&schema(), &[(NOTE, note(2, "speculative"))], &[]),
                    &FirstFieldKey,
                    &AdmitAll,
                )
                .expect("prepare")
            {
                Prepared::Admitted(prepared) => prepared,
                Prepared::Rejected(never) => match never {},
            };
            // Sealed but never committed: death must erase it.
            let sealed = prepared.seal(NO_HOST).expect("seal");
            println!("CRASH_CHILD_SEALED");
            std::mem::forget(sealed);
            std::process::abort();
        }
        "hold-lock" => {
            println!("CRASH_CHILD_HOLDING");
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        other => panic!("unknown crash role {other}"),
    }
}

fn spawn_child(role: &str, path: &std::path::Path) -> std::process::Child {
    let exe = std::env::current_exe().expect("test binary path");
    std::process::Command::new(exe)
        .arg("--exact")
        .arg("storage::store::tests::crash::child_role")
        .arg("--nocapture")
        .env(ROLE_ENV, role)
        .env(PATH_ENV, path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn crash child")
}

fn wait_for_marker(child: &mut std::process::Child, marker: &str) {
    use std::io::BufRead as _;
    let stdout = child.stdout.take().expect("child stdout");
    let reader = std::io::BufReader::new(stdout);
    for line in reader.lines() {
        let line = line.expect("child line");
        if line.contains(marker) {
            return;
        }
    }
    panic!("child exited without printing {marker}");
}

#[test]
fn rows_survive_process_death_after_durable_commit() {
    let (_dir, path) = store_dir("crash-after-commit");
    let mut child = spawn_child("commit-then-abort", &path);
    wait_for_marker(&mut child, "CRASH_CHILD_COMMITTED");
    let status = child.wait().expect("child exit");
    assert!(!status.success(), "the child aborted deliberately");
    // The next owner opens (death released the kernel lock) and finds the
    // acknowledged row: LMDB-default durability, no NO_SYNC weakening.
    let store = open_default(&path);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert!(
        snapshot
            .contains(
                NOTE,
                crate::canonical::CanonicalRow::encode(
                    schema().relation(NOTE).fields(),
                    &note(1, "durable"),
                    &work()
                )
                .expect("row")
                .as_bytes(),
                &work()
            )
            .expect("durable row present")
    );
    assert_eq!(
        store
            .committed_generation(&work())
            .expect("generation")
            .value(),
        1
    );
}

#[test]
fn a_sealed_uncommitted_candidate_dies_with_its_process() {
    let (_dir, path) = store_dir("crash-before-commit");
    // Seed one committed row so the child opens an existing store.
    {
        let store = create_default(&path);
        commit_changes(
            &store,
            &change_set(&schema(), &[(NOTE, note(1, "durable"))], &[]),
        );
    }
    let mut child = spawn_child("prepare-then-abort", &path);
    wait_for_marker(&mut child, "CRASH_CHILD_SEALED");
    let status = child.wait().expect("child exit");
    assert!(!status.success());
    let store = open_default(&path);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    // Only the seeded row: the sealed-but-uncommitted candidate left no
    // trace, and no half-written index entry survives either.
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    let rows: Vec<_> = snapshot
        .rows(NOTE)
        .expect("cursor")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(rows.len(), 1);
}

#[test]
fn a_paused_owner_keeps_the_lock_and_death_releases_it() {
    let (_dir, path) = store_dir("crash-lock-lifetime");
    {
        drop(create_default(&path));
    }
    let mut child = spawn_child("hold-lock", &path);
    wait_for_marker(&mut child, "CRASH_CHILD_HOLDING");
    // While the child lives, ownership refuses — time never mints an owner.
    match Store::open(&path, &schema(), MapPolicy::default()) {
        Err(StoreError::StoreLocked { .. }) => {}
        other => panic!("expected StoreLocked under a live child, got {other:?}"),
    }
    child.kill().expect("kill child");
    let _ = child.wait();
    // Death releases the kernel lock; the next owner proceeds.
    drop(open_default(&path));
}
