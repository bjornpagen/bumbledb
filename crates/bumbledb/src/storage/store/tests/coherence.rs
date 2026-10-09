//! One snapshot is one transaction: rows, generation, host records and the
//! head agree; later commits are invisible; export and content digests are
//! functions of the content alone.

use super::*;
use crate::storage::store::host::Head;

fn digest(store: &Store) -> [u8; 32] {
    store
        .snapshot(&work())
        .expect("snapshot")
        .content_digest(&work())
        .expect("digest")
}

#[test]
fn a_pinned_snapshot_keeps_its_rows_generation_and_host_bytes() {
    let (_dir, path) = store_dir("coherence-pinned");
    let store = create_default(&path);
    let context = work();
    {
        let mut owner = store.writer(&context).expect("writer");
        let changes = change_set(&schema(), &[(NOTE, note(1, "first"))], &[]);
        let records = host_put(b"r/1", b"one");
        owner
            .prepare_decided(std::slice::from_ref(&changes))
            .expect("prepare")
            .seal(HostChanges {
                records: &records,
                head: Head::Put(b"h1"),
            })
            .expect("seal")
            .commit()
            .expect("commit");
    }
    let pinned = store.snapshot(&work()).expect("pinned");
    let generation = pinned.generation();
    {
        let mut owner = store.writer(&context).expect("writer");
        let changes = change_set(&schema(), &[(NOTE, note(2, "second"))], &[]);
        let records = host_put(b"r/2", b"two");
        owner
            .prepare_decided(std::slice::from_ref(&changes))
            .expect("prepare")
            .seal(HostChanges {
                records: &records,
                head: Head::Put(b"h2"),
            })
            .expect("seal")
            .commit()
            .expect("commit");
    }
    assert_eq!(pinned.generation(), generation);
    assert_eq!(pinned.row_count(NOTE).expect("count"), 1);
    assert_eq!(pinned.head().expect("head"), Some(b"h1".as_slice()));
    assert_eq!(pinned.host_record(b"r/2").expect("record"), None);
    let mut scanned = Vec::new();
    pinned
        .host_scan::<Error>(b"r/", &work(), &mut |key, value| {
            scanned.push((key.to_vec(), value.to_vec()));
            Ok(())
        })
        .expect("scan");
    assert_eq!(scanned, [(b"r/1".to_vec(), b"one".to_vec())]);
    let latest = store.snapshot(&work()).expect("latest");
    assert_eq!(latest.generation().value(), generation.value() + 1);
    assert_eq!(latest.head().expect("head"), Some(b"h2".as_slice()));
    assert_eq!(latest.row_count(NOTE).expect("count"), 2);
}

#[test]
fn the_generation_starts_at_zero_and_moves_only_on_change() {
    let (_dir, path) = store_dir("coherence-generation");
    let store = create_default(&path);
    assert_eq!(store.committed_generation(&work()).expect("g").value(), 0);
    let changes = change_set(&schema(), &[(NOTE, note(1, "x"))], &[]);
    assert_eq!(commit_changes(&store, &changes).generation.value(), 1);
    assert_eq!(commit_changes(&store, &changes).generation.value(), 1);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(
        snapshot.relation_version(NOTE).expect("version"),
        super::super::format::RelationVersion::from_storage(1)
    );
    assert_eq!(
        snapshot.relation_version(TAG).expect("version"),
        super::super::format::RelationVersion::initial(),
        "an untouched relation keeps its version"
    );
}

#[test]
fn deleted_text_leaves_no_byte_behind_after_reopen() {
    let (_dir, path) = store_dir("coherence-no-dictionary");
    let secret = "a-secret-body-that-must-vanish";
    {
        let store = create_default(&path);
        commit_changes(
            &store,
            &change_set(&schema(), &[(NOTE, note(1, secret))], &[]),
        );
        commit_changes(
            &store,
            &change_set(&schema(), &[], &[(NOTE, note(1, secret))]),
        );
    }
    let store = open_default(&path);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let txn = snapshot.read_txn();
    for db in [store.inner.meta, store.inner.rows, store.inner.dets] {
        for entry in db.iter(txn).expect("iter") {
            let (key, value) = entry.expect("entry");
            assert!(!key.windows(secret.len()).any(|w| w == secret.as_bytes()));
            assert!(!value.windows(secret.len()).any(|w| w == secret.as_bytes()));
        }
    }
}

#[test]
fn export_orders_relations_then_homes_then_bytes() {
    let (_dir, path) = store_dir("coherence-export-order");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(
            &schema(),
            &[
                (TAG, tag("zeta")),
                (NOTE, note(300, "c")),
                (NOTE, note(2, "a")),
                (TAG, tag("alpha")),
                (NOTE, note(17, "b")),
            ],
            &[],
        ),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let mut order = Vec::new();
    snapshot
        .export(&work(), &mut |relation, row| {
            order.push((relation, row.to_vec()));
            Ok(())
        })
        .expect("export");
    let notes: Vec<_> = order
        .iter()
        .filter(|(relation, _)| *relation == NOTE)
        .map(|(_, row)| row.clone())
        .collect();
    assert_eq!(
        notes,
        [note(2, "a"), note(17, "b"), note(300, "c")]
            .iter()
            .map(|values| row_bytes(&schema(), NOTE, values))
            .collect::<Vec<_>>(),
        "the id home orders notes by id"
    );
    assert!(order[..3].iter().all(|(relation, _)| *relation == NOTE));
    assert!(order[3..].iter().all(|(relation, _)| *relation == TAG));
}

#[test]
fn content_digests_ignore_history_and_see_every_row() {
    let (_dir, first_path) = store_dir("coherence-digest-a");
    let (_dir2, second_path) = store_dir("coherence-digest-b");
    let first = create_default(&first_path);
    let second = create_default(&second_path);
    let rows = [
        (NOTE, note(1, "one")),
        (NOTE, note(2, "two")),
        (TAG, tag("t")),
    ];
    commit_changes(&first, &change_set(&schema(), &rows, &[]));
    for row in rows.iter().rev() {
        commit_changes(
            &second,
            &change_set(&schema(), &[(NOTE, note(9, "transient"))], &[]),
        );
        commit_changes(
            &second,
            &change_set(&schema(), std::slice::from_ref(row), &[]),
        );
        commit_changes(
            &second,
            &change_set(&schema(), &[], &[(NOTE, note(9, "transient"))]),
        );
    }
    assert_eq!(digest(&first), digest(&second));
    commit_changes(&second, &change_set(&schema(), &[(TAG, tag("u"))], &[]));
    assert_ne!(digest(&first), digest(&second));
}
