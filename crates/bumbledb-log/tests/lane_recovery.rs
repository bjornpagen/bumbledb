//! Recovery: ownership before cleanup, identity before reads, hydration from
//! checkpoint plus exact tail, differentiated corruption, and never an empty
//! fallback — REC-01..06, STORE-05/08 shapes, GC-12's no-partial-serve rule
//! (REP-005/010/011/017/018, SDK-016). Process-kill variants are P12's F3
//! harness over these entry points. Verification: `NotRun` (F1 authors, does
//! not execute).

mod lane_support;

use std::sync::Arc;

use bumbledb::RelationId;
use bumbledb_log::admin;
use bumbledb_log::checkpointer::{CheckpointKind, CheckpointPolicy, publish_checkpoint};
use bumbledb_log::history::authority::DeletedReason;
use bumbledb_log::recovery::{
    OriginBinding, RecoveryError, RecoveryRefusal, create_local, materialization_path, open_hosted,
    open_local,
};
use bumbledb_log::store::get_verified;
use bumbledb_log::store::mem::MemStore;
use bumbledb_log::writer::{LocalHistory, ResolveOutcome};
use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, op, temp_dir, theory, work};

fn ckpt_policy() -> CheckpointPolicy {
    CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: HEAD_CAP,
        ..CheckpointPolicy::DEFAULT
    }
}

fn count_users(db: &bumbledb::Db<bumbledb::SchemaDescriptor>) -> usize {
    let mut count = 0;
    db.read(|read| {
        for row in read.scan(RelationId(0)).expect("scan") {
            row.expect("row");
            count += 1;
        }
        Ok(())
    })
    .expect("read");
    count
}

/// A mirrored tenant with a checkpoint at decision 2 and a tail decision 3.
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

#[test]
fn cold_hydration_builds_checkpoint_plus_exact_tail_and_resolves_receipts() {
    // REC-01/02: only a complete identified published snapshot becomes Ready,
    // and a fresh host resolves the original receipt without re-execution.
    let store = MemStore::new();
    let mirror = hosted_fixture("rec-hydrate", &store);
    let dir = temp_dir("rec-hydrate-target");
    let recovered = open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    )
    .expect("cold hydration succeeds");
    assert_eq!(
        count_users(&recovered.db),
        3,
        "checkpoint facts plus the exact tail"
    );
    // The hydrated materialization is a working LocalHistory cache: the
    // original command's receipt resolves without re-execution.
    let history = LocalHistory::open(Arc::clone(&recovered.db), LIMITS).expect("hydrated opens");
    let position = history
        .authority()
        .expect("authority")
        .position()
        .expect("live");
    assert_eq!(position.decision.seq, 3, "replay reached the captured tip");
    let command = insert_user(&recovered.db, mirror.identity, 3, 30);
    match history
        .resolve(command.command_ref(), &work())
        .expect("resolve")
    {
        ResolveOutcome::Found(found) => {
            assert_eq!(found.decision_at.seq, 3);
        }
        other => panic!("the hydrated cache resolves the original receipt: {other:?}"),
    }
    // The ready materialization landed at the canonical path; no staging
    // directory remains.
    assert!(materialization_path(&dir).exists());
    let staging: Vec<_> = std::fs::read_dir(&dir)
        .expect("list")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("staging-"))
        .collect();
    assert!(
        staging.is_empty(),
        "no staging scratch survives a completed hydration"
    );
}

#[test]
fn missing_head_is_database_missing_and_open_never_initializes() {
    let store = MemStore::new();
    let dir = temp_dir("rec-missing");
    let refused = open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    );
    assert!(
        matches!(
            refused,
            Err(RecoveryError::Refused(RecoveryRefusal::DatabaseMissing))
        ),
        "{:?}",
        refused.as_ref().err()
    );
    assert!(
        !materialization_path(&dir).exists(),
        "a refused open creates nothing"
    );
}

#[test]
fn deleted_authority_refuses_ordinary_open_before_hydration() {
    let store = MemStore::new();
    let mirror = hosted_fixture("rec-deleted", &store);
    admin::tombstone_hosted(
        &store,
        "t",
        op(0xd1),
        DeletedReason::Erasure,
        HEAD_CAP,
        &work(),
    )
    .expect("tombstone");
    let dir = temp_dir("rec-deleted-target");
    let refused = open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    );
    assert!(
        matches!(
            refused,
            Err(RecoveryError::Refused(RecoveryRefusal::DatabaseDeleted))
        ),
        "{:?}",
        refused.as_ref().err()
    );
    let _ = mirror;
}

#[test]
fn foreign_and_unidentified_caches_refuse_before_any_read_or_cleanup() {
    // REC-03/STORE-08: matching row counts/schema/generations never establish
    // identity; a cache hydrated for one origin refuses under another.
    let store = MemStore::new();
    let _mirror = hosted_fixture("rec-foreign", &store);
    let dir = temp_dir("rec-foreign-target");
    open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    )
    .expect("first hydration");
    // The same directory under a DIFFERENT configured origin string refuses.
    let refused = open_hosted(
        &dir,
        theory(),
        &store,
        "other-origin",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    );
    assert!(
        matches!(
            refused,
            Err(RecoveryError::Refused(RecoveryRefusal::ForeignCache { .. }))
        ),
        "{:?}",
        refused.as_ref().err()
    );
    // The materialization is untouched by the refusal and still opens under
    // its true origin.
    let again = open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    )
    .expect("true origin still opens");
    assert_eq!(count_users(&again.db), 3);
}

#[test]
fn corrupt_authoritative_chunk_stops_hydration_with_evidence_never_empty() {
    // REC-06/STORE-05: a corrupt authoritative object is a stopped tenant
    // with evidence — no readable partial cache, no empty fallback, no
    // delete/reseed loop.
    let store = MemStore::new();
    let mirror = hosted_fixture("rec-corrupt", &store);
    let reference = mirror
        .head()
        .recovery
        .expect("recovery")
        .checkpoint
        .expect("checkpoint");
    let manifest_bytes = get_verified(&store, "t", &reference).expect("manifest");
    let manifest = bumbledb_log::codec::decode_manifest(&manifest_bytes, ckpt_policy().stream)
        .expect("decodes");
    let chunk_key = manifest.chunks[0].key("t");
    assert!(store.corrupt_object(&chunk_key, |bytes| bytes[10] ^= 0xff));
    let dir = temp_dir("rec-corrupt-target");
    let refused = open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    );
    assert!(refused.is_err(), "corrupt chunk refuses");
    assert!(
        !materialization_path(&dir).exists(),
        "an interrupted candidate stays invisible staging, never a ready cache"
    );
}

#[test]
fn a_second_open_refuses_while_the_directory_is_owned_and_mutates_nothing() {
    // FS-04/RUN-05 shape (in-process arm; the subprocess arm lives in
    // local_ownership.rs): a competing open performs zero mutation.
    let store = MemStore::new();
    let _mirror = hosted_fixture("rec-owned", &store);
    let dir = temp_dir("rec-owned-target");
    let held = open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    )
    .expect("first open owns");
    let refused = open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    );
    assert!(
        matches!(
            refused,
            Err(RecoveryError::Refused(RecoveryRefusal::AlreadyOwned))
        ),
        "{:?}",
        refused.as_ref().err()
    );
    drop(held);
    // After the owner drops (lock released last), the directory opens again.
    open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "t",
        LIMITS,
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    )
    .expect("reopen after release");
}

#[test]
fn local_create_is_explicit_and_local_open_needs_no_remote_machinery() {
    // LOCAL-03 shape: LocalHistory recovery is the committed LMDB state; no
    // remote tail envelope or replay checkpoint exists merely to reopen.
    let dir = temp_dir("rec-local");
    let binding = OriginBinding {
        origin: "local".into(),
        prefix: "t".into(),
        identity: bumbledb_log::history::DatabaseIdentity {
            database_id: bumbledb_log::history::DatabaseId::from_core(bumbledb::Id128::from_bytes(
                [0xa1; 16],
            )),
            incarnation_id: bumbledb_log::history::IncarnationId::from_core(
                bumbledb::Id128::from_bytes([0xb2; 16]),
            ),
            schema_id: bumbledb::schema::fingerprint::fingerprint(
                bumbledb::Db::create(&temp_dir("rec-local-fp").join("db"), theory())
                    .expect("create")
                    .expect("admits")
                    .schema(),
            ),
        },
    };
    // Open of a missing local database refuses; creation is separate.
    let missing = open_local(&dir, theory(), &work());
    assert!(
        matches!(
            missing,
            Err(RecoveryError::Refused(RecoveryRefusal::DatabaseMissing))
        ),
        "{:?}",
        missing.as_ref().err()
    );
    let (lock, db) = create_local(&dir, theory(), &binding, &work()).expect("explicit creation");
    let identity = binding.identity;
    let history = LocalHistory::create(
        Arc::clone(&db),
        identity.database_id,
        identity.incarnation_id,
        op(0xe1),
        LIMITS,
        &work(),
    )
    .expect("history creates");
    let command = insert_user(&db, identity, 1, 10);
    let receipt = match history.submit(&command, &work()) {
        bumbledb_log::writer::SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("{other:?}"),
    };
    drop(history);
    drop(db);
    drop(lock);
    // Reopen: LMDB alone recovers facts, receipts and the head attachment.
    let (_lock, db) = open_local(&dir, theory(), &work()).expect("reopen");
    assert_eq!(count_users(&db), 1);
    let history = LocalHistory::open(Arc::clone(&db), LIMITS).expect("history reopens");
    match history.resolve(receipt.command, &work()).expect("resolve") {
        ResolveOutcome::Found(found) => assert_eq!(found, receipt),
        other => panic!("the original receipt survives reopen: {other:?}"),
    }
}
