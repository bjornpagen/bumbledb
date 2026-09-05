//! Streamed coherent checkpoints with bounded validated suffix rebase —
//! STORE-03/04/07 shapes, the epoch-moved restaging rule, the tail envelope
//! backpressure and the shared blank-projection digests (REP-014, PERF-004).
//! Deterministic `MemStore` schedules; real-S3 evidence is the separate
//! credential-gated lane. Verification: `NotRun` (F1 authors, does not execute).

mod lane_support;

use bumbledb_log::checkpointer::{
    CheckpointKind, CheckpointOutcome, CheckpointPolicy, Headroom, admission_headroom,
    publish_checkpoint,
};
use bumbledb_log::codec;
use bumbledb_log::gc::{GcPolicy, close_epoch};
use bumbledb_log::history::decision;
use bumbledb_log::manifest::{RecoveryRoot, TailPolicy};
use bumbledb_log::store::get_verified;
use bumbledb_log::store::mem::{MemStore, Op};
use bumbledb_log::store::{ReceiveLimits, TransportContext};
use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, op, work};

fn policy() -> CheckpointPolicy {
    CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: HEAD_CAP,
        ..CheckpointPolicy::DEFAULT
    }
}

fn fetch_verified(
    store: &MemStore,
    reference: &bumbledb_log::store::ObjectRef,
) -> bumbledb::work::ChargedBytes {
    get_verified(
        store,
        "t",
        reference,
        TransportContext::new(&work(), ReceiveLimits::exact(reference.length)),
    )
    .expect("verified")
}

#[test]
fn checkpoint_captures_one_coherent_snapshot_and_publishes_exact_suffix() {
    let store = MemStore::new();
    let mut mirror = Mirror::create("ckpt-basic", &store, "t");
    let identity = mirror.identity;
    for (request, id) in [(1u8, 10u64), (2, 20), (3, 30)] {
        mirror.submit(&insert_user(mirror.db(), identity, request, id));
    }
    let tip_before = mirror.head().recovery.expect("recovery").tip;
    let outcome = publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &policy(),
        &work(),
    )
    .expect("checkpoint publishes");
    let CheckpointOutcome::Published {
        manifest,
        base,
        tip,
        ..
    } = outcome
    else {
        panic!("expected published, got {outcome:?}");
    };
    assert_eq!(base, tip_before, "the capture is the local committed tip");
    assert_eq!(tip, tip_before);
    let head = mirror.head();
    let recovery = head.recovery.expect("recovery root");
    assert_eq!(recovery.checkpoint, Some(manifest));
    assert_eq!(recovery.base, base);
    assert_eq!(recovery.tip, tip);
    assert_eq!(
        recovery.tail_count(),
        0,
        "checkpoint at the tip has no tail"
    );
    // The manifest decodes and its chunks verify from the store.
    let bytes = fetch_verified(&store, &manifest);
    let decoded = codec::decode_manifest(bytes.as_bytes(), policy().stream).expect("manifest decodes");
    assert_eq!(decoded.rows, 3);
    assert_eq!(decoded.identity, identity);
    for chunk in &decoded.chunks {
        let _ = fetch_verified(&store, chunk);
    }
}

#[test]
fn moved_head_causes_bounded_rebase_with_validated_suffix_not_reexport() {
    // STORE-03: the head moves after capture and before the checkpoint CAS;
    // the same export publishes with the exact suffix (S, T] validated —
    // never a quiet-window restart. Deterministic: the checkpoint's first CAS
    // pauses at a gate while another decision publishes.
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    let store = MemStore::new();
    let mut mirror = Mirror::create("ckpt-rebase", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    let captured_tip = mirror.head().recovery.expect("recovery").tip;

    let gate = Arc::new(bumbledb_log::store::mem::Gate::new());
    let fired = Arc::new(AtomicBool::new(false));
    let (reached_tx, reached_rx) = mpsc::channel::<()>();
    {
        let gate = Arc::clone(&gate);
        let fired = Arc::clone(&fired);
        store.set_gate(move |op, _key| {
            if op == Op::ReplaceHead && !fired.swap(true, Ordering::SeqCst) {
                let _ = reached_tx.send(());
                return Some(Arc::clone(&gate));
            }
            None
        });
    }
    let db = Arc::clone(&mirror.db_arc);
    let outcome = std::thread::scope(|scope| {
        let checkpointer = scope.spawn(|| {
            publish_checkpoint(
                &db,
                &store,
                "t",
                LIMITS,
                CheckpointKind::Ordinary,
                &policy(),
                &work(),
            )
        });
        // The checkpoint captured decision 1 and paused at its CAS. Publish
        // decision 2 now (its own CAS passes the one-shot gate hook), then
        // release the checkpoint: its CAS loses and it rebases.
        reached_rx.recv().expect("checkpoint reached its CAS");
        mirror.submit(&insert_user(mirror.db(), identity, 2, 20));
        gate.open();
        checkpointer.join().expect("checkpoint thread")
    })
    .expect("checkpoint rebases and publishes");
    let CheckpointOutcome::Published { base, tip, .. } = outcome else {
        panic!("expected published, got {outcome:?}");
    };
    assert_eq!(
        base, captured_tip,
        "the export captured decision 1 exactly once"
    );
    assert_eq!(
        tip.seq,
        base.seq + 1,
        "the rebase preserved the newer decision"
    );
    let recovery = mirror.head().recovery.expect("recovery");
    assert_eq!(recovery.base, base);
    assert_eq!(recovery.tip, tip);
    assert_eq!(
        recovery.tail_count(),
        1,
        "exactly the suffix (S, T] is retained"
    );
    assert!(
        recovery.tail_bytes > 0,
        "the validated suffix counted its bytes"
    );
    // Exactly one export happened: chunk uploads are not repeated per retry.
    let puts = store
        .operations()
        .into_iter()
        .filter(|(op, key)| *op == Op::PutObject && key.contains("/chunk/"))
        .count();
    let manifest_bytes = {
        let reference = recovery.checkpoint.expect("checkpoint");
        fetch_verified(&store, &reference)
    };
    let decoded = codec::decode_manifest(manifest_bytes.as_bytes(), policy().stream).expect("decodes");
    assert_eq!(
        puts,
        decoded.chunks.len(),
        "no chunk was uploaded twice; a moved head is a rebase, not a re-export"
    );
}

#[test]
fn a_checkpoint_that_did_not_advance_the_base_is_discarded_not_republished() {
    // STORE-04: the recovery base never moves backwards and equality causes
    // no pointless republication.
    let store = MemStore::new();
    let mut mirror = Mirror::create("ckpt-discard", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    let first = publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &policy(),
        &work(),
    )
    .expect("first checkpoint");
    let CheckpointOutcome::Published { base, .. } = first else {
        panic!("first publishes");
    };
    let second = publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &policy(),
        &work(),
    )
    .expect("second returns");
    match second {
        CheckpointOutcome::Discarded { current_base_seq } => {
            assert_eq!(current_base_seq, base.seq);
        }
        other @ CheckpointOutcome::Published { .. } => {
            panic!("expected discarded, got {other:?}")
        }
    }
    let recovery = mirror.head().recovery.expect("recovery");
    assert_eq!(recovery.base, base, "the base did not move");
}

#[test]
fn epoch_moved_during_export_restages_chunks_under_the_current_epoch() {
    // Chapter 21 / GC-01 shape: dependencies staged under a now-closed epoch
    // are restaged under the current epoch; relabeling the manifest alone is
    // insufficient. Deterministic: the checkpoint's first CAS pauses at a
    // gate while the epoch-closing barrier publishes.
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    let store = MemStore::new();
    let mut mirror = Mirror::create("ckpt-epoch", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    let staged_epoch = mirror.head().object_epoch;

    let gate = Arc::new(bumbledb_log::store::mem::Gate::new());
    let fired = Arc::new(AtomicBool::new(false));
    let (reached_tx, reached_rx) = mpsc::channel::<()>();
    {
        let gate = Arc::clone(&gate);
        let fired = Arc::clone(&fired);
        store.set_gate(move |op, _key| {
            if op == Op::ReplaceHead && !fired.swap(true, Ordering::SeqCst) {
                let _ = reached_tx.send(());
                return Some(Arc::clone(&gate));
            }
            None
        });
    }
    let db = Arc::clone(&mirror.db_arc);
    let gc_policy = GcPolicy {
        head_cap: HEAD_CAP,
        ..GcPolicy::DEFAULT
    };
    let (outcome, barrier) = std::thread::scope(|scope| {
        let checkpointer = scope.spawn(|| {
            publish_checkpoint(
                &db,
                &store,
                "t",
                LIMITS,
                CheckpointKind::Ordinary,
                &policy(),
                &work(),
            )
        });
        reached_rx.recv().expect("checkpoint reached its CAS");
        // The barrier's own CAS passes the one-shot gate hook and closes the
        // epoch the checkpoint staged its chunks under.
        let barrier =
            close_epoch(&store, "t", op(0x77), &gc_policy, &work()).expect("barrier publishes");
        gate.open();
        (checkpointer.join().expect("checkpoint thread"), barrier)
    });
    let outcome = outcome.expect("checkpoint restages and publishes");
    let CheckpointOutcome::Published { manifest, .. } = outcome else {
        panic!("expected published, got {outcome:?}");
    };
    assert_eq!(barrier.cutoff_epoch, staged_epoch);
    assert!(
        manifest.epoch > barrier.cutoff_epoch,
        "the manifest lives in the newly opened epoch, not the closed one"
    );
    let bytes = fetch_verified(&store, &manifest);
    let decoded = codec::decode_manifest(bytes.as_bytes(), policy().stream).expect("decodes");
    for chunk in &decoded.chunks {
        assert!(
            chunk.epoch > barrier.cutoff_epoch,
            "every chunk was restaged under the current epoch"
        );
        let _ = fetch_verified(&store, chunk);
    }
}

#[test]
fn tail_envelope_backpressure_is_typed_and_clears_after_checkpoint() {
    // STORE-07 shape: admission beyond the envelope returns
    // MaintenanceRequired (including no-op decisions); a checkpoint clears it.
    let tight = TailPolicy {
        max_count: 2,
        max_bytes: 1 << 20,
    };
    let recovery_ok = RecoveryRoot {
        checkpoint: None,
        base: decision_stamp(0),
        tip: decision_stamp(1),
        tip_object: None,
        tail_bytes: 100,
        epoch_floor: 0,
    };
    assert_eq!(
        admission_headroom(&recovery_ok, &tight),
        Headroom::StartCheckpoint
    );
    let recovery_full = RecoveryRoot {
        tip: decision_stamp(2),
        ..recovery_ok
    };
    assert_eq!(
        admission_headroom(&recovery_full, &tight),
        Headroom::MaintenanceRequired
    );
    let bytes_full = RecoveryRoot {
        tail_bytes: 1 << 21,
        ..recovery_ok
    };
    assert_eq!(
        admission_headroom(&bytes_full, &tight),
        Headroom::MaintenanceRequired,
        "count AND bytes both bound the envelope"
    );

    // The composed head refuses growth beyond the envelope with the typed
    // MaintenanceRequired error.
    let store = MemStore::new();
    let mut mirror = Mirror::create("ckpt-envelope", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    let head = mirror.head();
    let control = mirror.authority();
    let refused = head.decided(
        control,
        100,
        None,
        &TailPolicy {
            max_count: 0,
            max_bytes: 10,
        },
    );
    assert!(
        matches!(
            refused,
            Err(bumbledb_log::manifest::HeadError::MaintenanceRequired { .. })
        ),
        "admission beyond the envelope is typed backpressure: {refused:?}"
    );
}

fn decision_stamp(seq: u64) -> bumbledb_log::history::DecisionStamp {
    bumbledb_log::history::DecisionStamp {
        seq,
        hash: bumbledb_log::history::DecisionDigest::from_bytes(
            [u8::try_from(seq % 256).expect("bounded above"); 32],
        ),
    }
}

#[test]
fn blank_initial_digests_equal_the_empty_export_projection() {
    // C12 cross-lane obligation (recorded in implementation/packets/P05.md):
    // P04's genesis sentinels and P05's canonical empty export projection
    // must name the SAME blank digests. This test is the forcing function for
    // the recorded P04 patch (decision.rs blank_initial_digests domains).
    let (application, system) = decision::blank_initial_digests();
    assert_eq!(
        application,
        codec::empty_application_digest(),
        "genesis blank application digest equals the empty export projection"
    );
    assert_eq!(
        system,
        codec::empty_system_digest(),
        "genesis blank system digest equals the empty export projection"
    );
}

#[test]
fn receipt_retirement_advances_atomically_with_its_checkpoint() {
    // GC-13/chapter 20: hosted retirement rides the checkpoint that no longer
    // promises the retired rows; the frontier is monotone.
    let store = MemStore::new();
    let mut mirror = Mirror::create("ckpt-retire", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    // Rotate the open epoch to 2 hosted-side so epoch 1 becomes retirable.
    bumbledb_log::admin::hosted_result(bumbledb_log::admin::rotate_receipts_hosted(
        &store,
        "t",
        bumbledb_log::history::ReceiptEpoch::new(2).expect("epoch"),
        HEAD_CAP,
        &work(),
    ))
    .expect("rotation publishes");
    // Mirror the rotation locally so the next capture carries it.
    bumbledb_log::admin::rotate_receipts_local(
        mirror.db(),
        bumbledb_log::history::ReceiptEpoch::new(2).expect("epoch"),
        HEAD_CAP,
        &work(),
    )
    .expect("local rotation");
    let outcome = publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::RetireReceipts { through: 1 },
        &policy(),
        &work(),
    )
    .expect("retirement checkpoint publishes");
    assert!(matches!(outcome, CheckpointOutcome::Published { .. }));
    let head = mirror.head();
    let live = head.control.live().expect("live");
    assert_eq!(live.receipts.retired_through(), 1);
    // The published checkpoint's stream excludes the retired rows: hydrating
    // it later filters nothing extra (recovery lane covers hydration).
    let reference = head
        .recovery
        .expect("recovery")
        .checkpoint
        .expect("checkpoint");
    let bytes = fetch_verified(&store, &reference);
    let decoded = codec::decode_manifest(bytes.as_bytes(), policy().stream).expect("decodes");
    assert_eq!(
        decoded.system_records, 0,
        "epoch-1 receipt rows are not promised by the retiring checkpoint"
    );
}

/// D16: nonzero checkpoint-only root (seq 7) has base == tip and no tip
/// ObjectRef. Same-tip retirement/rebase remains legal. Verification: NotRun.
#[test]
fn d16_checkpoint_only_at_sequence_seven_has_no_tip_locator() {
    let store = MemStore::new();
    let mut mirror = Mirror::create("ckpt-seq7", &store, "t");
    let identity = mirror.identity;
    for request in 1u8..=7 {
        mirror.submit(&insert_user(
            mirror.db(),
            identity,
            request,
            u64::from(request) * 10,
        ));
    }
    let tip_before = mirror.head().recovery.expect("recovery").tip;
    assert_eq!(tip_before.seq, 7);
    let outcome = publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &policy(),
        &work(),
    )
    .expect("checkpoint at seq 7");
    let CheckpointOutcome::Published { base, tip, .. } = outcome else {
        panic!("expected published, got {outcome:?}");
    };
    assert_eq!(base, tip_before);
    assert_eq!(tip, tip_before);
    let recovery = mirror.head().recovery.expect("recovery");
    assert_eq!(recovery.base, recovery.tip);
    assert!(
        recovery.tip_object.is_none(),
        "checkpoint-only nonzero root has no tip locator (C6)"
    );
    assert_eq!(recovery.tail_count(), 0);
    let retired = publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::RetireReceipts { through: 0 },
        &policy(),
        &work(),
    );
    assert!(
        matches!(
            retired,
            Ok(CheckpointOutcome::Published { .. }) | Ok(CheckpointOutcome::Discarded { .. })
        ),
        "same-tip retirement/rebase is legal, got {retired:?}"
    );
}
