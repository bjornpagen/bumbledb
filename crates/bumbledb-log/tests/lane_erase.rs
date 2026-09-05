//! Erasure via the Deleted authority — ERASE-01..04 (OPS-002/003 boundary).
//! Fact deletion is an ordinary command; whole-tenant erasure tombstones,
//! honors explicitly retained roots, collects former live objects, and
//! reports residuals instead of claiming secure erasure. Verification:
//! `NotRun` (F1 authors, does not execute).

mod lane_support;

use bumbledb_log::admin;
use bumbledb_log::checkpointer::{CheckpointKind, CheckpointPolicy, publish_checkpoint};
use bumbledb_log::erase::{erase_hosted, erase_local, residual_report};
use bumbledb_log::gc::GcPolicy;
use bumbledb_log::history::authority::{DeleteOutcome, DeletedReason};
use bumbledb_log::manifest::{RootKind, RootPolicy};
use bumbledb_log::store::mem::MemStore;
use lane_support::{HEAD_CAP, LIMITS, Mirror, delete_user, insert_user, op, work};

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

#[test]
fn erase01_fact_deletion_is_a_command_and_history_retains_until_release() {
    // ERASE-01: delete facts, then rebuild/checkpoint — the current logical
    // state excludes the values while the named root still retains them.
    let store = MemStore::new();
    let mut mirror = Mirror::create("er-facts", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 42));
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("checkpoint with the value");
    let pinned = admin::add_named_root_hosted(
        &store,
        "t",
        op(0x11),
        RootKind::RestorePoint,
        "with-42",
        op(0x12),
        &RootPolicy::DEFAULT,
        HEAD_CAP,
        &work(),
    )
    .expect("pin");
    // Ordinary admitted deletion, then a new checkpoint of the current state.
    mirror.submit(&delete_user(mirror.db(), identity, 2, 42));
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("checkpoint without the value");
    let current_ref = mirror
        .head()
        .recovery
        .expect("recovery")
        .checkpoint
        .expect("ckpt");
    let current_bytes =
        bumbledb_log::store::get_verified(&store, "t", &current_ref).expect("current manifest");
    let current = bumbledb_log::codec::decode_manifest(&current_bytes, ckpt_policy().stream)
        .expect("decodes");
    assert_eq!(
        current.rows, 0,
        "the current logical state excludes the value"
    );
    // The retained root still holds the old state until explicit release.
    let old_ref = pinned.recovery.checkpoint.expect("pinned ckpt");
    let old_bytes = bumbledb_log::store::get_verified(&store, "t", &old_ref).expect("old manifest");
    let old =
        bumbledb_log::codec::decode_manifest(&old_bytes, ckpt_policy().stream).expect("decodes");
    assert_eq!(
        old.rows, 1,
        "retained history intentionally keeps the value"
    );
}

#[test]
fn erase02_tombstone_prevents_publication_preserves_retained_roots_then_collects() {
    let store = MemStore::new();
    let mut mirror = Mirror::create("er-tenant", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
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
    let retained = admin::add_named_root_hosted(
        &store,
        "t",
        op(0x21),
        RootKind::RestorePoint,
        "compliance-hold",
        op(0x22),
        &RootPolicy::DEFAULT,
        HEAD_CAP,
        &work(),
    )
    .expect("retained root");
    let report = erase_hosted(&store, "t", op(0x23), &[], LIMITS, &gc_policy(), &work())
        .expect("erasure runs");
    assert!(matches!(report.tombstone, DeleteOutcome::Deleted(_)));
    assert!(report.residual.head_tombstone_retained);
    assert_eq!(
        report.residual.retained_roots.len(),
        1,
        "explicit roots are honored"
    );
    // The retained root's closure survived collection.
    let old_ref = retained.recovery.checkpoint.expect("ckpt");
    bumbledb_log::store::get_verified(&store, "t", &old_ref).expect("retained closure survives");
    // A delayed old writer cannot publish: ordinary admission refuses on the
    // tombstone, and its stale exact-version CAS loses on the moved head.
    let refused = admin::fence_revision_hosted(&store, "t", HEAD_CAP, &work());
    assert!(
        refused.is_err(),
        "no maintenance revives a tombstone: {refused:?}"
    );
    // The erasure operation retried is evidence, not a second transition.
    let again = erase_hosted(&store, "t", op(0x23), &[], LIMITS, &gc_policy(), &work())
        .expect("idempotent retry");
    assert!(matches!(
        again.tombstone,
        DeleteOutcome::AlreadyDeleted { .. }
    ));
    // Release the retained root and collect again: former live objects go,
    // the tombstone itself remains.
    admin::release_named_root_hosted(&store, "t", op(0x21), false, HEAD_CAP, &work())
        .expect("explicit release");
    let final_report = erase_hosted(&store, "t", op(0x23), &[], LIMITS, &gc_policy(), &work())
        .expect("later pass");
    assert!(
        bumbledb_log::store::get_verified(&store, "t", &old_ref).is_err(),
        "released closure is collected in a later pass"
    );
    assert!(
        final_report.residual.head_tombstone_retained,
        "the tombstone remains"
    );
}

#[test]
fn erase03_residuals_are_reported_never_a_secure_erasure_claim() {
    let store = MemStore::new();
    let mut mirror = Mirror::create("er-residual", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
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
    admin::add_named_root_hosted(
        &store,
        "t",
        op(0x31),
        RootKind::RestorePoint,
        "retained",
        op(0x32),
        &RootPolicy::DEFAULT,
        HEAD_CAP,
        &work(),
    )
    .expect("retained root");
    let report =
        erase_hosted(&store, "t", op(0x33), &[], LIMITS, &gc_policy(), &work()).expect("erasure");
    // The report enumerates what actually remains rather than claiming zero.
    assert!(
        report.residual.remaining_objects > 0,
        "the retained closure remains and is counted"
    );
    assert!(report.residual.backups_exports_blobs_keys_untouched);
    // The standalone residual inventory agrees.
    let standalone = residual_report(&store, "t", &gc_policy(), &work()).expect("inventory");
    assert_eq!(
        standalone.remaining_objects,
        report.residual.remaining_objects
    );
    assert_eq!(standalone.retained_roots.len(), 1);
}

#[test]
fn erase02_policy_allowed_roots_release_and_erasure_collects_everything_else() {
    let store = MemStore::new();
    let mut mirror = Mirror::create("er-release", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
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
    admin::add_named_root_hosted(
        &store,
        "t",
        op(0x41),
        RootKind::RestorePoint,
        "releasable",
        op(0x42),
        &RootPolicy::DEFAULT,
        HEAD_CAP,
        &work(),
    )
    .expect("root");
    let report = erase_hosted(
        &store,
        "t",
        op(0x43),
        &[op(0x41)],
        LIMITS,
        &gc_policy(),
        &work(),
    )
    .expect("erasure with release");
    assert_eq!(report.released_roots, vec![op(0x41)]);
    assert!(report.residual.retained_roots.is_empty());
    // A second pass collects everything the first pass conservatively kept.
    // Exactly the second pass's own mark evidence remains: a current-epoch
    // object that is ordinary unreachable input to a LATER collection
    // (chapter 21) — never application data.
    let second = erase_hosted(&store, "t", op(0x43), &[], LIMITS, &gc_policy(), &work())
        .expect("second pass");
    assert_eq!(
        second.residual.remaining_objects,
        1,
        "only the finishing pass's own mark evidence remains: {:?}",
        store.object_keys()
    );
    assert!(
        store.object_keys().iter().all(|key| key.contains("/mark/")),
        "no former live object survived: {:?}",
        store.object_keys()
    );
    assert!(
        second.residual.head_tombstone_retained,
        "identity marker stays"
    );
}

#[test]
fn erase04_whole_tenant_and_user_level_scopes_are_distinct_operations() {
    // ERASE-04: user-level erasure inside a surviving tenant is an ordinary
    // admitted fact deletion (application retention policy), NOT the tenant
    // tombstone; the two scopes cannot be confused through these APIs.
    let store = MemStore::new();
    let mut mirror = Mirror::create("er-scopes", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    mirror.submit(&insert_user(mirror.db(), identity, 2, 20));
    // User-level: delete one user's facts; the tenant stays live and serves.
    mirror.submit(&delete_user(mirror.db(), identity, 3, 10));
    let head = mirror.head();
    let live = head.control.live().expect("still live");
    assert_eq!(live.state.data_revision, 3);
    // Whole-tenant: the local variant is a terminal authority transition; the
    // facts physically remain until the owner removes the cache directory —
    // reported, never silently claimed erased.
    let outcome = erase_local(mirror.db(), op(0x51), HEAD_CAP, &work()).expect("local tombstone");
    assert!(matches!(outcome, DeleteOutcome::Deleted(_)));
    let authority = admin::local_authority(mirror.db(), HEAD_CAP).expect("attachment reads");
    assert!(
        authority.live().is_err(),
        "the local authority is a terminal tombstone"
    );
    assert!(
        matches!(
            authority.lifecycle,
            bumbledb_log::history::authority::Lifecycle::Deleted {
                reason: DeletedReason::Erasure,
                ..
            }
        ),
        "the tombstone names its reason"
    );
}
