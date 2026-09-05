//! End-to-end local migration execution: canonical plans on a pinned frozen
//! source, ordered-step semantics with the core judge at every boundary,
//! one staged final target, seeds exactly once, and complete typed refusal
//! of invalid targets. Maps to MIG-01/02/04/11/12/13 and OPS-001.
//! Verification: `NotRun` (F1 authors, does not execute).

#[path = "migration_support/mod.rs"]
mod support;

use std::path::PathBuf;
use std::sync::Arc;

use bumbledb::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, Row, SchemaDescriptor, Side,
    StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{ChangeSet, Db, Id128, Value};

use bumbledb_log::history::command::{Command, CommandMetadata};
use bumbledb_log::history::{
    AccessMode, CommandId, CommandResult, Condition, ReceiptEpoch, RequestId,
};
use bumbledb_log::migration::executor::{
    LocalMigration, MigrateOutcome, MigrationError, MigrationStatus, StepInput, SuffixRequest,
    activate_target, initialize,
};
use bumbledb_log::migration::history::{AppliedSource, HistoryRecord};
use bumbledb_log::migration::manifest::Manifest;
use bumbledb_log::migration::plan::{FieldMap, Operation, Plan, PlanExpr};
use bumbledb_log::migration::state::StateError;
use bumbledb_log::schema_file::schema_id;
use bumbledb_log::writer::{LocalHistory, LogError, SubmitOutcome};

use support::{
    CAP, LIMITS, base_schema, copy_field, db_id, fresh_source, incarnation, manifest, op,
    pinned_schema, plan_pinned, plan_tagged, tagged_schema, tiny_work, work,
};

/// Hex directory name of a 16-byte id, as the executor stages targets.
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

#[test]
fn whole_suffix_executes_into_one_frozen_verified_target() {
    let (db, root) = fresh_source("suffix");
    let history = open_history(&db);
    insert_notes(&history, &[(1, "alpha"), (2, "beta")], 1);
    let manifest = manifest();
    let steps = steps_full();
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);

    // Before: two pending entries.
    assert!(matches!(
        runner.status(&manifest, &work()).unwrap(),
        MigrationStatus::Pending {
            applied: 0,
            pending: 2
        }
    ));

    let request = request(&manifest, &steps, 0xd1, 0xe1);
    let (reference, applied) = match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch {
            activation_ref,
            applied,
        } => (activation_ref, applied),
        other => panic!("expected ReadyToSwitch, got {other:?}"),
    };

    // ONE Applied batch covering both manifest entries, from the captured
    // frozen source position.
    assert_eq!(applied.steps.len(), 2);
    assert!(matches!(applied.source, AppliedSource::Database { .. }));

    // The source is frozen: new admission refuses, reads continue.
    let frozen_submit = {
        let mut draft = ChangeSet::builder(history.db().schema(), work());
        draft
            .insert(
                RelationId(0),
                &[Value::U64(9), Value::String("late".into())],
            )
            .unwrap();
        let command = Command::seal(
            CommandMetadata {
                identity: history.identity(),
                id: CommandId {
                    receipt_epoch: ReceiptEpoch::INITIAL,
                    request_id: RequestId::from_core(Id128::from_bytes([9; 16])),
                },
                condition: Condition::Unconditional,
            },
            draft.finish().unwrap(),
            CommandResult::empty(),
            LIMITS,
            &work(),
        )
        .unwrap();
        history.submit(&command, &work())
    };
    assert!(matches!(
        frozen_submit,
        SubmitOutcome::NotSubmitted {
            error: LogError::DatabaseFrozen,
            ..
        }
    ));
    assert_eq!(scan_all(&db, RelationId(0)).len(), 2, "reads stay valid");

    // The published target is complete, still frozen, and carries the
    // transformed rows plus the seed exactly once.
    let target_dir: PathBuf = root
        .join("targets")
        .join(hex_name(incarnation(0xe1).as_core().as_bytes()));
    {
        // (Scoped so the handle closes before any later open of this env.)
        let target: Db<SchemaDescriptor> = Db::open(&target_dir, tagged_schema(), work()).unwrap();
        let notes = scan_all(&target, RelationId(0));
        assert_eq!(notes.len(), 2);
        for row in &notes {
            assert_eq!(row.len(), 3, "id, body, pinned");
            assert_eq!(row[2], Value::Bool(false), "backfilled default");
        }
        let tags = scan_all(&target, RelationId(1));
        assert_eq!(tags, vec![vec![Value::String("seeded".into())]]);

        // The target refuses commands while AwaitingCutover.
        let target_history = LocalHistory::open(Arc::new(target), LIMITS).unwrap();
        let view = target_history.authority().unwrap();
        assert!(view.admission_view().is_some());
        assert_eq!(
            view.admission_view().unwrap().access,
            AccessMode::Frozen,
            "ReadyToSwitch is frozen until explicit activation"
        );
    }

    // Explicit activation flips exactly the target; the source stays frozen.
    let report = activate_target(
        &root.join("targets"),
        &reference,
        &tagged_schema(),
        LIMITS,
        &work(),
    )
    .unwrap();
    assert_eq!(report.access, AccessMode::Active);
    assert!(matches!(
        runner.status(&manifest, &work()).unwrap(),
        MigrationStatus::Frozen { .. }
    ));

    // The activated target's own status is UpToDate against the manifest.
    let target: Db<SchemaDescriptor> = Db::open(&target_dir, tagged_schema(), work()).unwrap();
    let target_history = LocalHistory::open(Arc::new(target), LIMITS).unwrap();
    let target_runner = LocalMigration::new(&target_history, &root.join("targets2"), LIMITS);
    assert!(matches!(
        target_runner.status(&manifest, &work()).unwrap(),
        MigrationStatus::UpToDate { applied: 2 }
    ));
}

#[test]
fn migrate_with_nothing_pending_is_up_to_date_not_a_new_incarnation() {
    let (db, root) = fresh_source("noop");
    let history = open_history(&db);
    let empty = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = request(&empty, &[], 0xd2, 0xe2);
    assert!(matches!(
        runner.migrate(&request, &work()).unwrap(),
        MigrateOutcome::UpToDate { applied: 0 }
    ));
    // No freeze happened and no namespace was created.
    assert!(matches!(
        runner.status(&empty, &work()).unwrap(),
        MigrationStatus::UpToDate { applied: 0 }
    ));
}

#[test]
fn intermediate_law_failure_refuses_before_any_target_exists() {
    // Step 1 duplicates note ids under the pinned key: the INTERMEDIATE
    // judge boundary must report it even though step 2 could mask it.
    let (db, root) = fresh_source("intermediate");
    let history = open_history(&db);
    insert_notes(&history, &[(1, "alpha"), (2, "beta")], 1);

    // A hostile plan 0: map id -> literal 7 (collides both rows under the
    // key), then a plan 1 that would drop and recreate the relation empty.
    let broken_pinned = Plan {
        operations: vec![
            Operation::MapRelation {
                source: "Note".into(),
                target: "Note".into(),
                fields: vec![
                    FieldMap {
                        target: "id".into(),
                        expression: PlanExpr::Literal(Value::U64(7)),
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
        destructive: vec![bumbledb_log::migration::plan::Loss {
            relation: "Note".into(),
            field: Some("id".into()),
        }],
        ..plan_pinned()
    };
    let mut manifest = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    bumbledb_log::migration::manifest::append_entry(&mut manifest, &broken_pinned, CAP).unwrap();
    bumbledb_log::migration::manifest::append_entry(&mut manifest, &plan_tagged(), CAP).unwrap();
    let steps = vec![
        StepInput {
            plan: broken_pinned,
            to_descriptor: pinned_schema(),
        },
        StepInput {
            plan: plan_tagged(),
            to_descriptor: tagged_schema(),
        },
    ];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = request(&manifest, &steps, 0xd3, 0xe3);
    match runner.migrate(&request, &work()) {
        Err(MigrationError::State(StateError::Rejected { violations, .. })) => {
            assert!(!violations.is_empty(), "the exact violated statements");
        }
        other => panic!("expected intermediate rejection, got {other:?}"),
    }
    // The source is frozen (resumable, no timer thaws it) and NO target
    // was published.
    assert!(matches!(
        runner.status(&manifest, &work()).unwrap(),
        MigrationStatus::Frozen {
            target_present: false,
            target_cancelled: false,
            ..
        }
    ));
}

#[test]
fn expression_failure_on_an_actual_row_is_the_step_error_boundary() {
    // A convert that narrows body-length arithmetic… simpler: a divide by
    // zero on an actual row refuses with the core scalar error.
    let (db, root) = fresh_source("scalar");
    let history = open_history(&db);
    insert_notes(&history, &[(1, "alpha")], 1);
    let hostile = Plan {
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
    };
    let mut manifest = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    bumbledb_log::migration::manifest::append_entry(&mut manifest, &hostile, CAP).unwrap();
    let steps = vec![StepInput {
        plan: hostile,
        to_descriptor: pinned_schema(),
    }];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = request(&manifest, &steps, 0xd4, 0xe4);
    assert!(matches!(
        runner.migrate(&request, &work()),
        Err(MigrationError::State(StateError::Scalar { .. }))
    ));
}

#[test]
fn initialization_runs_the_chain_and_seeds_exactly_once() {
    let root = support::temp_dir("init");
    let manifest = manifest();
    let steps = steps_full();
    let request = SuffixRequest {
        operation: op(0xd5),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa2),
        target_incarnation: incarnation(0xe5),
    };
    let outcome = initialize(&root.join("targets"), &request, LIMITS, &work()).unwrap();
    let (reference, applied) = match outcome {
        MigrateOutcome::ReadyToSwitch {
            activation_ref,
            applied,
        } => (activation_ref, applied),
        other => panic!("expected ReadyToSwitch, got {other:?}"),
    };
    assert!(matches!(applied.source, AppliedSource::EmptyBase { .. }));
    assert_eq!(applied.steps.len(), 2);

    // Rerunning the initializer reuses the published verified target — it
    // never re-executes seeds into a new lineage.
    let again = initialize(&root.join("targets"), &request, LIMITS, &work()).unwrap();
    match again {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => {
            assert_eq!(activation_ref, reference);
        }
        other => panic!("expected idempotent ReadyToSwitch, got {other:?}"),
    }

    // Activate and observe exactly one seed row.
    activate_target(
        &root.join("targets"),
        &reference,
        &tagged_schema(),
        LIMITS,
        &work(),
    )
    .unwrap();
    let target_dir = root
        .join("targets")
        .join(hex_name(incarnation(0xe5).as_core().as_bytes()));
    let target: Db<SchemaDescriptor> = Db::open(&target_dir, tagged_schema(), work()).unwrap();
    assert_eq!(scan_all(&target, RelationId(1)).len(), 1, "one seed row");
    assert_eq!(
        scan_all(&target, RelationId(0)).len(),
        0,
        "no phantom notes"
    );

    // After activation, a rerun reports the recorded activation evidence.
    let after = initialize(&root.join("targets"), &request, LIMITS, &work()).unwrap();
    assert!(matches!(after, MigrateOutcome::AlreadyActivated { .. }));
}

#[test]
fn ordered_step_meaning_matches_the_independent_two_pass_evaluation() {
    // MIG-13's shape (mirroring the bench staged eval_graph oracle): the
    // one-operation fused suffix must equal running the two plans as two
    // separate operations through two incarnations — same final facts.
    let manifest = manifest();

    // Fused: both steps in one operation.
    let (db_a, root_a) = fresh_source("fused");
    let history_a = open_history(&db_a);
    insert_notes(&history_a, &[(1, "alpha"), (2, "beta")], 1);
    let steps = steps_full();
    let runner_a = LocalMigration::new(&history_a, &root_a.join("targets"), LIMITS);
    let fused_ref = match runner_a
        .migrate(&request(&manifest, &steps, 0xd6, 0xe6), &work())
        .unwrap()
    {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };

    // Stepwise: step 1 into one incarnation, activate, then step 2 from it.
    let (db_b, root_b) = fresh_source("stepwise");
    let history_b = open_history(&db_b);
    insert_notes(&history_b, &[(1, "alpha"), (2, "beta")], 1);
    let first_steps = vec![StepInput {
        plan: plan_pinned(),
        to_descriptor: pinned_schema(),
    }];
    let runner_b = LocalMigration::new(&history_b, &root_b.join("targets"), LIMITS);
    let first_ref = match runner_b
        .migrate(&request(&manifest, &first_steps, 0xd7, 0xe7), &work())
        .unwrap()
    {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };
    activate_target(
        &root_b.join("targets"),
        &first_ref,
        &pinned_schema(),
        LIMITS,
        &work(),
    )
    .unwrap();
    let mid_dir = root_b
        .join("targets")
        .join(hex_name(incarnation(0xe7).as_core().as_bytes()));
    let mid: Arc<Db<SchemaDescriptor>> = Arc::new(Db::open(&mid_dir, pinned_schema(), work()).unwrap());
    let mid_history = LocalHistory::open(Arc::clone(&mid), LIMITS).unwrap();
    let second_steps = vec![StepInput {
        plan: plan_tagged(),
        to_descriptor: tagged_schema(),
    }];
    let runner_mid = LocalMigration::new(&mid_history, &root_b.join("targets"), LIMITS);
    let second_request = SuffixRequest {
        operation: op(0xd8),
        manifest: &manifest,
        source_descriptor: pinned_schema(),
        steps: &second_steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe8),
    };
    let second_ref = match runner_mid.migrate(&second_request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => activation_ref,
        other => panic!("{other:?}"),
    };

    // Same final application facts either way.
    let fused_dir = root_a.join("targets").join(hex_name(
        fused_ref.target.incarnation_id.as_core().as_bytes(),
    ));
    let stepwise_dir = root_b.join("targets").join(hex_name(
        second_ref.target.incarnation_id.as_core().as_bytes(),
    ));
    let fused: Db<SchemaDescriptor> = Db::open(&fused_dir, tagged_schema(), work()).unwrap();
    let stepwise: Db<SchemaDescriptor> = Db::open(&stepwise_dir, tagged_schema(), work()).unwrap();
    assert_eq!(
        scan_all(&fused, RelationId(0)),
        scan_all(&stepwise, RelationId(0))
    );
    assert_eq!(
        scan_all(&fused, RelationId(1)),
        scan_all(&stepwise, RelationId(1))
    );
}

#[test]
fn exhausted_work_is_a_resource_refusal_never_a_partial_target() {
    let (db, root) = fresh_source("budget");
    let history = open_history(&db);
    insert_notes(&history, &[(1, "alpha"), (2, "beta"), (3, "gamma")], 1);
    let manifest = manifest();
    let steps = steps_full();
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = request(&manifest, &steps, 0xd9, 0xe9);
    // The tiny allowance exhausts mid-execution: a typed Work refusal.
    match runner.migrate(&request, &tiny_work()) {
        Err(MigrationError::Work(_) | MigrationError::State(StateError::Work(_))) => {}
        other => panic!("expected work refusal, got {other:?}"),
    }
    // No target namespace entry was published; the source froze durably
    // (freeze precedes execution) and the SAME operation completes later.
    match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { .. } => {}
        other => panic!("resume with the same operation, got {other:?}"),
    }
}

#[test]
fn history_chain_flattens_to_the_exact_manifest_prefix() {
    let (db, root) = fresh_source("chain");
    let history = open_history(&db);
    insert_notes(&history, &[(1, "alpha")], 1);
    let manifest = manifest();
    let steps = steps_full();
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let reference = match runner
        .migrate(&request(&manifest, &steps, 0xda, 0xea), &work())
        .unwrap()
    {
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
    // The target's chain is the inherited (empty) history plus ONE Applied
    // record whose flattened steps equal the whole manifest.
    let target_dir = root
        .join("targets")
        .join(hex_name(incarnation(0xea).as_core().as_bytes()));
    let target: Db<SchemaDescriptor> = Db::open(&target_dir, tagged_schema(), work()).unwrap();
    let mut chain_rows = Vec::new();
    target
        .read(work(), |read| {
            let mut index = 0u64;
            while let Some(bytes) = read
                .integration_host_record(&bumbledb_log::migration::history::history_key(index))
                .unwrap()
            {
                chain_rows.push(bytes.to_vec());
                index += 1;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(chain_rows.len(), 1);
    let record = bumbledb_log::migration::history::decode_record(&chain_rows[0], CAP).unwrap();
    let HistoryRecord::Applied(applied) = record else {
        panic!("one applied batch");
    };
    assert_eq!(
        bumbledb_log::migration::history::verify_chain(
            &[HistoryRecord::Applied(applied)],
            &manifest,
            CAP
        )
        .unwrap(),
        2,
        "the flattened steps are the exact applied prefix"
    );
    // The target's own descriptor validates: the recorded schema is real.
    tagged_schema().validate().unwrap();
}

/// Legal target whose empty state violates a capacity floor (closed parent
/// group, zero children). Incremental mapped rows can repair it.
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
                name: "Note".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "body".into(),
                        value_type: ValueType::String,
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
                    projection: Box::new([FieldId(2)]),
                    selection: Box::new([]),
                },
            },
        ],
    }
}

fn plan_nonempty_required() -> Plan {
    Plan {
        sequence: 0,
        label: bumbledb_log::migration::plan::StepLabel::new("0000-nonempty-required").unwrap(),
        from_schema: schema_id(&base_schema()).unwrap(),
        to_schema: schema_id(&nonempty_required()).unwrap(),
        operations: vec![
            Operation::MapRelation {
                source: "Note".into(),
                target: "Note".into(),
                fields: vec![
                    copy_field("id"),
                    copy_field("body"),
                    FieldMap {
                        target: "slot".into(),
                        expression: PlanExpr::Literal(Value::U64(0)),
                    },
                ],
            },
            Operation::ValidateSchema {
                schema: schema_id(&nonempty_required()).unwrap(),
            },
        ],
        destructive: vec![],
    }
}

fn target_entries(root: &std::path::Path) -> usize {
    std::fs::read_dir(root.join("targets"))
        .map(|listing| listing.filter_map(Result::ok).count())
        .unwrap_or(0)
}

/// D20/D26: compile every expression under verified schemas before freeze
/// or install, including zero source rows. Empty nonempty-required target
/// stays absent; the same plan with valid rows admits. Verification: NotRun.
#[test]
fn d20_d26_compile_before_effects_and_invalid_target_stays_absent() {
    let plan = plan_nonempty_required();
    nonempty_required().validate().expect("target validates");
    let compiled = bumbledb_log::migration::compile::compile(
        &plan,
        &base_schema(),
        &nonempty_required(),
    )
    .expect("empty-input compile binds every field before effects");
    assert!(
        !compiled.actions.is_empty(),
        "compiled actions exist even for zero rows"
    );

    let (empty_db, empty_root) = fresh_source("d26-empty");
    let empty_history = open_history(&empty_db);
    let mut empty_manifest = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    bumbledb_log::migration::manifest::append_entry(&mut empty_manifest, &plan, CAP).unwrap();
    let empty_steps = [StepInput {
        plan: plan.clone(),
        to_descriptor: nonempty_required(),
    }];
    let empty_runner = LocalMigration::new(&empty_history, &empty_root.join("targets"), LIMITS);
    let empty_request = request(&empty_manifest, &empty_steps, 0xd6, 0xe6);
    match empty_runner.migrate(&empty_request, &work()) {
        Err(MigrationError::State(StateError::Rejected { .. }))
        | Err(MigrationError::AdmissionRejected(_))
        | Err(MigrationError::Hydration(_)) => {}
        other => panic!("empty nonempty-required target must refuse, got {other:?}"),
    }
    assert_eq!(
        target_entries(&empty_root),
        0,
        "D26: empty capacity-floor target never becomes ready"
    );

    let (filled_db, filled_root) = fresh_source("d26-filled");
    let filled_history = open_history(&filled_db);
    insert_notes(&filled_history, &[(1, "a"), (2, "b")], 1);
    let mut filled = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    bumbledb_log::migration::manifest::append_entry(&mut filled, &plan, CAP).unwrap();
    let filled_steps = [StepInput {
        plan,
        to_descriptor: nonempty_required(),
    }];
    let filled_runner = LocalMigration::new(&filled_history, &filled_root.join("targets"), LIMITS);
    let filled_request = request(&filled, &filled_steps, 0xd7, 0xe7);
    match filled_runner.migrate(&filled_request, &work()) {
        Ok(MigrateOutcome::ReadyToSwitch { .. }) => {}
        other => panic!("populated nonempty-required target must admit, got {other:?}"),
    }
    assert!(
        target_entries(&filled_root) > 0,
        "D26: valid rows across the mapped batch admit and install"
    );
}
