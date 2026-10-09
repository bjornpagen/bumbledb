//! The fixed virtual map: a write past the ceiling is the typed `Full`
//! refusal with nothing committed, and the store stays usable.

use super::*;

#[test]
fn a_candidate_past_the_ceiling_is_full_and_commits_nothing() {
    let (_dir, path) = store_dir("ceiling-candidate");
    let store = Store::create(&path, &schema(), SMALL_CEILING)
        .expect("create")
        .0;
    let before = store.committed_generation(&work()).expect("generation");
    let huge = note(1, &"x".repeat(2 << 20));
    let changes = change_set(&schema(), &[(NOTE, huge)], &[]);
    match try_commit_changes(&store, &changes) {
        Err(StoreError::Full { ceiling }) => assert_eq!(ceiling, SMALL_CEILING),
        other => panic!("expected Full, got {other:?}"),
    }
    assert_eq!(
        store.committed_generation(&work()).expect("generation"),
        before
    );
    let commit = commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(2, "fits"))], &[]),
    );
    assert!(commit.changed);
    assert_eq!(
        store
            .snapshot(&work())
            .expect("snapshot")
            .row_count(NOTE)
            .expect("count"),
        1
    );
}

#[test]
fn a_host_record_past_the_ceiling_drops_the_whole_sealed_candidate() {
    let (_dir, path) = store_dir("ceiling-seal");
    let store = Store::create(&path, &schema(), SMALL_CEILING)
        .expect("create")
        .0;
    let before = store.committed_generation(&work()).expect("generation");
    let huge = vec![0xABu8; 2 << 20];
    let records = host_put(b"receipt/huge", &huge);
    let changes = change_set(&schema(), &[(NOTE, note(1, "payload"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let prepared = match owner
        .prepare(&changes, &FirstFieldKey, &AdmitAll)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared,
        Prepared::Rejected {
            rejection: never, ..
        } => match never {},
    };
    let error = prepared
        .seal(HostChanges {
            records: &records,
            attachment: AttachmentChange::Keep,
        })
        .err()
        .expect("the oversized host record cannot fit");
    assert!(matches!(error, StoreError::Full { .. }), "{error:?}");
    drop(owner);
    assert_eq!(
        store.committed_generation(&work()).expect("generation"),
        before
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 0);
    assert_eq!(snapshot.host_record(b"receipt/huge").expect("record"), None);
}

#[test]
fn reopening_keeps_the_rows_under_a_different_ceiling() {
    let (_dir, path) = store_dir("ceiling-reopen");
    let store = Store::create(&path, &schema(), SMALL_CEILING)
        .expect("create")
        .0;
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "kept"))], &[]),
    );
    drop(store);
    let reopened = Store::open(&path, &schema(), DEFAULT_MAP_CEILING).expect("reopen");
    assert_eq!(reopened.ceiling(), DEFAULT_MAP_CEILING);
    assert!(reopened.file_bytes().expect("file bytes") < SMALL_CEILING);
    assert_eq!(
        reopened
            .snapshot(&work())
            .expect("snapshot")
            .row_count(NOTE)
            .expect("count"),
        1
    );
}
