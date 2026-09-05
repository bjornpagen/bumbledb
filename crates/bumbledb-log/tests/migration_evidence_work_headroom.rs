//! F3 finding-D adjacent local repairs, pinned permanently over the ACTUAL
//! public call paths (`LocalHistory` → `LocalMigration`/`initialize`/
//! `activate_target`/`fence_target`):
//!
//! - a bounded-work refusal is ONE typed resource refusal no matter which
//!   layer charged it (`MigrationError::Work`, never a nested `Log(Work)`
//!   the SDK would respell as migration drift), and the SAME operation
//!   completes later under a real budget (MIG-11/12 shape);
//! - recorded activation evidence resolves migrate/initialize reruns and
//!   refuses automatic abort (`ActivationWon`) while a LIVE OWNER holds the
//!   activated target store open (the served-database steady state), via the
//!   durable namespace activation marker written by `activate_target`;
//! - a live-owned target WITHOUT matching recorded evidence stays the typed
//!   store-lock refusal — never guessed into an outcome — and the marker's
//!   crash window (control committed, marker unwritten) heals on the next
//!   matching activate retry; tampered marker bytes refuse loudly;
//! - `admission_headroom` warns (`StartCheckpoint`) before the
//!   `MaintenanceRequired` cliff even for a tiny envelope where the 3/4
//!   reservation alone could never fire (STORE-07 backpressure shape).

#[path = "migration_support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{ChangeSet, Db, Id128, RelationId, Value};

use bumbledb_log::checkpointer::{Headroom, admission_headroom};
use bumbledb_log::history::command::{Command, CommandMetadata};
use bumbledb_log::history::{
    AccessMode, CommandId, CommandResult, Condition, ReceiptEpoch, RequestId,
};
use bumbledb_log::manifest::{RecoveryRoot, TailPolicy};
use bumbledb_log::migration::executor::{
    AbortRequest, LocalMigration, MigrateOutcome, MigrationError, MigrationStatus, StepInput,
    SuffixRequest, activate_target, initialize,
};
use bumbledb_log::migration::manifest::{Manifest, plan_set_digest};
use bumbledb_log::schema_file::schema_id;
use bumbledb_log::writer::{LocalHistory, SubmitOutcome};

use support::{
    CAP, LIMITS, base_schema, db_id, fresh_source, incarnation, manifest, op, pinned_schema,
    plan_pinned, plan_tagged, tagged_schema, temp_dir, tiny_work, work,
};

fn hex_name(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut hex = String::new();
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn open_history(db: &Arc<Db<SchemaDescriptor>>) -> LocalHistory<SchemaDescriptor> {
    LocalHistory::create(
        Arc::clone(db),
        db_id(0xa1),
        incarnation(0xb1),
        op(0xc1),
        LIMITS,
        &work(),
    )
    .unwrap()
}

fn insert_notes(history: &LocalHistory<SchemaDescriptor>, rows: &[(u64, &str)], request: u8) {
    let mut draft = ChangeSet::builder(history.db().schema(), work());
    for (id, body) in rows {
        draft
            .insert(
                RelationId(0),
                &[Value::U64(*id), Value::String((*body).into())],
            )
            .unwrap();
    }
    let changes = draft.finish().unwrap();
    let command = Command::seal(
        CommandMetadata {
            identity: history.identity(),
            id: CommandId {
                receipt_epoch: ReceiptEpoch::INITIAL,
                request_id: RequestId::from_core(Id128::from_bytes([request; 16])),
            },
            condition: Condition::Unconditional,
        },
        changes,
        CommandResult::empty(),
        LIMITS,
        &work(),
    )
    .unwrap();
    match history.submit(&command, &work()) {
        SubmitOutcome::Decided { .. } => {}
        other => panic!("seed submit failed: {other:?}"),
    }
}

fn steps_full() -> Vec<StepInput> {
    vec![
        StepInput {
            plan: plan_pinned(),
            to_descriptor: pinned_schema(),
        },
        StepInput {
            plan: plan_tagged(),
            to_descriptor: tagged_schema(),
        },
    ]
}

fn request<'a>(
    manifest: &'a Manifest,
    steps: &'a [StepInput],
    operation: u8,
    target_inc: u8,
) -> SuffixRequest<'a> {
    SuffixRequest {
        operation: op(operation),
        manifest,
        source_descriptor: base_schema(),
        steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(target_inc),
    }
}

fn target_dir(targets_root: &Path, target_inc: u8) -> PathBuf {
    targets_root.join(hex_name(incarnation(target_inc).as_core().as_bytes()))
}

fn marker_path(targets_root: &Path, target_inc: u8) -> PathBuf {
    targets_root.join(format!(
        "{}.activation",
        hex_name(incarnation(target_inc).as_core().as_bytes())
    ))
}

fn is_store_locked(error: &MigrationError) -> bool {
    matches!(
        error,
        MigrationError::Core(bumbledb::Error::Store(inner))
            if matches!(**inner, bumbledb::store::StoreError::StoreLocked { .. })
    )
}

// ---------------------------------------------------------------------------
// Work exhaustion is ONE typed resource refusal, wherever the charge lands.
// ---------------------------------------------------------------------------

#[test]
fn a_work_refusal_is_typed_work_even_when_the_freeze_commit_charges_it() {
    let (db, root) = fresh_source("evwork");
    let history = open_history(&db);
    insert_notes(&history, &[(1, "alpha"), (2, "beta")], 1);
    let plans = manifest();
    let steps = steps_full();
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = request(&plans, &steps, 0xd1, 0xe1);
    // The tiny budget cannot even afford the durable freeze commit: the
    // charge lands inside the log writer session, and MUST still surface as
    // the ONE typed resource refusal (the SDK maps `Work` to the exact core
    // reason; a nested `Log(Work)` would be respelled as migration drift).
    match runner.migrate(&request, &tiny_work()) {
        Err(MigrationError::Work(_)) => {}
        other => panic!("expected the typed Work refusal, got {other:?}"),
    }
    // The SAME operation completes under a real budget.
    match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { .. } => {}
        other => panic!("resume with the same operation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Recorded activation evidence under a live-owned target store.
// ---------------------------------------------------------------------------

#[test]
fn activation_evidence_resolves_while_a_live_owner_serves_the_target() {
    let (db, root) = fresh_source("evlive");
    let history = open_history(&db);
    insert_notes(&history, &[(1, "alpha")], 1);
    let plans = manifest();
    let steps = steps_full();
    let targets = root.join("targets");
    let runner = LocalMigration::new(&history, &targets, LIMITS);
    let reference = match runner.migrate(&request(&plans, &steps, 0xd2, 0xe2), &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };
    activate_target(&targets, &reference, &tagged_schema(), LIMITS, &work()).unwrap();
    assert!(
        marker_path(&targets, 0xe2).is_file(),
        "activation records its durable namespace evidence"
    );

    // The activated target is now OPEN and being served: its store lock is
    // held for the whole steady state.
    let served: Db<SchemaDescriptor> =
        Db::open(&target_dir(&targets, 0xe2), tagged_schema(), work()).unwrap();

    // A migrate rerun of the settled operation resolves from the recorded
    // evidence without opening the served store, and mutates nothing.
    match runner.migrate(&request(&plans, &steps, 0xd2, 0xe2), &work()).unwrap() {
        MigrateOutcome::AlreadyActivated { access, .. } => {
            assert_eq!(access, AccessMode::Active);
        }
        other => panic!("expected recorded activation evidence, got {other:?}"),
    }

    // Automatic abort refuses: activation already won this namespace, even
    // while the winner holds the store open.
    let psd = plan_set_digest(&plans, 0, 2, CAP).unwrap();
    let abort = AbortRequest {
        operation: op(0xd2),
        plan_set_digest: psd,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe2),
        target_schema: schema_id(&tagged_schema()).unwrap(),
        target_descriptor: &tagged_schema(),
    };
    match runner.abort(&abort, &work()) {
        Err(MigrationError::ActivationWon) => {}
        other => panic!("expected ActivationWon, got {other:?}"),
    }

    // Status stays read-only and lock-free; the source is still frozen with
    // the published target observed.
    match runner.status(&plans, &work()).unwrap() {
        MigrationStatus::Frozen {
            target_present,
            target_cancelled,
            ..
        } => {
            assert!(target_present);
            assert!(!target_cancelled);
        }
        other => panic!("expected Frozen, got {other:?}"),
    }

    // The served handle was never disturbed.
    let mut rows = 0u64;
    served
        .read(work(), |read| {
            rows = read.count(RelationId(0))?;
            Ok(())
        })
        .unwrap();
    assert_eq!(rows, 1);
}

#[test]
fn initialize_rerun_resolves_evidence_while_the_target_is_served() {
    let root = temp_dir("evinit");
    let targets = root.join("targets");
    let plans = manifest();
    let steps = steps_full();
    let request = request(&plans, &steps, 0xd3, 0xe3);
    let reference = match initialize(&targets, &request, LIMITS, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };
    activate_target(&targets, &reference, &tagged_schema(), LIMITS, &work()).unwrap();
    let served: Db<SchemaDescriptor> =
        Db::open(&target_dir(&targets, 0xe3), tagged_schema(), work()).unwrap();
    match initialize(&targets, &request, LIMITS, &work()).unwrap() {
        MigrateOutcome::AlreadyActivated { access, .. } => {
            assert_eq!(access, AccessMode::Active);
        }
        other => panic!("expected recorded activation evidence, got {other:?}"),
    }
    drop(served);
}

#[test]
fn a_locked_unactivated_target_stays_a_typed_refusal_never_guessed() {
    let root = temp_dir("evlocked");
    let targets = root.join("targets");
    let plans = manifest();
    let steps = steps_full();
    let request = request(&plans, &steps, 0xd4, 0xe4);
    match initialize(&targets, &request, LIMITS, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { .. } => {}
        other => panic!("{other:?}"),
    }
    // Someone holds the PUBLISHED, NOT-YET-ACTIVATED target open. There is
    // no recorded activation evidence, so the rerun must stay the typed
    // store-lock refusal — never a guessed outcome, never false evidence.
    let held: Db<SchemaDescriptor> =
        Db::open(&target_dir(&targets, 0xe4), tagged_schema(), work()).unwrap();
    match initialize(&targets, &request, LIMITS, &work()) {
        Err(error) => assert!(
            is_store_locked(&error),
            "expected the typed store-lock refusal, got {error:?}"
        ),
        Ok(other) => panic!("a locked unactivated target resolved to {other:?}"),
    }
    drop(held);
    // Released, the rerun reuses the published verified target normally.
    match initialize(&targets, &request, LIMITS, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_marker_crash_window_heals_and_tampered_evidence_refuses() {
    let root = temp_dir("evheal");
    let targets = root.join("targets");
    let plans = manifest();
    let steps = steps_full();
    let request = request(&plans, &steps, 0xd5, 0xe5);
    let reference = match initialize(&targets, &request, LIMITS, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };
    activate_target(&targets, &reference, &tagged_schema(), LIMITS, &work()).unwrap();
    let marker = marker_path(&targets, 0xe5);
    assert!(marker.is_file());

    // Simulate the crash window: the control commit landed, the marker did
    // not. The control stays the ONE authority — an unheld rerun still
    // resolves the evidence from it.
    std::fs::remove_file(&marker).unwrap();
    match initialize(&targets, &request, LIMITS, &work()).unwrap() {
        MigrateOutcome::AlreadyActivated { .. } => {}
        other => panic!("{other:?}"),
    }
    // A matching activate retry is evidence-only for the control AND heals
    // the marker.
    let retry = activate_target(&targets, &reference, &tagged_schema(), LIMITS, &work()).unwrap();
    assert_eq!(retry.access, AccessMode::Active);
    assert!(marker.is_file(), "the matching retry re-records the marker");

    // With the marker healed, evidence resolves under a live owner again.
    let served: Db<SchemaDescriptor> =
        Db::open(&target_dir(&targets, 0xe5), tagged_schema(), work()).unwrap();
    match initialize(&targets, &request, LIMITS, &work()).unwrap() {
        MigrateOutcome::AlreadyActivated { .. } => {}
        other => panic!("{other:?}"),
    }

    // Tampered marker bytes are loud corruption-class evidence, never a
    // silently absent marker and never an outcome.
    std::fs::write(&marker, b"not a control frame").unwrap();
    match initialize(&targets, &request, LIMITS, &work()) {
        Err(MigrationError::Namespace(_)) => {}
        other => panic!("tampered evidence must refuse, got {other:?}"),
    }
    drop(served);
}

// ---------------------------------------------------------------------------
// Backpressure headroom warns before the cliff even for tiny envelopes.
// ---------------------------------------------------------------------------

#[test]
fn admission_headroom_warns_before_the_cliff_for_every_envelope_size() {
    let stamp = |seq: u64| bumbledb_log::history::DecisionStamp {
        seq,
        hash: bumbledb_log::history::DecisionDigest::from_bytes(
            [u8::try_from(seq % 256).expect("bounded above"); 32],
        ),
    };
    let recovery = |tip: u64, tail_bytes: u64| RecoveryRoot {
        checkpoint: None,
        base: stamp(0),
        tip: stamp(tip),
        tip_object: None,
        tail_bytes,
        epoch_floor: 0,
    };
    let tiny = TailPolicy {
        max_count: 2,
        max_bytes: 1 << 20,
    };
    // One admission left: warn — a tiny envelope must never jump straight
    // from Ok to MaintenanceRequired.
    assert_eq!(
        admission_headroom(&recovery(1, 100), &tiny),
        Headroom::StartCheckpoint
    );
    assert_eq!(
        admission_headroom(&recovery(2, 100), &tiny),
        Headroom::MaintenanceRequired
    );
    let wide = TailPolicy {
        max_count: 100,
        max_bytes: 1 << 20,
    };
    assert_eq!(admission_headroom(&recovery(10, 100), &wide), Headroom::Ok);
    // The 3/4 reservation still governs large envelopes.
    assert_eq!(
        admission_headroom(&recovery(75, 100), &wide),
        Headroom::StartCheckpoint
    );
    assert_eq!(
        admission_headroom(&recovery(100, 100), &wide),
        Headroom::MaintenanceRequired
    );
    // Bytes bound the envelope independently.
    assert_eq!(
        admission_headroom(&recovery(10, (1 << 20) - 1), &wide),
        Headroom::StartCheckpoint
    );
    assert_eq!(
        admission_headroom(&recovery(10, 1 << 20), &wide),
        Headroom::MaintenanceRequired
    );
}
