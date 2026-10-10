//! The private candidate: judgment over the proposed final state,
//! invisibility to readers, sessions surviving rejection, host-only seals,
//! failed seals dropping everything, and batch decisions.

use super::*;
use crate::error::HostKeyFault;
use crate::schema::judge::Judgment;
use crate::storage::store::Applied;
use crate::storage::store::host::Head;

fn note_row(id: u64, body: &str) -> Vec<u8> {
    row_bytes(&schema(), NOTE, &note(id, body))
}

#[test]
fn a_prepared_candidate_is_invisible_to_committed_readers() {
    let (_dir, path) = store_dir("cand-invisible");
    let store = create_default(&path);
    let pinned = store.snapshot(&work()).expect("pinned");
    let changes = change_set(&schema(), &[(NOTE, note(1, "spectral"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let prepared = owner
        .prepare_decided(std::slice::from_ref(&changes))
        .expect("prepare");
    let row = note_row(1, "spectral");
    assert!(!pinned.contains(NOTE, &row, &work()).expect("pinned probe"));
    let fresh = store.snapshot(&work()).expect("fresh during candidate");
    assert!(!fresh.contains(NOTE, &row, &work()).expect("fresh probe"));
    drop(fresh);
    prepared.abort();
    drop(owner);
    assert_eq!(
        store
            .snapshot(&work())
            .expect("after abort")
            .row_count(NOTE)
            .expect("count"),
        0
    );
    commit_changes(&store, &changes);
    assert!(
        !pinned
            .contains(NOTE, &row, &work())
            .expect("pinned after commit")
    );
    let latest = store.snapshot(&work()).expect("latest");
    assert!(latest.contains(NOTE, &row, &work()).expect("latest probe"));
}

#[test]
fn a_rejection_names_every_competitor_and_keeps_the_session_for_a_receipt() {
    let (_dir, path) = store_dir("cand-rejected-receipt");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "first"))], &[]),
    );
    let before = store.committed_generation(&work()).expect("generation");
    let conflicting = change_set(&schema(), &[(NOTE, note(1, "second"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let Candidate::Rejected(violations) = owner
        .prepare_judged(&schema(), &conflicting)
        .expect("prepare")
    else {
        panic!("two rows under one note id must reject");
    };
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].examples.len(), 2, "both competitors cited");
    let receipt = owner.prepare_unchanged().expect("receipt transaction");
    assert_eq!(receipt.applied(), Applied::default());
    let records = host_put(b"receipt/1", b"rejected");
    let commit = receipt
        .seal(HostChanges {
            records: &records,
            head: Head::Put(b"head"),
        })
        .expect("seal receipt")
        .commit()
        .expect("commit receipt");
    assert!(commit.changed);
    assert_eq!(commit.generation.value(), before.value() + 1);
    drop(owner);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert!(
        snapshot
            .contains(NOTE, &note_row(1, "first"), &work())
            .expect("original row intact")
    );
    assert_eq!(
        snapshot.host_record(b"receipt/1").expect("receipt"),
        Some(b"rejected".as_slice())
    );
    assert_eq!(snapshot.head().expect("head"), Some(b"head".as_slice()));
}

#[test]
fn a_replacement_under_one_key_judges_the_final_state() {
    let (_dir, path) = store_dir("cand-replacement");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "old"))], &[]),
    );
    let replacement = change_set(
        &schema(),
        &[(NOTE, note(1, "new"))],
        &[(NOTE, note(1, "old"))],
    );
    judged_commit(&store, &schema(), &replacement).expect("legal in the final state");
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert!(
        snapshot
            .contains(NOTE, &note_row(1, "new"), &work())
            .expect("new row")
    );
}

#[test]
fn a_failed_seal_drops_facts_and_the_host_prefix() {
    let (_dir, path) = store_dir("cand-failed-seal");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "base"))], &[]),
    );
    let before = store.committed_generation(&work()).expect("generation");
    store.fail_host_seal_after(Some(1));
    let changes = change_set(&schema(), &[(NOTE, note(2, "doomed"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let prepared = owner
        .prepare_decided(std::slice::from_ref(&changes))
        .expect("prepare");
    let records = [
        HostRecord::Put {
            key: b"receipt/a",
            value: b"prefix-written",
        },
        HostRecord::Put {
            key: b"receipt/b",
            value: b"never-reached",
        },
    ];
    assert_full(prepared.seal(HostChanges {
        records: &records,
        head: Head::Keep,
    }));
    store.fail_host_seal_after(None);
    drop(owner);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert_eq!(snapshot.host_record(b"receipt/a").expect("prefix"), None);
    assert_eq!(
        store.committed_generation(&work()).expect("generation"),
        before
    );
}

#[test]
fn host_key_grammar_is_checked_before_any_write() {
    let (_dir, path) = store_dir("cand-seal-grammar");
    let store = create_default(&path);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let unordered = [
        HostRecord::Put {
            key: b"b",
            value: b"1",
        },
        HostRecord::Put {
            key: b"a",
            value: b"2",
        },
    ];
    assert!(matches!(
        owner.prepare_unchanged().expect("txn").seal(HostChanges {
            records: &unordered,
            head: Head::Keep,
        }),
        Err(Error::HostKey(HostKeyFault::NotStrictlyOrdered))
    ));
    let big_key = vec![7u8; 600];
    let oversized = [HostRecord::Delete { key: &big_key }];
    assert!(matches!(
        owner.prepare_unchanged().expect("txn").seal(HostChanges {
            records: &oversized,
            head: Head::Keep,
        }),
        Err(Error::HostKey(HostKeyFault::TooLong { actual: 600 }))
    ));
    drop(owner);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.host_record(b"a").expect("lookup"), None);
}

#[test]
fn unchanged_host_bytes_and_noop_deltas_do_not_move_the_generation() {
    let (_dir, path) = store_dir("cand-noops");
    let store = create_default(&path);
    let seal = |records: &[HostRecord<'_>], head: Head<'_>| {
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        owner
            .prepare_unchanged()
            .expect("txn")
            .seal(HostChanges { records, head })
            .expect("seal")
            .commit()
            .expect("commit")
    };
    let stamp = host_put(b"stamp", b"same");
    let first = seal(&stamp, Head::Put(b"h"));
    assert!(first.changed);
    let second = seal(&stamp, Head::Put(b"h"));
    assert!(!second.changed);
    assert_eq!(second.generation, first.generation);
    let cleared = seal(&[], Head::Clear);
    assert!(cleared.changed);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "here"))], &[]),
    );
    let before = store.committed_generation(&work()).expect("generation");
    let noop = commit_changes(
        &store,
        &change_set(
            &schema(),
            &[(NOTE, note(1, "here"))],
            &[(NOTE, note(9, "never-existed"))],
        ),
    );
    assert!(!noop.changed);
    assert_eq!(noop.generation, before);
}

#[test]
fn a_foreign_schema_change_set_refuses() {
    let (_dir, path) = store_dir("cand-foreign-schema");
    let store = create_default(&path);
    let foreign = change_set(
        &other_schema(),
        &[(RelationId(0), vec![Value::I64(-1)])],
        &[],
    );
    assert!(matches!(
        try_commit_changes(&store, &foreign),
        Err(Error::ForeignSchema)
    ));
}

#[test]
fn a_decider_sees_earlier_admissions_and_rolls_back_rejections_alone() {
    let (_dir, path) = store_dir("cand-decide-all");
    let store = create_default(&path);
    let sets = [
        change_set(&schema(), &[(NOTE, note(1, "a"))], &[]),
        change_set(&schema(), &[(NOTE, note(1, "b"))], &[]),
        change_set(&schema(), &[(NOTE, note(2, "c")), (TAG, tag("t"))], &[]),
        change_set(&schema(), &[(NOTE, note(2, "d"))], &[(NOTE, note(2, "c"))]),
    ];
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let mut judge = owner.decider().expect("decider");
    let verdicts: Vec<_> = sets
        .iter()
        .map(|changes| {
            let decided = judge.decide(&schema(), changes).expect("decide");
            (decided.applied, decided.judgment != Judgment::Admitted)
        })
        .collect();
    drop(judge);
    assert_eq!(
        verdicts,
        [
            (
                Applied {
                    added: 1,
                    removed: 0
                },
                false
            ),
            (
                Applied {
                    added: 1,
                    removed: 0
                },
                true
            ),
            (
                Applied {
                    added: 2,
                    removed: 0
                },
                false
            ),
            (
                Applied {
                    added: 1,
                    removed: 1
                },
                false
            ),
        ]
    );
    drop(owner);
    assert_eq!(
        store
            .snapshot(&work())
            .expect("snapshot")
            .row_count(NOTE)
            .expect("count"),
        0,
        "a decision commits nothing"
    );
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let accepted = [sets[0].clone(), sets[2].clone(), sets[3].clone()];
    let prepared = owner.prepare_decided(&accepted).expect("apply decided");
    assert_eq!(
        prepared.applied_each(),
        [
            Applied {
                added: 1,
                removed: 0
            },
            Applied {
                added: 2,
                removed: 0
            },
            Applied {
                added: 1,
                removed: 1
            },
        ]
    );
    prepared
        .seal(HostChanges::NONE)
        .expect("seal")
        .commit()
        .expect("commit");
    drop(owner);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 2);
    assert!(
        snapshot
            .contains(NOTE, &note_row(2, "d"), &work())
            .expect("d")
    );
}
