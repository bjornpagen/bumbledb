//! The cross-language interop lane: one `FsStore` prefix, both drivers.
//! Rust writes and TS reads byte-for-byte, TS writes and Rust reads, and
//! a mixed fleet of real Node child processes and Rust threads races one
//! prefix — exactly one Created per slot, every CAS linearized, etags
//! agreeing on every object. The Node half is
//! ts-log/test/interop-child.ts; children print structured `INTEROP`
//! lines and this orchestrator asserts hard.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::{io, io::BufRead, io::Write};

use bumbledb_log::store::fs::{FsStore, content_etag};
use bumbledb_log::store::{Create, ObjectStore, StoreKey, Swap};

/// The shared corpus rule, implemented identically in the Node child:
/// body[j] of object i is (i * 31 + j * 7) mod 256.
const CORPUS_SIZES: [usize; 6] = [0, 1, 3, 256, 4096, 65536];

const RACE_SLOTS: usize = 6;
const SWAPS_PER_CONTENDER: u64 = 8;

fn corpus_key(index: usize) -> String {
    format!("interop/obj-{index}")
}

fn corpus_body(index: usize) -> Vec<u8> {
    (0..CORPUS_SIZES[index])
        .map(|j| u8::try_from((index * 31 + j * 7) % 256).expect("mod 256 fits"))
        .collect()
}

fn base_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "bdb-log-b-interop-{}-{name}-{nanos}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create base dir");
    dir
}

fn store_root(base: &Path) -> PathBuf {
    base.join("store")
}

fn child_script() -> PathBuf {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ts-log/test/interop-child.ts");
    assert!(
        script.exists(),
        "the Node half of the lane is missing: {}",
        script.display()
    );
    script
}

fn spawn_child(base: &Path, args: &[String]) -> Child {
    let mut all = vec![child_script().into_os_string().into_string().expect("utf8")];
    all.extend_from_slice(args);
    Command::new("node")
        .args(&all)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(base)
        .spawn()
        .expect("spawn node child")
}

fn harvest(children: Vec<Child>) -> Vec<serde_json::Value> {
    let mut reports = Vec::new();
    for child in children {
        let out = child.wait_with_output().expect("child exit");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            out.status.success(),
            "node child failed: {stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        for line in stdout.lines() {
            if let Some(json) = line.strip_prefix("INTEROP ") {
                reports.push(serde_json::from_str(json).expect("child line parses"));
            }
        }
    }
    reports
}

fn field<'v>(report: &'v serde_json::Value, name: &str) -> &'v str {
    report[name].as_str().expect("string field")
}

#[test]
fn rust_writes_ts_reads_byte_for_byte() {
    let base = base_dir("rust_writes");
    let store = FsStore::new(store_root(&base));
    let mut keys = Vec::new();
    for i in 0..CORPUS_SIZES.len() {
        let body = corpus_body(i);
        let outcome = store
            .put_create(&StoreKey::of(&corpus_key(i)), &body)
            .expect("create");
        assert_eq!(outcome, Create::Created(content_etag(&body)));
        keys.push(corpus_key(i));
    }

    let mut args = vec!["read".to_string(), store_root(&base).display().to_string()];
    args.extend(keys);
    let reports = harvest(vec![spawn_child(&base, &args)]);
    assert_eq!(reports.len(), CORPUS_SIZES.len(), "one report per object");
    for (i, report) in reports.iter().enumerate() {
        let body = corpus_body(i);
        assert_eq!(field(report, "key"), corpus_key(i));
        let hex: String = body.iter().fold(String::new(), |mut acc, b| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{b:02x}");
            acc
        });
        assert_eq!(field(report, "hex"), hex, "object {i}: byte-for-byte");
        assert_eq!(
            field(report, "etag"),
            content_etag(&body).0,
            "object {i}: etags agree across languages"
        );
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn ts_writes_rust_reads_byte_for_byte() {
    let base = base_dir("ts_writes");
    let args = ["write".to_string(), store_root(&base).display().to_string()];
    let reports = harvest(vec![spawn_child(&base, &args)]);
    assert_eq!(reports.len(), CORPUS_SIZES.len(), "one report per object");

    let store = FsStore::new(store_root(&base));
    for (i, report) in reports.iter().enumerate() {
        let body = corpus_body(i);
        let fetched = store
            .get(&StoreKey::of(&corpus_key(i)))
            .expect("get")
            .expect("object present");
        assert_eq!(fetched.bytes, body, "object {i}: byte-for-byte");
        assert_eq!(fetched.etag, content_etag(&body), "object {i}: Rust etag");
        assert_eq!(
            field(report, "etag"),
            content_etag(&body).0,
            "object {i}: the TS Created etag is the same blake3"
        );
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn mixed_fleet_create_only_is_exclusive_per_slot() {
    let base = base_dir("race_create");
    let root = store_root(&base);
    std::fs::create_dir_all(&root).expect("store root");

    let children: Vec<Child> = (0..3)
        .map(|id| {
            spawn_child(
                &base,
                &[
                    "race-create".to_string(),
                    root.display().to_string(),
                    id.to_string(),
                    RACE_SLOTS.to_string(),
                ],
            )
        })
        .collect();

    let store = Arc::new(FsStore::new(&root));
    let go = base.join("go");
    let threads: Vec<_> = (0..3u32)
        .map(|id| {
            let store = Arc::clone(&store);
            let go = go.clone();
            std::thread::spawn(move || {
                while !go.exists() {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                let mut outcomes = Vec::new();
                for s in 0..RACE_SLOTS {
                    let body = format!("rs-{id}-slot-{s}");
                    let outcome = store
                        .put_create(&StoreKey::of(&format!("race/slot-{s}")), body.as_bytes())
                        .expect("put_create");
                    outcomes.push((s, matches!(outcome, Create::Created(_)), body));
                }
                outcomes
            })
        })
        .collect();

    std::fs::write(&go, b"go").expect("raise barrier");
    let reports = harvest(children);
    let mut created_per_slot = [0u32; RACE_SLOTS];
    let mut winner_body: [Option<String>; RACE_SLOTS] = [const { None }; RACE_SLOTS];
    for report in &reports {
        let slot = usize::try_from(report["slot"].as_u64().expect("slot")).expect("fits");
        if field(report, "outcome") == "created" {
            created_per_slot[slot] += 1;
            winner_body[slot] = Some(format!("ts-{}-slot-{slot}", field(report, "id")));
        }
    }
    for handle in threads {
        for (slot, created, body) in handle.join().expect("thread") {
            if created {
                created_per_slot[slot] += 1;
                winner_body[slot] = Some(body);
            }
        }
    }
    assert_eq!(
        reports.len(),
        3 * RACE_SLOTS,
        "one report per node contender per slot"
    );
    for (slot, count) in created_per_slot.iter().enumerate() {
        assert_eq!(
            *count, 1,
            "slot {slot}: exactly one Created across the fleet"
        );
        let fetched = store
            .get(&StoreKey::of(&format!("race/slot-{slot}")))
            .expect("get")
            .expect("slot present");
        let winner = winner_body[slot].clone().expect("winner recorded");
        assert_eq!(
            fetched.bytes,
            winner.as_bytes(),
            "slot {slot}: the winner's bytes stand untouched"
        );
        assert_eq!(
            fetched.etag,
            content_etag(winner.as_bytes()),
            "slot {slot}: etags agree"
        );
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn mixed_fleet_cas_linearizes() {
    let base = base_dir("race_swap");
    let root = store_root(&base);
    let store = Arc::new(FsStore::new(&root));
    assert!(matches!(
        store
            .put_create(&StoreKey::of("race/counter"), b"0")
            .expect("birth"),
        Create::Created(_)
    ));

    let children: Vec<Child> = (0..2)
        .map(|id| {
            spawn_child(
                &base,
                &[
                    "race-swap".to_string(),
                    root.display().to_string(),
                    id.to_string(),
                    SWAPS_PER_CONTENDER.to_string(),
                ],
            )
        })
        .collect();

    let go = base.join("go");
    let threads: Vec<_> = (0..2u32)
        .map(|_| {
            let store = Arc::clone(&store);
            let go = go.clone();
            std::thread::spawn(move || {
                while !go.exists() {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                let mut swapped = 0u64;
                while swapped < SWAPS_PER_CONTENDER {
                    let current = store
                        .get(&StoreKey::of("race/counter"))
                        .expect("get")
                        .expect("counter present");
                    let value: u64 = String::from_utf8(current.bytes)
                        .expect("utf8")
                        .parse()
                        .expect("decimal");
                    let next = (value + 1).to_string();
                    if let Swap::Swapped(etag) = store
                        .put_swap(
                            &StoreKey::of("race/counter"),
                            next.as_bytes(),
                            &current.etag,
                        )
                        .expect("swap")
                    {
                        assert_eq!(etag, content_etag(next.as_bytes()), "swap etag agrees");
                        swapped += 1;
                    }
                }
            })
        })
        .collect();

    std::fs::write(&go, b"go").expect("raise barrier");
    let reports = harvest(children);
    for handle in threads {
        handle.join().expect("thread");
    }
    assert_eq!(reports.len(), 2, "one report per node contender");
    for report in &reports {
        assert_eq!(
            report["swapped"].as_u64().expect("swapped"),
            SWAPS_PER_CONTENDER,
            "every node contender lands all its swaps"
        );
    }
    let total: u64 = String::from_utf8(
        store
            .get(&StoreKey::of("race/counter"))
            .expect("get")
            .expect("counter present")
            .bytes,
    )
    .expect("utf8")
    .parse()
    .expect("decimal");
    assert_eq!(
        total,
        4 * SWAPS_PER_CONTENDER,
        "no swap was lost and none applied twice across the mixed fleet"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// An owned subprocess with an event gate, never a timing-based assertion.
/// The watchdog only fails a wedged harness; expiration cannot pass a test.
struct CasChild {
    child: Child,
    events: Receiver<String>,
    reader: Option<JoinHandle<()>>,
}

impl CasChild {
    fn spawn(base: &Path, mode: &str, expected: &str) -> Self {
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ts-log/test/interop-cas-child.ts");
        let mut child = Command::new("node")
            .arg(script)
            .arg(mode)
            .arg(store_root(base))
            .arg(expected)
            .env_remove("NODE_OPTIONS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .current_dir(base)
            .spawn()
            .expect("spawn one-shot Node CAS child");
        let stdout = child.stdout.take().unwrap();
        let (send, events) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in io::BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Some(event) = line.strip_prefix("INTEROP_CAS ")
                    && send.send(event.to_owned()).is_err()
                {
                    break;
                }
            }
        });
        Self {
            child,
            events,
            reader: Some(reader),
        }
    }

    fn event(&self) -> serde_json::Value {
        let event = self
            .events
            .recv_timeout(Duration::from_secs(20))
            .expect("child event, not an elapsed-time ownership assumption");
        serde_json::from_str(&event).expect("structured child event")
    }

    fn release(&mut self) {
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"continue\n")
            .expect("release paused compare-read");
    }

    fn finish(&mut self) {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "Node CAS child failed: {status}");
                return;
            }
            assert!(start.elapsed() < Duration::from_secs(20), "child wedged");
            thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for CasChild {
    fn drop(&mut self) {
        // Reap even when an assertion fails while the child is at the gate.
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

#[test]
fn mixed_cas_paused_compare_has_exactly_one_winner() {
    let base = base_dir("staged_swap");
    let store = FsStore::new(store_root(&base));
    let key = StoreKey::of("race/counter");
    let Create::Created(before) = store.put_create(&key, b"before").unwrap() else {
        panic!("counter birth");
    };
    let mut child = CasChild::spawn(&base, "pause-read", &before.0);
    let first = child.event();
    let rust = store.put_swap(&key, b"rust-after", &before).unwrap();
    let node = match field(&first, "event") {
        "read-paused" => {
            assert_eq!(field(&first, "bytes"), "before");
            child.release();
            child.event()
        }
        "completed" => first,
        other => panic!("unexpected first CAS event: {other}"),
    };
    assert_eq!(field(&node, "event"), "completed");
    child.finish();
    let node_won = match field(&node, "outcome") {
        "swapped" => true,
        "moved" => false,
        other => panic!("one-shot CAS did not decide: {other}"),
    };
    let rust_won = matches!(rust, Swap::Swapped(_));
    assert_eq!(
        usize::from(rust_won) + usize::from(node_won),
        1,
        "both real adapters accepted one predecessor etag: Rust={rust:?}, Node={node}"
    );
    let expected = if rust_won {
        b"rust-after"
    } else {
        b"node-after"
    };
    let final_object = store.get(&key).unwrap().unwrap();
    assert_eq!(final_object.bytes, expected);
    assert_eq!(final_object.etag, content_etag(expected));
    let _ = std::fs::remove_dir_all(&base);
}

#[cfg(unix)]
#[test]
fn node_cas_refuses_a_symlinked_rust_mutation_lock() {
    use std::os::unix::fs::symlink;
    let base = base_dir("poison_lock");
    let root = store_root(&base);
    let store = FsStore::new(&root);
    let key = StoreKey::of("race/counter");
    let Create::Created(before) = store.put_create(&key, b"before").unwrap() else {
        panic!("counter birth");
    };
    let sentinel = base.join("lock-sentinel");
    std::fs::write(&sentinel, b"must remain untouched").unwrap();
    let lock = root.join("~lease/race/counter/mutation.lock");
    std::fs::remove_file(&lock).unwrap();
    symlink(&sentinel, &lock).unwrap();
    // Verify this fixture is a real refusal at Rust's mutation boundary.
    let refused = store.put_swap(&key, b"rust-after", &before).unwrap_err();
    assert_eq!(refused.source.kind(), io::ErrorKind::InvalidInput);
    let mut child = CasChild::spawn(&base, "poison-lock", &before.0);
    let result = child.event();
    child.finish();
    assert_eq!(
        field(&result, "event"),
        "refused",
        "Node must enforce the same poisoned lock refusal: {result}"
    );
    assert_eq!(store.get(&key).unwrap().unwrap().bytes, b"before");
    assert_eq!(std::fs::read(sentinel).unwrap(), b"must remain untouched");
    let _ = std::fs::remove_dir_all(&base);
}
