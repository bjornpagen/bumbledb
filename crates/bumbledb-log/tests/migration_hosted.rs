//! Hosted migration cutover over the C07 conditional grammar: freeze/genesis/
//! activation/fence race through one HEAD per database, lost responses are
//! `Indeterminate` and resolve only by re-reading evidence, `Unknown` never
//! thaws, delayed genesis loses the create race to the cancellation
//! tombstone, and activation versus abort has exactly one winner. Every head
//! body is the composed `manifest::HeadRecord` frame — the same grammar
//! `writer::hosted` publishes — so retention fields survive every cutover
//! transition and a bare control frame refuses as corruption. Driven by the
//! production `store::mem::MemStore` double (faults are explicit script
//! entries; the store never invents state); real S3 is the P05/P12 F3 lane.
//! Maps to MIG-01/05/09/14 (hosted halves), the C08 composed-head seam and
//! OPS-001. Verification: `NotRun` (authored, not executed).

#[path = "migration_support/mod.rs"]
mod support;

use bumbledb_log::history::authority::{
    Access, Activation, ActivationCause, DeletedReason, FreezeIntent, FreezeOutcome, HeadAuthority,
    Lifecycle, encode_control,
};
use bumbledb_log::history::decision::{
    GenesisProvenance, GenesisRecord, blank_initial_digests, genesis_stamp,
};
use bumbledb_log::history::{AccessMode, DatabaseIdentity, OperationId, StateStamp};
use bumbledb_log::manifest::{self, HeadRecord, NamedRoot, RootKind, RootPolicy};
use bumbledb_log::migration::executor::{ActivationRef, MigrationError, TargetFence};
use bumbledb_log::migration::hosted::{HostedCutover, HostedOutcome};
use bumbledb_log::schema_file::schema_id;
use bumbledb_log::store::mem::{Behavior, MemStore, Op};
use bumbledb_log::writer::LogError;
use bumbledb_log::writer::verbs::{ConditionalStore as _, HeadRead, HeadVersion};

use support::{CAP, LIMITS, base_schema, db_id, incarnation, op, tagged_schema};

// ---------------------------------------------------------------------------
// Fixture identities and composed heads.
// ---------------------------------------------------------------------------

const SOURCE: &str = "tenant/source";
const TARGET: &str = "tenant/target";
const SOURCE_EPOCH: u64 = 1;
const TARGET_EPOCH: u64 = 1;
/// The driver's own bounded CAS re-read budget (`migration::hosted::ATTEMPTS`):
/// an "undeliverable" schedule scripts one dropped response per attempt.
const CAS_ATTEMPTS: usize = 4;
const PSD: [u8; 32] = [0x42; 32];

fn source_identity() -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: db_id(0xa1),
        incarnation_id: incarnation(0xb1),
        schema_id: schema_id(&base_schema()).unwrap(),
    }
}

fn target_identity() -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: db_id(0xa1),
        incarnation_id: incarnation(0xb2),
        schema_id: schema_id(&tagged_schema()).unwrap(),
    }
}

fn active_source() -> HeadAuthority {
    let identity = source_identity();
    let (application, system) = blank_initial_digests();
    let record = GenesisRecord {
        identity,
        initial_application_digest: application,
        initial_system_digest: system,
        provenance: GenesisProvenance::Create,
    };
    let stamp = genesis_stamp(&record, CAP).unwrap();
    HeadAuthority::genesis(
        identity,
        stamp,
        Activation::Activated {
            operation: op(0xc1),
            target_genesis: stamp.hash,
            cause: ActivationCause::Create,
        },
    )
    .unwrap()
}

/// The final target authority exactly as the builder publishes it:
/// genesis + `NotActivated` + Frozen/AwaitingCutover under the operation.
fn frozen_target(operation: OperationId) -> HeadAuthority {
    let identity = target_identity();
    let (application, system) = blank_initial_digests();
    let record = GenesisRecord {
        identity,
        initial_application_digest: application,
        initial_system_digest: system,
        provenance: GenesisProvenance::Migration {
            source_database: source_identity().database_id,
            source_incarnation: source_identity().incarnation_id,
            plan_set_digest: PSD,
        },
    };
    let stamp = genesis_stamp(&record, CAP).unwrap();
    let genesis = HeadAuthority::genesis(identity, stamp, Activation::NotActivated).unwrap();
    match genesis
        .freeze(
            operation,
            FreezeIntent::Migration {
                plan_set_digest: PSD,
                target: identity.incarnation_id,
            },
        )
        .unwrap()
    {
        FreezeOutcome::Frozen(frozen) => frozen,
        FreezeOutcome::AlreadyFrozen { .. } => unreachable!("fresh genesis"),
    }
}

fn activation_ref(operation: OperationId) -> ActivationRef {
    let frozen = frozen_target(operation);
    let live = frozen.live().unwrap();
    ActivationRef {
        operation,
        plan_set_digest: PSD,
        target: target_identity(),
        target_genesis: live.decision.hash,
    }
}

/// Seed the source HEAD with its composed genesis record — exactly the body
/// `writer::hosted::HostedHistory::create` publishes.
fn store_with_source() -> MemStore {
    let store = MemStore::new();
    let body = manifest::genesis_head_body(&active_source(), SOURCE_EPOCH, CAP).unwrap();
    store.create_head(&format!("{SOURCE}/HEAD"), &body).unwrap();
    store
}

fn cutover(store: &MemStore) -> HostedCutover<'_, MemStore> {
    HostedCutover::new(store, SOURCE, TARGET, TARGET_EPOCH, LIMITS)
}

fn abort_reason() -> DeletedReason {
    DeletedReason::MigrationAborted {
        source_database: source_identity().database_id,
        source_incarnation: source_identity().incarnation_id,
        plan_set_digest: PSD,
    }
}

fn head_record(store: &MemStore, key: &str) -> HeadRecord {
    match store.read_head(key).unwrap() {
        HeadRead::Present { body, .. } => manifest::decode_head(&body, CAP).unwrap(),
        HeadRead::Absent => panic!("head must exist: {key}"),
    }
}

fn head_authority(store: &MemStore, key: &str) -> HeadAuthority {
    head_record(store, key).control
}

fn head_version(store: &MemStore, key: &str) -> HeadVersion {
    match store.read_head(key).unwrap() {
        HeadRead::Present { version, .. } => version,
        HeadRead::Absent => panic!("head must exist: {key}"),
    }
}

fn source_access(store: &MemStore) -> Access {
    let authority = head_authority(store, &format!("{SOURCE}/HEAD"));
    authority.live().unwrap().access
}

fn intent() -> FreezeIntent {
    FreezeIntent::Migration {
        plan_set_digest: PSD,
        target: target_identity().incarnation_id,
    }
}

// ---------------------------------------------------------------------------
// Composed-head grammar: the CAS bodies ARE HeadRecord frames (C08).
// ---------------------------------------------------------------------------

#[test]
fn cutover_heads_are_composed_records_preserving_retention() {
    let store = store_with_source();
    let driver = cutover(&store);
    let source_key = format!("{SOURCE}/HEAD");
    let before = head_record(&store, &source_key);
    let genesis_tip = before.recovery.expect("live head names its recovery").tip;

    // Freeze: the successor is composed onto the exact parent record — the
    // transitioned control swapped in, every retention field preserved.
    assert!(matches!(
        driver.freeze_source(op(0xd1), intent()).unwrap(),
        HostedOutcome::Completed(())
    ));
    let frozen = head_record(&store, &source_key);
    assert!(matches!(
        frozen.control.live().unwrap().access,
        Access::Frozen { .. }
    ));
    assert_eq!(frozen.object_epoch, SOURCE_EPOCH, "epoch preserved");
    let recovery = frozen.recovery.expect("recovery root preserved");
    assert_eq!(recovery.tip, genesis_tip, "freeze publishes no decision");
    assert_eq!(recovery.tail_bytes, before.recovery.unwrap().tail_bytes);

    // Target genesis: the composed genesis record, not a bare control frame.
    let target = frozen_target(op(0xd1));
    driver.publish_target_genesis(&target, op(0xd1)).unwrap();
    let published = head_record(&store, &format!("{TARGET}/HEAD"));
    assert_eq!(published.control, target);
    assert_eq!(published.object_epoch, TARGET_EPOCH);
    let target_recovery = published.recovery.expect("genesis recovery root");
    assert!(target_recovery.checkpoint.is_none(), "no checkpoint yet");
    assert_eq!(target_recovery.base, target_recovery.tip);
    assert_eq!(target_recovery.tip, target.live().unwrap().decision);
    assert!(published.roots.is_empty());

    // Pre-genesis fence on a SECOND store: the cancellation tombstone is a
    // composed record with NO recovery root (nothing to recover).
    let store2 = store_with_source();
    let driver2 = cutover(&store2);
    driver2.freeze_source(op(0xd1), intent()).unwrap();
    driver2
        .abort(target_identity(), op(0xd1), abort_reason())
        .unwrap();
    let tombstone = head_record(&store2, &format!("{TARGET}/HEAD"));
    assert!(matches!(
        tombstone.control.lifecycle,
        Lifecycle::Deleted { .. }
    ));
    assert!(tombstone.recovery.is_none(), "a tombstone has no recovery");
    assert_eq!(tombstone.object_epoch, TARGET_EPOCH);
}

#[test]
fn named_roots_survive_every_cutover_transition() {
    // Seed a source head that carries retention state a bare control frame
    // could not: one named restore point.
    let store = MemStore::new();
    let genesis = HeadRecord::genesis(active_source(), SOURCE_EPOCH).unwrap();
    let root_recovery = genesis.recovery.unwrap();
    let with_root = genesis
        .add_root(
            NamedRoot {
                id: op(0x77),
                kind: RootKind::RestorePoint,
                recovery: root_recovery,
                state: StateStamp {
                    incarnation: source_identity().incarnation_id,
                    data_revision: 0,
                },
                label: "before-cutover".into(),
                operation: op(0x77),
            },
            &RootPolicy::DEFAULT,
        )
        .unwrap();
    let body = manifest::encode_head(&with_root, CAP).unwrap();
    store.create_head(&format!("{SOURCE}/HEAD"), &body).unwrap();

    let driver = cutover(&store);
    driver.freeze_source(op(0xd1), intent()).unwrap();
    let frozen = head_record(&store, &format!("{SOURCE}/HEAD"));
    assert_eq!(frozen.roots.len(), 1, "freeze preserves named roots");
    assert_eq!(frozen.roots[0].id, op(0x77));
    assert!(frozen.recovery.is_some());

    driver.thaw_source(op(0xd1)).unwrap();
    let thawed = head_record(&store, &format!("{SOURCE}/HEAD"));
    assert_eq!(thawed.roots.len(), 1, "thaw preserves named roots");
    assert!(matches!(
        thawed.control.live().unwrap().access,
        Access::Active
    ));
}

#[test]
fn a_bare_control_head_frame_refuses_as_corruption() {
    // Hosted heads are composed HeadRecord frames; the 0.x bare control
    // projection at a HEAD key is corruption-class evidence, never silently
    // accepted (the recorded P04R→P09 seam defect, closed).
    let store = MemStore::new();
    let bare = encode_control(&active_source(), CAP).unwrap();
    store.create_head(&format!("{SOURCE}/HEAD"), &bare).unwrap();
    let driver = cutover(&store);
    assert!(matches!(
        driver.freeze_source(op(0xd1), intent()),
        Err(MigrationError::Log(LogError::Corruption))
    ));
    assert!(matches!(
        driver.thaw_source(op(0xd1)),
        Err(MigrationError::Log(LogError::Corruption))
    ));
    // A bare-frame TARGET refuses activation the same way.
    let store2 = store_with_source();
    let bare_target = encode_control(&frozen_target(op(0xd1)), CAP).unwrap();
    store2
        .create_head(&format!("{TARGET}/HEAD"), &bare_target)
        .unwrap();
    assert!(matches!(
        cutover(&store2).activate(&activation_ref(op(0xd1))),
        Err(MigrationError::Log(LogError::Corruption))
    ));
}

// ---------------------------------------------------------------------------
// Freeze.
// ---------------------------------------------------------------------------

#[test]
fn freeze_is_durable_idempotent_and_operation_bound() {
    let store = store_with_source();
    let driver = cutover(&store);
    assert!(matches!(
        driver.freeze_source(op(0xd1), intent()).unwrap(),
        HostedOutcome::Completed(())
    ));
    assert!(matches!(source_access(&store), Access::Frozen { .. }));
    // The matching retry is evidence, not a second mutation.
    let version_before = head_version(&store, &format!("{SOURCE}/HEAD"));
    assert!(matches!(
        driver.freeze_source(op(0xd1), intent()).unwrap(),
        HostedOutcome::Completed(())
    ));
    assert_eq!(
        head_version(&store, &format!("{SOURCE}/HEAD")),
        version_before,
        "no head revision was spent on the retry"
    );
    // A foreign operation refuses.
    match driver.freeze_source(op(0xd2), intent()) {
        Err(MigrationError::SourceFrozenByOther { operation }) => {
            assert_eq!(operation, op(0xd1));
        }
        other => panic!("expected SourceFrozenByOther, got {other:?}"),
    }
}

#[test]
fn a_lost_freeze_response_resolves_by_rereading_recorded_evidence() {
    let store = store_with_source();
    // The CAS applies but its response is lost (the "lost 200" arm).
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
    let driver = cutover(&store);
    // The driver re-reads and finds its own frozen operation — Completed,
    // never a duplicate freeze.
    assert!(matches!(
        driver.freeze_source(op(0xd1), intent()).unwrap(),
        HostedOutcome::Completed(())
    ));
    assert!(matches!(source_access(&store), Access::Frozen { .. }));
}

#[test]
fn an_undeliverable_freeze_reports_unknown_and_mutates_nothing() {
    let store = store_with_source();
    // Every bounded attempt's CAS is dropped before it lands.
    for _ in 0..CAS_ATTEMPTS {
        store.fail_next(Op::ReplaceHead, Behavior::IndeterminateDropped);
    }
    let driver = cutover(&store);
    assert!(matches!(
        driver.freeze_source(op(0xd1), intent()).unwrap(),
        HostedOutcome::Unknown
    ));
    // Nothing changed; certainty comes from a later retry, not a guess.
    assert!(matches!(source_access(&store), Access::Active));
    // Transport recovers (the script is consumed): the retry completes.
    assert!(matches!(
        driver.freeze_source(op(0xd1), intent()).unwrap(),
        HostedOutcome::Completed(())
    ));
}

// ---------------------------------------------------------------------------
// Target genesis.
// ---------------------------------------------------------------------------

#[test]
fn target_genesis_publishes_once_and_resolves_lost_creates_by_content() {
    let store = store_with_source();
    let driver = cutover(&store);
    let frozen = frozen_target(op(0xd1));
    assert!(matches!(
        driver.publish_target_genesis(&frozen, op(0xd1)).unwrap(),
        HostedOutcome::Completed(())
    ));
    // A retry with the identical body is completion evidence.
    assert!(matches!(
        driver.publish_target_genesis(&frozen, op(0xd1)).unwrap(),
        HostedOutcome::Completed(())
    ));
    // A DIFFERENT genesis for the same key refuses on recorded evidence.
    let foreign = frozen_target(op(0xd9));
    assert!(matches!(
        driver.publish_target_genesis(&foreign, op(0xd9)),
        Err(MigrationError::TargetConflict)
    ));

    // Lost create response on a fresh store: applied but unreported; the
    // driver resolves by reading the exact composed record it wrote.
    let store2 = store_with_source();
    store2.fail_next(Op::CreateHead, Behavior::IndeterminateApplied);
    let driver2 = cutover(&store2);
    assert!(matches!(
        driver2.publish_target_genesis(&frozen, op(0xd1)).unwrap(),
        HostedOutcome::Completed(())
    ));
}

#[test]
fn delayed_genesis_loses_the_create_race_to_the_cancellation_tombstone() {
    let store = store_with_source();
    let driver = cutover(&store);
    // The abort fences the ABSENT target first (conditional create of the
    // exact composed cancellation tombstone)…
    driver.freeze_source(op(0xd1), intent()).unwrap();
    match driver
        .abort(target_identity(), op(0xd1), abort_reason())
        .unwrap()
    {
        HostedOutcome::Completed(report) => {
            assert_eq!(report.fence, TargetFence::TombstonePreGenesis);
            assert!(report.thawed);
        }
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    assert!(matches!(source_access(&store), Access::Active));
    // …so the paused runner's delayed genesis create is refused by the
    // recorded terminal evidence and reports Aborted for its own operation.
    let frozen = frozen_target(op(0xd1));
    match driver.publish_target_genesis(&frozen, op(0xd1)) {
        Err(MigrationError::Aborted { operation }) => assert_eq!(operation, op(0xd1)),
        other => panic!("expected Aborted, got {other:?}"),
    }
    // A cancelled operation's activation also permanently reports Aborted.
    match cutover(&store).activate(&activation_ref(op(0xd1))) {
        Err(MigrationError::Aborted { operation }) => assert_eq!(operation, op(0xd1)),
        other => panic!("expected Aborted, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Abort/fence/thaw ordering and uncertainty.
// ---------------------------------------------------------------------------

#[test]
fn uncertain_target_cancellation_never_authorizes_thaw() {
    let store = store_with_source();
    let driver = cutover(&store);
    driver.freeze_source(op(0xd1), intent()).unwrap();
    // Every cancellation dispatch is lost: the fence is uncertain, so the
    // whole abort is Unknown and the source REMAINS frozen.
    for _ in 0..CAS_ATTEMPTS {
        store.fail_next(Op::CreateHead, Behavior::IndeterminateDropped);
    }
    assert!(matches!(
        driver
            .abort(target_identity(), op(0xd1), abort_reason())
            .unwrap(),
        HostedOutcome::Unknown
    ));
    assert!(
        matches!(source_access(&store), Access::Frozen { .. }),
        "an uncertain fence never thaws"
    );
    // Transport recovers (the script is consumed): the same stable operation
    // resumes from evidence and completes fence-then-thaw in order.
    match driver
        .abort(target_identity(), op(0xd1), abort_reason())
        .unwrap()
    {
        HostedOutcome::Completed(report) => {
            assert_eq!(report.fence, TargetFence::TombstonePreGenesis);
            assert!(report.thawed);
        }
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    assert!(matches!(source_access(&store), Access::Active));
    // The abort retry is evidence-only.
    match driver
        .abort(target_identity(), op(0xd1), abort_reason())
        .unwrap()
    {
        HostedOutcome::Completed(report) => {
            assert_eq!(report.fence, TargetFence::AlreadyFenced);
            assert!(!report.thawed);
        }
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
}

#[test]
fn fencing_a_published_unactivated_target_terminally_deletes_it() {
    let store = store_with_source();
    let driver = cutover(&store);
    driver.freeze_source(op(0xd1), intent()).unwrap();
    let frozen = frozen_target(op(0xd1));
    driver.publish_target_genesis(&frozen, op(0xd1)).unwrap();
    match driver
        .abort(target_identity(), op(0xd1), abort_reason())
        .unwrap()
    {
        HostedOutcome::Completed(report) => {
            assert_eq!(report.fence, TargetFence::TargetDeleted);
            assert!(report.thawed);
        }
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    // The target head is a terminal tombstone that preserved identity, still
    // framed as a composed record (recovery dropped, epoch preserved).
    let record = head_record(&store, &format!("{TARGET}/HEAD"));
    assert_eq!(record.control.identity, target_identity());
    assert!(matches!(
        record.control.lifecycle,
        Lifecycle::Deleted { operation, .. } if operation == op(0xd1)
    ));
    assert!(
        record.recovery.is_none(),
        "a tombstone drops the recovery root"
    );
    assert_eq!(record.object_epoch, TARGET_EPOCH);
    // Activation with the previously valid reference reports Aborted.
    match driver.activate(&activation_ref(op(0xd1))) {
        Err(MigrationError::Aborted { operation }) => assert_eq!(operation, op(0xd1)),
        other => panic!("expected Aborted, got {other:?}"),
    }
    // A foreign operation cannot reuse the terminal namespace.
    assert!(matches!(
        driver.abort(target_identity(), op(0xd2), abort_reason()),
        Err(MigrationError::TargetConflict)
    ));
}

// ---------------------------------------------------------------------------
// Activation.
// ---------------------------------------------------------------------------

#[test]
fn activation_and_abort_race_has_exactly_one_winner() {
    // Activation first: the fence refuses with ActivationWon and the source
    // stays frozen for an explicit operator decision.
    let store = store_with_source();
    let driver = cutover(&store);
    driver.freeze_source(op(0xd1), intent()).unwrap();
    let frozen = frozen_target(op(0xd1));
    driver.publish_target_genesis(&frozen, op(0xd1)).unwrap();
    match driver.activate(&activation_ref(op(0xd1))).unwrap() {
        HostedOutcome::Completed(report) => assert_eq!(report.access, AccessMode::Active),
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    assert!(matches!(
        driver.abort(target_identity(), op(0xd1), abort_reason()),
        Err(MigrationError::ActivationWon)
    ));
    assert!(
        matches!(source_access(&store), Access::Frozen { .. }),
        "a refused abort thaws nothing"
    );
}

#[test]
fn a_lost_activation_response_resolves_to_the_recorded_marker() {
    let store = store_with_source();
    let driver = cutover(&store);
    driver.freeze_source(op(0xd1), intent()).unwrap();
    let frozen = frozen_target(op(0xd1));
    driver.publish_target_genesis(&frozen, op(0xd1)).unwrap();
    // The activation CAS applies but its response is lost; the driver
    // re-reads and returns the recorded one-time marker.
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
    let reference = activation_ref(op(0xd1));
    let first = match driver.activate(&reference).unwrap() {
        HostedOutcome::Completed(report) => report,
        HostedOutcome::Unknown => panic!("resolvable by re-read"),
    };
    assert_eq!(first.access, AccessMode::Active);
    // The matching retry after later reads returns the same evidence plus
    // the CURRENT access mode, without spending another head revision.
    let version_before = head_version(&store, &format!("{TARGET}/HEAD"));
    let second = match driver.activate(&reference).unwrap() {
        HostedOutcome::Completed(report) => report,
        HostedOutcome::Unknown => panic!("evidence retry"),
    };
    assert_eq!(first.activation, second.activation);
    assert_eq!(
        head_version(&store, &format!("{TARGET}/HEAD")),
        version_before
    );
}

#[test]
fn stale_references_and_foreign_targets_refuse_activation() {
    let store = store_with_source();
    let driver = cutover(&store);
    driver.freeze_source(op(0xd1), intent()).unwrap();
    // No target head at all: stale reference.
    assert!(matches!(
        driver.activate(&activation_ref(op(0xd1))),
        Err(MigrationError::StaleActivationRef)
    ));
    let frozen = frozen_target(op(0xd1));
    driver.publish_target_genesis(&frozen, op(0xd1)).unwrap();
    // A doctored genesis digest refuses before any transition.
    let mut doctored = activation_ref(op(0xd1));
    let mut bytes = *doctored.target_genesis.as_bytes();
    bytes[0] ^= 1;
    doctored.target_genesis = bumbledb_log::history::DecisionDigest::from_bytes(bytes);
    assert!(matches!(
        driver.activate(&doctored),
        Err(MigrationError::StaleActivationRef)
    ));
    // A foreign operation against the frozen target refuses as a conflict.
    let mut foreign = activation_ref(op(0xd2));
    foreign.target_genesis = activation_ref(op(0xd1)).target_genesis;
    assert!(matches!(
        driver.activate(&foreign),
        Err(MigrationError::TargetConflict)
    ));
}

#[test]
fn thaw_requires_the_exact_matching_operation() {
    let store = store_with_source();
    let driver = cutover(&store);
    driver.freeze_source(op(0xd1), intent()).unwrap();
    match driver.thaw_source(op(0xd2)) {
        Err(MigrationError::SourceFrozenByOther { operation }) => {
            assert_eq!(operation, op(0xd1));
        }
        other => panic!("expected SourceFrozenByOther, got {other:?}"),
    }
    assert!(matches!(source_access(&store), Access::Frozen { .. }));
    // The matching thaw completes; thawing an active source reports false
    // (evidence, not an error) so abort retries stay idempotent.
    assert!(matches!(
        driver.thaw_source(op(0xd1)).unwrap(),
        HostedOutcome::Completed(true)
    ));
    assert!(matches!(
        driver.thaw_source(op(0xd1)).unwrap(),
        HostedOutcome::Completed(false)
    ));
}
