//! Create, open, format and schema refusal, the directory lock, durability
//! flags, close, row-id exhaustion and the single writer.

use super::*;
use crate::storage::store::format::{FORMAT, K_FORMAT, K_NEXT_ROW_ID};
use crate::storage::store::store_env::{CloseReport, Durability};

#[test]
fn create_then_reopen_round_trips_identity_and_rows() {
    let (_dir, path) = store_dir("store-create-reopen");
    let database = DatabaseId::mint();
    {
        let store = Store::create(&path, &schema(), database, Options::default()).expect("create");
        let commit = commit_changes(
            &store,
            &change_set(&schema(), &[(NOTE, note(1, "alpha"))], &[]),
        );
        assert!(commit.changed);
        assert_eq!(store.identity().database, database);
    }
    let store = open_default(&path);
    assert_eq!(store.identity().database, database);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert_eq!(snapshot.rows(NOTE).expect("cursor").count(), 1);
}

#[test]
fn environment_identity_differs_per_open() {
    let (_dir, path) = store_dir("store-env-identity");
    let first = create_default(&path).identity();
    let second = open_default(&path).identity();
    assert_eq!(first.database, second.database);
    assert_ne!(first.environment, second.environment);
}

#[test]
fn create_refuses_an_existing_destination() {
    let (_dir, path) = store_dir("store-create-exists");
    drop(create_default(&path));
    match Store::create(&path, &schema(), DatabaseId::mint(), Options::default()) {
        Err(Error::DestinationExists { path: reported }) => assert_eq!(reported, path),
        other => panic!("expected DestinationExists, got {other:?}"),
    }
}

#[test]
fn a_second_open_refuses_while_the_owner_lives_and_succeeds_after_drop() {
    let (_dir, path) = store_dir("store-lock");
    let owner = create_default(&path);
    assert!(matches!(
        Store::open(&path, &schema(), Options::default()),
        Err(Error::Locked { .. })
    ));
    drop(owner);
    drop(open_default(&path));
}

#[test]
fn duplicated_lock_description_does_not_outlive_the_final_environment_owner() {
    for retain_snapshot in [false, true] {
        let (_dir, path) = store_dir("store-inherited-lock-description");
        let owner = create_default(&path);
        // A duplicate of the locked description models a subprocess that
        // inherited the descriptor and has not reached exec yet.
        let inherited = owner.duplicate_lock_for_tests();
        let held = retain_snapshot.then(|| owner.snapshot(&work()).unwrap());
        drop(owner);
        if held.is_some() {
            assert!(matches!(
                Store::open(&path, &schema(), Options::default()),
                Err(Error::Locked { .. })
            ));
        }
        drop(held);
        let reopened = open_default(&path);
        drop(inherited);
        assert!(matches!(
            Store::open(&path, &schema(), Options::default()),
            Err(Error::Locked { .. })
        ));
        drop(reopened);
        drop(open_default(&path));
    }
}

#[test]
fn a_directory_without_this_format_refuses_and_is_left_untouched() {
    let (_dir, path) = store_dir("store-not-bumbledb");
    std::fs::create_dir_all(&path).expect("dir");
    std::fs::write(path.join("data.mdb"), b"not an lmdb file").expect("garbage");
    assert!(matches!(
        Store::open(&path, &schema(), Options::default()),
        Err(Error::NotABumbleDb { .. } | Error::Lmdb(_))
    ));
    assert_eq!(
        std::fs::read(path.join("data.mdb")).expect("still there"),
        b"not an lmdb file"
    );
}

#[test]
fn any_other_format_entry_refuses() {
    for format in [
        &FORMAT[..11],
        b"bumbledb\0\0\0\x02".as_slice(),
        b"".as_slice(),
    ] {
        let (_dir, path) = store_dir("store-format");
        {
            let store = create_default(&path);
            store.put_meta_for_tests(K_FORMAT, format);
        }
        assert!(matches!(
            Store::open(&path, &schema(), Options::default()),
            Err(Error::NotABumbleDb { .. })
        ));
    }
}

#[test]
fn a_foreign_schema_refuses_to_open() {
    let (_dir, path) = store_dir("store-schema-mismatch");
    drop(create_default(&path));
    assert!(matches!(
        Store::open(&path, &other_schema(), Options::default()),
        Err(Error::SchemaMismatch)
    ));
    drop(open_default(&path));
}

#[test]
fn durability_selects_exactly_the_no_sync_flag() {
    let weakening = heed::EnvFlags::NO_SYNC.bits()
        | heed::EnvFlags::MAP_ASYNC.bits()
        | heed::EnvFlags::NO_META_SYNC.bits();
    let (_dir, path) = store_dir("store-durability");
    let durable = create_default(&path);
    assert_eq!(durable.flags_for_tests() & weakening, 0);
    drop(durable);
    let cache = Store::open(
        &path,
        &schema(),
        Options {
            durability: Durability::Cache,
            ..Options::default()
        },
    )
    .expect("cache open");
    assert_eq!(
        cache.flags_for_tests() & weakening,
        heed::EnvFlags::NO_SYNC.bits()
    );
}

#[test]
fn close_reports_live_snapshots_and_refuses_new_admission() {
    let (_dir, path) = store_dir("store-close");
    let store = create_default(&path);
    let pinned = store.snapshot(&work()).expect("pinned snapshot");
    let stopped = work();
    stopped.cancel();
    assert!(matches!(
        store.close(&stopped),
        CloseReport::Incomplete {
            live_transactions: 1,
            ..
        }
    ));
    assert!(matches!(store.snapshot(&work()), Err(Error::Closed)));
    assert_eq!(pinned.row_count(NOTE).expect("still readable"), 0);
    drop(pinned);
    assert_eq!(store.close(&work()), CloseReport::Closed);
}

#[test]
fn the_lock_releases_after_the_owner_and_all_snapshots_drop() {
    let (_dir, path) = store_dir("store-lock-release-order");
    let snapshot = {
        let store = create_default(&path);
        store.snapshot(&work()).expect("snapshot")
    };
    assert!(matches!(
        Store::open(&path, &schema(), Options::default()),
        Err(Error::Locked { .. })
    ));
    drop(snapshot);
    drop(open_default(&path));
}

#[test]
fn row_id_exhaustion_aborts_the_batch_without_advancing_the_high_water_mark() {
    let (_dir, path) = store_dir("store-rowid-exhaustion");
    let store = create_default(&path);
    store.put_meta_for_tests(K_NEXT_ROW_ID, &(u64::MAX - 1).to_be_bytes());
    let two = change_set(
        &schema(),
        &[(NOTE, note(1, "first")), (NOTE, note(2, "overflow"))],
        &[],
    );
    assert!(matches!(
        try_commit_changes(&store, &two),
        Err(Error::Exhausted(crate::error::Counter::RowIds))
    ));
    assert_eq!(store.snapshot(&work()).unwrap().row_count(NOTE).unwrap(), 0);
    let one = change_set(&schema(), &[(NOTE, note(1, "first"))], &[]);
    commit_changes(&store, &one);
    assert!(
        !try_commit_changes(&store, &one).unwrap().changed,
        "an idempotent insertion allocates nothing"
    );
}

#[test]
fn the_writer_is_exclusive_and_reentrancy_refuses() {
    let (_dir, path) = store_dir("store-writer-reentrancy");
    let store = create_default(&path);
    let context = work();
    let owner = store.writer(&context).expect("first writer");
    assert!(matches!(
        store.writer(&context),
        Err(Error::ReentrantWriter)
    ));
    let stopped = work();
    stopped.cancel();
    std::thread::scope(|scope| {
        let waiting = scope.spawn(|| store.writer(&stopped).map(drop));
        assert!(matches!(waiting.join().unwrap(), Err(Error::Cancelled)));
    });
    drop(owner);
    drop(store.writer(&context).expect("writer after release"));
}

#[test]
fn install_populated_leaves_no_destination_on_population_failure() {
    let (_dir, path) = store_dir("store-install-populated");
    let error = Store::install_populated(
        &path,
        &schema(),
        DatabaseId::mint(),
        Options::default(),
        |_| Err(Error::ReentrantWriter),
    )
    .expect_err("population failure");
    assert!(matches!(error, Error::ReentrantWriter));
    assert!(!path.exists());
    let parent = path.parent().expect("parent");
    assert_eq!(
        std::fs::read_dir(parent).expect("parent listing").count(),
        0,
        "the staging sibling is removed"
    );
}

#[test]
fn install_populated_publishes_a_complete_store() {
    let (_dir, path) = store_dir("store-install-complete");
    let changes = change_set(&schema(), &[(NOTE, note(1, "published"))], &[]);
    let store = Store::install_populated(
        &path,
        &schema(),
        DatabaseId::mint(),
        Options::default(),
        |store| {
            commit_changes(store, &changes)
                .changed
                .then_some(())
                .ok_or(Error::Closed)
        },
    )
    .expect("installed");
    assert_eq!(
        store
            .snapshot(&work())
            .expect("snapshot")
            .row_count(NOTE)
            .expect("count"),
        1
    );
}
