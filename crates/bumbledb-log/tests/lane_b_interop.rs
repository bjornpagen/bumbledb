//! The preserved mixed-fleet CAS regression, on the ONE authoritative
//! implementation (C07; frozen-handoff CAS failure record).
//!
//! The 0.x failure: TS used numeric tokens/`~head`/unconditional rename while
//! Rust used `mutation.lock` — a paused TS old-value read, a Rust CAS, and a
//! resumed TS CAS acknowledged BOTH updates (counter 31 after 32 acks). The
//! successor has no second authority: TS delegates through native, and this
//! lane proves the exact failure shapes cannot recur against `FsStore` —
//! every acknowledged swap is linearized, a paused holder's stale version
//! loses, and hostile mutation-lock shapes refuse or are inert.
//! Verification: `NotRun` (F1 authors, does not execute).

#![cfg(unix)]

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{ConditionalOutcome, ConditionalStore as _, HeadRead};

const CONTENDERS: usize = 4;
const SWAPS_PER_CONTENDER: u64 = 8;

fn fresh_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "bdb-log-interop-{}-{name}-{nanos}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create test root");
    root
}

fn counter_of(body: &[u8]) -> u64 {
    u64::from_be_bytes(body[..8].try_into().expect("counter width"))
}

#[test]
fn mixed_fleet_acknowledged_swaps_equal_the_final_counter_exactly() {
    // The historical regression: 32 acknowledged swaps must leave the counter
    // at exactly 32 — an acknowledgment that did not linearize is the bug.
    let root = fresh_root("fleet");
    let store = FsStore::new(&root);
    match store
        .create_head("t/HEAD", &0u64.to_be_bytes())
        .expect("create")
    {
        ConditionalOutcome::Published { .. } => {}
        other => panic!("{other:?}"),
    }
    let total_acked = thread::scope(|scope| {
        let mut handles = Vec::new();
        for contender in 0..CONTENDERS {
            let store = FsStore::new(&root);
            handles.push(scope.spawn(move || {
                let mut acked = 0u64;
                let mut spins = 0u64;
                while acked < SWAPS_PER_CONTENDER {
                    spins += 1;
                    assert!(spins < 100_000, "contender {contender} live-locked");
                    let (version, body) = match store.read_head("t/HEAD").expect("read") {
                        HeadRead::Present { version, body } => (version, body),
                        HeadRead::Absent => panic!("head exists"),
                    };
                    let next = (counter_of(&body) + 1).to_be_bytes();
                    match store.replace_head("t/HEAD", &version, &next).expect("swap") {
                        ConditionalOutcome::Published { .. } => acked += 1,
                        ConditionalOutcome::PreconditionFailed => {}
                        ConditionalOutcome::Indeterminate => {
                            panic!("no injected ambiguity in this schedule")
                        }
                    }
                }
                acked
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("contender"))
            .sum::<u64>()
    });
    assert_eq!(total_acked, (CONTENDERS as u64) * SWAPS_PER_CONTENDER);
    match store.read_head("t/HEAD").expect("final read") {
        HeadRead::Present { body, .. } => assert_eq!(
            counter_of(&body),
            total_acked,
            "every acknowledged swap is exactly one linearized increment"
        ),
        HeadRead::Absent => panic!("head exists"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_paused_holder_with_a_stale_version_loses_after_another_writer_publishes() {
    // The deterministic 0.x double-acknowledgment schedule: reader captures
    // the old version, pauses; another writer CASes; the resumed CAS with the
    // captured version must LOSE (the 0.x fleet acknowledged both).
    let root = fresh_root("paused");
    let store = FsStore::new(&root);
    let v1 = match store.create_head("t/HEAD", b"value-1").expect("create") {
        ConditionalOutcome::Published { version } => version,
        other => panic!("{other:?}"),
    };
    // "Pause": the first writer holds only its captured version value across
    // the other writer's publication — exactly the old TS token semantics.
    let (swapped_tx, swapped_rx) = mpsc::channel::<()>();
    let resumed = thread::scope(|scope| {
        let store_b = FsStore::new(&root);
        let v1_clone = v1.clone();
        let other = scope.spawn(move || {
            match store_b
                .replace_head("t/HEAD", &v1_clone, b"value-2-from-b")
                .expect("b swaps")
            {
                ConditionalOutcome::Published { .. } => {}
                other => panic!("b publishes first: {other:?}"),
            }
            swapped_tx.send(()).expect("signal");
        });
        swapped_rx.recv().expect("b published");
        let outcome = store
            .replace_head("t/HEAD", &v1, b"value-2-from-a")
            .expect("a resumes");
        other.join().expect("b thread");
        outcome
    });
    assert_eq!(
        resumed,
        ConditionalOutcome::PreconditionFailed,
        "the resumed stale CAS is a definite loss, never a second acknowledgment"
    );
    match store.read_head("t/HEAD").expect("read") {
        HeadRead::Present { body, .. } => assert_eq!(&*body, b"value-2-from-b"),
        HeadRead::Absent => panic!("head exists"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn hostile_mutation_lock_shapes_refuse_or_are_inert_for_every_caller() {
    // The 0.x regression accepted a poisoned/symlinked mutation lock in one
    // driver only. The successor's single implementation: garbage lock BODIES
    // are inert (the kernel lock is the authority), symlinked lock PATHS
    // refuse outright, and a symlinked object path refuses without following.
    use std::os::unix::fs::symlink;
    let root = fresh_root("hostile");
    let store = FsStore::new(&root);
    // Poisoned body: inert.
    std::fs::create_dir_all(root.join("~lease/t/HEAD")).expect("lease dir");
    std::fs::write(
        root.join("~lease/t/HEAD/mutation.lock"),
        b"expiry=1970-01-01 token=999999",
    )
    .expect("poison");
    assert!(matches!(
        store.create_head("t/HEAD", b"v1").expect("create"),
        ConditionalOutcome::Published { .. }
    ));
    // Symlinked lock: refuse, and the target is untouched.
    let sentinel = root.join("sentinel.bin");
    std::fs::write(&sentinel, b"sentinel").expect("sentinel");
    std::fs::create_dir_all(root.join("~lease/u/HEAD")).expect("lease dir");
    symlink(&sentinel, root.join("~lease/u/HEAD/mutation.lock")).expect("symlink");
    assert!(store.create_head("u/HEAD", b"v1").is_err());
    assert_eq!(std::fs::read(&sentinel).expect("sentinel"), b"sentinel");
    // Symlinked object path: refused for read and write, never followed.
    symlink(&sentinel, root.join("redirect")).expect("symlink");
    assert!(store.get_object("redirect").is_err());
    assert!(store.put_object("redirect", b"x").is_err());
    assert_eq!(std::fs::read(&sentinel).expect("sentinel"), b"sentinel");
    let _ = std::fs::remove_dir_all(&root);
}
