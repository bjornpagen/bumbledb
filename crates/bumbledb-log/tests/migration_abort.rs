//! Local abort, target fencing and cutover-race semantics: the target fence
//! is durable BEFORE the matching source thaws, a paused installer or
//! activator loses to the tombstone under the same stable namespace lock,
//! activation winning refuses automatic abort, and a cancelled operation
//! permanently reports Aborted. Maps to MIG-03/05/09/14 and OPS-001.
//! Verification: `NotRun` (F1 authors, does not execute).

#[path = "migration_support/mod.rs"]
mod support;

use std::sync::Arc;

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{ChangeSet, Db, Id128, RelationId, Value};

use bumbledb_log::history::authority::{
    Activation, DeletedReason, HeadAuthority, Lifecycle, decode_control,
};
use bumbledb_log::history::command::{Command, CommandMetadata};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseIdentity, ReceiptEpoch, RequestId,
};
use bumbledb_log::migration::executor::{
    AbortRequest, LocalMigration, MigrateOutcome, MigrationError, MigrationStatus, StepInput,
    SuffixRequest, TargetFence, activate_target, fence_target,
};
use bumbledb_log::migration::lock::{NamespaceError, TargetNamespace};
use bumbledb_log::migration::manifest::{Manifest, append_entry, plan_set_digest};
use bumbledb_log::migration::plan::{FieldMap, Operation, Plan, PlanExpr};
use bumbledb_log::schema_file::schema_id;
use bumbledb_log::writer::{LocalHistory, LogError, SubmitOutcome};

use support::{
    CAP, LIMITS, base_schema, copy_field, db_id, fresh_source, incarnation, manifest, op,
    pinned_schema, plan_pinned, tagged_schema, temp_dir, work,
};

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

fn insert_note(history: &LocalHistory<SchemaDescriptor>, id: u64, body: &str, request: u8) {
    let mut draft = ChangeSet::builder(history.db().schema(), work());
    draft
        .insert(RelationId(0), &[Value::U64(id), Value::String(body.into())])
        .unwrap();
    let command = Command::seal(
        CommandMetadata {
            identity: history.identity(),
            id: CommandId {
                receipt_epoch: ReceiptEpoch::INITIAL,
                request_id: RequestId::from_core(Id128::from_bytes([request; 16])),
            },
            condition: Condition::Unconditional,
        },
        draft.finish().unwrap(),
        CommandResult::empty(),
        LIMITS,
        &work(),
    )
    .unwrap();
    match history.submit(&command, &work()) {
        SubmitOutcome::Decided { .. } => {}
        other => panic!("insert failed: {other:?}"),
    }
}

fn submit_refuses_frozen(history: &LocalHistory<SchemaDescriptor>, request: u8) -> bool {
    let mut draft = ChangeSet::builder(history.db().schema(), work());
    draft
        .insert(
            RelationId(0),
            &[Value::U64(u64::from(request)), Value::String("late".into())],
        )
        .unwrap();
    let command = Command::seal(
        CommandMetadata {
            identity: history.identity(),
            id: CommandId {
                receipt_epoch: ReceiptEpoch::INITIAL,
                request_id: RequestId::from_core(Id128::from_bytes([request; 16])),
            },
            condition: Condition::Unconditional,
        },
        draft.finish().unwrap(),
        CommandResult::empty(),
        LIMITS,
        &work(),
    )
    .unwrap();
    matches!(
        history.submit(&command, &work()),
        SubmitOutcome::NotSubmitted {
            error: LogError::DatabaseFrozen,
            ..
        }
    )
}

/// A plan that freezes durably then fails during execution (divide by zero
/// on an actual row): the deterministic way to reach Frozen-with-no-target.
fn failing_plan() -> Plan {
    Plan {
        operations: vec![
            Operation::MapRelation {
                source: "Note".into(),
                target: "Note".into(),
                fields: vec![
                    FieldMap {
                        target: "id".into(),
                        expression: PlanExpr::Divide(
                            Box::new(PlanExpr::Field("id".into())),
                            Box::new(PlanExpr::Literal(Value::U64(0))),
                        ),
                    },
                    copy_field("body"),
                    FieldMap {
                        target: "pinned".into(),
                        expression: PlanExpr::Literal(Value::Bool(false)),
                    },
                ],
            },
            Operation::ValidateSchema {
                schema: schema_id(&pinned_schema()).unwrap(),
            },
        ],
        ..plan_pinned()
    }
}

fn failing_manifest() -> Manifest {
    let mut manifest = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    append_entry(&mut manifest, &failing_plan(), CAP).unwrap();
    manifest
}

#[test]
fn abort_fences_the_absent_target_then_thaws_exactly_the_matching_source() {
    let (db, root) = fresh_source("abort-pregen");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = failing_manifest();
    let steps = vec![StepInput {
        plan: failing_plan(),
        to_descriptor: pinned_schema(),
    }];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xd1),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe1),
    };
    // The execution fails after the durable freeze: Frozen, no target.
    assert!(runner.migrate(&request, &work()).is_err());
    assert!(submit_refuses_frozen(&history, 0x71), "frozen source");

    let psd = plan_set_digest(&manifest, 0, 1, CAP).unwrap();
    let abort = AbortRequest {
        operation: op(0xd1),
        plan_set_digest: psd,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe1),
        target_schema: schema_id(&pinned_schema()).unwrap(),
        target_descriptor: &pinned_schema(),
    };
    let report = runner.abort(&abort, &work()).unwrap();
    assert_eq!(report.fence, TargetFence::TombstonePreGenesis);
    assert!(report.thawed, "this call thawed the matching source");

    // The tombstone is durable, readable and terminal.
    let namespace = TargetNamespace::new(&root.join("targets"), incarnation(0xe1)).unwrap();
    let tombstone = namespace.read_tombstone(CAP).unwrap().expect("durable");
    assert!(matches!(tombstone.lifecycle, Lifecycle::Deleted { .. }));

    // The source accepts commands again.
    insert_note(&history, 2, "after-thaw", 2);

    // Retrying the cancelled operation permanently reports Aborted; it can
    // never resume target creation (and never re-freezes the source).
    match runner.migrate(&request, &work()) {
        Err(MigrationError::Aborted { operation }) => assert_eq!(operation, op(0xd1)),
        other => panic!("expected Aborted, got {other:?}"),
    }
    insert_note(&history, 3, "still-active", 3);

    // Abort retry is idempotent evidence, not a second mutation.
    let again = runner.abort(&abort, &work()).unwrap();
    assert_eq!(again.fence, TargetFence::AlreadyFenced);
    assert!(!again.thawed, "evidence-only retry");
}

#[test]
fn a_paused_installer_loses_to_the_tombstone_under_the_same_lock() {
    // The lock/tombstone/install discipline directly: a fence installed
    // while a build is "paused" makes the delayed no-overwrite install
    // refuse — a precomputed rename cannot bypass the namespace.
    let root = temp_dir("lock-race");
    let namespace = TargetNamespace::new(&root, incarnation(0xe2)).unwrap();

    // The paused installer prepared complete staging bytes.
    let staged = namespace.fresh_staging();
    std::fs::create_dir_all(&staged).unwrap();
    std::fs::write(staged.join("data.mdb"), b"complete staged bytes").unwrap();

    // Abort wins the lock first and installs the durable cancellation.
    let identity = DatabaseIdentity {
        database_id: db_id(0xa1),
        incarnation_id: incarnation(0xe2),
        schema_id: schema_id(&pinned_schema()).unwrap(),
    };
    let tombstone = HeadAuthority::cancelled_before_genesis(
        identity,
        op(0xd2),
        DeletedReason::MigrationAborted {
            source_database: db_id(0xa1),
            source_incarnation: incarnation(0xb1),
            plan_set_digest: [7; 32],
        },
    );
    {
        let lock = namespace.lock().unwrap();
        namespace.install_tombstone(&lock, &tombstone, CAP).unwrap();
        // Matching reinstall under the lock is idempotent evidence…
        namespace.install_tombstone(&lock, &tombstone, CAP).unwrap();
        // …and a conflicting cancellation refuses.
        let foreign =
            HeadAuthority::cancelled_before_genesis(identity, op(0xd3), DeletedReason::Erasure);
        assert!(matches!(
            namespace.install_tombstone(&lock, &foreign, CAP),
            Err(NamespaceError::ForeignTombstone)
        ));
    }

    // The delayed installer wakes up: its install refuses the tombstone.
    let lock = namespace.lock().unwrap();
    assert!(matches!(
        namespace.install_target(&lock, &staged, CAP),
        Err(NamespaceError::ForeignTombstone)
    ));
    drop(lock);
    assert!(!namespace.target_exists(), "nothing was published");
    // The tombstone itself survives: it is namespace state, never scratch.
    assert!(namespace.read_tombstone(CAP).unwrap().is_some());
}

#[test]
fn the_namespace_lock_is_exclusive_and_installation_is_no_overwrite() {
    let root = temp_dir("lock-excl");
    let namespace = TargetNamespace::new(&root, incarnation(0xe3)).unwrap();
    let held = namespace.lock().unwrap();
    // A second handle observes Busy — a paused owner keeps the namespace;
    // elapsed time proves nothing and there is no expiry takeover.
    let second = TargetNamespace::new(&root, incarnation(0xe3)).unwrap();
    assert!(matches!(second.lock(), Err(NamespaceError::Busy)));
    drop(held);

    // First install publishes; a second complete build cannot overwrite it.
    let staged_a = namespace.fresh_staging();
    std::fs::create_dir_all(&staged_a).unwrap();
    std::fs::write(staged_a.join("data.mdb"), b"a").unwrap();
    let staged_b = namespace.fresh_staging();
    std::fs::create_dir_all(&staged_b).unwrap();
    std::fs::write(staged_b.join("data.mdb"), b"b").unwrap();
    let lock = namespace.lock().unwrap();
    namespace.install_target(&lock, &staged_a, CAP).unwrap();
    assert!(matches!(
        namespace.install_target(&lock, &staged_b, CAP),
        Err(NamespaceError::TargetExists)
    ));
    drop(lock);
    assert!(namespace.target_exists());
}

#[test]
fn abort_after_ready_to_switch_deletes_the_target_and_fences_activation() {
    let (db, root) = fresh_source("abort-published");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = manifest();
    let steps = vec![
        StepInput {
            plan: plan_pinned(),
            to_descriptor: pinned_schema(),
        },
        StepInput {
            plan: support::plan_tagged(),
            to_descriptor: tagged_schema(),
        },
    ];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xd4),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe4),
    };
    let reference = match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };

    let psd = plan_set_digest(&manifest, 0, 2, CAP).unwrap();
    let abort = AbortRequest {
        operation: op(0xd4),
        plan_set_digest: psd,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe4),
        target_schema: schema_id(&tagged_schema()).unwrap(),
        target_descriptor: &tagged_schema(),
    };
    let report = runner.abort(&abort, &work()).unwrap();
    assert_eq!(report.fence, TargetFence::TargetDeleted);
    assert!(report.thawed);
    insert_note(&history, 9, "post-abort", 9);

    // The published target's control is a terminal tombstone: delayed
    // activation with the previously valid reference reports Aborted.
    match activate_target(
        &root.join("targets"),
        &reference,
        &tagged_schema(),
        LIMITS,
        &work(),
    ) {
        Err(MigrationError::Aborted { operation }) => assert_eq!(operation, op(0xd4)),
        other => panic!("expected Aborted, got {other:?}"),
    }

    // Retrying the cancelled operation reports Aborted from the recorded
    // control evidence — WITHOUT re-freezing the thawed source.
    match runner.migrate(&request, &work()) {
        Err(MigrationError::Aborted { operation }) => assert_eq!(operation, op(0xd4)),
        other => panic!("expected Aborted, got {other:?}"),
    }
    insert_note(&history, 10, "never-refrozen", 10);

    // The terminal namespace is never reused: a NEW operation naming the
    // same planned target incarnation refuses on the recorded evidence,
    // also without freezing anything.
    let retry = SuffixRequest {
        operation: op(0xd5),
        ..request
    };
    assert!(matches!(
        runner.migrate(&retry, &work()),
        Err(MigrationError::TargetConflict)
    ));
    insert_note(&history, 11, "still-never-refrozen", 11);
}

#[test]
fn activation_winning_refuses_automatic_abort_and_the_source_stays_frozen() {
    let (db, root) = fresh_source("activation-won");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = manifest();
    let steps = vec![
        StepInput {
            plan: plan_pinned(),
            to_descriptor: pinned_schema(),
        },
        StepInput {
            plan: support::plan_tagged(),
            to_descriptor: tagged_schema(),
        },
    ];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xd6),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe6),
    };
    let reference = match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };
    activate_target(
        &root.join("targets"),
        &reference,
        &tagged_schema(),
        LIMITS,
        &work(),
    )
    .unwrap();

    let psd = plan_set_digest(&manifest, 0, 2, CAP).unwrap();
    let abort = AbortRequest {
        operation: op(0xd6),
        plan_set_digest: psd,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe6),
        target_schema: schema_id(&tagged_schema()).unwrap(),
        target_descriptor: &tagged_schema(),
    };
    assert!(matches!(
        runner.abort(&abort, &work()),
        Err(MigrationError::ActivationWon)
    ));
    // Nothing thawed: the source remains frozen under the operation.
    assert!(submit_refuses_frozen(&history, 0x72));
    assert!(matches!(
        runner.status(&manifest, &work()).unwrap(),
        MigrationStatus::Frozen { .. }
    ));
}

#[test]
fn foreign_operations_and_foreign_plan_sets_cannot_abort() {
    let (db, root) = fresh_source("abort-foreign");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = failing_manifest();
    let steps = vec![StepInput {
        plan: failing_plan(),
        to_descriptor: pinned_schema(),
    }];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xd7),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe7),
    };
    assert!(runner.migrate(&request, &work()).is_err());
    let psd = plan_set_digest(&manifest, 0, 1, CAP).unwrap();

    // A different operation cannot abort another operation's freeze.
    let foreign_op = AbortRequest {
        operation: op(0xd8),
        plan_set_digest: psd,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe7),
        target_schema: schema_id(&pinned_schema()).unwrap(),
        target_descriptor: &pinned_schema(),
    };
    match runner.abort(&foreign_op, &work()) {
        Err(MigrationError::SourceFrozenByOther { operation }) => {
            assert_eq!(operation, op(0xd7));
        }
        other => panic!("expected SourceFrozenByOther, got {other:?}"),
    }

    // The matching operation with different plan bytes/target refuses too:
    // an abort must name exactly what was frozen.
    let foreign_plan = AbortRequest {
        operation: op(0xd7),
        plan_set_digest: [0x55; 32],
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe7),
        target_schema: schema_id(&pinned_schema()).unwrap(),
        target_descriptor: &pinned_schema(),
    };
    assert!(matches!(
        runner.abort(&foreign_plan, &work()),
        Err(MigrationError::PlanSetMismatch)
    ));

    // Nothing thawed either way.
    assert!(submit_refuses_frozen(&history, 0x73));
}

#[test]
fn a_kill_between_fence_and_thaw_resumes_frozen_and_thaws_on_retry() {
    // The crash window chapter 22 names: the target fence is durable but
    // the process died before the source thawed. The source stays frozen
    // (safely resumable), and the abort retry completes fence-then-thaw.
    let (db, root) = fresh_source("fence-then-kill");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = failing_manifest();
    let steps = vec![StepInput {
        plan: failing_plan(),
        to_descriptor: pinned_schema(),
    }];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xde),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xee),
    };
    assert!(runner.migrate(&request, &work()).is_err());
    assert!(submit_refuses_frozen(&history, 0x74));

    // "Kill before thaw": only the standalone fence ran.
    let psd = plan_set_digest(&manifest, 0, 1, CAP).unwrap();
    let identity = DatabaseIdentity {
        database_id: db_id(0xa1),
        incarnation_id: incarnation(0xee),
        schema_id: schema_id(&pinned_schema()).unwrap(),
    };
    let reason = DeletedReason::MigrationAborted {
        source_database: db_id(0xa1),
        source_incarnation: incarnation(0xb1),
        plan_set_digest: psd,
    };
    assert_eq!(
        fence_target(
            &root.join("targets"),
            identity,
            op(0xde),
            reason,
            &pinned_schema(),
            LIMITS,
            &work(),
        )
        .unwrap(),
        TargetFence::TombstonePreGenesis
    );
    // The fence alone thawed nothing: still frozen, still resumable.
    assert!(submit_refuses_frozen(&history, 0x75));

    // The abort retry finds the durable fence and completes the thaw.
    let abort = AbortRequest {
        operation: op(0xde),
        plan_set_digest: psd,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xee),
        target_schema: schema_id(&pinned_schema()).unwrap(),
        target_descriptor: &pinned_schema(),
    };
    let report = runner.abort(&abort, &work()).unwrap();
    assert_eq!(report.fence, TargetFence::AlreadyFenced);
    assert!(report.thawed, "the retry completed the interrupted thaw");
    insert_note(&history, 2, "after-resumed-thaw", 2);
}

#[test]
fn fence_target_alone_never_touches_any_source() {
    // The standalone fence entrypoint (the hosted/local shared shape):
    // fencing a never-planned namespace installs the tombstone and does not
    // interact with any source authority at all.
    let root = temp_dir("fence-standalone");
    let identity = DatabaseIdentity {
        database_id: db_id(0xa9),
        incarnation_id: incarnation(0xe9),
        schema_id: schema_id(&pinned_schema()).unwrap(),
    };
    let fence = fence_target(
        &root,
        identity,
        op(0xda),
        DeletedReason::MigrationAborted {
            source_database: db_id(0xa9),
            source_incarnation: incarnation(0xb9),
            plan_set_digest: [3; 32],
        },
        &pinned_schema(),
        LIMITS,
        &work(),
    )
    .unwrap();
    assert_eq!(fence, TargetFence::TombstonePreGenesis);
    // The recorded control decodes to the exact cancelled-before-genesis
    // authority: identity, operation and reason all bound.
    let namespace = TargetNamespace::new(&root, incarnation(0xe9)).unwrap();
    let recorded = namespace.read_tombstone(CAP).unwrap().unwrap();
    assert_eq!(recorded.identity, identity);
    assert!(matches!(
        recorded.lifecycle,
        Lifecycle::Deleted { operation, .. } if operation == op(0xda)
    ));
    assert_eq!(recorded.activation, Activation::NotActivated);
    // Idempotent for the matching operation; conflicting for another.
    assert_eq!(
        fence_target(
            &root,
            identity,
            op(0xda),
            DeletedReason::MigrationAborted {
                source_database: db_id(0xa9),
                source_incarnation: incarnation(0xb9),
                plan_set_digest: [3; 32],
            },
            &pinned_schema(),
            LIMITS,
            &work(),
        )
        .unwrap(),
        TargetFence::AlreadyFenced
    );
    assert!(matches!(
        fence_target(
            &root,
            identity,
            op(0xdb),
            DeletedReason::Erasure,
            &pinned_schema(),
            LIMITS,
            &work(),
        ),
        Err(MigrationError::TargetConflict)
    ));
}

#[test]
fn wrong_and_stale_activation_references_refuse() {
    let (db, root) = fresh_source("stale-ref");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = manifest();
    let steps = vec![
        StepInput {
            plan: plan_pinned(),
            to_descriptor: pinned_schema(),
        },
        StepInput {
            plan: support::plan_tagged(),
            to_descriptor: tagged_schema(),
        },
    ];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xdc),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xec),
    };
    let reference = match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };

    // A reference against an absent namespace is stale.
    let mut absent = reference;
    absent.target.incarnation_id = incarnation(0xff);
    assert!(matches!(
        activate_target(
            &root.join("targets"),
            &absent,
            &tagged_schema(),
            LIMITS,
            &work()
        ),
        Err(MigrationError::StaleActivationRef)
    ));

    // A reference with a doctored genesis digest refuses before any
    // transition is attempted.
    let mut doctored = reference;
    let mut bytes = *doctored.target_genesis.as_bytes();
    bytes[0] ^= 1;
    doctored.target_genesis = bumbledb_log::history::DecisionDigest::from_bytes(bytes);
    assert!(matches!(
        activate_target(
            &root.join("targets"),
            &doctored,
            &tagged_schema(),
            LIMITS,
            &work()
        ),
        Err(MigrationError::StaleActivationRef)
    ));

    // The exact reference activates once; the matching retry returns the
    // recorded evidence plus the CURRENT access mode without remutation.
    let first = activate_target(
        &root.join("targets"),
        &reference,
        &tagged_schema(),
        LIMITS,
        &work(),
    )
    .unwrap();
    let second = activate_target(
        &root.join("targets"),
        &reference,
        &tagged_schema(),
        LIMITS,
        &work(),
    )
    .unwrap();
    assert_eq!(first.activation, second.activation);
    assert_eq!(second.access, bumbledb_log::history::AccessMode::Active);

    // The control on disk carries the one-time activation marker.
    let namespace = TargetNamespace::new(&root.join("targets"), incarnation(0xec)).unwrap();
    let target: Db<SchemaDescriptor> = Db::open(&namespace.target_dir(), tagged_schema()).unwrap();
    let mut control = None;
    target
        .read(|read| {
            control = read.integration_host_attachment()?.map(<[u8]>::to_vec);
            Ok(())
        })
        .unwrap();
    let authority = decode_control(&control.unwrap(), CAP).unwrap();
    assert!(matches!(
        authority.activation,
        Activation::Activated { operation, .. } if operation == op(0xdc)
    ));
}
