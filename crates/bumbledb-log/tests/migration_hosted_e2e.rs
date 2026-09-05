//! The COMPLETE hosted migration/initialization workflow (F3 finding-D
//! regressions): S3 IS the hosted authority. One `HostedMigration::migrate`
//! freezes the composed source head, executes the generated plans, builds
//! ONE judged staged target, uploads its verified checkpoint (state AND
//! migration-history metadata) under the target's open object epoch,
//! publishes the composed genesis head once, and returns a durable
//! `ReadyToSwitch`; activation is explicit; `hosted::initialize` runs the
//! generated chain (seeds exactly once) into a brand-new hosted incarnation.
//! Lost responses resolve from durable evidence; tampered/divergent targets
//! refuse; a GC barrier never collects the staged recovery objects before
//! activation or abort; process loss resumes from the store alone.
//! Driven by the production `MemStore`/`FsStore` adapters (faults are
//! explicit script entries / phase hooks); real S3 is the P05/P12 F3 lane.
//! Maps to MIG-01/02/03/05/08/09/10/11/12/14 hosted halves and OPS-001.

#[path = "migration_support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{ChangeSet, Db, Id128, RelationId, Value};

use bumbledb_log::checkpointer::CheckpointPolicy;
use bumbledb_log::codec::StreamLimits;
use bumbledb_log::gc::{GcPolicy, run_collection};
use bumbledb_log::history::authority::{
    Access, Activation, FreezeIntent, FreezeOutcome, HeadAuthority,
};
use bumbledb_log::history::command::{Command, CommandMetadata};
use bumbledb_log::history::decision::{GenesisProvenance, GenesisRecord, genesis_stamp};
use bumbledb_log::history::{
    AccessMode, CommandId, CommandResult, Condition, DatabaseIdentity, ReceiptEpoch, RequestId,
};
use bumbledb_log::manifest::{GcPhase, HeadRecord, RecoveryRoot, decode_head, encode_head};
use bumbledb_log::migration::executor::{
    ActivationRef, MigrateOutcome, MigrationError, MigrationStatus, StepInput, SuffixRequest,
    TargetFence,
};
use bumbledb_log::migration::history::{AppliedSource, HistoryRecord, decode_record, history_key};
use bumbledb_log::migration::hosted::{self, HostedMigration, HostedOutcome};
use bumbledb_log::migration::manifest::{Manifest, append_entry, plan_set_digest};
use bumbledb_log::recovery;
use bumbledb_log::schema_file::schema_id;
use bumbledb_log::store::mem::{Behavior, Gate, MemStore, Op};
use bumbledb_log::writer::verbs::{ConditionalStore as _, HeadRead};
use bumbledb_log::writer::{HostedHistory, LogError, SubmitOutcome};

use support::{
    CAP, LIMITS, base_schema, db_id, incarnation, manifest, op, pinned_schema, plan_pinned,
    plan_tagged, tagged_schema, temp_dir, work,
};

const SOURCE: &str = "tenant/source";
const TARGET: &str = "tenant/target";
const SOURCE_EPOCH: u64 = 1;
const TARGET_EPOCH: u64 = 1;
/// The stable migration operation every arm fixes before dispatch.
const MIGRATE_OP: u8 = 0xe1;

fn source_head_key() -> String {
    format!("{SOURCE}/HEAD")
}

fn target_head_key() -> String {
    format!("{TARGET}/HEAD")
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
    plans: &'a Manifest,
    steps: &'a [StepInput],
    operation: u8,
    target_inc: u8,
) -> SuffixRequest<'a> {
    SuffixRequest {
        operation: op(operation),
        manifest: plans,
        source_descriptor: base_schema(),
        steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(target_inc),
    }
}

fn target_identity() -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: db_id(0xa1),
        incarnation_id: incarnation(0xb2),
        schema_id: schema_id(&tagged_schema()).unwrap(),
    }
}

/// A hosted source: local materialization plus its composed genesis head in
/// the store — exactly what `HostedHistory::create` publishes.
fn hosted_source<'s>(
    store: &'s MemStore,
    root: &Path,
) -> (
    Arc<Db<SchemaDescriptor>>,
    HostedHistory<SchemaDescriptor, &'s MemStore>,
) {
    let dir = root.join("source-db");
    let db = Arc::new(
        Db::create(&dir, base_schema(), work())
            .expect("create store")
            .expect("empty store admits"),
    );
    let history = HostedHistory::create(
        Arc::clone(&db),
        store,
        SOURCE.to_string(),
        SOURCE_EPOCH,
        db_id(0xa1),
        incarnation(0xb1),
        op(0xc1),
        LIMITS,
        &work(),
    )
    .expect("create hosted source");
    (db, history)
}

fn insert_notes(
    history: &HostedHistory<SchemaDescriptor, &MemStore>,
    rows: &[(u64, &str)],
    request_byte: u8,
) {
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
                request_id: RequestId::from_core(Id128::from_bytes([request_byte; 16])),
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

fn driver<'a>(
    db: &'a Arc<Db<SchemaDescriptor>>,
    store: &'a MemStore,
    scratch: &Path,
) -> HostedMigration<'a, SchemaDescriptor, MemStore> {
    HostedMigration::new(
        db,
        store,
        SOURCE,
        TARGET,
        TARGET_EPOCH,
        scratch,
        LIMITS,
        CheckpointPolicy::DEFAULT,
    )
}

fn head_record(store: &MemStore, key: &str) -> HeadRecord {
    match store.read_head(key).unwrap() {
        HeadRead::Present { body, .. } => decode_head(&body, CAP).unwrap(),
        HeadRead::Absent => panic!("head must exist: {key}"),
    }
}

fn source_access(store: &MemStore) -> Access {
    head_record(store, &source_head_key())
        .control
        .live()
        .unwrap()
        .access
}

fn ready(outcome: HostedOutcome<MigrateOutcome>) -> (ActivationRef, u64) {
    match outcome {
        HostedOutcome::Completed(MigrateOutcome::ReadyToSwitch {
            activation_ref,
            applied,
        }) => (activation_ref, applied.steps.len() as u64),
        other => panic!("expected ReadyToSwitch, got {other:?}"),
    }
}

fn scan_all(db: &Db<SchemaDescriptor>, relation: RelationId) -> Vec<Vec<Value>> {
    let mut rows = Vec::new();
    db.read(work(), |read| {
        for row in read.scan(relation)? {
            rows.push(row?);
        }
        Ok(())
    })
    .unwrap();
    rows.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    rows
}

fn read_target_chain(db: &Db<SchemaDescriptor>) -> Vec<HistoryRecord> {
    let mut rows: Vec<Vec<u8>> = Vec::new();
    db.read(work(), |read| {
        let mut index = 0u64;
        while let Some(bytes) = read.integration_host_record(&history_key(index)).unwrap() {
            rows.push(bytes.to_vec());
            index += 1;
        }
        Ok(())
    })
    .unwrap();
    rows.iter()
        .map(|bytes| decode_record(bytes, CAP).unwrap())
        .collect()
}

fn target_object_keys(store: &MemStore, kind: &str) -> Vec<String> {
    store
        .object_keys()
        .into_iter()
        .filter(|key| key.starts_with("tenant/target/objects/") && key.contains(kind))
        .collect()
}

// ---------------------------------------------------------------------------
// Creation/initialization: the hosted genesis-rooted target.
// ---------------------------------------------------------------------------

#[test]
fn hosted_initialization_publishes_a_hydratable_genesis_rooted_target() {
    let root = temp_dir("host-init");
    let store = MemStore::new();
    let plans = manifest();
    let steps = steps_full();
    let init = request(&plans, &steps, MIGRATE_OP, 0xb2);

    let outcome = hosted::initialize(
        &store,
        TARGET,
        TARGET_EPOCH,
        &root.join("scratch"),
        &init,
        LIMITS,
        &CheckpointPolicy::DEFAULT,
        &work(),
    )
    .unwrap();
    let (reference, step_count) = ready(outcome);
    assert_eq!(step_count, 2, "one Applied record covers the whole chain");
    assert_eq!(reference.target, target_identity());

    // The published head is the composed genesis record: frozen,
    // NotActivated, and its recovery root NAMES the uploaded checkpoint —
    // the hosted data plane, not a bare authority pointer.
    let record = head_record(&store, &target_head_key());
    assert_eq!(record.control.identity, target_identity());
    assert_eq!(record.control.activation, Activation::NotActivated);
    assert!(matches!(
        record.control.live().unwrap().access,
        Access::Frozen { .. }
    ));
    assert_eq!(record.object_epoch, TARGET_EPOCH);
    let recovery_root = record.recovery.expect("live head names its recovery");
    let checkpoint = recovery_root.checkpoint.expect("checkpoint recovery root");
    assert_eq!(checkpoint.epoch, TARGET_EPOCH, "staged under the open epoch");
    assert_eq!(recovery_root.base, recovery_root.tip, "genesis base");
    assert!(
        !target_object_keys(&store, "/ckpt/").is_empty(),
        "the checkpoint manifest object exists"
    );

    // The matching rerun is verified reuse of the identical durable output:
    // same reference, no head revision spent.
    let again = hosted::initialize(
        &store,
        TARGET,
        TARGET_EPOCH,
        &root.join("scratch"),
        &init,
        LIMITS,
        &CheckpointPolicy::DEFAULT,
        &work(),
    )
    .unwrap();
    let (reference_again, _) = ready(again);
    assert_eq!(reference, reference_again);

    // Explicit activation, then evidence-only retry.
    let cutover = bumbledb_log::migration::hosted::HostedCutover::new(
        &store,
        SOURCE,
        TARGET,
        TARGET_EPOCH,
        LIMITS,
    );
    match cutover.activate(&reference).unwrap() {
        HostedOutcome::Completed(report) => assert_eq!(report.access, AccessMode::Active),
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    let rerun = hosted::initialize(
        &store,
        TARGET,
        TARGET_EPOCH,
        &root.join("scratch"),
        &init,
        LIMITS,
        &CheckpointPolicy::DEFAULT,
        &work(),
    )
    .unwrap();
    assert!(matches!(
        rerun,
        HostedOutcome::Completed(MigrateOutcome::AlreadyActivated { .. })
    ));

    // A fresh host hydrates the incarnation from the store alone: seeds ran
    // exactly once and the authoritative history chain rides the checkpoint.
    let recovered = recovery::open_hosted(
        &root.join("hydrated"),
        tagged_schema(),
        &store,
        "mem",
        TARGET,
        LIMITS,
        StreamLimits::DEFAULT,
        CAP,
        &work(),
    )
    .unwrap();
    assert_eq!(scan_all(&recovered.db, RelationId(0)), Vec::<Vec<Value>>::new());
    assert_eq!(
        scan_all(&recovered.db, RelationId(1)),
        vec![vec![Value::String("seeded".into())]],
        "the canonical seed row exists exactly once"
    );
    let chain = read_target_chain(&recovered.db);
    assert_eq!(chain.len(), 1);
    let HistoryRecord::Applied(applied) = &chain[0] else {
        panic!("initialization records one Applied");
    };
    assert_eq!(applied.operation, op(MIGRATE_OP));
    assert!(matches!(applied.source, AppliedSource::EmptyBase { .. }));
    assert_eq!(applied.steps.len(), 2);
}

// ---------------------------------------------------------------------------
// The full migrate workflow: freeze -> execute -> data plane -> ReadyToSwitch
// -> explicit activation -> hydration.
// ---------------------------------------------------------------------------

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end freeze/execute/publish/activate/hydrate schedule"
)]
fn full_hosted_migration_reaches_ready_to_switch_and_activates_explicitly() {
    let root = temp_dir("host-migrate");
    let store = MemStore::new();
    let (db, history) = hosted_source(&store, &root);
    insert_notes(&history, &[(1, "alpha"), (2, "beta")], 1);
    let plans = manifest();
    let steps = steps_full();
    let runner = driver(&db, &store, &root.join("scratch"));

    // Read-only status first: two pending entries, nothing written.
    assert!(matches!(
        runner.status(&plans, &work()).unwrap(),
        MigrationStatus::Pending {
            applied: 0,
            pending: 2
        }
    ));

    let outcome = runner
        .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
        .unwrap();
    let (reference, _) = ready(outcome);

    // The source is durably frozen under the operation; the target head is
    // the composed genesis whose recovery root names the checkpoint.
    assert!(matches!(source_access(&store), Access::Frozen { .. }));
    let record = head_record(&store, &target_head_key());
    assert_eq!(record.control.activation, Activation::NotActivated);
    assert!(record.recovery.unwrap().checkpoint.is_some());
    match runner.status(&plans, &work()).unwrap() {
        MigrationStatus::Frozen {
            operation,
            target_present,
            target_cancelled,
            ..
        } => {
            assert_eq!(operation, op(MIGRATE_OP));
            assert!(target_present);
            assert!(!target_cancelled);
        }
        other => panic!("expected Frozen, got {other:?}"),
    }

    // A matching migrate retry is verified reuse: identical reference,
    // identical Applied, no second lineage, no head revision spent.
    let version_before = match store.read_head(&target_head_key()).unwrap() {
        HeadRead::Present { version, .. } => version,
        HeadRead::Absent => panic!("target head exists"),
    };
    let again = runner
        .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
        .unwrap();
    let (reference_again, _) = ready(again);
    assert_eq!(reference, reference_again);
    match store.read_head(&target_head_key()).unwrap() {
        HeadRead::Present { version, .. } => assert_eq!(version, version_before),
        HeadRead::Absent => panic!("target head exists"),
    }

    // Explicit activation; the source STAYS frozen (thaw is a separate
    // deliberate release, never a side effect of activation).
    match runner.activate(&reference).unwrap() {
        HostedOutcome::Completed(report) => assert_eq!(report.access, AccessMode::Active),
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    assert!(matches!(source_access(&store), Access::Frozen { .. }));

    // After activation the same operation reports recorded evidence.
    let rerun = runner
        .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
        .unwrap();
    assert!(matches!(
        rerun,
        HostedOutcome::Completed(MigrateOutcome::AlreadyActivated { .. })
    ));

    // A fresh host hydrates the MIGRATED incarnation from the store alone —
    // this is the actual public-to-native call path a new deployment takes.
    let recovered = recovery::open_hosted(
        &root.join("hydrated"),
        tagged_schema(),
        &store,
        "mem",
        TARGET,
        LIMITS,
        StreamLimits::DEFAULT,
        CAP,
        &work(),
    )
    .unwrap();
    assert_eq!(
        scan_all(&recovered.db, RelationId(0)),
        vec![
            vec![
                Value::U64(1),
                Value::String("alpha".into()),
                Value::Bool(false)
            ],
            vec![
                Value::U64(2),
                Value::String("beta".into()),
                Value::Bool(false)
            ],
        ],
        "mapped rows carry the defaulted pinned field"
    );
    assert_eq!(
        scan_all(&recovered.db, RelationId(1)),
        vec![vec![Value::String("seeded".into())]]
    );
    let chain = read_target_chain(&recovered.db);
    assert_eq!(chain.len(), 1, "hosted-create sources start an empty chain");
    let HistoryRecord::Applied(applied) = &chain[0] else {
        panic!("one Applied for the whole suffix");
    };
    match applied.source {
        AppliedSource::Database {
            database,
            incarnation: source_inc,
            ..
        } => {
            assert_eq!(database, db_id(0xa1));
            assert_eq!(source_inc, incarnation(0xb1));
        }
        AppliedSource::EmptyBase { .. } => panic!("migration source is the database"),
    }
    assert_eq!(applied.steps.len(), 2);
}

// ---------------------------------------------------------------------------
// Certainty: lost responses resolve from durable evidence; Unknown never
// thaws; status resolves the operation afterwards.
// ---------------------------------------------------------------------------

#[test]
fn lost_responses_resolve_from_durable_evidence_and_unknown_never_thaws() {
    // Arm 1: the freeze CAS lands but its response is lost — the driver
    // re-reads its own recorded freeze and completes the whole migration.
    let root = temp_dir("host-lost-freeze");
    let store = MemStore::new();
    let (db, history) = hosted_source(&store, &root);
    insert_notes(&history, &[(1, "alpha")], 1);
    let plans = manifest();
    let steps = steps_full();
    let runner = driver(&db, &store, &root.join("scratch"));
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
    let (reference, _) = ready(
        runner
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );

    // Arm 2: the genesis create lands but its response is lost — resolved by
    // re-reading the exact recorded control.
    let root2 = temp_dir("host-lost-create");
    let store2 = MemStore::new();
    let (db2, history2) = hosted_source(&store2, &root2);
    insert_notes(&history2, &[(1, "alpha")], 1);
    let runner2 = driver(&db2, &store2, &root2.join("scratch"));
    store2.fail_next(Op::CreateHead, Behavior::IndeterminateApplied);
    let (reference2, _) = ready(
        runner2
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
    assert_eq!(
        reference, reference2,
        "identical source/plans/operation produce the identical durable output"
    );

    // Arm 3: the genesis create request is DROPPED — the workflow reports
    // Unknown, mutates no target, and the source stays frozen (uncertainty
    // never thaws). Status resolves the operation; the retry under the SAME
    // operation resumes from durable evidence and completes identically.
    let root3 = temp_dir("host-unknown-create");
    let store3 = MemStore::new();
    let (db3, history3) = hosted_source(&store3, &root3);
    insert_notes(&history3, &[(1, "alpha")], 1);
    let runner3 = driver(&db3, &store3, &root3.join("scratch"));
    store3.fail_next(Op::CreateHead, Behavior::IndeterminateDropped);
    assert!(matches!(
        runner3
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
        HostedOutcome::Unknown
    ));
    assert!(
        matches!(source_access(&store3), Access::Frozen { .. }),
        "Unknown never thaws"
    );
    assert!(
        matches!(store3.read_head(&target_head_key()).unwrap(), HeadRead::Absent),
        "the dropped create landed nothing"
    );
    match runner3.status(&plans, &work()).unwrap() {
        MigrationStatus::Frozen {
            operation,
            target_present,
            target_cancelled,
            ..
        } => {
            assert_eq!(operation, op(MIGRATE_OP));
            assert!(!target_present);
            assert!(!target_cancelled);
        }
        other => panic!("expected Frozen, got {other:?}"),
    }
    let (reference3, _) = ready(
        runner3
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
    assert_eq!(reference, reference3);
}

// ---------------------------------------------------------------------------
// Divergence and tampering refuse; different plan bytes cannot take over a
// frozen operation.
// ---------------------------------------------------------------------------

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "three isolated refusal arms: tampered objects, doctored head, plan-set takeover"
)]
fn divergence_and_tampering_refuse_instead_of_overwriting() {
    // Tampered checkpoint manifest bytes: the digest-verified fetch refuses
    // as corruption-class evidence, never a reused target.
    let root = temp_dir("host-tamper");
    let store = MemStore::new();
    let (db, history) = hosted_source(&store, &root);
    insert_notes(&history, &[(1, "alpha")], 1);
    let plans = manifest();
    let steps = steps_full();
    let runner = driver(&db, &store, &root.join("scratch"));
    let (_, _) = ready(
        runner
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
    let ckpt_keys = target_object_keys(&store, "/ckpt/");
    assert_eq!(ckpt_keys.len(), 1);
    assert!(store.corrupt_object(&ckpt_keys[0], |bytes| bytes[0] ^= 1));
    match runner.migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work()) {
        Err(MigrationError::Checkpoint(_)) => {}
        other => panic!("tampered checkpoint must refuse, got {other:?}"),
    }

    // Conflicting completed output under the SAME operation/plan set: a
    // doctored head whose genesis does not re-derive from the durable
    // evidence is OutputMismatch — never overwrite, never adopt.
    let root2 = temp_dir("host-conflict");
    let store2 = MemStore::new();
    let (db2, history2) = hosted_source(&store2, &root2);
    insert_notes(&history2, &[(1, "alpha")], 1);
    let runner2 = driver(&db2, &store2, &root2.join("scratch"));
    let (_, _) = ready(
        runner2
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
    let real = head_record(&store2, &target_head_key());
    let psd = plan_set_digest(&plans, 0, 2, CAP).unwrap();
    let doctored_genesis = GenesisRecord {
        identity: target_identity(),
        initial_application_digest: [9; 32],
        initial_system_digest: [8; 32],
        provenance: GenesisProvenance::Migration {
            source_database: db_id(0xa1),
            source_incarnation: incarnation(0xb1),
            plan_set_digest: psd,
        },
    };
    let stamp = genesis_stamp(&doctored_genesis, CAP).unwrap();
    let genesis_authority =
        HeadAuthority::genesis(target_identity(), stamp, Activation::NotActivated).unwrap();
    let frozen = match genesis_authority
        .freeze(
            op(MIGRATE_OP),
            FreezeIntent::Migration {
                plan_set_digest: psd,
                target: incarnation(0xb2),
            },
        )
        .unwrap()
    {
        FreezeOutcome::Frozen(frozen) => frozen,
        FreezeOutcome::AlreadyFrozen { .. } => unreachable!("fresh genesis"),
    };
    let doctored = HeadRecord {
        control: frozen,
        object_epoch: TARGET_EPOCH,
        recovery: Some(RecoveryRoot {
            checkpoint: real.recovery.unwrap().checkpoint,
            base: stamp,
            tip: stamp,
            tip_object: None,
            tail_bytes: 0,
            epoch_floor: TARGET_EPOCH,
        }),
        roots: Vec::new(),
        gc: GcPhase::Idle,
    };
    let doctored_bytes = encode_head(&doctored, CAP).unwrap();
    assert!(store2.corrupt_head(&target_head_key(), |body| *body = doctored_bytes.clone()));
    match runner2.migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work()) {
        Err(MigrationError::OutputMismatch) => {}
        other => panic!("conflicting completed output must refuse, got {other:?}"),
    }

    // Different plan bytes cannot take over the frozen operation: the same
    // operation with a different plan set is PlanSetMismatch, and the
    // ORIGINAL bytes still own and complete it.
    let root3 = temp_dir("host-takeover");
    let store3 = MemStore::new();
    let (db3, history3) = hosted_source(&store3, &root3);
    insert_notes(&history3, &[(1, "alpha")], 1);
    let runner3 = driver(&db3, &store3, &root3.join("scratch"));
    // One dropped create: the workflow dispatches exactly one genesis
    // create per invocation and resolves ambiguity by re-reading.
    store3.fail_next(Op::CreateHead, Behavior::IndeterminateDropped);
    assert!(matches!(
        runner3
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
        HostedOutcome::Unknown
    ));
    let mut short = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    append_entry(&mut short, &plan_pinned(), CAP).unwrap();
    let short_steps = vec![StepInput {
        plan: plan_pinned(),
        to_descriptor: pinned_schema(),
    }];
    match runner3.migrate(&request(&short, &short_steps, MIGRATE_OP, 0xb2), &work()) {
        Err(MigrationError::PlanSetMismatch) => {}
        other => panic!("plan-set takeover must refuse, got {other:?}"),
    }
    let (_, _) = ready(
        runner3
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
}

// ---------------------------------------------------------------------------
// The freeze boundary: a concurrent old writer is refused; reads remain.
// ---------------------------------------------------------------------------

#[test]
fn a_concurrent_old_writer_is_refused_after_the_source_freeze() {
    let root = temp_dir("host-old-writer");
    let store = MemStore::new();
    let (db, history) = hosted_source(&store, &root);
    insert_notes(&history, &[(1, "alpha")], 1);
    let plans = manifest();
    let steps = steps_full();
    let runner = driver(&db, &store, &root.join("scratch"));
    let (_, _) = ready(
        runner
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );

    // The old writer's new command is refused by the durable frozen head —
    // through the ACTUAL public submit path, not a mocked mode flag.
    let mut draft = ChangeSet::builder(history.db().schema(), work());
    draft
        .insert(
            RelationId(0),
            &[Value::U64(9), Value::String("late".into())],
        )
        .unwrap();
    let late = Command::seal(
        CommandMetadata {
            identity: history.identity(),
            id: CommandId {
                receipt_epoch: ReceiptEpoch::INITIAL,
                request_id: RequestId::from_core(Id128::from_bytes([0x77; 16])),
            },
            condition: Condition::Unconditional,
        },
        draft.finish().unwrap(),
        CommandResult::empty(),
        LIMITS,
        &work(),
    )
    .unwrap();
    match history.submit(&late, &work()) {
        SubmitOutcome::NotSubmitted { error, .. } => {
            assert_eq!(error, LogError::DatabaseFrozen);
        }
        other => panic!("a frozen source refuses new admission, got {other:?}"),
    }

    // Reads (catch-up) remain valid on the frozen source.
    let tip = history.catch_up(&work()).unwrap();
    assert_eq!(tip.seq, 1, "the pre-freeze decision is still readable");
    assert_eq!(
        scan_all(history.db(), RelationId(0)),
        vec![vec![Value::U64(1), Value::String("alpha".into())]]
    );
}

// ---------------------------------------------------------------------------
// GC interaction: staged recovery objects survive a barrier until activation
// or abort; a fenced target's objects become collectible.
// ---------------------------------------------------------------------------

#[test]
fn staged_recovery_objects_survive_gc_until_activation_or_abort() {
    // ReadyToSwitch, then a full GC pass over the TARGET prefix: the frozen
    // unactivated head's recovery root protects the checkpoint closure.
    let root = temp_dir("host-gc-keep");
    let store = MemStore::new();
    let (db, history) = hosted_source(&store, &root);
    insert_notes(&history, &[(1, "alpha")], 1);
    let plans = manifest();
    let steps = steps_full();
    let runner = driver(&db, &store, &root.join("scratch"));
    let (reference, _) = ready(
        runner
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
    let ckpt_before = target_object_keys(&store, "/ckpt/");
    let chunks_before = target_object_keys(&store, "/chunk/");
    assert!(!ckpt_before.is_empty());
    run_collection(
        &store,
        TARGET,
        op(0xf1),
        LIMITS,
        &GcPolicy::DEFAULT,
        &work(),
    )
    .unwrap();
    assert_eq!(
        target_object_keys(&store, "/ckpt/"),
        ckpt_before,
        "the staged checkpoint survives the barrier before activation"
    );
    assert_eq!(target_object_keys(&store, "/chunk/"), chunks_before);
    // Activation still works after the barrier advanced the object epoch.
    match runner.activate(&reference).unwrap() {
        HostedOutcome::Completed(report) => assert_eq!(report.access, AccessMode::Active),
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    run_collection(
        &store,
        TARGET,
        op(0xf2),
        LIMITS,
        &GcPolicy::DEFAULT,
        &work(),
    )
    .unwrap();
    assert_eq!(
        target_object_keys(&store, "/ckpt/"),
        ckpt_before,
        "the ACTIVE incarnation's recovery closure survives collection"
    );

    // Abort arm on a second setup: the fence tombstones the target (its
    // recovery root drops), and only then does a collection reclaim the
    // staged objects.
    let root2 = temp_dir("host-gc-abort");
    let store2 = MemStore::new();
    let (db2, history2) = hosted_source(&store2, &root2);
    insert_notes(&history2, &[(1, "alpha")], 1);
    let runner2 = driver(&db2, &store2, &root2.join("scratch"));
    let (_, _) = ready(
        runner2
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
    assert!(!target_object_keys(&store2, "/ckpt/").is_empty());
    let psd = plan_set_digest(&plans, 0, 2, CAP).unwrap();
    match runner2
        .abort(target_identity(), op(MIGRATE_OP), psd)
        .unwrap()
    {
        HostedOutcome::Completed(report) => {
            assert_eq!(report.fence, TargetFence::TargetDeleted);
            assert!(report.thawed);
        }
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    assert!(matches!(source_access(&store2), Access::Active));
    run_collection(
        &store2,
        TARGET,
        op(0xf3),
        LIMITS,
        &GcPolicy::DEFAULT,
        &work(),
    )
    .unwrap();
    assert!(
        target_object_keys(&store2, "/ckpt/").is_empty(),
        "a fenced target's checkpoint is collectible after the abort"
    );
    assert!(target_object_keys(&store2, "/chunk/").is_empty());
}

// ---------------------------------------------------------------------------
// Activation versus abort: exactly one winner, durable fencing first.
// ---------------------------------------------------------------------------

#[test]
fn activation_and_abort_race_through_the_driver_has_one_winner() {
    // Abort wins against a PAUSED activation: the paused CAS loses to the
    // durable fence and the activation resolves to Aborted.
    let root = temp_dir("host-race-abort");
    let store = MemStore::new();
    let (db, history) = hosted_source(&store, &root);
    insert_notes(&history, &[(1, "alpha")], 1);
    let plans = manifest();
    let steps = steps_full();
    let runner = driver(&db, &store, &root.join("scratch"));
    let (reference, _) = ready(
        runner
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
    let psd = plan_set_digest(&plans, 0, 2, CAP).unwrap();

    let gate = Arc::new(Gate::new());
    let paused = Arc::new(AtomicBool::new(false));
    {
        let gate = Arc::clone(&gate);
        let paused = Arc::clone(&paused);
        let key = target_head_key();
        store.set_gate(move |operation, gated_key| {
            if operation == Op::ReplaceHead
                && gated_key == key
                && !paused.swap(true, Ordering::SeqCst)
            {
                return Some(Arc::clone(&gate));
            }
            None
        });
    }
    std::thread::scope(|scope| {
        let activation = scope.spawn(|| {
            let runner = driver(&db, &store, &root.join("scratch"));
            runner.activate(&reference)
        });
        // Wait until the activation CAS is provably paused mid-flight.
        while !paused.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        // The abort fences the target durably FIRST, then thaws the source.
        match runner
            .abort(target_identity(), op(MIGRATE_OP), psd)
            .unwrap()
        {
            HostedOutcome::Completed(report) => {
                assert_eq!(report.fence, TargetFence::TargetDeleted);
                assert!(report.thawed);
            }
            HostedOutcome::Unknown => panic!("deterministic double"),
        }
        gate.open();
        // The paused activation loses its CAS, re-reads the tombstone and
        // permanently reports Aborted — never a revived target.
        match activation.join().expect("activation thread") {
            Err(MigrationError::Aborted { operation }) => {
                assert_eq!(operation, op(MIGRATE_OP));
            }
            other => panic!("the fenced activation reports Aborted, got {other:?}"),
        }
    });
    assert!(matches!(source_access(&store), Access::Active));

    // Activation wins first: the later abort refuses with ActivationWon and
    // thaws nothing.
    let root2 = temp_dir("host-race-activate");
    let store2 = MemStore::new();
    let (db2, history2) = hosted_source(&store2, &root2);
    insert_notes(&history2, &[(1, "alpha")], 1);
    let runner2 = driver(&db2, &store2, &root2.join("scratch"));
    let (reference2, _) = ready(
        runner2
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap(),
    );
    match runner2.activate(&reference2).unwrap() {
        HostedOutcome::Completed(report) => assert_eq!(report.access, AccessMode::Active),
        HostedOutcome::Unknown => panic!("deterministic double"),
    }
    assert!(matches!(
        runner2.abort(target_identity(), op(MIGRATE_OP), psd),
        Err(MigrationError::ActivationWon)
    ));
    assert!(
        matches!(source_access(&store2), Access::Frozen { .. }),
        "a refused abort thaws nothing"
    );
}

// ---------------------------------------------------------------------------
// Process loss: SIGABRT exactly at the genesis-publication boundary (before
// and after the create lands), resumed by a NEW process from the durable
// store alone (FsStore; the adversarial-process re-exec pattern).
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod process_loss {
    use super::*;
    use std::process::{Child, Command as ProcessCommand, Stdio};

    use bumbledb_log::store::fs::{FsStore, Inject, Phase};

    const CHILD_ENV: &str = "BDB_MIG_HOSTED_E2E_CHILD";
    const DIR_ENV: &str = "BDB_MIG_HOSTED_E2E_DIR";

    fn spawn_child(mode: &str, dir: &Path) -> Child {
        let mut command = ProcessCommand::new(std::env::current_exe().expect("test binary"));
        command
            .args([
                "--exact",
                "process_loss::child_process_entry",
                "--nocapture",
                "--test-threads",
                "1",
            ])
            .env(CHILD_ENV, mode)
            .env(DIR_ENV, dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.spawn().expect("spawn child")
    }

    fn source_db_dir(root: &Path) -> PathBuf {
        root.join("source-db")
    }

    fn store_dir(root: &Path) -> PathBuf {
        root.join("store")
    }

    fn fs_head_record(store: &FsStore, key: &str) -> Option<HeadRecord> {
        match store.read_head(key).unwrap() {
            HeadRead::Present { body, .. } => Some(decode_head(&body, CAP).unwrap()),
            HeadRead::Absent => None,
        }
    }

    /// Child mode `setup`: create the hosted source (local materialization +
    /// composed genesis head + one decided command) and exit cleanly so the
    /// parent's later opens are the only owner.
    fn child_setup(root: &Path) {
        let store = FsStore::new(store_dir(root));
        let db = Arc::new(
            Db::create(&source_db_dir(root), base_schema(), work())
                .expect("create store")
                .expect("empty store admits"),
        );
        let history = HostedHistory::create(
            Arc::clone(&db),
            &store,
            SOURCE.to_string(),
            SOURCE_EPOCH,
            db_id(0xa1),
            incarnation(0xb1),
            op(0xc1),
            LIMITS,
            &work(),
        )
        .expect("create hosted source");
        let mut draft = ChangeSet::builder(history.db().schema(), work());
        draft
            .insert(
                RelationId(0),
                &[Value::U64(1), Value::String("alpha".into())],
            )
            .unwrap();
        let command = Command::seal(
            CommandMetadata {
                identity: history.identity(),
                id: CommandId {
                    receipt_epoch: ReceiptEpoch::INITIAL,
                    request_id: RequestId::from_core(Id128::from_bytes([1; 16])),
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
            other => panic!("seed submit failed: {other:?}"),
        }
    }

    /// Child modes `kill-before-create` / `kill-after-create`: run the full
    /// hosted migrate but die by SIGABRT exactly at the target genesis
    /// create boundary — before the head lands (dropped request + process
    /// loss) or after it lands (durable effect, response never observed).
    fn child_migrate_killed(root: &Path, after: bool) {
        let store = FsStore::new(store_dir(root));
        let key = target_head_key();
        let armed_phase = if after {
            Phase::Published
        } else {
            Phase::HeadObserved
        };
        store.set_hook(move |phase, hook_key| {
            if hook_key == key && phase == armed_phase {
                // Real process loss mid-protocol, kernel locks released by
                // death — not an injected error return.
                std::process::abort();
            }
            Inject::Continue
        });
        let db = Arc::new(Db::open(&source_db_dir(root), base_schema(), work()).expect("open source"));
        let runner = HostedMigration::new(
            &db,
            &store,
            SOURCE,
            TARGET,
            TARGET_EPOCH,
            &root.join("scratch"),
            LIMITS,
            CheckpointPolicy::DEFAULT,
        );
        let plans = manifest();
        let steps = steps_full();
        // The abort fires inside this call; reaching the match is a failure.
        let outcome = runner.migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work());
        panic!("the child must die at the create boundary, got {outcome:?}");
    }

    /// The child dispatcher the parent re-execs (skips when not a child).
    #[test]
    fn child_process_entry() {
        let Ok(mode) = std::env::var(CHILD_ENV) else {
            return;
        };
        let root = PathBuf::from(std::env::var(DIR_ENV).expect("child dir"));
        match mode.as_str() {
            "setup" => child_setup(&root),
            "kill-before-create" => child_migrate_killed(&root, false),
            "kill-after-create" => child_migrate_killed(&root, true),
            other => panic!("unknown child mode {other}"),
        }
    }

    fn run_arm(tag: &str, mode: &str, target_present_after_kill: bool) {
        let root = super::temp_dir(tag);
        let status = spawn_child("setup", &root).wait().expect("setup child");
        assert!(status.success(), "setup child failed");
        let status = spawn_child(mode, &root).wait().expect("kill child");
        assert!(!status.success(), "the kill child must die by SIGABRT");

        // Durable evidence after process loss: the source is frozen (no
        // timer thaws it), and the target head reflects exactly how far the
        // durable protocol got.
        let store = FsStore::new(store_dir(&root));
        let source = fs_head_record(&store, &source_head_key()).expect("source head");
        assert!(matches!(
            source.control.live().unwrap().access,
            Access::Frozen { .. }
        ));
        let target = fs_head_record(&store, &target_head_key());
        assert_eq!(target.is_some(), target_present_after_kill);
        if let Some(record) = &target {
            assert_eq!(record.control.activation, Activation::NotActivated);
            assert!(record.recovery.unwrap().checkpoint.is_some());
        }

        // A NEW process resumes the SAME operation from the store alone:
        // reuse of the published verified target, or a clean re-execution
        // from the fixed frozen source — never two lineages.
        let db = Arc::new(Db::open(&source_db_dir(&root), base_schema(), work()).expect("open source"));
        let runner = HostedMigration::new(
            &db,
            &store,
            SOURCE,
            TARGET,
            TARGET_EPOCH,
            &root.join("scratch"),
            LIMITS,
            CheckpointPolicy::DEFAULT,
        );
        let plans = manifest();
        let steps = steps_full();
        let (reference, _) = match runner
            .migrate(&request(&plans, &steps, MIGRATE_OP, 0xb2), &work())
            .unwrap()
        {
            HostedOutcome::Completed(MigrateOutcome::ReadyToSwitch {
                activation_ref,
                applied,
            }) => (activation_ref, applied),
            other => panic!("resume must reach ReadyToSwitch, got {other:?}"),
        };
        match runner.activate(&reference).unwrap() {
            HostedOutcome::Completed(report) => assert_eq!(report.access, AccessMode::Active),
            HostedOutcome::Unknown => panic!("deterministic local store"),
        }

        // A fresh host hydrates the migrated incarnation from the store.
        let recovered = recovery::open_hosted(
            &root.join("hydrated"),
            tagged_schema(),
            &store,
            "fs",
            TARGET,
            LIMITS,
            StreamLimits::DEFAULT,
            CAP,
            &work(),
        )
        .unwrap();
        assert_eq!(
            scan_all(&recovered.db, RelationId(0)),
            vec![vec![
                Value::U64(1),
                Value::String("alpha".into()),
                Value::Bool(false)
            ]]
        );
        assert_eq!(
            scan_all(&recovered.db, RelationId(1)),
            vec![vec![Value::String("seeded".into())]]
        );
    }

    #[test]
    fn process_loss_before_the_genesis_create_resumes_by_reexecution() {
        run_arm("host-kill-before", "kill-before-create", false);
    }

    #[test]
    fn process_loss_after_the_genesis_create_resumes_by_verified_reuse() {
        run_arm("host-kill-after", "kill-after-create", true);
    }
}
