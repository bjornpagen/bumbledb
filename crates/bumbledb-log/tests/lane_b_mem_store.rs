//! Single-process semantics of the five verbs over `MemStore`, plus the
//! retry-law helpers that never needed a disk.

use std::io;
use std::sync::Mutex;

use bumbledb_log::store::fs::content_etag;
use bumbledb_log::store::mem::MemStore;
use bumbledb_log::store::{
    Create, CreateProbe, Etag, Fetched, ObjectStore, Poll, Result, StoreError, StoreKey, Swap,
    SwapProbe, resolve_ambiguous_create, resolve_ambiguous_swap, retry_read,
};

#[test]
fn get_missing_is_none() {
    let store = MemStore::new();
    assert_eq!(
        store
            .get(&StoreKey::of("log/c00000001/nothing"))
            .expect("get"),
        None
    );
}

#[test]
fn put_create_wins_once_and_reports_content_etag() {
    let store = MemStore::new();
    let key = StoreKey::of("log/c00000001/slot-a");
    let body = b"first writer's batch".as_slice();
    let outcome = store.put_create(&key, body).expect("create");
    assert_eq!(outcome, Create::Created(content_etag(body)));

    let second = store
        .put_create(&key, b"second writer's batch")
        .expect("create");
    assert_eq!(second, Create::Exists);

    let fetched = store.get(&key).expect("get").expect("present");
    assert_eq!(fetched.bytes, body);
    assert_eq!(fetched.etag, content_etag(body));
}

#[test]
fn get_if_changed_distinguishes_304_from_change() {
    let store = MemStore::new();
    let key = StoreKey::of("manifest.json");
    let body = br#"{"v":2}"#.as_slice();
    let Create::Created(etag) = store.put_create(&key, body).expect("create") else {
        panic!("fresh key must be Created");
    };
    assert_eq!(
        store.get_if_changed(&key, &etag).expect("poll"),
        Poll::Unchanged
    );

    let stale = content_etag(b"some other body");
    let Poll::Changed(fetched) = store.get_if_changed(&key, &stale).expect("poll") else {
        panic!("stale etag must observe the change");
    };
    assert_eq!(fetched.bytes, body);
    assert_eq!(fetched.etag, etag);
}

#[test]
fn put_swap_swaps_on_match_and_moves_on_mismatch() {
    let store = MemStore::new();
    let key = StoreKey::of("manifest.json");
    let Create::Created(birth) = store.put_create(&key, b"v1").expect("create") else {
        panic!("fresh key must be Created");
    };

    let swapped = store.put_swap(&key, b"v2", &birth).expect("swap");
    assert_eq!(swapped, Swap::Swapped(content_etag(b"v2")));
    assert_eq!(store.get(&key).expect("get").expect("present").bytes, b"v2");

    let moved = store.put_swap(&key, b"v3", &birth).expect("swap");
    assert_eq!(moved, Swap::Moved);
    assert_eq!(store.get(&key).expect("get").expect("present").bytes, b"v2");
}

#[test]
fn put_swap_on_missing_key_is_moved() {
    let store = MemStore::new();
    let etag = content_etag(b"anything");
    assert_eq!(
        store
            .put_swap(&StoreKey::of("manifest.json"), b"v1", &etag)
            .expect("swap"),
        Swap::Moved
    );
}

#[test]
fn delete_is_unconditional_and_idempotent() {
    let store = MemStore::new();
    let key = StoreKey::of("log/c00000001/gc-victim");
    store.put_create(&key, b"doomed").expect("create");
    store.delete(&key).expect("delete");
    assert_eq!(store.get(&key).expect("get"), None);
    store.delete(&key).expect("delete of absent key");
}

#[test]
fn put_swap_serializes_across_threads_without_lost_updates() {
    let store = std::sync::Arc::new(MemStore::new());
    assert!(matches!(
        store
            .put_create(&StoreKey::of("manifest.json"), b"0")
            .expect("birth"),
        Create::Created(_)
    ));
    let threads: Vec<_> = (0..4u64)
        .map(|_| {
            let store = std::sync::Arc::clone(&store);
            std::thread::spawn(move || {
                let mut landed = 0u64;
                while landed < 8 {
                    let current = store
                        .get(&StoreKey::of("manifest.json"))
                        .expect("get")
                        .expect("present");
                    let value: u64 = String::from_utf8(current.bytes)
                        .expect("utf8")
                        .parse()
                        .expect("decimal");
                    let next = (value + 1).to_string();
                    if let Swap::Swapped(_) = store
                        .put_swap(
                            &StoreKey::of("manifest.json"),
                            next.as_bytes(),
                            &current.etag,
                        )
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
    let total: u64 = String::from_utf8(
        store
            .get(&StoreKey::of("manifest.json"))
            .expect("get")
            .expect("present")
            .bytes,
    )
    .expect("utf8")
    .parse()
    .expect("decimal");
    assert_eq!(total, 32, "no swap was lost and none applied twice");
}

#[test]
fn retry_read_recovers_from_transient_failures() {
    let mut failures_left = 2u32;
    let value = retry_read(|| {
        if failures_left > 0 {
            failures_left -= 1;
            Err(StoreError {
                op: "get",
                key: "flaky".to_string(),
                source: io::Error::from(io::ErrorKind::ConnectionReset),
            })
        } else {
            Ok(7u32)
        }
    })
    .expect("recovers");
    assert_eq!(value, 7);
    assert_eq!(failures_left, 0);
}

#[test]
fn retry_read_surfaces_err_after_six_attempts() {
    let mut attempts = 0u32;
    let outcome: Result<()> = retry_read(|| {
        attempts += 1;
        Err(StoreError {
            op: "get",
            key: "down".to_string(),
            source: io::Error::from(io::ErrorKind::ConnectionReset),
        })
    });
    assert!(outcome.is_err());
    assert_eq!(attempts, 6);
}

/// A store whose `get` fails a fixed number of times before delegating.
struct Flaky {
    inner: MemStore,
    failures_left: Mutex<u32>,
}

impl Flaky {
    fn failing(failures: u32) -> Self {
        Self {
            inner: MemStore::new(),
            failures_left: Mutex::new(failures),
        }
    }
}

impl ObjectStore for Flaky {
    fn get(&self, key: &StoreKey) -> Result<Option<Fetched>> {
        let mut left = self.failures_left.lock().expect("lock");
        if *left > 0 {
            *left -= 1;
            return Err(StoreError {
                op: "get",
                key: key.to_string(),
                source: io::Error::from(io::ErrorKind::TimedOut),
            });
        }
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> Result<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> Result<Create> {
        self.inner.put_create(key, bytes)
    }

    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> Result<Swap> {
        self.inner.put_swap(key, bytes, etag)
    }

    fn delete(&self, key: &StoreKey) -> Result<()> {
        self.inner.delete(key)
    }
}

#[test]
fn ambiguous_create_resolves_by_content_comparison() {
    let store = Flaky::failing(1);
    let key = StoreKey::of("log/c00000001/slot-b");

    assert_eq!(
        resolve_ambiguous_create(&store, &key, b"mine").expect("probe"),
        CreateProbe::Absent
    );

    store.put_create(&key, b"mine").expect("create");
    assert_eq!(
        resolve_ambiguous_create(&store, &key, b"mine").expect("probe"),
        CreateProbe::Landed(content_etag(b"mine"))
    );

    let CreateProbe::Lost(winner) =
        resolve_ambiguous_create(&store, &key, b"theirs").expect("probe")
    else {
        panic!("foreign bytes must resolve as a lost slot");
    };
    assert_eq!(winner.bytes, b"mine");
}

#[test]
fn ambiguous_swap_resolves_by_etag_reread() {
    let store = Flaky::failing(1);
    let key = StoreKey::of("manifest.json");

    assert_eq!(
        resolve_ambiguous_swap(&store, &key, b"v2").expect("probe"),
        SwapProbe::Absent
    );

    let Create::Created(birth) = store.put_create(&key, b"v1").expect("create") else {
        panic!("fresh key must be Created");
    };
    store.put_swap(&key, b"v2", &birth).expect("swap");

    assert_eq!(
        resolve_ambiguous_swap(&store, &key, b"v2").expect("probe"),
        SwapProbe::Landed(content_etag(b"v2"))
    );

    let SwapProbe::Lost(current) = resolve_ambiguous_swap(&store, &key, b"v9").expect("probe")
    else {
        panic!("unmatched bytes must resolve as lost");
    };
    assert_eq!(current.bytes, b"v2");
    assert_eq!(current.etag, content_etag(b"v2"));
}
