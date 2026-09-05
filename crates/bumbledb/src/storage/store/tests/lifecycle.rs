//! G06 lifecycle: create/open/lock/close, family/layout/schema refusal
//! before any cleanup, and the structural absence of a `NO_SYNC` lane
//! (ENG-008 / E-DURABILITY).

use super::*;
use crate::storage::store::store_env::CloseReport;

#[test]
fn create_then_reopen_round_trips_identity_and_rows() {
    let (_dir, path) = store_dir("store-create-reopen");
    let store_id = {
        let store = create_default(&path);
        let commit = commit_changes(
            &store,
            &change_set(&schema(), &[(NOTE, note(1, "alpha"))], &[]),
        );
        assert!(commit.changed);
        assert_eq!(commit.application.added, 1);
        store.store_id()
    };
    let store = open_default(&path);
    // Persistent identity survives reopen; environment identity is per-open.
    assert_eq!(store.store_id(), store_id);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    let rows: Vec<_> = snapshot
        .rows(NOTE)
        .expect("cursor")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(rows.len(), 1);
}

#[test]
fn environment_identity_differs_per_open() {
    let (_dir, path) = store_dir("store-env-identity");
    let first = create_default(&path).environment_id();
    let second = open_default(&path).environment_id();
    assert_ne!(first, second);
    assert_ne!(first.value(), second.value());
}

#[test]
fn create_refuses_an_existing_destination() {
    let (_dir, path) = store_dir("store-create-exists");
    drop(create_default(&path));
    match Store::create(&path, &schema(), MapPolicy::default()) {
        Err(StoreError::DestinationExists { path: reported }) => assert_eq!(reported, path),
        other => panic!("expected DestinationExists, got {other:?}"),
    }
}

#[test]
fn a_second_open_refuses_while_the_owner_lives_and_succeeds_after_drop() {
    let (_dir, path) = store_dir("store-lock");
    let owner = create_default(&path);
    match Store::open(&path, &schema(), MapPolicy::default()) {
        Err(StoreError::StoreLocked { .. }) => {}
        other => panic!("expected StoreLocked, got {other:?}"),
    }
    drop(owner);
    drop(open_default(&path)); // lock released with the owner
}

/// Build a directory shaped like the deleted transitional format-8 store:
/// an LMDB environment whose named databases are `_meta`/`_data`/`_dict`
/// and whose meta carries no successor family key. The transitional engine
/// is deleted (this demolition's whole point), so the fixture reproduces
/// its on-disk shape directly — the safety counterexample survives the
/// mechanism (chapter 50 G00 rule).
#[expect(
    unsafe_code,
    reason = "heed marks environment opening unsafe; the fixture directory \
              is private to this test and opened once"
)]
fn build_transitional_shaped_store(path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("fixture dir");
    let mut options = heed::EnvOpenOptions::new().read_txn_without_tls();
    options.map_size(16 << 20).max_dbs(3);
    // SAFETY: single open of a test-private directory.
    let env = unsafe { options.open(path) }.expect("fixture env");
    let mut wtxn = env.write_txn().expect("fixture txn");
    let meta: heed::Database<heed::types::Bytes, heed::types::Bytes> = env
        .create_database(&mut wtxn, Some("_meta"))
        .expect("fixture _meta");
    let _data: heed::Database<heed::types::Bytes, heed::types::Bytes> = env
        .create_database(&mut wtxn, Some("_data"))
        .expect("fixture _data");
    let _dict: heed::Database<heed::types::Bytes, heed::types::Bytes> = env
        .create_database(&mut wtxn, Some("_dict"))
        .expect("fixture _dict");
    // The transitional meta spoke single-byte keys with a u32 LE format
    // version; none of them is the successor family record.
    meta.put(&mut wtxn, &[0u8], 8u32.to_le_bytes().as_slice())
        .expect("fixture format version");
    wtxn.commit().expect("fixture commit");
}

#[test]
fn an_old_family_transitional_store_refuses_before_any_cleanup() {
    let (_dir, path) = store_dir("store-old-family");
    build_transitional_shaped_store(&path);
    let mut before: Vec<_> = std::fs::read_dir(&path)
        .expect("fixture listing")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    before.sort();
    let data_len = std::fs::metadata(path.join("data.mdb"))
        .expect("fixture data")
        .len();
    match Store::open(&path, &schema(), MapPolicy::default()) {
        Err(StoreError::UnrecognizedStore { path: reported }) => assert_eq!(reported, path),
        other => panic!("expected UnrecognizedStore, got {other:?}"),
    }
    // Refusal performed zero cleanup or adoption: same files, same bytes.
    let mut after: Vec<_> = std::fs::read_dir(&path)
        .expect("fixture listing after")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    after.sort();
    // The refused open adds at most its own kernel lock file (created
    // before verification, content-free); every transitional byte stays.
    after.retain(|name| name != "bumbledb.lock");
    before.retain(|name| name != "bumbledb.lock");
    assert_eq!(before, after);
    assert_eq!(
        std::fs::metadata(path.join("data.mdb"))
            .expect("fixture data")
            .len(),
        data_len
    );
}

// The reverse direction ("the transitional reader refuses successor
// files") is retired WITH the transitional reader: no old engine remains
// in the tree to misread a successor store. The distinct database names
// (`_core_meta`/`_core_data` vs `_meta`/`_data`/`_dict`) and the family
// record keep any out-of-tree 0.x binary refusing as before.

#[test]
fn a_layout_bump_refuses_with_both_counters() {
    let (_dir, path) = store_dir("store-layout");
    {
        let store = create_default(&path);
        store.force_layout_for_tests(super::super::format::LAYOUT + 1);
    }
    match Store::open(&path, &schema(), MapPolicy::default()) {
        Err(StoreError::LayoutMismatch { found, expected }) => {
            assert_eq!(found, super::super::format::LAYOUT + 1);
            assert_eq!(expected, super::super::format::LAYOUT);
        }
        other => panic!("expected LayoutMismatch, got {other:?}"),
    }
}

#[test]
fn recognizing_the_layout_integer_alone_is_forbidden() {
    // A corrupted family with an intact layout counter must refuse as
    // unrecognized: integer 1 alone never admits bytes (C12).
    let (_dir, path) = store_dir("store-family-corrupt");
    {
        let store = create_default(&path);
        store.corrupt_family_for_tests();
    }
    match Store::open(&path, &schema(), MapPolicy::default()) {
        Err(StoreError::UnrecognizedStore { .. }) => {}
        other => panic!("expected UnrecognizedStore, got {other:?}"),
    }
}

#[test]
fn a_foreign_schema_refuses_to_open() {
    let (_dir, path) = store_dir("store-schema-mismatch");
    drop(create_default(&path));
    match Store::open(&path, &other_schema(), MapPolicy::default()) {
        Err(StoreError::SchemaMismatch) => {}
        other => panic!("expected SchemaMismatch, got {other:?}"),
    }
    drop(open_default(&path)); // the right schema still opens
}

#[test]
fn every_open_is_durable_no_nosync_flag_is_reachable() {
    // ENG-008: the open chokepoint has no lane/flag parameter; verify the
    // actual environment flags carry no NO_SYNC/MAPASYNC weakening.
    let (_dir, path) = store_dir("store-durable-flags");
    let store = create_default(&path);
    let flags = store.flags_for_tests();
    let no_sync = heed::EnvFlags::NO_SYNC.bits();
    let map_async = heed::EnvFlags::MAP_ASYNC.bits();
    let no_meta_sync = heed::EnvFlags::NO_META_SYNC.bits();
    assert_eq!(flags & (no_sync | map_async | no_meta_sync), 0);
}

#[test]
fn close_reports_live_snapshots_and_refuses_new_admission() {
    let (_dir, path) = store_dir("store-close");
    let store = create_default(&path);
    let pinned = store.snapshot(&work()).expect("pinned snapshot");
    match store.close(&short_work(std::time::Duration::from_millis(50))) {
        CloseReport::Incomplete {
            live_transactions, ..
        } => assert_eq!(live_transactions, 1),
        CloseReport::Closed => panic!("close cannot complete under a live snapshot"),
    }
    // Closing state refuses new admission but never invalidates the pinned
    // snapshot's live borrow.
    match store.snapshot(&work()) {
        Err(StoreError::Closed) => {}
        other => panic!("expected Closed, got {other:?}"),
    }
    assert_eq!(pinned.row_count(NOTE).expect("still readable"), 0);
    drop(pinned);
    assert_eq!(store.close(&work()), CloseReport::Closed);
}

#[test]
fn the_lock_releases_after_the_owner_and_all_snapshots_drop() {
    let (_dir, path) = store_dir("store-lock-release-order");
    let store = open_snapshot_then_drop_store(&path);
    // The snapshot transitively holds the inner store (and lock): a new
    // owner must refuse while it lives.
    match Store::open(&path, &schema(), MapPolicy::default()) {
        Err(StoreError::StoreLocked { .. }) => {}
        other => panic!("expected StoreLocked under a live snapshot, got {other:?}"),
    }
    drop(store);
    drop(open_default(&path));
}

fn open_snapshot_then_drop_store(path: &std::path::Path) -> super::super::OwnedSnapshot {
    let store = create_default(path);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    drop(store);
    snapshot
}

#[test]
fn row_id_exhaustion_is_a_typed_refusal() {
    let (_dir, path) = store_dir("store-rowid-exhaustion");
    let store = create_default(&path);
    store.force_next_row_id_for_tests(u64::MAX);
    match try_commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "last"))], &[]),
    ) {
        Err(StoreError::RowIdExhausted) => {}
        other => panic!("expected RowIdExhausted, got {other:?}"),
    }
}

#[test]
fn the_writer_is_exclusive_and_reentrancy_refuses() {
    let (_dir, path) = store_dir("store-writer-reentrancy");
    let store = create_default(&path);
    let context = work();
    let owner = store.writer(&context).expect("first writer");
    match store.writer(&context) {
        Err(StoreError::ReentrantWriter) => {}
        other => panic!("expected ReentrantWriter, got {other:?}"),
    }
    drop(owner);
    drop(store.writer(&context).expect("writer after release"));
}
