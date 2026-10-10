//! The fixed virtual map: a write past the ceiling is the typed `Full`
//! refusal with nothing committed, and the store stays usable.

use super::*;

fn small_store(path: &std::path::Path) -> Store {
    Store::create(path, &schema(), DatabaseId::mint(), small_ceiling()).expect("create")
}

#[test]
fn a_candidate_past_the_ceiling_is_full_and_commits_nothing() {
    let (_dir, path) = store_dir("ceiling-candidate");
    let store = small_store(&path);
    let before = store.committed_generation(&work()).expect("generation");
    let huge = note(1, &"x".repeat(2 << 20));
    match try_commit_changes(&store, &change_set(&schema(), &[(NOTE, huge)], &[])) {
        Err(Error::Full { ceiling }) => assert_eq!(ceiling, SMALL_CEILING),
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
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
}

#[test]
fn a_host_record_past_the_ceiling_drops_the_whole_sealed_candidate() {
    let (_dir, path) = store_dir("ceiling-seal");
    let store = small_store(&path);
    let before = store.committed_generation(&work()).expect("generation");
    let huge = vec![0xABu8; 2 << 20];
    let records = host_put(b"receipt/huge", &huge);
    let changes = change_set(&schema(), &[(NOTE, note(1, "payload"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let prepared = owner
        .prepare_decided(std::slice::from_ref(&changes))
        .expect("prepare");
    assert_full(prepared.seal(HostChanges {
        records: &records,
        head: super::super::host::Head::Keep,
    }));
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
    let store = small_store(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "kept"))], &[]),
    );
    drop(store);
    let reopened = open_default(&path);
    assert_eq!(
        reopened.options().map_ceiling,
        Options::default().map_ceiling
    );
    assert!(reopened.file_bytes().expect("file bytes") < SMALL_CEILING);
    let snapshot = reopened.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
}
