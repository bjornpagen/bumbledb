//! Real-process kernel exclusion: a paused holder remains owner, death
//! releases, a competing open mutates nothing, and a mid-mutation kill leaves
//! old complete bytes — FS-01/02/04 and RUN-05 shapes (REP-005/009/010/017,
//! SDK-006). Each test owns an exclusive temporary tree and signals only its
//! own children. The child arms re-exec this test binary with a mode
//! environment variable. Verification: `NotRun` (F1 authors, does not execute).

#![cfg(unix)]

use std::io::{BufRead, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use bumbledb_log::store::fence::{acquire_directory, acquire_mutation};
use bumbledb_log::store::fs::{FsStore, Inject, Phase};
use bumbledb_log::store::{ConditionalOutcome, ConditionalStore as _, HeadRead};

const CHILD_ENV: &str = "BDB_P05_OWNERSHIP_CHILD";
const DIR_ENV: &str = "BDB_P05_OWNERSHIP_DIR";
const WAIT: Duration = Duration::from_secs(20);

fn fresh_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let root =
        std::env::temp_dir().join(format!("bdb-log-own-{}-{name}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create test root");
    root
}

fn spawn_child(mode: &str, dir: &Path) -> Child {
    Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "child_process_entry",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env(CHILD_ENV, mode)
        .env(DIR_ENV, dir)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child")
}

/// Wait for the child to print `marker` on stdout.
fn await_marker(child: &mut Child, marker: &str) {
    let stdout = child.stdout.as_mut().expect("child stdout");
    let mut reader = BufReader::new(stdout);
    let start = Instant::now();
    let mut line = String::new();
    loop {
        assert!(start.elapsed() < WAIT, "child never printed {marker}");
        line.clear();
        let read = reader.read_line(&mut line).expect("read child line");
        assert!(read > 0, "child stdout closed before {marker}");
        if line.trim() == marker {
            return;
        }
    }
}

fn signal(child: &Child, name: &str) {
    let status = Command::new("kill")
        .args([format!("-{name}"), child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -{name} failed");
}

/// The child arms. In an ordinary run (no env) this test is an immediate
/// no-op pass; when re-executed by a parent test it performs its mode and
/// never returns normally (the parent kills it).
#[test]
fn child_process_entry() {
    let Ok(mode) = std::env::var(CHILD_ENV) else {
        return;
    };
    let dir = PathBuf::from(std::env::var(DIR_ENV).expect("child dir"));
    match mode.as_str() {
        "hold-directory" => {
            let lock = acquire_directory(&dir.join("tenant")).expect("child owns");
            println!("LOCKED");
            let _ = std::io::stdout().flush();
            let _hold = lock;
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        }
        "hold-mutation" => {
            let lock = acquire_mutation(&dir, "t/HEAD").expect("child owns mutation");
            println!("LOCKED");
            let _ = std::io::stdout().flush();
            let _hold = lock;
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        }
        "die-mid-replace" => {
            // Publish an initial head, then pause forever at the staged
            // boundary of a replacement: the parent kills us there.
            let store = FsStore::new(&dir);
            let version = match store
                .create_head("t/HEAD", b"old-complete")
                .expect("create")
            {
                ConditionalOutcome::Published { version } => version,
                other => panic!("{other:?}"),
            };
            store.set_hook(|phase, _| {
                if phase == Phase::Staged {
                    println!("STAGED");
                    let _ = std::io::stdout().flush();
                    loop {
                        std::thread::sleep(Duration::from_secs(3600));
                    }
                }
                Inject::Continue
            });
            let _ = store.replace_head("t/HEAD", &version, b"new-torn?");
            unreachable!("the parent kills the staged child");
        }
        other => panic!("unknown child mode {other}"),
    }
}

#[test]
fn fs01_a_paused_holder_remains_owner_and_death_releases_the_directory() {
    let root = fresh_root("fs01");
    let mut child = spawn_child("hold-directory", &root);
    await_marker(&mut child, "LOCKED");
    // A competing open refuses immediately while the child owns.
    let refused = acquire_directory(&root.join("tenant"));
    assert!(refused.is_err(), "the live owner excludes");
    // SIGSTOP: a merely paused process retains its lock; time mints nothing.
    signal(&child, "STOP");
    std::thread::sleep(Duration::from_millis(200));
    let still = acquire_directory(&root.join("tenant"));
    assert!(still.is_err(), "a paused holder remains owner");
    // The refused opener changed nothing on disk.
    assert!(root.join("~lease/tenant/owner.lock").exists());
    // SIGCONT + SIGKILL: process death releases the kernel lock.
    signal(&child, "CONT");
    child.kill().expect("kill");
    let _ = child.wait().expect("reap");
    let start = Instant::now();
    loop {
        match acquire_directory(&root.join("tenant")) {
            Ok(_lock) => break,
            Err(_) if start.elapsed() < WAIT => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => panic!("death releases ownership: {error}"),
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn fs01b_a_held_mutation_lock_bounds_the_waiter_without_takeover() {
    let root = fresh_root("fs01b");
    let mut child = spawn_child("hold-mutation", &root);
    await_marker(&mut child, "LOCKED");
    // The bounded waiter exhausts its wait and refuses; it never steals.
    let store = FsStore::new(&root);
    let started = Instant::now();
    let refused = store.create_head("t/HEAD", b"contender");
    assert!(
        refused.is_err(),
        "the held mutation lock excludes the whole critical section"
    );
    assert!(
        started.elapsed() >= Duration::from_secs(4),
        "the waiter actually waited its bounded window"
    );
    // Nothing was mutated: no head exists.
    assert!(matches!(
        store.read_head("t/HEAD").expect("read"),
        HeadRead::Absent
    ));
    // Death releases; the same operation then succeeds.
    child.kill().expect("kill");
    let _ = child.wait().expect("reap");
    assert!(matches!(
        store.create_head("t/HEAD", b"contender").expect("create"),
        ConditionalOutcome::Published { .. }
    ));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn fs02_a_kill_mid_replacement_leaves_the_old_complete_head() {
    let root = fresh_root("fs02");
    let mut child = spawn_child("die-mid-replace", &root);
    await_marker(&mut child, "STAGED");
    child.kill().expect("kill staged child");
    let _ = child.wait().expect("reap");
    // Reopen: the old complete bytes hold; the staged temp is owned scratch
    // under ~tmp, never a readable head; a fresh mutation succeeds.
    let store = FsStore::new(&root);
    let version = match store.read_head("t/HEAD").expect("read") {
        HeadRead::Present { version, body } => {
            assert_eq!(&*body, b"old-complete", "never a torn head");
            version
        }
        HeadRead::Absent => panic!("the old head survives the kill"),
    };
    assert!(matches!(
        store
            .replace_head("t/HEAD", &version, b"successor")
            .expect("swap"),
        ConditionalOutcome::Published { .. }
    ));
    let _ = std::fs::remove_dir_all(&root);
}
