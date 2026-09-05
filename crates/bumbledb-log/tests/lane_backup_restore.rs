//! Independent verified-bytes backup and new-incarnation restore —
//! BACKUP-01..05 shapes and RESTORE-01..03 (OPS-002). The destination is a
//! DISTINCT store; restores read the destination only. Cross-platform and
//! >RAM arms are F3 lanes over these entry points. Verification: NotRun (F1
//! > authors, does not execute).

mod lane_support;

use std::sync::Arc;

use bumbledb::RelationId;
use bumbledb_log::backup::{
    BackupError, backup_manifest_key, backup_root, read_backup_manifest, read_backup_tail,
    verify_backup,
};
use bumbledb_log::checkpointer::{CheckpointKind, CheckpointPolicy, publish_checkpoint};
use bumbledb_log::history::IncarnationId;
use bumbledb_log::restore::{inspect, restore_writable_with_tail};
use bumbledb_log::store::mem::{Behavior, MemStore, Op};
use bumbledb_log::store::{ConditionalStore as _, get_verified};
use bumbledb_log::writer::{LocalHistory, LogError, SubmitOutcome};
use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, op, temp_dir, theory, work};

fn ckpt_policy() -> CheckpointPolicy {
    CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: HEAD_CAP,
        ..CheckpointPolicy::DEFAULT
    }
}

/// A mirrored tenant: checkpoint at decision 2, tail decision 3.
fn hosted_fixture<'b>(tag: &str, store: &'b MemStore) -> Mirror<'b, MemStore> {
    let mut mirror = Mirror::create(tag, store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    mirror.submit(&insert_user(mirror.db(), identity, 2, 20));
    publish_checkpoint(
        mirror.db(),
        store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("checkpoint publishes");
    mirror.submit(&insert_user(mirror.db(), identity, 3, 30));
    mirror
}

fn run_backup(
    mirror: &Mirror<'_, MemStore>,
    destination: &MemStore,
    operation: u8,
) -> bumbledb_log::backup::BackupReport {
    let head = mirror.head();
    let live = head.control.live().expect("live");
    let recovery = head.recovery.expect("recovery");
    backup_root(
        mirror.backend,
        "t",
        destination,
        "vault",
        head.control.identity,
        live.state,
        &recovery,
        head.object_epoch,
        op(operation),
        LIMITS,
        ckpt_policy().stream,
        &work(),
    )
    .expect("backup completes")
}

#[test]
fn backup01_05_backup_verifies_and_restores_from_the_destination_only() {
    let store = MemStore::new();
    let destination = MemStore::new();
    let mirror = hosted_fixture("bk-roundtrip", &store);
    let report = run_backup(&mirror, &destination, 0x01);
    assert!(report.installed);
    assert_eq!(report.manifest.tip.seq, 3);
    assert_eq!(
        report.manifest.decisions.len(),
        1,
        "exactly the tail (S, T]"
    );
    // Full verification from the destination alone.
    let verified = verify_backup(
        &destination,
        "vault",
        op(0x01),
        LIMITS,
        ckpt_policy().stream,
        &work(),
    )
    .expect("verifies");
    assert_eq!(verified.objects_verified, report.objects_copied);
    // DESTROY the source completely: independence is actually tested
    // (BACKUP-03 shape — the origin/cache is gone).
    for key in store.object_keys() {
        store.delete_object(&key).expect("source object gone");
    }
    // Restore into a new incarnation from the destination only, reaching the
    // backed-up tip through the copied tail.
    let (manifest, digest) =
        read_backup_manifest(&destination, "vault", op(0x01)).expect("manifest reads");
    let checkpoint_ref = manifest.checkpoint.expect("checkpoint copied");
    let checkpoint_bytes = get_verified(&destination, "vault", &checkpoint_ref).expect("from dest");
    let checkpoint = bumbledb_log::codec::decode_manifest(&checkpoint_bytes, ckpt_policy().stream)
        .expect("decodes");
    let chunks: Vec<Result<Vec<u8>, bumbledb_log::recovery::RecoveryError>> = checkpoint
        .chunks
        .iter()
        .map(|chunk| {
            get_verified(&destination, "vault", chunk)
                .map_err(bumbledb_log::recovery::RecoveryError::Object)
        })
        .collect();
    let tail = read_backup_tail(&destination, "vault", &manifest, LIMITS, &work()).expect("tail");
    let target = temp_dir("bk-restore-target").join("db");
    let restored = restore_writable_with_tail(
        &target,
        theory(),
        &checkpoint,
        chunks,
        &tail,
        manifest.tip,
        IncarnationId::from_core(bumbledb::Id128::from_bytes([0xdd; 16])),
        op(0x0f),
        digest,
        "mem",
        "t-restored",
        LIMITS,
        &ckpt_policy(),
        HEAD_CAP,
        &work(),
    )
    .expect("restore reaches the tip");
    // Exact captured facts: all three users, byte-preserved entity values.
    let mut users = Vec::new();
    restored
        .db
        .read(|read| {
            for row in read.scan(RelationId(0)).expect("scan") {
                users.push(row.expect("row"));
            }
            Ok(())
        })
        .expect("read");
    assert_eq!(
        users.len(),
        3,
        "checkpoint facts plus the exact backed-up tail"
    );
    assert_eq!(restored.source_decision, manifest.tip);
    // RESTORE-01: new incarnation; old-scoped requests refuse; no lineage
    // remapping of application bytes.
    assert_ne!(
        restored.identity.incarnation_id,
        mirror.identity.incarnation_id
    );
    let history = LocalHistory::open(Arc::clone(&restored.db), LIMITS).expect("opens");
    let old_scope = insert_user(&restored.db, mirror.identity, 1, 10);
    match history.submit(&old_scope, &work()) {
        SubmitOutcome::NotSubmitted { error, .. } => assert_eq!(error, LogError::Identity),
        other => panic!("old scope refuses: {other:?}"),
    }
}

#[test]
fn backup02_incomplete_operations_are_never_listed_and_retry_is_idempotent() {
    let store = MemStore::new();
    let destination = MemStore::new();
    let mirror = hosted_fixture("bk-interrupt", &store);
    // Interrupt an object copy: the manifest is installed LAST, so the
    // operation is incomplete and unlisted.
    destination.fail_next(Op::PutObject, Behavior::Error);
    let head = mirror.head();
    let live = head.control.live().expect("live");
    let recovery = head.recovery.expect("recovery");
    let interrupted = backup_root(
        &store,
        "t",
        &destination,
        "vault",
        head.control.identity,
        live.state,
        &recovery,
        head.object_epoch,
        op(0x02),
        LIMITS,
        ckpt_policy().stream,
        &work(),
    );
    assert!(interrupted.is_err(), "the interrupted copy fails");
    let unlisted = read_backup_manifest(&destination, "vault", op(0x02));
    assert!(
        matches!(unlisted, Err(BackupError::Incomplete { .. })),
        "an incomplete backup is never listed as complete: {unlisted:?}"
    );
    // The operation-ID retry completes idempotently over the partial copy.
    let retried = run_backup(&mirror, &destination, 0x02);
    assert!(retried.installed);
    verify_backup(
        &destination,
        "vault",
        op(0x02),
        LIMITS,
        ckpt_policy().stream,
        &work(),
    )
    .expect("the retried operation verifies");
    // A second identical retry resolves by manifest digest: evidence, not a
    // second install and never an overwrite.
    let again = run_backup(&mirror, &destination, 0x02);
    assert!(!again.installed);
    assert_eq!(again.manifest_digest, retried.manifest_digest);
}

#[test]
fn backup02b_lost_completion_response_resolves_by_operation_and_digest() {
    let store = MemStore::new();
    let destination = MemStore::new();
    let mirror = hosted_fixture("bk-lostack", &store);
    // The completion create lands but its response is lost.
    destination.fail_next(Op::CreateHead, Behavior::IndeterminateApplied);
    let report = run_backup(&mirror, &destination, 0x03);
    assert!(
        !report.installed,
        "resolution by operation identity and manifest digest, not a claim"
    );
    verify_backup(
        &destination,
        "vault",
        op(0x03),
        LIMITS,
        ckpt_policy().stream,
        &work(),
    )
    .expect("the resolved backup verifies");
}

#[test]
fn backup04_corruption_wrong_operation_and_conflicts_refuse_with_evidence() {
    let store = MemStore::new();
    let destination = MemStore::new();
    let mirror = hosted_fixture("bk-corrupt", &store);
    let report = run_backup(&mirror, &destination, 0x04);
    // Corrupt one copied chunk in the DESTINATION: verification fails with
    // the precise object, before any restore activation.
    let checkpoint_ref = report.manifest.checkpoint.expect("checkpoint");
    let checkpoint_bytes = get_verified(&destination, "vault", &checkpoint_ref).expect("manifest");
    let checkpoint = bumbledb_log::codec::decode_manifest(&checkpoint_bytes, ckpt_policy().stream)
        .expect("decodes");
    let chunk_key = checkpoint.chunks[0].key("vault");
    assert!(destination.corrupt_object(&chunk_key, |bytes| bytes[7] ^= 0xff));
    let refused = verify_backup(
        &destination,
        "vault",
        op(0x04),
        LIMITS,
        ckpt_policy().stream,
        &work(),
    );
    assert!(refused.is_err(), "corrupt backup bytes refuse: {refused:?}");
    // A foreign manifest at the operation key refuses rather than resolving.
    let foreign_key = backup_manifest_key("vault", op(0x05));
    destination
        .create_head(&foreign_key, b"foreign bytes at the operation key")
        .expect("planted");
    let conflicting = read_backup_manifest(&destination, "vault", op(0x05));
    assert!(conflicting.is_err(), "{conflicting:?}");
}

#[test]
fn restore02_read_only_inspection_grants_no_mutation_capability() {
    let store = MemStore::new();
    let destination = MemStore::new();
    let mirror = hosted_fixture("bk-inspect", &store);
    let report = run_backup(&mirror, &destination, 0x06);
    let checkpoint_ref = report.manifest.checkpoint.expect("checkpoint");
    let checkpoint_bytes = get_verified(&destination, "vault", &checkpoint_ref).expect("manifest");
    let checkpoint = bumbledb_log::codec::decode_manifest(&checkpoint_bytes, ckpt_policy().stream)
        .expect("decodes");
    let chunks: Vec<Result<Vec<u8>, bumbledb_log::recovery::RecoveryError>> = checkpoint
        .chunks
        .iter()
        .map(|chunk| {
            get_verified(&destination, "vault", chunk)
                .map_err(bumbledb_log::recovery::RecoveryError::Object)
        })
        .collect();
    let scratch = temp_dir("bk-inspect-scratch").join("db");
    let inspection = inspect(
        &scratch,
        theory(),
        &checkpoint,
        chunks,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    )
    .expect("inspection materializes");
    // Original provenance and stamps are retained; reads work; the type
    // exposes NO write/submit surface (compile-time: `Inspection` has only
    // `read`/`provenance`).
    let (decision, _state) = inspection.provenance();
    assert_eq!(
        decision.seq, 2,
        "the checkpoint's original stamp, not a new lineage"
    );
    let mut count = 0;
    inspection
        .read(|read| {
            for row in read.scan(RelationId(0)).expect("scan") {
                row.expect("row");
                count += 1;
            }
            Ok(())
        })
        .expect("read");
    assert_eq!(count, 2, "the captured base state");
    // RESTORE-02's rewind arm: a writable restore into the SAME incarnation
    // refuses.
    let rewind = restore_writable_with_tail(
        &temp_dir("bk-rewind").join("db"),
        theory(),
        &checkpoint,
        Vec::<Result<Vec<u8>, bumbledb_log::recovery::RecoveryError>>::new(),
        &[],
        checkpoint.decision,
        mirror.identity.incarnation_id,
        op(0x07),
        [0; 32],
        "mem",
        "t",
        LIMITS,
        &ckpt_policy(),
        HEAD_CAP,
        &work(),
    );
    assert!(
        matches!(
            rewind,
            Err(bumbledb_log::restore::RestoreError::RewindRefused)
        ),
        "{:?}",
        rewind.as_ref().err()
    );
}

#[test]
fn restore03_restored_outbox_style_facts_document_duplicate_delivery_hazard() {
    // RESTORE-03: restoring old application idempotency facts makes
    // previously delivered actions appear pending again; the stable receiver
    // identity is the application's dedup key, not the database receipt.
    let store = MemStore::new();
    let destination = MemStore::new();
    let mut mirror = Mirror::create("bk-outbox", &store, "t");
    let identity = mirror.identity;
    // "Outbox row present" = pending delivery; the app later deletes it.
    mirror.submit(&insert_user(mirror.db(), identity, 1, 777));
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("checkpoint with the outbox row");
    let report = run_backup(&mirror, &destination, 0x08);
    // Delivery completes; the outbox row is removed from the live database.
    mirror.submit(&lane_support::delete_user(mirror.db(), identity, 2, 777));
    // Restore the backup: the outbox row is pending AGAIN in the new lineage.
    let checkpoint_ref = report.manifest.checkpoint.expect("checkpoint");
    let checkpoint_bytes = get_verified(&destination, "vault", &checkpoint_ref).expect("manifest");
    let checkpoint = bumbledb_log::codec::decode_manifest(&checkpoint_bytes, ckpt_policy().stream)
        .expect("decodes");
    let chunks: Vec<Result<Vec<u8>, bumbledb_log::recovery::RecoveryError>> = checkpoint
        .chunks
        .iter()
        .map(|chunk| {
            get_verified(&destination, "vault", chunk)
                .map_err(bumbledb_log::recovery::RecoveryError::Object)
        })
        .collect();
    let tail =
        read_backup_tail(&destination, "vault", &report.manifest, LIMITS, &work()).expect("tail");
    let restored = restore_writable_with_tail(
        &temp_dir("bk-outbox-target").join("db"),
        theory(),
        &checkpoint,
        chunks,
        &tail,
        report.manifest.tip,
        IncarnationId::from_core(bumbledb::Id128::from_bytes([0xee; 16])),
        op(0x09),
        report.manifest_digest,
        "mem",
        "t-restored",
        LIMITS,
        &ckpt_policy(),
        HEAD_CAP,
        &work(),
    )
    .expect("restore");
    let mut pending = 0;
    restored
        .db
        .read(|read| {
            for row in read.scan(RelationId(0)).expect("scan") {
                row.expect("row");
                pending += 1;
            }
            Ok(())
        })
        .expect("read");
    assert_eq!(
        pending, 1,
        "the restored outbox row appears pending again: the duplicate-delivery \
         hazard is real and the receiver's stable business identity must dedup"
    );
}
