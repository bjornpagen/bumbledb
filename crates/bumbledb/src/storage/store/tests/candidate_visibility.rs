//! E-VISIBILITY / PROTO-07 substrate and the C04 candidate protocol:
//! prepared candidates are never readable, losing candidates vanish, seals
//! are host-record-only, failed seals dispatch nothing, and metadata-only
//! decisions advance the generation exactly once.

use super::snapshot_coherence::row_bytes;
use super::*;
use crate::store::AppliedChanges;

// The inline `View` judge below asserts the exact proposed final state —
// the C03 candidate iteration surface (`rows`/`contains`/`row_count`/
// `changes`) reads the candidate transaction's world, not the parent.
#[test]
fn the_judge_sees_the_proposed_final_state_not_the_parent() {
    struct View;
    impl CandidateJudge for View {
        type Rejection = std::convert::Infallible;
        fn judge(
            &self,
            candidate: &CandidateState<'_, '_>,
            work: &WorkContext,
        ) -> StoreResult<Judgment<Self::Rejection>> {
            assert_eq!(candidate.row_count(NOTE)?, 2);
            assert_eq!(candidate.rows(NOTE)?.count(), 2);
            let gone = crate::canonical::CanonicalRow::encode(
                schema().relation(NOTE).fields(),
                &note(1, "old"),
                work,
            )
            .expect("row");
            assert!(!candidate.contains(NOTE, gone.as_bytes(), work)?);
            let added = crate::canonical::CanonicalRow::encode(
                schema().relation(NOTE).fields(),
                &note(2, "two"),
                work,
            )
            .expect("row");
            assert!(candidate.contains(NOTE, added.as_bytes(), work)?);
            assert!(candidate.changes().is_some());
            Ok(Judgment::Admitted)
        }
    }

    let (_dir, path) = store_dir("cand-final-state-view");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "old"))], &[]),
    );
    // Final state proposal: remove note 1, add notes 2 and 3 → 2 rows.
    let changes = change_set(
        &schema(),
        &[(NOTE, note(2, "two")), (NOTE, note(3, "three"))],
        &[(NOTE, note(1, "old"))],
    );
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&changes, &FirstFieldKey, &View)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => {
            let _ = prepared
                .seal(NO_HOST)
                .expect("seal")
                .commit()
                .expect("commit");
        }
        Prepared::Rejected(never) => match never {},
    }
    drop(owner);
    // Meanwhile the committed parent had exactly one row until commit.
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 2);
}

#[test]
fn the_c04_identity_surface_names_one_store() {
    let (_dir, path) = store_dir("cand-identity-surface");
    let store = create_default(&path);
    assert_eq!(store.path(), path);
    assert_eq!(
        store.schema_fingerprint(),
        crate::schema::fingerprint::fingerprint(&schema())
    );
    let identity = store.identity();
    assert_eq!(identity.store, store.store_id());
    assert_eq!(identity.environment, store.environment_id());
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "row"))], &[]),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.identity(), identity);
    // fetch resolves the cursor's row id to the same bytes.
    let (row_id, bytes) = snapshot
        .rows(NOTE)
        .expect("cursor")
        .next()
        .expect("one row")
        .expect("row");
    let owned = bytes.to_vec();
    assert_eq!(
        snapshot.fetch(NOTE, row_id).expect("fetch"),
        Some(owned.as_slice())
    );
}

#[test]
fn a_prepared_candidate_is_invisible_to_committed_readers() {
    let (_dir, path) = store_dir("cand-invisible");
    let store = create_default(&path);
    let pinned = store.snapshot(&work()).expect("pinned");
    let changes = change_set(&schema(), &[(NOTE, note(1, "spectral"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let prepared = match owner
        .prepare(&changes, &FirstFieldKey, &AdmitAll)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared,
        Prepared::Rejected(never) => match never {},
    };
    // While the candidate is prepared: neither the pinned snapshot nor a
    // fresh one can see it.
    assert!(
        !pinned
            .contains(NOTE, row_bytes(&note(1, "spectral")), &work())
            .expect("pinned probe")
    );
    let fresh = store.snapshot(&work()).expect("fresh during candidate");
    assert!(
        !fresh
            .contains(NOTE, row_bytes(&note(1, "spectral")), &work())
            .expect("fresh probe")
    );
    drop(fresh);
    // The losing candidate aborts: still invisible, forever.
    prepared.abort();
    drop(owner);
    let after = store.snapshot(&work()).expect("after abort");
    assert_eq!(after.row_count(NOTE).expect("count"), 0);
    // The same delta prepared again and committed becomes visible to NEW
    // snapshots only; the pinned pre-candidate snapshot never moves.
    commit_changes(&store, &changes);
    assert!(
        !pinned
            .contains(NOTE, row_bytes(&note(1, "spectral")), &work())
            .expect("pinned after commit")
    );
    let latest = store.snapshot(&work()).expect("latest");
    assert!(
        latest
            .contains(NOTE, row_bytes(&note(1, "spectral")), &work())
            .expect("latest probe")
    );
}

#[test]
fn a_judge_rejection_retains_the_session_for_the_receipt_transaction() {
    let (_dir, path) = store_dir("cand-rejected-receipt");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "first"))], &[]),
    );
    let before = store.committed_generation(&work()).expect("generation");
    // Two rows under one note id: the key judge must see both competing
    // candidate rows (ENG-005) and reject with both row ids as evidence.
    let conflicting = change_set(&schema(), &[(NOTE, note(1, "second"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let rejection = match owner
        .prepare(&conflicting, &FirstFieldKey, &UniqueNoteId)
        .expect("prepare")
    {
        Prepared::Rejected(rows) => rows,
        Prepared::Admitted(_) => panic!("the key law must reject the second note-1 row"),
    };
    assert_eq!(rejection.len(), 2, "both competing proposals are evidence");
    // Same exclusive session, no gap: seal the rejection receipt against
    // the unchanged committed parent as a metadata-only transaction.
    let receipt = owner.prepare_unchanged().expect("receipt transaction");
    assert_eq!(receipt.application_changes(), AppliedChanges::default());
    let records = host_put(b"receipt/1", b"rejected");
    let commit = receipt
        .seal(HostChanges {
            records: &records,
            attachment: AttachmentChange::Keep,
        })
        .expect("seal receipt")
        .commit()
        .expect("commit receipt");
    // Host mutation on a no-op advances the generation exactly once.
    assert!(commit.changed);
    assert_eq!(commit.generation.value(), before.value() + 1);
    assert_eq!(commit.application.added, 0);
    drop(owner);
    // Facts unchanged; the receipt is readable from the same snapshot
    // world that shows the unchanged facts.
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert!(
        snapshot
            .contains(NOTE, row_bytes(&note(1, "first")), &work())
            .expect("original row intact")
    );
    assert_eq!(
        snapshot.host_record(b"receipt/1").expect("receipt lookup"),
        Some(b"rejected".as_slice())
    );
}

#[test]
fn one_command_replacement_judges_the_final_state_not_statement_order() {
    // delete(old) + insert(new) under one key must admit: the judged view
    // is the proposed final state, where only the new row holds the key.
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
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&replacement, &FirstFieldKey, &UniqueNoteId)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => {
            let _ = prepared
                .seal(NO_HOST)
                .expect("seal")
                .commit()
                .expect("commit");
        }
        Prepared::Rejected(rows) => {
            panic!("replacement is legal in the final state, got rejection {rows:?}")
        }
    }
    drop(owner);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert!(
        snapshot
            .contains(NOTE, row_bytes(&note(1, "new")), &work())
            .expect("new row")
    );
}

#[test]
fn a_failed_seal_drops_facts_and_host_prefix_and_dispatches_nothing() {
    let (_dir, path) = store_dir("cand-failed-seal");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "base"))], &[]),
    );
    let before = store.committed_generation(&work()).expect("generation");
    // Fail after the first host record is applied: the already-written
    // prefix AND the judged facts must both vanish with the transaction.
    store.fail_host_seal_after(Some(1));
    let changes = change_set(&schema(), &[(NOTE, note(2, "doomed"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let prepared = match owner
        .prepare(&changes, &FirstFieldKey, &AdmitAll)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared,
        Prepared::Rejected(never) => match never {},
    };
    let records = [
        HostRecordChange::Put {
            key: b"receipt/a",
            value: b"prefix-written",
        },
        HostRecordChange::Put {
            key: b"receipt/b",
            value: b"never-reached",
        },
    ];
    match prepared.seal(HostChanges {
        records: &records,
        attachment: AttachmentChange::Keep,
    }) {
        Err(StoreError::MapFull { .. }) => {}
        Err(other) => panic!("expected the injected map-full seal failure, got {other:?}"),
        Ok(_) => panic!("expected the injected map-full seal failure, got a sealed write"),
    }
    store.fail_host_seal_after(None);
    drop(owner);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 1);
    assert_eq!(snapshot.host_record(b"receipt/a").expect("prefix"), None);
    assert_eq!(snapshot.host_record(b"receipt/b").expect("suffix"), None);
    assert_eq!(
        store.committed_generation(&work()).expect("generation"),
        before
    );
}

#[test]
fn seal_input_grammar_is_checked_before_any_write() {
    let (_dir, path) = store_dir("cand-seal-grammar");
    let store = create_default(&path);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    // Unordered keys refuse.
    let unordered = [
        HostRecordChange::Put {
            key: b"b",
            value: b"1",
        },
        HostRecordChange::Put {
            key: b"a",
            value: b"2",
        },
    ];
    let prepared = owner.prepare_unchanged().expect("txn");
    match prepared.seal(HostChanges {
        records: &unordered,
        attachment: AttachmentChange::Keep,
    }) {
        Err(StoreError::HostKey(crate::storage::store::HostKeyFault::NotStrictlyOrdered)) => {}
        Err(other) => panic!("expected the strict-order host-key fault, got {other:?}"),
        Ok(_) => panic!("expected the strict-order host-key fault, got a sealed write"),
    }
    // Oversized keys refuse; nothing was written by the failed attempts.
    let big_key = vec![7u8; 600];
    let oversized = [HostRecordChange::Put {
        key: &big_key,
        value: b"x",
    }];
    let prepared = owner.prepare_unchanged().expect("txn");
    match prepared.seal(HostChanges {
        records: &oversized,
        attachment: AttachmentChange::Keep,
    }) {
        Err(StoreError::HostKey(crate::storage::store::HostKeyFault::TooLong { actual: 600 })) => {}
        Err(other) => panic!("expected the bounded-width host-key fault, got {other:?}"),
        Ok(_) => panic!("expected the bounded-width host-key fault, got a sealed write"),
    }
    drop(owner);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.host_record(b"a").expect("lookup"), None);
    assert_eq!(snapshot.host_record(b"b").expect("lookup"), None);
}

#[test]
fn idempotent_host_records_do_not_move_the_generation() {
    let (_dir, path) = store_dir("cand-host-idempotent");
    let store = create_default(&path);
    let records = host_put(b"stamp", b"same-value");
    let seal_stamp = |store: &Store| {
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        owner
            .prepare_unchanged()
            .expect("txn")
            .seal(HostChanges {
                records: &records,
                attachment: AttachmentChange::Keep,
            })
            .expect("seal")
            .commit()
            .expect("commit")
    };
    let first = seal_stamp(&store);
    assert!(first.changed);
    // The identical value again: byte-equal put is a no-op, no generation.
    let second = seal_stamp(&store);
    assert!(!second.changed);
    assert_eq!(second.generation, first.generation);
}

#[test]
fn a_foreign_schema_change_set_refuses_before_any_candidate_exists() {
    let (_dir, path) = store_dir("cand-foreign-schema");
    let store = create_default(&path);
    let foreign = {
        let other = other_schema();
        let mut builder = ChangeSet::builder(&other, work());
        builder
            .insert(RelationId(0), &[Value::I64(-1)])
            .expect("stage foreign");
        builder.finish().expect("foreign change set")
    };
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner.prepare(&foreign, &NoIndex, &AdmitAll) {
        Err(StoreError::ForeignSchema) => {}
        other => panic!("expected ForeignSchema, got {other:?}"),
    }
}

#[test]
fn deleting_an_absent_fact_and_adding_a_present_fact_are_noops() {
    let (_dir, path) = store_dir("cand-noop-delta");
    let store = create_default(&path);
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
    assert_eq!(noop.application, AppliedChanges::default());
}
