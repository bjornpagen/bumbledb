use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bumbledb::work::{ExecutionPolicy, Resource, WorkContext, WorkError};

use super::{FsStore, FsWorkError};
use crate::store::fence::{WorkIoError, acquire_mutation, acquire_mutation_with};
use crate::store::{Create, ObjectStore, Poll, StoreKey, Swap};

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bdb-fs-work-{}-{label}-{stamp}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: 1 << 20,
        result_bytes: 1 << 20,
        rows: 100,
        work_units: 1 << 24,
        timeout: Duration::from_secs(20),
    }
}

fn empty_reservations(work: &WorkContext) {
    for resource in [
        Resource::WorkingBytes,
        Resource::ScratchBytes,
        Resource::ResultBytes,
    ] {
        assert_eq!(work.used(resource), 0, "unreleased {resource:?}");
    }
}

#[test]
fn get_reserves_inode_length_before_body_read_and_keeps_its_result_charge() {
    let root = Scratch::new("get");
    let store = FsStore::new(&root.0);
    let key = StoreKey::of("body");
    let body = vec![7; 65_536];
    store.put_create(&key, body.as_slice()).unwrap();
    let mut tiny = policy();
    tiny.result_bytes = 128;
    let work = tiny.start().unwrap();
    assert!(matches!(
        store.get_with(&key, &work),
        Err(FsWorkError::Work(WorkError::Exhausted {
            resource: Resource::ResultBytes,
            requested: 65_600,
            ..
        }))
    ));
    assert!(
        work.used(Resource::WorkUnits) < 10,
        "no body bytes read or hashed before size refusal"
    );
    empty_reservations(&work);

    let work = policy().start().unwrap();
    let fetched = store.get_with(&key, &work).unwrap();
    assert_eq!(fetched.value.as_ref().unwrap().bytes, body);
    assert_eq!(fetched.reservation.bytes(), 65_600);
    assert_eq!(work.used(Resource::ResultBytes), 65_600);
    assert_eq!(work.used(Resource::WorkingBytes), 0);
    drop(fetched);
    empty_reservations(&work);
}

#[test]
fn read_work_exhaustion_releases_partial_result() {
    let root = Scratch::new("read-budget");
    let store = FsStore::new(&root.0);
    let key = StoreKey::of("body");
    store.put_create(&key, vec![3; 65_536].as_slice()).unwrap();
    let mut limited = policy();
    limited.work_units = 5_000;
    let work = limited.start().unwrap();
    assert!(matches!(
        store.get_with(&key, &work),
        Err(FsWorkError::Work(WorkError::Exhausted {
            resource: Resource::WorkUnits,
            ..
        }))
    ));
    empty_reservations(&work);
}

#[test]
fn absent_unchanged_and_delete_results_have_zero_owned_bytes() {
    let root = Scratch::new("empty");
    let store = FsStore::new(&root.0);
    let key = StoreKey::of("body");
    let work = policy().start().unwrap();
    let absent = store.get_with(&key, &work).unwrap();
    assert!(absent.value.is_none());
    assert_eq!(absent.reservation.bytes(), 0);
    let Create::Created(tag) = store.put_create(&key, b"body").unwrap() else {
        panic!("birth")
    };
    let unchanged = store.get_if_changed_with(&key, &tag, &work).unwrap();
    assert_eq!(unchanged.value, Poll::Unchanged);
    assert_eq!(unchanged.reservation.bytes(), 0);
    let deleted = store.delete_with(&key, &work).unwrap();
    assert_eq!(deleted.reservation.bytes(), 0);
    empty_reservations(&work);
}

#[test]
fn staging_or_hash_refusal_cannot_publish_body_or_generation() {
    let root = Scratch::new("stage");
    let store = FsStore::new(&root.0);
    let key = StoreKey::of("body");
    let Create::Created(tag) = store.put_create(&key, b"before").unwrap() else {
        panic!("birth")
    };
    let generation = fs::read(store.generation_path(&key)).unwrap();
    let mut no_scratch = policy();
    no_scratch.scratch_bytes = 0;
    let work = no_scratch.start().unwrap();
    assert!(matches!(
        store.put_swap_with(&key, b"after", &tag, &work),
        Err(FsWorkError::Work(WorkError::Exhausted {
            resource: Resource::ScratchBytes,
            ..
        }))
    ));
    empty_reservations(&work);
    assert_eq!(store.get(&key).unwrap().unwrap().bytes, b"before");
    assert_eq!(fs::read(store.generation_path(&key)).unwrap(), generation);
    assert_eq!(fs::read_dir(root.0.join("~tmp")).unwrap().count(), 0);

    let mut little_work = policy();
    little_work.work_units = 100;
    let work = little_work.start().unwrap();
    assert!(matches!(
        store.put_create_with(&StoreKey::of("new"), vec![1; 65_536].as_slice(), &work),
        Err(FsWorkError::Work(WorkError::Exhausted {
            resource: Resource::WorkUnits,
            ..
        }))
    ));
    assert!(store.get(&StoreKey::of("new")).unwrap().is_none());
    empty_reservations(&work);
}

#[test]
fn cas_hashes_current_body_in_bounded_chunks_without_result_allocation() {
    let root = Scratch::new("compare");
    let store = FsStore::new(&root.0);
    let key = StoreKey::of("body");
    let original = vec![9; 65_536];
    let Create::Created(tag) = store.put_create(&key, original.as_slice()).unwrap() else {
        panic!("birth")
    };
    let mut limited = policy();
    limited.result_bytes = 64;
    limited.work_units = 5_000;
    let work = limited.start().unwrap();
    assert!(matches!(
        store.put_swap_with(&key, b"after", &tag, &work),
        Err(FsWorkError::Work(WorkError::Exhausted {
            resource: Resource::WorkUnits,
            ..
        }))
    ));
    empty_reservations(&work);
    assert_eq!(store.get(&key).unwrap().unwrap().bytes, original);

    let work = policy().start().unwrap();
    let output = store.put_swap_with(&key, b"after", &tag, &work).unwrap();
    assert!(matches!(output.value, Swap::Swapped(_)));
    assert_eq!(output.reservation.bytes(), 64);
    drop(output);
    empty_reservations(&work);
}

#[test]
fn cancellation_stops_a_registered_lock_wait_without_taking_ownership() {
    let root = Scratch::new("cancel");
    let key = StoreKey::of("body");
    let held = acquire_mutation(&root.0, &key).unwrap();
    let work = policy().start().unwrap();
    let contender_work = work.clone();
    let contender_root = root.0.clone();
    let (send, receive) = mpsc::channel();
    let contender = thread::spawn(move || {
        send.send(acquire_mutation_with(
            &contender_root,
            &key,
            &contender_work,
        ))
        .unwrap();
    });
    let watchdog = Instant::now();
    while work.used(Resource::WorkingBytes) == 0 {
        assert!(
            watchdog.elapsed() < Duration::from_secs(20),
            "contender never registered"
        );
        thread::yield_now();
    }
    work.cancel();
    assert!(matches!(
        receive.recv_timeout(Duration::from_secs(20)).unwrap(),
        Err(WorkIoError::Work(WorkError::Cancelled))
    ));
    contender.join().unwrap();
    empty_reservations(&work);
    drop(held);
    let fresh = policy().start().unwrap();
    assert!(acquire_mutation_with(&root.0, &StoreKey::of("body"), &fresh).is_ok());
}
