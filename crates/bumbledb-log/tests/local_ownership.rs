//! Real-process kernel exclusion: a paused holder remains owner, death
//! releases, a competing open mutates nothing, and a mid-mutation kill leaves
//! old complete bytes — FS-01/02/04 and RUN-05 shapes (REP-005/009/010/017,
//! SDK-006). Each test owns an exclusive temporary tree and signals only its
//! own children. The child arms re-exec this test binary with a mode
//! environment variable.

#![cfg(unix)]

#[path = "lane_support/process.rs"]
mod process;

use process::TestChild;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use bumbledb_log::store::fence::{acquire_directory, acquire_mutation};
use bumbledb_log::store::fs::{FsStore, Inject, Phase};
use bumbledb_log::store::{
    ConditionalOutcome, ConditionalStore as _, ReceivedHead, ReceivingStore, TransportContext,
};

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

fn spawn_child(mode: &str, dir: &Path) -> TestChild {
    TestChild::spawn(
        Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "child_process_entry",
                "--ignored",
                "--nocapture",
                "--test-threads",
                "1",
            ])
            .env(CHILD_ENV, mode)
            .env(DIR_ENV, dir),
        WAIT,
    )
}

/// Driver for the parent tests, not an independent passing test.
#[test]
#[ignore = "subprocess entrypoint invoked by the parent crash tests"]
fn child_process_entry() {
    let mode = std::env::var(CHILD_ENV).expect("parent selects the child mode");
    let dir = PathBuf::from(std::env::var(DIR_ENV).expect("child dir"));
    match mode.as_str() {
        "buffered-markers" => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(b"FIRST\nSECOND\n").unwrap();
            stdout.flush().unwrap();
        }
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
fn child_protocol_preserves_read_ahead_and_reports_exit() {
    let root = fresh_root("buffered-markers");
    let mut child = spawn_child("buffered-markers", &root);
    // Exit first so both lines are already available to the buffered reader.
    assert!(child.process.wait().unwrap().success());
    child.await_marker("FIRST");
    child.await_marker("SECOND");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_panicking_parent_reaps_its_child_and_releases_the_lock() {
    let root = fresh_root("parent-panic");
    let mut child = spawn_child("hold-directory", &root);
    child.await_marker("LOCKED");
    assert!(acquire_directory(&root.join("tenant")).is_err());
    let failure = std::panic::catch_unwind(move || {
        let _child = child;
        panic!("simulate a failed parent assertion");
    });
    assert!(failure.is_err());
    let lock = acquire_directory(&root.join("tenant")).expect("unwind reaped the lock holder");
    drop(lock);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fs01_a_paused_holder_remains_owner_and_death_releases_the_directory() {
    let root = fresh_root("fs01");
    let mut child = spawn_child("hold-directory", &root);
    child.await_marker("LOCKED");
    // A competing open refuses immediately while the child owns.
    let refused = acquire_directory(&root.join("tenant"));
    assert!(refused.is_err(), "the live owner excludes");
    // SIGSTOP: a merely paused process retains its lock; time mints nothing.
    child.signal("STOP");
    std::thread::sleep(Duration::from_millis(200));
    let still = acquire_directory(&root.join("tenant"));
    assert!(still.is_err(), "a paused holder remains owner");
    // The refused opener changed nothing on disk.
    assert!(root.join("~lease/tenant/owner.lock").exists());
    // SIGCONT + SIGKILL: process death releases the kernel lock.
    child.signal("CONT");
    child.process.kill().expect("kill");
    let _ = child.process.wait().expect("reap");
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
    child.await_marker("LOCKED");
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
        store
            .receive_head("t/HEAD", TransportContext::limited(64))
            .expect("read"),
        ReceivedHead::Absent
    ));
    // Death releases; the same operation then succeeds.
    child.process.kill().expect("kill");
    let _ = child.process.wait().expect("reap");
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
    child.await_marker("STAGED");
    child.process.kill().expect("kill staged child");
    let _ = child.process.wait().expect("reap");
    // Reopen: the old complete bytes hold; the staged temp is owned scratch
    // under ~tmp, never a readable head; a fresh mutation succeeds.
    let store = FsStore::new(&root);
    let version = match store
        .receive_head("t/HEAD", TransportContext::limited(64))
        .expect("read")
    {
        ReceivedHead::Present { version, body } => {
            assert_eq!(body.as_slice(), b"old-complete", "never a torn head");
            version
        }
        ReceivedHead::Absent => panic!("the old head survives the kill"),
    };
    assert!(matches!(
        store
            .replace_head("t/HEAD", &version, b"successor")
            .expect("swap"),
        ConditionalOutcome::Published { .. }
    ));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn d28_successor_reuses_the_persistent_lock_inode_without_deleting_it() {
    use bumbledb_log::store::fence::acquire_repository_lock;
    use std::os::unix::fs::MetadataExt;
    let root = fresh_root("d28");
    let tenant = root.join("tenant");
    let mut child = spawn_child("hold-directory", &root);
    child.await_marker("LOCKED");
    let lock_path = root.join("~lease/tenant/owner.lock");
    assert!(lock_path.exists(), "owner created the persistent inode");
    let meta = std::fs::metadata(&lock_path).expect("inode");
    let inode = (meta.dev(), meta.ino());
    child.signal("STOP");
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        acquire_repository_lock(&tenant).is_err(),
        "paused owner remains exclusive"
    );
    assert!(lock_path.exists(), "pause does not replace the inode");
    child.signal("CONT");
    child.process.kill().expect("kill");
    let _ = child.process.wait().expect("reap");
    let start = Instant::now();
    let successor = loop {
        match acquire_repository_lock(&tenant) {
            Ok(lock) => break lock,
            Err(_) if start.elapsed() < WAIT => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => panic!("death releases ownership: {error}"),
        }
    };
    assert_eq!(successor.lock_path(), lock_path.canonicalize().unwrap());
    let after = successor.lock_inode().expect("successor inode");
    assert_eq!(after.dev, inode.0);
    assert_eq!(after.ino, inode.1);
    assert!(
        lock_path.exists(),
        "successor acquisition never unlinks the lock inode"
    );
    drop(successor);
    let _ = std::fs::remove_dir_all(&root);
}
