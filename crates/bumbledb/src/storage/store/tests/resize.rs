//! G06 elastic map: growth under load with automatic candidate replay,
//! reader-blocked resize as a typed refusal, ceiling refusal, and map-full
//! before/after seal. Deliberately tiny initial maps force many growth
//! events; bounds come from LMDB/the platform, never a product ceiling.

use super::*;

fn big_note(id: u64, bytes: usize) -> Vec<Value> {
    note(id, &"x".repeat(bytes))
}

#[test]
fn the_candidate_path_grows_the_map_and_reapplies_the_same_delta() {
    let (_dir, path) = store_dir("resize-grow-replay");
    let store = Store::create(&path, &schema(), tiny_map()).expect("create tiny store");
    let initial = store.current_map_bytes();
    // Well beyond the 1 MiB initial map: prepare must hit MDB_MAP_FULL,
    // abort, grow geometrically, and reapply the same owned delta.
    for id in 0..8u64 {
        let commit = commit_changes(
            &store,
            &change_set(&schema(), &[(NOTE, big_note(id, 512 * 1024))], &[]),
        );
        assert!(commit.changed);
    }
    assert!(store.current_map_bytes() > initial, "the map actually grew");
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 8);
    // Exactness after growth: every row is present under exact lookup.
    for id in 0..8u64 {
        assert!(
            snapshot
                .contains(
                    NOTE,
                    crate::canonical::CanonicalRow::encode(
                        schema().relation(NOTE).fields(),
                        &big_note(id, 512 * 1024),
                        &work(),
                    )
                    .expect("row")
                    .as_bytes(),
                    &work()
                )
                .expect("contains")
        );
    }
}

#[test]
fn growth_survives_reopen_and_the_populated_file_is_reported_distinctly() {
    let (_dir, path) = store_dir("resize-reopen");
    {
        let store = Store::create(&path, &schema(), tiny_map()).expect("create");
        for id in 0..4u64 {
            commit_changes(
                &store,
                &change_set(&schema(), &[(NOTE, big_note(id, 512 * 1024))], &[]),
            );
        }
    }
    // Reopen with the same tiny policy: the open extent derives from the
    // actual populated file plus headroom, not the tiny initial constant.
    let store = Store::open(&path, &schema(), tiny_map()).expect("reopen grown store");
    let report = store.map_report(&work()).expect("map report");
    assert!(report.virtual_map_bytes >= report.populated_file_bytes);
    assert!(report.populated_file_bytes > 1 << 20);
    assert!(report.non_free_page_bytes > 0);
    assert!(report.page_size > 0);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 4);
    // More growth after reopen still works.
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, big_note(100, 2 * 1024 * 1024))], &[]),
    );
}

#[test]
fn a_pinned_reader_blocks_growth_as_a_typed_refusal_with_age() {
    let (_dir, path) = store_dir("resize-blocked");
    let store = Store::create(&path, &schema(), tiny_map()).expect("create");
    let pinned = store.snapshot(&work()).expect("pinned reader");
    std::thread::sleep(std::time::Duration::from_millis(5));
    match store.grow(&short_work(std::time::Duration::from_millis(50)), None) {
        Err(StoreError::ResizeBlockedByReaders {
            live_transactions,
            oldest_age,
        }) => {
            assert_eq!(live_transactions, 1);
            let age = oldest_age.expect("oldest age reported");
            assert!(age >= std::time::Duration::from_millis(5));
        }
        other => panic!("expected ResizeBlockedByReaders, got {other:?}"),
    }
    // The refusal restored admission: the pinned snapshot still reads and
    // new snapshots are admitted.
    assert_eq!(pinned.row_count(NOTE).expect("pinned reads"), 0);
    drop(store.snapshot(&work()).expect("admission restored"));
    // Releasing the reader unblocks growth.
    drop(pinned);
    let report = store.grow(&work(), None).expect("grow after release");
    assert!(report.new_map_bytes > report.old_map_bytes);
}

#[test]
fn a_growth_ceiling_is_a_typed_exhaustion_never_a_wrap() {
    let (_dir, path) = store_dir("resize-ceiling");
    let ceiling = MapPolicy {
        initial_map_bytes: 1 << 20,
        max_map_bytes: Some(2 << 20),
    };
    let store = Store::create(&path, &schema(), ceiling).expect("create");
    // First growth reaches the ceiling.
    let report = store.grow(&work(), None).expect("grow to ceiling");
    assert_eq!(report.new_map_bytes, 2 << 20);
    // Beyond the ceiling: typed refusal with the exact extents.
    match store.grow(&work(), None) {
        Err(StoreError::MapGrowthExhausted { map_bytes, .. }) => {
            assert_eq!(map_bytes, 2 << 20);
        }
        other => panic!("expected MapGrowthExhausted, got {other:?}"),
    }
    // The candidate path surfaces the same typed refusal instead of looping
    // when the data cannot fit under the ceiling.
    match try_commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, big_note(1, 4 * 1024 * 1024))], &[]),
    ) {
        Err(StoreError::MapGrowthExhausted { .. }) => {}
        other => panic!("expected MapGrowthExhausted from the candidate path, got {other:?}"),
    }
    // The store remains usable for data that fits.
    let commit = commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(2, "small"))], &[]),
    );
    assert!(commit.changed);
}

#[test]
fn map_full_during_seal_dispatches_nothing_and_growth_recovers() {
    // "Map-full before seal completes": a host record larger than the whole
    // map fails the seal; the private transaction (facts + host prefix)
    // drops, nothing commits, and after growth the same immutable inputs
    // replay successfully.
    let (_dir, path) = store_dir("resize-seal-mapfull");
    let store = Store::create(&path, &schema(), tiny_map()).expect("create");
    let before = store.committed_generation(&work()).expect("generation");
    let huge = vec![0xABu8; 3 * 1024 * 1024];
    let records = host_put(b"receipt/huge", &huge);
    let changes = change_set(&schema(), &[(NOTE, note(1, "payload"))], &[]);
    let seal_error = {
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        let prepared = match owner
            .prepare(&changes, &FirstFieldKey, &AdmitAll)
            .expect("prepare")
        {
            Prepared::Admitted(prepared) => prepared,
            Prepared::Rejected(never) => match never {},
        };
        prepared
            .seal(HostChanges {
                records: &records,
                attachment: AttachmentChange::Keep,
            })
            .err()
            .expect("the oversized host record must fail the tiny map")
    };
    match seal_error {
        StoreError::MapFull { .. } | StoreError::Lmdb(_) => {}
        other => panic!("expected a map-capacity seal failure, got {other:?}"),
    }
    // Nothing was dispatched or committed.
    assert_eq!(
        store.committed_generation(&work()).expect("generation"),
        before
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 0);
    assert_eq!(snapshot.host_record(b"receipt/huge").expect("record"), None);
    drop(snapshot);
    // The log-shaped caller grows and replays the same immutable inputs.
    while store.current_map_bytes() < 16 << 20 {
        store.grow(&work(), None).expect("grow");
    }
    let context = work();
    let mut owner = store.writer(&context).expect("writer after growth");
    let prepared = match owner
        .prepare(&changes, &FirstFieldKey, &AdmitAll)
        .expect("prepare replay")
    {
        Prepared::Admitted(prepared) => prepared,
        Prepared::Rejected(never) => match never {},
    };
    let commit = prepared
        .seal(HostChanges {
            records: &records,
            attachment: AttachmentChange::Keep,
        })
        .expect("seal after growth")
        .commit()
        .expect("commit after growth");
    assert!(commit.changed);
    drop(owner);
    let snapshot = store.snapshot(&work()).expect("snapshot after replay");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert_eq!(
        snapshot
            .host_record(b"receipt/huge")
            .expect("record")
            .map(<[u8]>::len),
        Some(huge.len())
    );
}

#[test]
fn cancellation_while_waiting_for_exclusive_access_is_cancelled_not_blocked() {
    let (_dir, path) = store_dir("resize-cancel");
    let store = Store::create(&path, &schema(), tiny_map()).expect("create");
    let _pinned = store.snapshot(&work()).expect("pinned");
    let context = work();
    context.cancel();
    match store.grow(&context, None) {
        Err(StoreError::Work(crate::work::WorkError::Cancelled)) => {}
        other => panic!("expected Cancelled, got {other:?}"),
    }
}

#[test]
fn map_report_quantities_are_distinct_and_ordered_sanely() {
    let (_dir, path) = store_dir("resize-report");
    let store = Store::create(&path, &schema(), MapPolicy::default()).expect("create");
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "row"))], &[]),
    );
    let report = store.map_report(&work()).expect("report");
    // Virtual reservation is the default 4 GiB; the populated file is tiny.
    assert!(report.virtual_map_bytes >= (4u64 << 30));
    assert!(report.populated_file_bytes < 1 << 24);
    assert!(report.non_free_page_bytes <= report.populated_file_bytes);
    #[cfg(unix)]
    {
        let allocated = report.allocated_disk_bytes.expect("posix block accounting");
        // Allocated blocks never exceed the virtual reservation and are not
        // conflated with it: a sparse 4 GiB map allocates ~nothing.
        assert!(allocated < report.virtual_map_bytes / 4);
    }
    assert_eq!(report.live_transactions, 0);
}
