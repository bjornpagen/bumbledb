//! Epoch-barrier GC: one active barrier, immutable mark evidence, resumable
//! sweep progress — GC-01/02/05..11 deterministic schedules over `MemStore`
//! (REP-007/008/012/013/019). Real-S3 listing/fault evidence is the separate
//! credential-gated lane; process-kill lanes are P12's F3 harness over these
//! same entry points. Verification: `NotRun` (F1 authors, does not execute).

mod lane_support;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use bumbledb_log::admin;
use bumbledb_log::checkpointer::{CheckpointKind, CheckpointPolicy, publish_checkpoint};
use bumbledb_log::gc::{GcError, GcPolicy, close_epoch, decode_marks, mark, run_collection, sweep};
use bumbledb_log::manifest::{GcPhase, RootKind, RootPolicy};
use bumbledb_log::store::mem::{Behavior, Gate, MemStore, Op};
use bumbledb_log::store::{
    ConditionalOutcome, ConditionalStore as _, ObjectKind, get_verified, head_key, put_verified,
};
use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, op, work};

fn gc_policy() -> GcPolicy {
    GcPolicy {
        head_cap: HEAD_CAP,
        ..GcPolicy::DEFAULT
    }
}

fn ckpt_policy() -> CheckpointPolicy {
    CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: HEAD_CAP,
        ..CheckpointPolicy::DEFAULT
    }
}

/// Prepare a mirrored tenant with `decisions` published decisions.
fn tenant<'b>(tag: &str, store: &'b MemStore, decisions: u8) -> Mirror<'b, MemStore> {
    let mut mirror = Mirror::create(tag, store, "t");
    let identity = mirror.identity;
    for request in 1..=decisions {
        mirror.submit(&insert_user(
            mirror.db(),
            identity,
            request,
            u64::from(request) * 10,
        ));
    }
    mirror
}

#[test]
fn gc01_unreferenced_old_epoch_object_is_collected_and_new_epoch_twin_survives() {
    let store = MemStore::new();
    let mirror = tenant("gc01", &store, 2);
    // A writer stages X under the open epoch but never publishes a head
    // referencing it (the paused-writer orphan).
    let orphan = put_verified(
        &store,
        "t",
        mirror.head().object_epoch,
        ObjectKind::Chunk,
        b"orphan-x",
    )
    .expect("orphan stages");
    let report = run_collection(&store, "t", op(0x11), LIMITS, &gc_policy(), &work())
        .expect("collection runs");
    assert!(report.finished);
    assert_eq!(
        report.deleted, 1,
        "exactly the unreferenced old-epoch object went"
    );
    assert!(
        get_verified(&store, "t", &orphan).is_err(),
        "the orphan is gone"
    );
    // Every protected object (the whole tail of decisions) survived.
    let head = mirror.head();
    let recovery = head.recovery.expect("recovery");
    assert_eq!(recovery.tip.seq, 2);
    // The same bytes restaged under the NEW epoch are a distinct storage name
    // and are never hit by a collector for the old cutoff.
    let restaged = put_verified(
        &store,
        "t",
        head.object_epoch,
        ObjectKind::Chunk,
        b"orphan-x",
    )
    .expect("restage under the open epoch");
    assert_ne!(restaged.key("t"), orphan.key("t"));
    get_verified(&store, "t", &restaged).expect("restaged twin is untouched");
}

#[test]
fn gc02_a_writer_paused_across_the_barrier_cannot_publish_its_old_head() {
    // The epoch-closing CAS invalidates every old publication attempt: a
    // writer paused between reading the head and its CAS loses when the
    // barrier lands first. Deterministic via the gate.
    let store = MemStore::new();
    let mirror = tenant("gc02", &store, 1);
    let (head, version) = match store.read_head(&head_key("t")).expect("read") {
        bumbledb_log::store::HeadRead::Present { version, body } => (body, version),
        bumbledb_log::store::HeadRead::Absent => panic!("head exists"),
    };
    let gate = Arc::new(Gate::new());
    let fired = Arc::new(AtomicBool::new(false));
    let (reached_tx, reached_rx) = mpsc::channel::<()>();
    {
        let gate = Arc::clone(&gate);
        let fired = Arc::clone(&fired);
        store.set_gate(move |op, _| {
            if op == Op::ReplaceHead && !fired.swap(true, Ordering::SeqCst) {
                let _ = reached_tx.send(());
                return Some(Arc::clone(&gate));
            }
            None
        });
    }
    let outcome = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            // The paused old writer resumes its exact-version CAS after the
            // barrier: it must lose, never acknowledge.
            store.replace_head(&head_key("t"), &version, &head)
        });
        reached_rx.recv().expect("writer reached its CAS");
        close_epoch(&store, "t", op(0x22), &gc_policy(), &work()).expect("barrier publishes");
        gate.open();
        writer.join().expect("writer thread")
    })
    .expect("no transport failure");
    assert_eq!(
        outcome,
        ConditionalOutcome::PreconditionFailed,
        "the barrier invalidated the old expected version"
    );
    let _ = mirror;
}

#[test]
fn gc03_named_restore_point_protects_an_old_checkpoint_until_release() {
    let store = MemStore::new();
    let mut mirror = tenant("gc03", &store, 2);
    let identity = mirror.identity;
    // First checkpoint, pinned as a named restore point.
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("first checkpoint");
    let pinned = admin::add_named_root_hosted(
        &store,
        "t",
        op(0x31),
        RootKind::RestorePoint,
        "before-change",
        op(0x32),
        &RootPolicy::DEFAULT,
        HEAD_CAP,
        &work(),
    )
    .expect("root registers");
    let old_checkpoint = pinned.recovery.checkpoint.expect("pinned checkpoint");
    // Newer decisions and a newer checkpoint supersede the pinned one.
    mirror.submit(&insert_user(mirror.db(), identity, 9, 90));
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("second checkpoint");
    // GC with the pin held: the old checkpoint's whole closure survives.
    let report = run_collection(&store, "t", op(0x33), LIMITS, &gc_policy(), &work())
        .expect("collection with pin");
    assert!(report.finished);
    let bytes = get_verified(&store, "t", &old_checkpoint).expect("pinned manifest survives");
    let manifest =
        bumbledb_log::codec::decode_manifest(&bytes, ckpt_policy().stream).expect("decodes");
    for chunk in &manifest.chunks {
        get_verified(&store, "t", chunk).expect("pinned chunk survives");
    }
    // Release the pin: deletion reports the lost recovery capability, then a
    // LATER collection reclaims the old closure.
    let released =
        admin::release_named_root_hosted(&store, "t", op(0x31), false, HEAD_CAP, &work())
            .expect("release")
            .expect("the root existed");
    assert_eq!(released.recovery.checkpoint, Some(old_checkpoint));
    run_collection(&store, "t", op(0x34), LIMITS, &gc_policy(), &work())
        .expect("second collection");
    assert!(
        get_verified(&store, "t", &old_checkpoint).is_err(),
        "the released closure is collected"
    );
    // The current recovery root still fully verifies.
    let current = mirror
        .head()
        .recovery
        .expect("recovery")
        .checkpoint
        .expect("checkpoint");
    get_verified(&store, "t", &current).expect("current checkpoint retained");
}

#[test]
fn gc05_roots_added_during_a_collection_survive_progress_rebase() {
    let store = MemStore::new();
    let mirror = tenant("gc05", &store, 2);
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("checkpoint");
    // Open the barrier, then register a root BEFORE marking/sweeping: the
    // barrier's protected closure is immutable, but the head rebase must
    // preserve the intervening root through every progress CAS.
    close_epoch(&store, "t", op(0x51), &gc_policy(), &work()).expect("barrier");
    let root = admin::add_named_root_hosted(
        &store,
        "t",
        op(0x52),
        RootKind::RestorePoint,
        "mid-collection",
        op(0x53),
        &RootPolicy::DEFAULT,
        HEAD_CAP,
        &work(),
    )
    .expect("root registers during Marking");
    mark(&store, "t", LIMITS, &gc_policy(), &work()).expect("mark");
    sweep(&store, "t", &gc_policy(), &work()).expect("sweep");
    let head = mirror.head();
    assert!(matches!(head.gc, GcPhase::Idle));
    assert!(
        head.roots.iter().any(|held| held.id == root.id),
        "the intervening root was preserved through the finish CAS"
    );
    // Its (current-recovery) closure was inside the barrier's protection, so
    // its checkpoint still verifies.
    get_verified(&store, "t", &root.recovery.checkpoint.expect("checkpoint"))
        .expect("root closure survived the sweep");
}

#[test]
fn gc06_partial_or_foreign_mark_evidence_is_never_a_deletion_certificate() {
    let store = MemStore::new();
    let mirror = tenant("gc06", &store, 1);
    close_epoch(&store, "t", op(0x61), &gc_policy(), &work()).expect("barrier");
    let marks_ref = mark(&store, "t", LIMITS, &gc_policy(), &work()).expect("mark");
    // Corrupt the stored mark manifest: sweep must refuse without deletion.
    assert!(store.corrupt_object(&marks_ref.key("t"), |bytes| {
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
    }));
    let before = store.object_keys();
    let refused = sweep(&store, "t", &gc_policy(), &work());
    assert!(
        matches!(refused, Err(GcError::Corruption(_))),
        "corrupt mark evidence stops GC: {refused:?}"
    );
    assert_eq!(store.object_keys(), before, "nothing was deleted");
    // Foreign-barrier mark evidence can never certify this sweep.
    let honest = get_verified(&store, "t", &marks_ref);
    assert!(honest.is_err(), "the corrupted manifest fails verification");
    let foreign = decode_marks(b"not a mark manifest", op(0x61), HEAD_CAP);
    assert!(foreign.is_err());
    let _ = mirror;
}

#[test]
fn gc07_failed_deletion_retains_progress_and_resume_converges() {
    let store = MemStore::new();
    let mirror = tenant("gc07", &store, 1);
    // Two orphans to delete.
    let epoch = mirror.head().object_epoch;
    put_verified(&store, "t", epoch, ObjectKind::Chunk, b"orphan-a").expect("stage a");
    put_verified(&store, "t", epoch, ObjectKind::Chunk, b"orphan-b").expect("stage b");
    close_epoch(&store, "t", op(0x71), &gc_policy(), &work()).expect("barrier");
    mark(&store, "t", LIMITS, &gc_policy(), &work()).expect("mark");
    // The first eligible delete fails; durable progress must NOT advance
    // past it as if it succeeded.
    store.fail_next(Op::DeleteObject, Behavior::Error);
    let failed = sweep(&store, "t", &gc_policy(), &work());
    let Err(GcError::DeleteFailed { key }) = failed else {
        panic!("expected a retryable delete failure, got {failed:?}");
    };
    assert!(
        store.object_keys().contains(&key),
        "the failed object remains"
    );
    // Resume: the same sweep retries the failed key and finishes.
    let resumed = sweep(&store, "t", &gc_policy(), &work()).expect("resume");
    assert!(resumed.finished);
    assert!(
        !store.object_keys().contains(&key),
        "the resumed sweep deleted the previously failed key"
    );
    assert!(matches!(mirror.head().gc, GcPhase::Idle));
}

#[test]
fn gc08_gc09_late_uploads_are_orphans_found_by_actual_listing_not_slot_scans() {
    // Small pages: request count tracks extant keys, never historical slots.
    let store = MemStore::with_page_size(2);
    let mirror = tenant("gc0809", &store, 1);
    let old_epoch = mirror.head().object_epoch;
    run_collection(&store, "t", op(0x81), LIMITS, &gc_policy(), &work()).expect("first pass");
    // A suspended old client uploads an old-epoch object AFTER the collector
    // finished. It cannot publish it (GC-02); it is an orphan.
    let late = put_verified(&store, "t", old_epoch, ObjectKind::Chunk, b"late-upload")
        .expect("late upload lands");
    get_verified(&store, "t", &late).expect("the orphan exists for now");
    // The next actual-object reconciliation collects it.
    let second =
        run_collection(&store, "t", op(0x82), LIMITS, &gc_policy(), &work()).expect("second pass");
    assert!(second.finished);
    assert!(
        get_verified(&store, "t", &late).is_err(),
        "the late orphan is collected"
    );
    // Pagination happened (page size 2) and cost tracked extant keys.
    assert!(second.pages >= 1);
    assert!(
        second.pages < 100,
        "bounded pages over extant keys, not historical slot count: {}",
        second.pages
    );
}

#[test]
fn gc10_a_stale_collector_cannot_regress_progress_or_win_with_old_evidence() {
    let store = MemStore::new();
    let mirror = tenant("gc10", &store, 1);
    close_epoch(&store, "t", op(0xa1), &gc_policy(), &work()).expect("barrier");
    // A second collector trying to open its own barrier is told the
    // collection moved; the running barrier is not replaced.
    let competing = close_epoch(&store, "t", op(0xa2), &gc_policy(), &work());
    assert!(
        matches!(competing, Err(GcError::CollectionMoved)),
        "{competing:?}"
    );
    // The same collector's close_epoch is evidence-idempotent.
    let same = close_epoch(&store, "t", op(0xa1), &gc_policy(), &work()).expect("same barrier");
    assert_eq!(same.id, op(0xa1));
    // Finish the collection; a stale sweep afterwards reports finished, and
    // the epoch never regresses.
    mark(&store, "t", LIMITS, &gc_policy(), &work()).expect("mark");
    sweep(&store, "t", &gc_policy(), &work()).expect("sweep");
    let epoch_after = mirror.head().object_epoch;
    let stale = sweep(&store, "t", &gc_policy(), &work());
    assert!(matches!(stale, Err(GcError::AlreadyFinished)), "{stale:?}");
    assert_eq!(mirror.head().object_epoch, epoch_after, "epoch is monotone");
}

#[test]
fn gc11_root_capacity_refuses_without_discarding_and_stale_release_refuses() {
    let store = MemStore::new();
    let mirror = tenant("gc11", &store, 1);
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("checkpoint");
    let tight = RootPolicy {
        max_roots: 2,
        max_label_bytes: 32,
    };
    for (id, label) in [(0xb1u8, "first"), (0xb2, "second")] {
        admin::add_named_root_hosted(
            &store,
            "t",
            op(id),
            RootKind::RestorePoint,
            label,
            op(0xbf),
            &tight,
            HEAD_CAP,
            &work(),
        )
        .expect("registers");
    }
    let full = admin::add_named_root_hosted(
        &store,
        "t",
        op(0xb3),
        RootKind::RestorePoint,
        "third",
        op(0xbf),
        &tight,
        HEAD_CAP,
        &work(),
    );
    assert!(
        matches!(
            full,
            Err(admin::AdminError::Head(
                bumbledb_log::manifest::HeadError::RootCapacityExceeded
            ))
        ),
        "{full:?}"
    );
    let head = mirror.head();
    assert_eq!(head.roots.len(), 2, "no root was discarded by the refusal");
    // A stale release naming an unknown ID refuses; it cannot remove a
    // different root.
    let stale = admin::release_named_root_hosted(&store, "t", op(0xb9), false, HEAD_CAP, &work());
    assert!(
        matches!(
            stale,
            Err(admin::AdminError::Head(
                bumbledb_log::manifest::HeadError::UnknownRoot
            ))
        ),
        "{stale:?}"
    );
    assert_eq!(mirror.head().roots.len(), 2);
    // Duplicate root IDs are never reused.
    let duplicate = admin::add_named_root_hosted(
        &store,
        "t",
        op(0xb1),
        RootKind::RestorePoint,
        "first",
        op(0xbf),
        &tight,
        HEAD_CAP,
        &work(),
    )
    .expect("evidence, not failure");
    assert_eq!(duplicate.id, op(0xb1), "an identical retry is evidence");
}

#[test]
fn head_and_unparseable_namespaces_are_never_swept() {
    let store = MemStore::new();
    let mirror = tenant("gc-never", &store, 1);
    // A key inside objects/ that no honest writer spells: unparseable, so the
    // sweep must retain it rather than guess.
    assert_eq!(
        store
            .put_object("t/objects/7/braid/aa", b"foreign bytes")
            .expect("stored"),
        bumbledb_log::store::PutOutcome::Stored
    );
    let report =
        run_collection(&store, "t", op(0xc1), LIMITS, &gc_policy(), &work()).expect("collection");
    assert!(report.retained_unparsed >= 1);
    assert!(
        store
            .object_keys()
            .iter()
            .any(|key| key == "t/objects/7/braid/aa"),
        "unknown namespaces are never deleted"
    );
    // HEAD survives (it is not under objects/ and is never listed for sweep).
    assert!(matches!(
        store.read_head(&head_key("t")).expect("read"),
        bumbledb_log::store::HeadRead::Present { .. }
    ));
    let _ = mirror;
}
