//! `LocalHistory` named restore points: complete-then-register, owner-scoped
//! cleanup, never-reused root directories, and restore preserving the point's
//! original evidence — LOCAL-01..03 (REP-003 local arm). Kill-based variants
//! are P12's F3 process harness over these same boundaries. Verification:
//! `NotRun` (F1 authors, does not execute).

mod lane_support;

use std::sync::Arc;

use bumbledb::RelationId;
use bumbledb_log::checkpointer::CheckpointPolicy;
use bumbledb_log::history::IncarnationId;
use bumbledb_log::local_roots::{
    LocalRootError, clean_roots, create_restore_point, read_point_chunks, read_point_manifest,
    registered_roots, release_restore_point, root_directory, roots_base,
};
use bumbledb_log::manifest::RootPolicy;
use bumbledb_log::restore::restore_writable;
use bumbledb_log::writer::{LocalHistory, SubmitOutcome};
use lane_support::{
    HEAD_CAP, LIMITS, delete_user, fresh_db, insert_user, op, temp_dir, test_identity, theory, work,
};

fn ckpt_policy() -> CheckpointPolicy {
    CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: HEAD_CAP,
        ..CheckpointPolicy::DEFAULT
    }
}

struct Fixture {
    dir: std::path::PathBuf,
    db: Arc<bumbledb::Db<bumbledb::SchemaDescriptor>>,
    history: LocalHistory<bumbledb::SchemaDescriptor>,
    identity: bumbledb_log::history::DatabaseIdentity,
}

fn fixture(tag: &str) -> Fixture {
    let dir = temp_dir(tag);
    let db = fresh_db(tag);
    let identity = test_identity(&db);
    let history = LocalHistory::create(
        Arc::clone(&db),
        identity.database_id,
        identity.incarnation_id,
        op(0xc3),
        LIMITS,
        &work(),
    )
    .expect("history creates");
    Fixture {
        dir,
        db,
        history,
        identity,
    }
}

fn submit(fixture: &Fixture, command: &bumbledb_log::history::command::Command) {
    match fixture.history.submit(command, &work()) {
        SubmitOutcome::Decided { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn local01_registered_points_are_complete_and_unregistered_scratch_is_not_a_point() {
    let fixture = fixture("lr-complete");
    submit(&fixture, &insert_user(&fixture.db, fixture.identity, 1, 10));
    submit(&fixture, &insert_user(&fixture.db, fixture.identity, 2, 20));
    let root = create_restore_point(
        &fixture.db,
        &fixture.dir,
        op(0x01),
        "before-change",
        &ckpt_policy(),
        &RootPolicy::DEFAULT,
        &work(),
    )
    .expect("point registers");
    assert_eq!(root.decision.seq, 2);
    // The registered point is complete on disk and verifies.
    let manifest =
        read_point_manifest(&fixture.dir, &root, ckpt_policy().stream).expect("manifest");
    assert_eq!(manifest.rows, 2);
    for chunk in read_point_chunks(&fixture.dir, &root, &manifest) {
        chunk.expect("chunk verifies");
    }
    // An unregistered complete-looking directory is scratch: fabricate one
    // and reopen-time cleanup removes it while every registered root stays.
    let fake = roots_base(&fixture.dir).join("00000000000000000000000000000000");
    std::fs::create_dir_all(&fake).expect("fake dir");
    std::fs::write(fake.join("manifest"), b"not a restore point").expect("fake manifest");
    clean_roots(&fixture.db, &fixture.dir).expect("owner-scoped cleanup");
    assert!(
        !fake.exists(),
        "unregistered scratch is not a restore point"
    );
    assert!(
        root_directory(&fixture.dir, root.id).exists(),
        "registered points survive cleanup"
    );
    read_point_manifest(&fixture.dir, &root, ckpt_policy().stream).expect("still verifies");
}

#[test]
fn local02_release_is_transactional_and_failed_deletion_resumes_without_cross_damage() {
    let fixture = fixture("lr-release");
    submit(&fixture, &insert_user(&fixture.db, fixture.identity, 1, 10));
    let keep = create_restore_point(
        &fixture.db,
        &fixture.dir,
        op(0x01),
        "keep",
        &ckpt_policy(),
        &RootPolicy::DEFAULT,
        &work(),
    )
    .expect("keep registers");
    submit(&fixture, &insert_user(&fixture.db, fixture.identity, 2, 20));
    let drop_me = create_restore_point(
        &fixture.db,
        &fixture.dir,
        op(0x02),
        "drop",
        &ckpt_policy(),
        &RootPolicy::DEFAULT,
        &work(),
    )
    .expect("drop registers");
    // Distinct root directories share no collectible files.
    assert_ne!(
        root_directory(&fixture.dir, keep.id),
        root_directory(&fixture.dir, drop_me.id)
    );
    release_restore_point(&fixture.db, &fixture.dir, drop_me.id, &work()).expect("release");
    assert!(!root_directory(&fixture.dir, drop_me.id).exists());
    // A repeated release refuses (the entry is gone) and cannot remove keep.
    let stale = release_restore_point(&fixture.db, &fixture.dir, drop_me.id, &work());
    assert!(
        matches!(stale, Err(LocalRootError::UnknownRoot)),
        "{stale:?}"
    );
    assert!(root_directory(&fixture.dir, keep.id).exists());
    // Simulate a failed directory deletion: re-create the released dir (as a
    // dead process's leftover); reopen-time cleanup removes exactly it.
    let leftover = root_directory(&fixture.dir, drop_me.id);
    std::fs::create_dir_all(&leftover).expect("leftover");
    std::fs::write(leftover.join("manifest"), b"stale").expect("stale manifest");
    clean_roots(&fixture.db, &fixture.dir).expect("cleanup resumes");
    assert!(
        !leftover.exists(),
        "released-but-undeleted directories are resumed"
    );
    assert!(
        root_directory(&fixture.dir, keep.id).exists(),
        "other roots untouched"
    );
    let manifest =
        read_point_manifest(&fixture.dir, &keep, ckpt_policy().stream).expect("keep intact");
    for chunk in read_point_chunks(&fixture.dir, &keep, &manifest) {
        chunk.expect("keep chunks intact");
    }
    // Root IDs are never reused.
    let reuse = create_restore_point(
        &fixture.db,
        &fixture.dir,
        op(0x01),
        "reuse",
        &ckpt_policy(),
        &RootPolicy::DEFAULT,
        &work(),
    );
    assert!(
        matches!(reuse, Err(LocalRootError::DuplicateRoot)),
        "{reuse:?}"
    );
}

#[test]
fn local03_an_old_point_preserves_its_original_evidence_and_restores_a_new_lineage() {
    // The database moves on (including deletions); the old point still
    // contains its original facts, and a writable restore from it creates a
    // NEW incarnation with preserved application bytes.
    let fixture = fixture("lr-restore");
    submit(&fixture, &insert_user(&fixture.db, fixture.identity, 1, 10));
    submit(&fixture, &insert_user(&fixture.db, fixture.identity, 2, 20));
    let point = create_restore_point(
        &fixture.db,
        &fixture.dir,
        op(0x01),
        "evidence",
        &ckpt_policy(),
        &RootPolicy::DEFAULT,
        &work(),
    )
    .expect("point registers");
    // Later: one user is deleted from the live database.
    submit(&fixture, &delete_user(&fixture.db, fixture.identity, 3, 10));
    // The point still holds both users.
    let manifest =
        read_point_manifest(&fixture.dir, &point, ckpt_policy().stream).expect("manifest");
    assert_eq!(
        manifest.rows, 2,
        "the old point preserves its original evidence"
    );
    // Writable restore: new incarnation, preserved bytes, empty receipts.
    let chunks: Vec<Result<Vec<u8>, bumbledb_log::recovery::RecoveryError>> =
        read_point_chunks(&fixture.dir, &point, &manifest)
            .map(|chunk| chunk.map_err(|_| bumbledb_log::recovery::RecoveryError::Corrupt("chunk")))
            .collect();
    let target = temp_dir("lr-restore-target").join("db");
    let restored = restore_writable(
        &target,
        theory(),
        &manifest,
        chunks,
        IncarnationId::from_core(bumbledb::Id128::from_bytes([0xdd; 16])),
        op(0x0f),
        point.manifest_digest,
        "local",
        "t-restored",
        ckpt_policy().stream,
        HEAD_CAP,
        &work(),
    )
    .expect("restore succeeds");
    assert_ne!(
        restored.identity.incarnation_id, fixture.identity.incarnation_id,
        "a writable restore is a new lineage, never a rewind"
    );
    let mut count = 0;
    restored
        .db
        .read(|read| {
            for row in read.scan(RelationId(0)).expect("scan") {
                row.expect("row");
                count += 1;
            }
            Ok(())
        })
        .expect("read");
    assert_eq!(count, 2, "application bytes preserved from the point");
    // The new incarnation's executable receipt table is empty: the source's
    // command resolves as not-recorded here, and its old-incarnation scope
    // refuses admission.
    let history = LocalHistory::open(Arc::clone(&restored.db), LIMITS).expect("restored opens");
    let old_scope = insert_user(&restored.db, fixture.identity, 1, 10);
    match history.submit(&old_scope, &work()) {
        SubmitOutcome::NotSubmitted { error, .. } => {
            assert_eq!(error, bumbledb_log::writer::LogError::Identity);
        }
        other => panic!("old-incarnation scope refuses: {other:?}"),
    }
}

#[test]
fn capacity_and_labels_are_bounded_without_discarding_other_roots() {
    let fixture = fixture("lr-capacity");
    submit(&fixture, &insert_user(&fixture.db, fixture.identity, 1, 10));
    let tight = RootPolicy {
        max_roots: 1,
        max_label_bytes: 8,
    };
    create_restore_point(
        &fixture.db,
        &fixture.dir,
        op(0x01),
        "ok",
        &ckpt_policy(),
        &tight,
        &work(),
    )
    .expect("first registers");
    let full = create_restore_point(
        &fixture.db,
        &fixture.dir,
        op(0x02),
        "over",
        &ckpt_policy(),
        &tight,
        &work(),
    );
    assert!(
        matches!(full, Err(LocalRootError::RootCapacityExceeded)),
        "{full:?}"
    );
    assert_eq!(registered_roots(&fixture.db).expect("registry").len(), 1);
    let long_label = create_restore_point(
        &fixture.db,
        &fixture.dir,
        op(0x03),
        "a label far beyond eight bytes",
        &ckpt_policy(),
        &RootPolicy {
            max_roots: 4,
            max_label_bytes: 8,
        },
        &work(),
    );
    assert!(long_label.is_err(), "labels are bounded metadata");
}
