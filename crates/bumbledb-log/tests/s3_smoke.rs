//! Credential-gated S3 smokes. The same env as the CI gate:
//! `BUMBLEDB_S3_SMOKE_BUCKET`, `AWS_ACCESS_KEY_ID`,
//! `AWS_SECRET_ACCESS_KEY` required; `BUMBLEDB_S3_SMOKE_REGION`
//! defaults to `us-east-1`; `BUMBLEDB_S3_SMOKE_ENDPOINT` optional.
//! Missing credentials skip loudly and never fail.

mod lane_e_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use bumbledb::SchemaDescriptor;
use bumbledb_log::checkpointer::{Checkpointer, CheckpointerOpened, Compact, Ran};
use bumbledb_log::replica::{Opened, Replica};
use bumbledb_log::store::s3::{S3Config, S3Credentials, S3Store};
use bumbledb_log::store::{Create, ObjectStore, Poll, StoreKey, Swap};
use bumbledb_log::writer::{Commit, Options, Writer, WriterOpened};
use lane_e_support::{NOTE, note_row, temp_dir, theory};

const REQUIRED: [&str; 3] = [
    "BUMBLEDB_S3_SMOKE_BUCKET",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
];

static PREFIX_SEQ: AtomicU64 = AtomicU64::new(0);

fn missing_required() -> Vec<&'static str> {
    REQUIRED
        .into_iter()
        .filter(|key| !matches!(std::env::var(key), Ok(value) if !value.is_empty()))
        .collect()
}

/// Pid, clock nanos, and a process-local sequence. Machines and
/// re-runs cannot share a prefix: seq resets, pid reuses, nanos does
/// not.
fn unique_prefix(tag: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!(
        "smoke/{}/{}/{}/{tag}",
        std::process::id(),
        nanos,
        PREFIX_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

/// Sweeps the store prefix on drop. The binding must stay live until
/// the test ends — a `_` discard would sweep under a still-running body.
struct PrefixSweep(S3Store);

impl Drop for PrefixSweep {
    fn drop(&mut self) {
        if let Err(err) = self.0.sweep_prefix() {
            eprintln!("S3 smoke prefix sweep failed: {err}");
        }
    }
}

fn smoke_store(prefix: &str) -> Option<(S3Store, PrefixSweep)> {
    let missing = missing_required();
    if !missing.is_empty() {
        eprintln!("SKIPPED S3 smoke: credential-gated lane not run (missing {missing:?})");
        return None;
    }
    let region = std::env::var("BUMBLEDB_S3_SMOKE_REGION").unwrap_or_else(|_| "us-east-1".into());
    let endpoint = std::env::var("BUMBLEDB_S3_SMOKE_ENDPOINT")
        .ok()
        .filter(|value| !value.is_empty());
    let session_token = std::env::var("AWS_SESSION_TOKEN")
        .ok()
        .filter(|value| !value.is_empty());
    let store = S3Store::new(&S3Config {
        endpoint,
        region,
        bucket: std::env::var("BUMBLEDB_S3_SMOKE_BUCKET").expect("bucket"),
        credentials: S3Credentials::Static {
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").expect("id"),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").expect("secret"),
            session_token,
        },
        prefix: prefix.to_string(),
    })
    .expect("S3Store");
    Some((store.clone(), PrefixSweep(store)))
}

#[test]
fn s3_smoke_skips_loudly_without_credentials() {
    let missing = missing_required();
    if missing.is_empty() {
        eprintln!("S3 smoke credentials present; the s3_smoke* verbs run against the bucket");
    } else {
        eprintln!("SKIPPED S3 smoke: credential-gated lane not run (missing {missing:?})");
    }
}

#[test]
fn s3_smoke_create_only_race() {
    let Some((store, _sweep)) = smoke_store(&unique_prefix("create")) else {
        return;
    };
    let key = StoreKey::of("log/c00000000/1");
    let a = store.clone();
    let b = store.clone();
    let key_a = key.clone();
    let key_b = key.clone();
    let left = thread::spawn(move || a.put_create(&key_a, b"alpha"));
    let right = thread::spawn(move || b.put_create(&key_b, b"beta"));
    let left = left.join().expect("a");
    let right = right.join().expect("b");
    let winner = match (&left, &right) {
        (Ok(Create::Created(_)), Ok(Create::Exists)) => &b"alpha"[..],
        (Ok(Create::Exists), Ok(Create::Created(_))) => &b"beta"[..],
        other => panic!("exactly one Created and one Exists, got {other:?}"),
    };
    let fetched = store.get(&key).expect("get").expect("present");
    assert_eq!(
        fetched.bytes, winner,
        "the Created arm is the body that persisted"
    );
    store.delete(&key).expect("cleanup");
}

#[test]
fn s3_smoke_cas_linearizes() {
    let Some((store, _sweep)) = smoke_store(&unique_prefix("cas")) else {
        return;
    };
    let key = StoreKey::of("manifest.json");
    assert!(matches!(
        store.put_create(&key, b"0").expect("birth"),
        Create::Created(_)
    ));
    let store = Arc::new(store);
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let store = Arc::clone(&store);
            let key = key.clone();
            thread::spawn(move || {
                let mut landed = 0u64;
                while landed < 4 {
                    let current = store.get(&key).expect("get").expect("present");
                    let value: u64 = String::from_utf8(current.bytes)
                        .expect("utf8")
                        .parse()
                        .expect("decimal");
                    let next = (value + 1).to_string();
                    if let Swap::Swapped(_) = store
                        .put_swap(&key, next.as_bytes(), &current.etag)
                        .expect("swap")
                    {
                        landed += 1;
                    }
                }
            })
        })
        .collect();
    for handle in threads {
        handle.join().expect("thread");
    }
    let total: u64 = String::from_utf8(store.get(&key).expect("get").expect("present").bytes)
        .expect("utf8")
        .parse()
        .expect("decimal");
    assert_eq!(total, 8, "no swap was lost and none applied twice");
    store.delete(&key).expect("cleanup");
}

#[test]
fn s3_smoke_poll_unchanged() {
    let Some((store, _sweep)) = smoke_store(&unique_prefix("poll")) else {
        return;
    };
    let key = StoreKey::of("manifest.json");
    let Create::Created(etag) = store.put_create(&key, br#"{"v":1}"#).expect("create") else {
        panic!("fresh key must be Created");
    };
    assert_eq!(
        store.get_if_changed(&key, &etag).expect("poll"),
        Poll::Unchanged
    );
    store.delete(&key).expect("cleanup");
}

#[test]
fn s3_smoke_get_before_put() {
    let Some((store, _sweep)) = smoke_store(&unique_prefix("negcache")) else {
        return;
    };
    let key = StoreKey::of("log/c00000000/probe");
    assert_eq!(store.get(&key).expect("probe"), None);
    assert!(matches!(
        store.put_create(&key, b"after-miss").expect("create"),
        Create::Created(_)
    ));
    let fetched = store.get(&key).expect("get").expect("present");
    assert_eq!(fetched.bytes, b"after-miss");
    store.delete(&key).expect("cleanup");
}

fn open_writer(store: S3Store, dir: &Path) -> Writer<SchemaDescriptor, S3Store> {
    match Writer::open(store, "", dir, theory(), Options::new(91)).expect("open writer") {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn open_replica(store: S3Store, dir: &Path) -> Replica<SchemaDescriptor, S3Store> {
    match Replica::open(store, "", dir, theory()).expect("open replica") {
        Opened::Ready(replica) => *replica,
        Opened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

#[test]
fn s3_smoke_replica_writer_round_trip() {
    let Some((store, _sweep)) = smoke_store(&unique_prefix("roundtrip")) else {
        return;
    };
    let root = temp_dir("s3_roundtrip");
    let writer = open_writer(store.clone(), &root.join("w"));
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(7, "s3-smoke")]);
            Ok(())
        })
        .expect("commit");
    let Commit::Accepted { generation, .. } = outcome else {
        panic!("accepted expected");
    };
    assert_eq!(generation, 1);
    drop(writer);

    let replica = open_replica(store, &root.join("r"));
    let present = replica
        .db()
        .read(|instance| instance.contains_dyn(NOTE, &note_row(7, "s3-smoke")))
        .expect("read");
    assert!(present, "reopen restores the committed note");
}

fn child_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ts-log/test/interop-child.ts")
}

#[test]
fn s3_smoke_interop_rust_writes_ts_reads() {
    let prefix = unique_prefix("interop");
    let Some((store, _sweep)) = smoke_store(&prefix) else {
        return;
    };
    let key = StoreKey::of("interop/obj-0");
    let body = b"cross-language-s3";
    let Create::Created(etag) = store.put_create(&key, body).expect("create") else {
        panic!("fresh key must be Created");
    };

    let out = std::process::Command::new("node")
        .args([
            child_script().into_os_string().into_string().expect("utf8"),
            "read".into(),
            "s3".into(),
            prefix,
            key.to_string(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("spawn node child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "node child failed: {stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let line = stdout
        .lines()
        .find_map(|line| line.strip_prefix("INTEROP "))
        .expect("interop line");
    let report: serde_json::Value = serde_json::from_str(line).expect("json");
    assert_eq!(report["hex"].as_str().expect("hex"), hex_of(body));
    assert_eq!(
        report["etag"].as_str().expect("etag"),
        etag.0,
        "vendor etag agrees across languages; not blake3"
    );
    store.delete(&key).expect("cleanup");
}

#[test]
fn s3_smoke_duty_once() {
    let Some((store, _sweep)) = smoke_store(&unique_prefix("duty")) else {
        return;
    };
    let root = temp_dir("s3_duty");
    let writer = open_writer(store.clone(), &root.join("w"));
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(NOTE, [note_row(3, "duty-s3")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { .. }
    ));
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(NOTE, [note_row(4, "duty-s3")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { .. }
    ));
    writer.quiesce();
    drop(writer);

    let mut duty =
        match Checkpointer::open(store, "", &root.join("d"), theory(), 91).expect("open duty") {
            CheckpointerOpened::Ready(duty) => *duty,
            CheckpointerOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
        };
    duty.set_checkpoint_cadence(2, u64::MAX);
    match duty.run().expect("run") {
        Ran::Ready {
            compact: Compact::Published(_),
            ..
        } => {}
        other => panic!("duty should compact on S3, got {other:?}"),
    }
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    })
}
