//! Resume, lost responses and history drift: a published verified target is
//! reused (never rebuilt) under the same operation/plan bytes, conflicting
//! completed output refuses instead of overwriting, staged partial data is
//! never adopted, plan bytes cannot take over a frozen operation, hostile
//! history rows are corruption evidence, and cross-version/foreign frames
//! refuse before anything trusts them. Maps to MIG-03/06/08/09/10 (native
//! halves of TS-MIG-02/06/08) and OPS-001.
//! Verification: `NotRun` (F1 authors, does not execute).

#[path = "migration_support/mod.rs"]
mod support;

use std::sync::Arc;

use bumbledb::integration::{AttachmentChange, HostChanges, HostRecordChange};
use bumbledb::schema::SchemaDescriptor;
use bumbledb::{Admission, ChangeSet, Db, Id128, RelationId, Value};

use bumbledb_log::history::command::{Command, CommandMetadata};
use bumbledb_log::history::{CommandId, CommandResult, Condition, ReceiptEpoch, RequestId};
use bumbledb_log::migration::executor::{
    LocalMigration, MigrateOutcome, MigrationError, StepInput, SuffixRequest,
};
use bumbledb_log::migration::history::{
    Applied, AppliedSource, AppliedStep, Baseline, HistoryError, HistoryRecord, decode_record,
    encode_record, history_key, verify_chain,
};
use bumbledb_log::migration::lock::TargetNamespace;
use bumbledb_log::migration::manifest::{Manifest, append_entry, prefix_at};
use bumbledb_log::migration::plan::{
    FieldMap, Operation, Plan, PlanExpr, StepLabel, canonical_plan_bytes, decode_plan,
};
use bumbledb_log::schema_file::schema_id;
use bumbledb_log::writer::{LocalHistory, SubmitOutcome};

use support::{
    CAP, LIMITS, base_schema, copy_field, db_id, digest_of, fresh_source, incarnation, manifest,
    op, pinned_schema, plan_pinned, plan_tagged, tagged_schema, work,
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

#[test]
fn a_published_target_is_reused_after_a_lost_response_never_rebuilt() {
    let (db, root) = fresh_source("reuse");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = manifest();
    let steps = steps_full();
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xd1),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe1),
    };
    let first = match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch {
            activation_ref,
            applied,
        } => (activation_ref, applied),
        other => panic!("{other:?}"),
    };
    // The caller's response was "lost": the same stable operation and the
    // same plan bytes re-invoke. The published verified target is reused —
    // identical reference, identical Applied record, no second lineage.
    let second = match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch {
            activation_ref,
            applied,
        } => (activation_ref, applied),
        other => panic!("{other:?}"),
    };
    assert_eq!(first.0, second.0);
    assert_eq!(first.1, second.1);

    // A DIFFERENT operation cannot claim the same published target.
    let foreign = SuffixRequest {
        operation: op(0xd2),
        ..request
    };
    assert!(matches!(
        runner.migrate(&foreign, &work()),
        Err(MigrationError::SourceFrozenByOther { .. })
    ));
}

#[test]
fn conflicting_completed_output_refuses_instead_of_overwriting() {
    let (db, root) = fresh_source("conflict");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = manifest();
    let steps = steps_full();
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xd3),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe3),
    };
    match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { .. } => {}
        other => panic!("{other:?}"),
    }
    // Corrupt the published target's application state through a raw core
    // write (below the history machine): the recorded target digest no
    // longer matches the actual canonical rows.
    let namespace = TargetNamespace::new(&root.join("targets"), incarnation(0xe3)).unwrap();
    {
        let target: Db<SchemaDescriptor> =
            Db::open(&namespace.target_dir(), tagged_schema()).unwrap();
        let tamper = work();
        let mut session = target.integration_writer(&tamper).unwrap();
        let mut draft = ChangeSet::builder(target.schema(), tamper.clone());
        draft
            .insert(
                RelationId(0),
                &[
                    Value::U64(99),
                    Value::String("tampered".into()),
                    Value::Bool(true),
                ],
            )
            .unwrap();
        let changes = draft.finish().unwrap();
        let prepared = match session.prepare(&changes).unwrap() {
            Admission::Accepted(prepared) => prepared,
            Admission::Rejected(violations) => panic!("tamper admits: {violations}"),
        };
        prepared
            .seal(HostChanges {
                records: &[],
                attachment: AttachmentChange::Keep,
            })
            .unwrap()
            .commit()
            .unwrap();
    }
    // Same operation, same source, same plan bytes, conflicting completed
    // output: refuse — never overwrite, never activate.
    assert!(matches!(
        runner.migrate(&request, &work()),
        Err(MigrationError::OutputMismatch)
    ));
}

#[test]
fn different_plan_bytes_cannot_take_over_a_frozen_operation() {
    let (db, root) = fresh_source("takeover");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);

    // Freeze under operation X with a failing single-step manifest.
    let failing = Plan {
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
    let mut failing_manifest = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    append_entry(&mut failing_manifest, &failing, CAP).unwrap();
    let failing_steps = vec![StepInput {
        plan: failing,
        to_descriptor: pinned_schema(),
    }];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let frozen_request = SuffixRequest {
        operation: op(0xd4),
        manifest: &failing_manifest,
        source_descriptor: base_schema(),
        steps: &failing_steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe4),
    };
    assert!(runner.migrate(&frozen_request, &work()).is_err());

    // The SAME operation resubmitted with different plan bytes (the good
    // manifest) cannot take over the frozen operation by reusing its ref.
    let good_manifest = manifest();
    let good_steps = steps_full();
    let takeover = SuffixRequest {
        operation: op(0xd4),
        manifest: &good_manifest,
        source_descriptor: base_schema(),
        steps: &good_steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe5),
    };
    assert!(matches!(
        runner.migrate(&takeover, &work()),
        Err(MigrationError::PlanSetMismatch)
    ));

    // And the resume with the ORIGINAL bytes still owns the operation (the
    // same failure boundary reproduces — no silent substitution happened).
    assert!(matches!(
        runner.migrate(&frozen_request, &work()),
        Err(MigrationError::State(_))
    ));
}

#[test]
fn staged_partial_data_is_never_adopted_and_a_squatting_dir_refuses() {
    let (db, root) = fresh_source("staging");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = manifest();
    let steps = steps_full();
    let targets = root.join("targets");

    // Abandoned partial staging from a killed run: present, never adopted.
    let namespace = TargetNamespace::new(&targets, incarnation(0xe6)).unwrap();
    let abandoned = namespace.fresh_staging();
    std::fs::create_dir_all(&abandoned).unwrap();
    std::fs::write(abandoned.join("data.mdb"), b"partial garbage").unwrap();

    let runner = LocalMigration::new(&history, &targets, LIMITS);
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
    // The published target is a real database — the reuse path verifies its
    // recorded output digest, which garbage bytes could never satisfy.
    match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { activation_ref, .. } => {
            assert_eq!(activation_ref, reference);
        }
        other => panic!("{other:?}"),
    }
    // The abandoned staging bytes are still exactly where they were left:
    // nothing renamed a stage directory into authority.
    assert!(abandoned.join("data.mdb").exists());

    // A squatting non-database directory at another planned target path is
    // evidence of conflict, never a reusable target.
    let (db2, root2) = fresh_source("squat");
    let history2 = open_history(&db2);
    insert_note(&history2, 1, "alpha", 1);
    let targets2 = root2.join("targets");
    let squat = TargetNamespace::new(&targets2, incarnation(0xe7)).unwrap();
    std::fs::create_dir_all(squat.target_dir()).unwrap();
    std::fs::write(squat.target_dir().join("junk"), b"not a database").unwrap();
    let runner2 = LocalMigration::new(&history2, &targets2, LIMITS);
    let request2 = SuffixRequest {
        operation: op(0xd7),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe7),
    };
    assert!(
        runner2.migrate(&request2, &work()).is_err(),
        "a squatting directory is never adopted as a completed target"
    );
}

#[test]
fn hostile_history_rows_are_corruption_evidence_not_a_guess() {
    let (db, root) = fresh_source("hostile-chain");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    // Plant garbage bytes under the migration-history key through a raw
    // host-record write (a hostile/corrupt store, below the machine).
    {
        let tamper = work();
        let mut session = db.integration_writer(&tamper).unwrap();
        let empty = ChangeSet::builder(db.schema(), tamper.clone())
            .finish()
            .unwrap();
        let prepared = match session.prepare(&empty).unwrap() {
            Admission::Accepted(prepared) => prepared,
            Admission::Rejected(violations) => panic!("{violations}"),
        };
        let key = history_key(0);
        prepared
            .seal(HostChanges {
                records: &[HostRecordChange::Put {
                    key: &key,
                    value: b"not a migration record",
                }],
                attachment: AttachmentChange::Keep,
            })
            .unwrap()
            .commit()
            .unwrap();
    }
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    // Both the read-only status and the mutating runner refuse on the exact
    // corrupt evidence; neither guesses an applied prefix.
    assert!(matches!(
        runner.status(&manifest(), &work()),
        Err(MigrationError::History(_) | MigrationError::Frame(_))
    ));
    let steps = steps_full();
    let manifest = manifest();
    let request = SuffixRequest {
        operation: op(0xd8),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xe8),
    };
    assert!(matches!(
        runner.migrate(&request, &work()),
        Err(MigrationError::History(_) | MigrationError::Frame(_))
    ));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one rejection roster over a shared applied chain; each arm \
              mutates the same fixture"
)]
fn verify_chain_rejects_branches_edits_and_misplaced_baselines() {
    let manifest = manifest();
    let steps: Vec<AppliedStep> = manifest
        .entries
        .iter()
        .map(|entry| AppliedStep {
            sequence: entry.sequence,
            label: entry.label.clone(),
            from_schema: entry.from_schema,
            to_schema: entry.to_schema,
            plan_digest: entry.plan_digest,
        })
        .collect();
    let applied = |steps: Vec<AppliedStep>| {
        HistoryRecord::Applied(Applied {
            operation: op(0xd9),
            plan_set_digest: [1; 32],
            source: AppliedSource::EmptyBase {
                base_schema: manifest.base_schema,
            },
            target_incarnation: incarnation(0xe9),
            target_schema: schema_id(&tagged_schema()).unwrap(),
            target_digest: [2; 32],
            steps,
        })
    };
    // The exact chain verifies to the full prefix.
    assert_eq!(
        verify_chain(&[applied(steps.clone())], &manifest, CAP).unwrap(),
        2
    );
    // An edited applied step (a branched plan digest) refuses at position.
    let mut branched = steps.clone();
    branched[1].plan_digest[0] ^= 1;
    assert!(matches!(
        verify_chain(&[applied(branched)], &manifest, CAP),
        Err(HistoryError::StepMismatch { at: 1 })
    ));
    // A relabeled applied step is drift even with the right digest.
    let mut relabeled = steps.clone();
    relabeled[0].label = StepLabel::new("something-else").unwrap();
    assert!(matches!(
        verify_chain(&[applied(relabeled)], &manifest, CAP),
        Err(HistoryError::StepMismatch { at: 0 })
    ));
    // A chain that skips ahead is not contiguous.
    assert!(matches!(
        verify_chain(&[applied(vec![steps[1].clone()])], &manifest, CAP),
        Err(HistoryError::NotContiguous { at: 1 })
    ));
    // A database ahead of the manifest refuses (never truncate the record).
    let mut ahead = steps.clone();
    ahead.push(AppliedStep {
        sequence: 2,
        label: StepLabel::new("phantom").unwrap(),
        from_schema: steps[1].to_schema,
        to_schema: steps[1].to_schema,
        plan_digest: [9; 32],
    });
    assert!(matches!(
        verify_chain(&[applied(ahead)], &manifest, CAP),
        Err(HistoryError::DatabaseAhead { .. })
    ));

    // Baseline rules: only at index 0, prefix recomputed, visibly distinct.
    let baseline = |through: u64, prefix: [u8; 32]| {
        HistoryRecord::Baseline(Baseline {
            operation: op(0xda),
            steps_through: through,
            validated_prefix: prefix,
            target_schema: schema_id(&pinned_schema()).unwrap(),
            target_digest: [3; 32],
            reason: "adopted validated snapshot".into(),
        })
    };
    let good_prefix = prefix_at(&manifest, 1, CAP).unwrap();
    assert_eq!(
        verify_chain(
            &[baseline(1, good_prefix), applied(vec![steps[1].clone()])],
            &manifest,
            CAP
        )
        .unwrap(),
        2,
        "a baseline then the remaining applied suffix"
    );
    assert!(matches!(
        verify_chain(
            &[applied(vec![steps[0].clone()]), baseline(1, good_prefix)],
            &manifest,
            CAP
        ),
        Err(HistoryError::BaselineNotFirst)
    ));
    assert!(matches!(
        verify_chain(&[baseline(1, [7; 32])], &manifest, CAP),
        Err(HistoryError::BaselinePrefixMismatch)
    ));
    assert!(matches!(
        verify_chain(&[baseline(9, good_prefix)], &manifest, CAP),
        Err(HistoryError::DatabaseAhead { .. })
    ));

    // Records round-trip; a baseline without a reason refuses; an applied
    // batch without steps refuses.
    let encoded = encode_record(&baseline(1, good_prefix), CAP).unwrap();
    assert_eq!(
        decode_record(&encoded, CAP).unwrap(),
        baseline(1, good_prefix)
    );
    let encoded_applied = encode_record(&applied(steps.clone()), CAP).unwrap();
    assert_eq!(
        decode_record(&encoded_applied, CAP).unwrap(),
        applied(steps.clone())
    );
    let no_steps = encode_record(&applied(vec![]), CAP).unwrap();
    assert!(matches!(
        decode_record(&no_steps, CAP),
        Err(HistoryError::Shape("applied batch without steps"))
    ));
}

#[test]
fn foreign_families_layouts_and_kinds_refuse_before_content() {
    // Cross-version discipline: the full family string is checked first,
    // the layout second, the kind third — recognizing an integer alone is
    // forbidden, and an unknown newer layout refuses instead of guessing.
    let bytes = canonical_plan_bytes(&plan_pinned(), CAP).unwrap();

    let mut foreign_family = bytes.clone();
    foreign_family[0] ^= 1;
    assert!(decode_plan(&foreign_family, CAP).is_err());

    let mut newer_layout = bytes.clone();
    newer_layout[22..24].copy_from_slice(&2u16.to_be_bytes());
    assert!(decode_plan(&newer_layout, CAP).is_err());

    let mut wrong_kind = bytes.clone();
    wrong_kind[24] = 6; // an APPLIED record kind is not a plan
    assert!(decode_plan(&wrong_kind, CAP).is_err());
    // …and plan bytes are not a history record either (no aliasing between
    // kinds within the family).
    assert!(decode_record(&bytes, CAP).is_err());

    // A history frame whose kind byte is flipped to the OTHER record kind
    // fails that kind's grammar instead of decoding to nonsense.
    let manifest = manifest();
    let record = HistoryRecord::Applied(Applied {
        operation: op(0xdb),
        plan_set_digest: [1; 32],
        source: AppliedSource::EmptyBase {
            base_schema: manifest.base_schema,
        },
        target_incarnation: incarnation(0xeb),
        target_schema: schema_id(&tagged_schema()).unwrap(),
        target_digest: [2; 32],
        steps: vec![AppliedStep {
            sequence: 0,
            label: StepLabel::new("only").unwrap(),
            from_schema: manifest.base_schema,
            to_schema: schema_id(&pinned_schema()).unwrap(),
            plan_digest: digest_of(&plan_pinned()),
        }],
    });
    let mut crossed = encode_record(&record, CAP).unwrap();
    crossed[24] = 7; // claim it is a Baseline
    assert!(decode_record(&crossed, CAP).is_err());
}

#[test]
fn a_same_schema_data_only_plan_is_still_a_recorded_migration() {
    // A seed-only plan whose from/to schema are identical: still a real
    // step with a digest, a freeze, one target and one Applied record.
    let (db, root) = fresh_source("data-only");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    insert_note(&history, 2, "beta", 2);
    let base_id = schema_id(&base_schema()).unwrap();
    let data_plan = Plan {
        sequence: 0,
        label: StepLabel::new("0000-seed-note").unwrap(),
        from_schema: base_id,
        to_schema: base_id,
        operations: vec![
            Operation::MapRelation {
                source: "Note".into(),
                target: "Note".into(),
                fields: vec![copy_field("id"), copy_field("body")],
            },
            Operation::Seed {
                target: "Note".into(),
                rows: vec![Box::from([Value::U64(3), Value::String("seeded".into())])],
            },
            Operation::ValidateSchema { schema: base_id },
        ],
        destructive: vec![],
    };
    let mut data_manifest = Manifest {
        base_schema: base_id,
        entries: vec![],
    };
    append_entry(&mut data_manifest, &data_plan, CAP).unwrap();
    let steps = vec![StepInput {
        plan: data_plan,
        to_descriptor: base_schema(),
    }];
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);
    let request = SuffixRequest {
        operation: op(0xdc),
        manifest: &data_manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xec),
    };
    let applied = match runner.migrate(&request, &work()).unwrap() {
        MigrateOutcome::ReadyToSwitch { applied, .. } => applied,
        other => panic!("{other:?}"),
    };
    assert_eq!(applied.steps.len(), 1);
    assert_eq!(applied.target_schema, base_id, "same schema, new lineage");
    assert!(matches!(applied.source, AppliedSource::Database { .. }));
    // The target holds the mapped rows plus the seed: three notes.
    let namespace = TargetNamespace::new(&root.join("targets"), incarnation(0xec)).unwrap();
    let target: Db<SchemaDescriptor> = Db::open(&namespace.target_dir(), base_schema()).unwrap();
    let mut count = 0;
    target
        .read(|read| {
            for row in read.scan(RelationId(0))? {
                row?;
                count += 1;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(count, 3);
    // Its recorded history flattens to exactly this one-entry manifest.
    let mut rows = Vec::new();
    target
        .read(|read| {
            let record = read
                .integration_host_record(&history_key(0))
                .unwrap_or_else(|error| panic!("host record read: {error}"));
            if let Some(bytes) = record {
                rows.push(bytes.to_vec());
            }
            Ok(())
        })
        .unwrap();
    let record = decode_record(&rows[0], CAP).unwrap();
    assert_eq!(verify_chain(&[record], &data_manifest, CAP).unwrap(), 1);
}

#[test]
fn wrong_suffix_and_reused_incarnations_refuse_before_any_freeze() {
    let (db, root) = fresh_source("wrong-suffix");
    let history = open_history(&db);
    insert_note(&history, 1, "alpha", 1);
    let manifest = manifest();
    let runner = LocalMigration::new(&history, &root.join("targets"), LIMITS);

    // Pending entries exist but the request carries no steps.
    let empty_steps: Vec<StepInput> = vec![];
    let no_steps = SuffixRequest {
        operation: op(0xdd),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &empty_steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xed),
    };
    assert!(matches!(
        runner.migrate(&no_steps, &work()),
        Err(MigrationError::WrongSuffix { applied: 0 })
    ));

    // A suffix that does not start at the applied prefix refuses at binding.
    let tail_only = vec![StepInput {
        plan: plan_tagged(),
        to_descriptor: tagged_schema(),
    }];
    let skipping = SuffixRequest {
        operation: op(0xdd),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &tail_only,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xed),
    };
    assert!(matches!(
        runner.migrate(&skipping, &work()),
        Err(MigrationError::Manifest(_))
    ));

    // The planned target incarnation must be a NEW lineage.
    let steps = steps_full();
    let reused = SuffixRequest {
        operation: op(0xdd),
        manifest: &manifest,
        source_descriptor: base_schema(),
        steps: &steps,
        target_database: db_id(0xa1),
        target_incarnation: incarnation(0xb1), // the source's own incarnation
    };
    assert!(matches!(
        runner.migrate(&reused, &work()),
        Err(MigrationError::IncarnationReused)
    ));

    // None of the refusals froze the source.
    insert_note(&history, 5, "still-active", 5);
}
