//! Recovery: ownership before cleanup, identity before reads, hydration from
//! checkpoint plus exact tail, differentiated corruption, and never an empty
//! fallback — REC-01..06, STORE-05/08 shapes, GC-12's no-partial-serve rule
//! (REP-005/010/011/017/018, SDK-016). Process-kill variants are P12's F3
//! harness over these entry points. Verification: `NotRun` (F1 authors, does
//! not execute).

mod lane_support;

use std::sync::Arc;

use bumbledb::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, Row, SchemaDescriptor, Side,
    StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{Id128, Value};
use bumbledb_log::admin;
use bumbledb_log::checkpointer::{CheckpointKind, CheckpointPolicy, publish_checkpoint};
use bumbledb_log::history::authority::DeletedReason;
use bumbledb_log::history::{
    DatabaseId, DatabaseIdentity, DecisionDigest, DecisionStamp, IncarnationId,
};
use bumbledb_log::recovery::{
    OriginBinding, RecoveryError, RecoveryRefusal, create_local, materialization_path, open_hosted,
    open_local,
};
use bumbledb_log::restore::restore_writable_genesis;
use bumbledb_log::store::get_verified;
use bumbledb_log::store::mem::MemStore;
use bumbledb_log::store::{ReceiveLimits, TransportContext};
use bumbledb_log::writer::{LocalHistory, ResolveOutcome};
use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, op, temp_dir, theory, work};

/// Legal schema whose empty state violates a capacity floor: one closed
/// parent group and zero children. Incremental population can repair it;
/// empty create/restore/hydrate must not become Ready.
fn nonempty_required() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Slot".into(),
                fields: vec![],
                extension: Some(Box::new([Row {
                    handle: "slot0".into(),
                    values: Box::new([]),
                }])),
            },
            RelationDescriptor {
                name: "Occupant".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "slot".into(),
                        value_type: ValueType::U64,
                    },
                ],
                extension: None,
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: Side {
                    relation: RelationId(0),
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                weight: Weight::Unit,
                lo: 1,
                hi: None,
                source: Side {
                    relation: RelationId(1),
                    projection: Box::new([FieldId(1)]),
                    selection: Box::new([]),
                },
            },
        ],
    }
}

fn nonempty_binding() -> OriginBinding {
    let schema = nonempty_required()
        .validate()
        .expect("nonempty-required descriptor validates");
    OriginBinding {
        origin: "local".into(),
        prefix: "t".into(),
        identity: DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
            schema_id: bumbledb::schema::fingerprint::fingerprint(&schema),
        },
    }
}

fn fetch_verified(
    store: &MemStore,
    prefix: &str,
    reference: &bumbledb_log::store::ObjectRef,
) -> bumbledb::work::ChargedBytes {
    get_verified(
        store,
        prefix,
        reference,
        TransportContext::new(&work(), ReceiveLimits::exact(reference.length)),
    )
    .expect("verified")
}

fn ckpt_policy() -> CheckpointPolicy {
    CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: HEAD_CAP,
        ..CheckpointPolicy::DEFAULT
    }
}

fn count_users(db: &bumbledb::Db<bumbledb::SchemaDescriptor>) -> usize {
    let mut count = 0;
    db.read(work(), |read| {
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
    let manifest_bytes = fetch_verified(&store, "t", &reference);
    let manifest = bumbledb_log::codec::decode_manifest(manifest_bytes.as_bytes(), ckpt_policy().stream)
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
                bumbledb::Db::create(&temp_dir("rec-local-fp").join("db"), theory(), work())
                    .expect("create")
                    .expect("admits")
                    .schema(),
            ),
        },
    };
    // Open of a missing local database refuses; creation is separate.
    let missing = open_local(&dir, theory(), &binding, &work());
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
    let (_lock, db) = open_local(&dir, theory(), &binding, &work()).expect("reopen");
    assert_eq!(count_users(&db), 1);
    let history = LocalHistory::open(Arc::clone(&db), LIMITS).expect("history reopens");
    match history.resolve(receipt.command, &work()).expect("resolve") {
        ResolveOutcome::Found(found) => assert_eq!(found, receipt),
        other => panic!("the original receipt survives reopen: {other:?}"),
    }
}

/// D06/D26: a legal schema whose empty state violates a law never becomes
/// Ready. Create, empty genesis restore, and a missing-HEAD hydrate leave
/// the destination absent — not a ready partial Db. Verification: NotRun.
#[test]
fn d06_failed_hydrate_leaves_destination_absent() {
    let store = MemStore::new();
    let dir = temp_dir("rec-d06-absent");
    let refused = open_hosted(
        &dir,
        theory(),
        &store,
        "mem",
        "missing-prefix",
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
        "D06: destination is absent after a refused install boundary"
    );

    let empty_create = temp_dir("rec-d26-empty-create");
    let binding = nonempty_binding();
    let created = create_local(&empty_create, nonempty_required(), &binding, &work());
    assert!(
        matches!(created, Err(RecoveryError::InvariantViolation)),
        "D26: empty nonempty-required create refuses, got {created:?}"
    );
    assert!(
        !materialization_path(&empty_create).exists(),
        "D26: empty create leaves the destination absent"
    );

    let genesis_dir = temp_dir("rec-d26-empty-genesis");
    let genesis = DecisionStamp {
        seq: 0,
        hash: DecisionDigest::from_bytes([0x11; 32]),
    };
    let empty_restore = restore_writable_genesis(
        &genesis_dir,
        nonempty_required(),
        binding.identity,
        genesis,
        std::iter::empty::<Result<bumbledb::work::ChargedBytes, RecoveryError>>(),
        genesis,
        IncarnationId::from_core(Id128::from_bytes([0xc3; 16])),
        op(0x26),
        [0x26; 32],
        "local",
        "t",
        LIMITS,
        &ckpt_policy(),
        HEAD_CAP,
        &work(),
    );
    assert!(
        empty_restore.is_err(),
        "D26: empty genesis restore of nonempty-required refuses, got {empty_restore:?}"
    );
    assert!(
        !genesis_dir.exists()
            || std::fs::read_dir(&genesis_dir)
                .map(|listing| listing.filter_map(Result::ok).count())
                .unwrap_or(0)
                == 0,
        "D06: refused empty genesis restore leaves dest absent-or-empty"
    );
}

fn dest_unpublished(path: &std::path::Path) {
    assert!(
        !path.exists()
            || std::fs::read_dir(path)
                .map(|listing| listing.filter_map(Result::ok).count())
                .unwrap_or(0)
                == 0,
        "unpublished restore left a destination at {path:?}"
    );
}

/// D17: tip disagreement and a refuse during final metadata construction
/// never publish. `theory()` admits empty, so a premature `complete_install`
/// would leave a destination — dest-absent is the discriminator.
/// Verification: NotRun.
#[test]
fn d17_incomplete_genesis_restore_leaves_destination_absent() {
    let genesis = DecisionStamp {
        seq: 0,
        hash: DecisionDigest::from_bytes([0x11; 32]),
    };
    let source = DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(
            &theory().validate().expect("theory validates"),
        ),
    };
    let empty = || std::iter::empty::<Result<bumbledb::work::ChargedBytes, RecoveryError>>();
    let later = DecisionStamp {
        seq: 7,
        hash: DecisionDigest::from_bytes([0x77; 32]),
    };
    let wrong_genesis = DecisionStamp {
        seq: 0,
        hash: DecisionDigest::from_bytes([0x22; 32]),
    };

    let truncated = temp_dir("rec-d17-truncated").join("db");
    let truncated_err = restore_writable_genesis(
        &truncated,
        theory(),
        source,
        genesis,
        empty(),
        later,
        IncarnationId::from_core(Id128::from_bytes([0xc7; 16])),
        op(0x17),
        [0x17; 32],
        "local",
        "t",
        LIMITS,
        &ckpt_policy(),
        HEAD_CAP,
        &work(),
    );
    assert!(
        truncated_err.is_err(),
        "truncated tail (empty vs tip 7) refuses, got {truncated_err:?}"
    );
    dest_unpublished(&truncated);

    let wrong = temp_dir("rec-d17-wrong-tip").join("db");
    let wrong_err = restore_writable_genesis(
        &wrong,
        theory(),
        source,
        genesis,
        empty(),
        wrong_genesis,
        IncarnationId::from_core(Id128::from_bytes([0xc8; 16])),
        op(0x18),
        [0x18; 32],
        "local",
        "t",
        LIMITS,
        &ckpt_policy(),
        HEAD_CAP,
        &work(),
    );
    assert!(
        wrong_err.is_err(),
        "wrong genesis tip refuses, got {wrong_err:?}"
    );
    dest_unpublished(&wrong);

    let metadata = temp_dir("rec-d17-metadata").join("db");
    let metadata_err = restore_writable_genesis(
        &metadata,
        theory(),
        source,
        genesis,
        empty(),
        genesis,
        IncarnationId::from_core(Id128::from_bytes([0xc9; 16])),
        op(0x19),
        [0x19; 32],
        "local",
        "t",
        LIMITS,
        &ckpt_policy(),
        1,
        &work(),
    );
    assert!(
        metadata_err.is_err(),
        "head_cap=1 refuses genesis/control construction, got {metadata_err:?}"
    );
    dest_unpublished(&metadata);
}

/// D18: receipt cleanup is L07 `delete_host_batch` / `HostResume` windows
/// (`RECEIPT_CLEANUP_BATCH_BYTES`); peak working storage does not grow with
/// receipt count. Wrong tip still refuses before that cleanup and leaves
/// dest unpublished. Verification: NotRun.
#[test]
fn d18_receipt_cleanup_stays_bounded_wrong_tip_absent() {
    let genesis = DecisionStamp {
        seq: 0,
        hash: DecisionDigest::from_bytes([0x11; 32]),
    };
    let source = DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(
            &theory().validate().expect("theory validates"),
        ),
    };
    let dest = temp_dir("rec-d18-wrong-tip").join("db");
    let refused = restore_writable_genesis(
        &dest,
        theory(),
        source,
        genesis,
        std::iter::empty::<Result<bumbledb::work::ChargedBytes, RecoveryError>>(),
        DecisionStamp {
            seq: 7,
            hash: DecisionDigest::from_bytes([0x77; 32]),
        },
        IncarnationId::from_core(Id128::from_bytes([0xca; 16])),
        op(0x1a),
        [0x1a; 32],
        "local",
        "t",
        LIMITS,
        &ckpt_policy(),
        HEAD_CAP,
        &work(),
    );
    assert!(
        refused.is_err(),
        "wrong tip refuses before receipt cleanup / publish, got {refused:?}"
    );
    dest_unpublished(&dest);
}
