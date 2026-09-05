//! Real process ownership tests. Every test owns an exclusive temporary
//! tree and only signals its own child processes; no global test serialism.
#![cfg(unix)]

mod lane_d_support;

use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bumbledb_log::replica::{Fault, Opened, Replica, record_ckpt_scratch, sweep_at_open};
use bumbledb_log::store::fence::{acquire_directory, acquire_mutation, sync_parent, synced_temp};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{Create, ObjectStore, StoreKey, Swap};
use bumbledb_log::writer::{Options, Writer};
use lane_d_support::{TestLog, theory};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);
const WAIT: Duration = Duration::from_secs(20);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "bdb-kernel-owner-{}-{stamp}-{seq}",
            std::process::id()
        ));
        // Exclusive creation, never remove a preexisting path on collision.
        fs::create_dir(&path).expect("exclusive test directory");
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Worker {
    child: Child,
    output: Receiver<String>,
    reader: Option<JoinHandle<()>>,
}

impl Worker {
    fn spawn(mode: &str, path: &Path) -> Self {
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "ownership_child", "--nocapture"])
            .env("BDB_OWNERSHIP_CHILD", mode)
            .env("BDB_OWNERSHIP_PATH", path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn owned child");
        let stdout = child.stdout.take().unwrap();
        let (send, output) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in io::BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    break;
                };
                if let Some(event) = line.strip_prefix("BDB_OWNER:")
                    && send.send(event.to_string()).is_err()
                {
                    break;
                }
            }
        });
        Self {
            child,
            output,
            reader: Some(reader),
        }
    }

    fn event(&self) -> String {
        self.output.recv_timeout(WAIT).expect("bounded child event")
    }

    fn send(&mut self) {
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"continue\n")
            .expect("release child");
    }

    fn signal(&mut self, signal: &str) {
        assert!(
            self.child.try_wait().unwrap().is_none(),
            "signal only the owned live child"
        );
        assert!(
            Command::new("/bin/kill")
                .args([signal, &self.child.id().to_string()])
                .status()
                .expect("signal child")
                .success()
        );
    }

    fn finish(&mut self) {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "child failed: {status}");
                return;
            }
            assert!(start.elapsed() < WAIT, "child did not exit");
            thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // SIGKILL also terminates a stopped child if the parent test fails.
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn announce(event: &str) {
    println!("BDB_OWNER:{event}");
    io::stdout().flush().unwrap();
}

fn wait_for_parent() {
    let mut line = String::new();
    assert!(
        io::stdin().read_line(&mut line).unwrap() > 0,
        "parent closed the pipe"
    );
}

/// Subprocess entry; an ordinary test invocation performs no helper work.
#[test]
fn ownership_child() {
    let Ok(mode) = std::env::var("BDB_OWNERSHIP_CHILD") else {
        return;
    };
    let path = PathBuf::from(std::env::var_os("BDB_OWNERSHIP_PATH").unwrap());
    match mode.as_str() {
        "directory" => {
            let ownership = acquire_directory(&path).unwrap();
            fs::create_dir_all(ownership.directory()).unwrap();
            announce("held");
            wait_for_parent();
            fs::write(ownership.directory().join("resumed"), b"original owner").unwrap();
        }
        "contender" => {
            announce("ready");
            wait_for_parent();
            match acquire_directory(&path) {
                Ok(_ownership) => {
                    announce("held");
                    wait_for_parent();
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => announce("busy"),
                Err(err) => panic!("unexpected lock failure: {err}"),
            }
        }
        "mutation" => {
            let key = StoreKey::of("manifest");
            let _ownership = acquire_mutation(&path, &key).unwrap();
            assert_eq!(fs::read(path.join("manifest")).unwrap(), b"v1");
            let temp = synced_temp(&path, b"v2").unwrap();
            announce("held");
            wait_for_parent(); // Real stop/resume between compare/stage and rename.
            fs::rename(&temp, path.join("manifest")).unwrap();
            sync_parent(&path.join("manifest")).unwrap();
        }
        _ => panic!("unknown child mode"),
    }
}

#[test]
fn simultaneous_processes_have_one_directory_owner() {
    let root = TestDir::new();
    let directory = root.0.join("tenant");
    let mut workers: Vec<_> = (0..8)
        .map(|_| Worker::spawn("contender", &directory))
        .collect();
    for worker in &workers {
        assert_eq!(worker.event(), "ready");
    }
    for worker in &mut workers {
        worker.send();
    }
    let mut held = 0;
    for worker in &workers {
        match worker.event().as_str() {
            "held" => held += 1,
            "busy" => {}
            other => panic!("unexpected event {other}"),
        }
    }
    assert_eq!(held, 1);
    // Dropping all workers kills/reaps the one blocked owner as well.
    drop(workers);
    assert!(acquire_directory(&directory).is_ok());
}

#[test]
fn paused_owner_cannot_be_stolen_and_process_death_releases_it() {
    let root = TestDir::new();
    let directory = root.0.join("tenant");
    let mut worker = Worker::spawn("directory", &directory);
    assert_eq!(worker.event(), "held");
    worker.signal("-STOP");
    assert_eq!(
        acquire_directory(&directory).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    // No timestamp/head/token file can expire the kernel's held descriptor.
    let legacy = root.0.join("~lease/tenant");
    fs::write(legacy.join("~head"), b"18446744073709551615").unwrap();
    fs::write(legacy.join("1"), b"expired legacy lease").unwrap();
    assert_eq!(
        acquire_directory(&directory).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    worker.signal("-CONT");
    worker.send();
    worker.finish();
    assert_eq!(
        fs::read(directory.join("resumed")).unwrap(),
        b"original owner"
    );
    drop(worker);

    let mut killed = Worker::spawn("directory", &directory);
    assert_eq!(killed.event(), "held");
    killed.child.kill().unwrap();
    assert!(!killed.child.wait().unwrap().success());
    let _next = acquire_directory(&directory).expect("kernel releases after SIGKILL");
}

#[test]
fn paused_mutation_times_out_contender_instead_of_overwriting_it() {
    let root = TestDir::new();
    let store = FsStore::new(&root.0);
    let key = StoreKey::of("manifest");
    let Create::Created(before) = store.put_create(&key, b"v1").unwrap() else {
        panic!("birth");
    };
    let mut worker = Worker::spawn("mutation", &root.0);
    assert_eq!(worker.event(), "held");
    worker.signal("-STOP");
    // This bounded attempt outlasts the old five-second mutation TTL.
    let error = store.put_swap(&key, b"contender", &before).unwrap_err();
    assert_eq!(error.source.kind(), io::ErrorKind::WouldBlock);
    let _second_store = FsStore::new(&root.0);
    assert_eq!(store.get(&key).unwrap().unwrap().bytes, b"v1");
    worker.signal("-CONT");
    worker.send();
    worker.finish();
    assert_eq!(store.get(&key).unwrap().unwrap().bytes, b"v2");
    assert_eq!(
        store.put_swap(&key, b"stale", &before).unwrap(),
        Swap::Moved
    );
}

#[test]
fn lock_identity_survives_materialization_rename_and_delete() {
    use std::os::unix::fs::MetadataExt as _;
    let root = TestDir::new();
    let directory = root.0.join("tenant");
    let ownership = acquire_directory(&directory).unwrap();
    fs::create_dir(&directory).unwrap();
    let lock_path = root.0.join("~lease/tenant/owner.lock");
    let inode = fs::metadata(&lock_path).unwrap().ino();
    fs::rename(&directory, root.0.join("old-tenant")).unwrap();
    assert_eq!(
        acquire_directory(&directory).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    fs::create_dir(&directory).unwrap();
    fs::remove_dir(&directory).unwrap();
    assert_eq!(
        acquire_directory(&directory).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(ownership);
    let _next = acquire_directory(&directory).unwrap();
    assert_eq!(fs::metadata(lock_path).unwrap().ino(), inode);
}

#[test]
fn repeated_acquisition_never_creates_a_token_history() {
    let root = TestDir::new();
    let directory = root.0.join("tenant");
    for _ in 0..1_000 {
        drop(acquire_directory(&directory).unwrap());
    }
    let entries: Vec<_> = fs::read_dir(root.0.join("~lease/tenant"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, ["owner.lock"]);
    assert!(
        !directory.exists(),
        "locking must not create the materialization"
    );
}

#[test]
fn symlinked_materialization_or_lock_is_refused() {
    use std::os::unix::fs::symlink;
    let root = TestDir::new();
    let target = root.0.join("target");
    fs::create_dir(&target).unwrap();
    let alias = root.0.join("alias");
    symlink(&target, &alias).unwrap();
    assert_eq!(
        acquire_directory(&alias).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    let directory = root.0.join("tenant");
    fs::create_dir_all(root.0.join("~lease/tenant")).unwrap();
    let sentinel = root.0.join("sentinel");
    fs::write(&sentinel, b"unchanged").unwrap();
    symlink(&sentinel, root.0.join("~lease/tenant/owner.lock")).unwrap();
    assert_eq!(
        acquire_directory(&directory).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(fs::read(sentinel).unwrap(), b"unchanged");
}

#[test]
fn competing_public_opens_cannot_sweep_live_owner_scratch() {
    let root = TestDir::new();
    let remote = root.0.join("objects");
    let directory = root.0.join("tenant");
    let log = TestLog::new(remote.clone(), "");
    let Opened::Ready(replica) =
        Replica::open(FsStore::new(&remote), "", &directory, theory()).unwrap()
    else {
        panic!("initial open");
    };
    fs::create_dir_all(directory.join("~tmp")).unwrap();
    fs::write(directory.join("~tmp/live"), b"active temp").unwrap();
    fs::write(directory.join(".chain.tmp"), b"active sidecar").unwrap();
    let compact = directory.with_extension("ckpt");
    fs::create_dir(&compact).unwrap();
    fs::write(compact.join("live"), b"active compact").unwrap();
    let digest = [17; 32];
    record_ckpt_scratch(&directory, &digest).unwrap();
    let doc = bumbledb_log::manifest::ckpt_doc_key("", &digest);
    let data = bumbledb_log::manifest::ckpt_mdb_key("", &digest);
    log.store.put_create(&doc, b"active remote doc").unwrap();
    log.store.put_create(&data, b"active remote data").unwrap();
    assert!(
        matches!(Replica::open(FsStore::new(&remote), "", &directory, theory()),
        Err(Fault::Io(err)) if err.kind() == io::ErrorKind::WouldBlock)
    );
    assert!(
        matches!(Writer::open(FsStore::new(&remote), "new-prefix", &directory, theory(), Options::new(1)),
        Err(bumbledb_log::writer::Error::Fault(Fault::Io(err))) if err.kind() == io::ErrorKind::WouldBlock)
    );
    assert!(matches!(sweep_at_open(&log.store, "", &directory),
        Err(Fault::Io(err)) if err.kind() == io::ErrorKind::WouldBlock));
    assert_eq!(
        fs::read(directory.join("~tmp/live")).unwrap(),
        b"active temp"
    );
    assert_eq!(
        fs::read(directory.join(".chain.tmp")).unwrap(),
        b"active sidecar"
    );
    assert_eq!(fs::read(compact.join("live")).unwrap(), b"active compact");
    assert!(log.store.get(&doc).unwrap().is_some());
    assert!(log.store.get(&data).unwrap().is_some());
    assert!(
        log.store
            .get(&bumbledb_log::manifest::manifest_key("new-prefix"))
            .unwrap()
            .is_none(),
        "failed writer open cannot birth another remote namespace"
    );
    replica.dispose().unwrap();
    assert!(!directory.exists());
    let _next = acquire_directory(&directory).unwrap();
}

#[test]
fn constructors_do_not_sweep_old_temporary_files() {
    let root = TestDir::new();
    let temp = synced_temp(&root.0, b"still live").unwrap();
    File::options()
        .write(true)
        .open(&temp)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH))
        .unwrap();
    let _store = FsStore::new(&root.0);
    assert_eq!(fs::read(temp).unwrap(), b"still live");
}
