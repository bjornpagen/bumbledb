//! Imperative population uses ordinary changes; native ownership and durable
//! authority alone decide admission, restart, activation and cancellation.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::{
    FieldDescriptor, RelationDescriptor, SchemaDescriptor, StatementDescriptor,
    ValidateDescriptor as _, ValueType,
};
use bumbledb::{ChangeSet, Db, FieldId, RelationId, Uuid, Value, WorkContext};
use bumbledb_log::history::authority::Access;
use bumbledb_log::history::command::{Command, CommandMetadata, Limits};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseId, DatabaseIdentity, IncarnationId, OperationId,
    ReceiptEpoch, RequestId,
};
use bumbledb_log::schema_file::schema_id;
use bumbledb_log::transition::local::{self, Begin, Error, Population, Resolution};
use bumbledb_log::transition::namespace::{NamespaceError, TargetNamespace};
use bumbledb_log::transition::{Contract, Installed};
use bumbledb_log::writer::{LocalHistory, LogError, SubmitOutcome};

const LIMITS: Limits = Limits {
    envelope_bytes: 1_000_000,
    change_bytes: 900_000,
    evidence_bytes: 10_000,
    result_bytes: 1_000,
};
fn work() -> WorkContext {
    WorkContext::new()
}
fn op(n: u8) -> OperationId {
    OperationId::from_core(Uuid::from_bytes([n; 16]))
}
fn inc(n: u8) -> IncarnationId {
    IncarnationId::from_core(Uuid::from_bytes([n; 16]))
}
fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
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
            ],
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    }
}
static SEQ: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    history: LocalHistory<SchemaDescriptor>,
    contract: Contract,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bdb-transition-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self::at(root, true)
    }
    fn at(root: PathBuf, create: bool) -> Self {
        let db = Arc::new(if create {
            Db::create(&root.join("source"), schema(), work())
                .unwrap()
                .expect("empty schema admits")
        } else {
            Db::open(&root.join("source"), schema(), work()).unwrap()
        });
        let history = if create {
            LocalHistory::create(
                db,
                DatabaseId::from_core(Uuid::from_bytes([1; 16])),
                inc(2),
                op(3),
                LIMITS,
                &work(),
            )
            .unwrap()
        } else {
            LocalHistory::open(db, LIMITS).unwrap()
        };
        let contract = Contract {
            operation: op(4),
            source: history.identity(),
            target: DatabaseIdentity {
                incarnation_id: inc(5),
                ..history.identity()
            },
            commitment: [6; 32],
        };
        Self {
            root,
            history,
            contract,
        }
    }
    fn targets(&self) -> PathBuf {
        self.root.join("targets")
    }
    fn begin(&self) -> Result<Begin, Error> {
        Population::begin(
            &self.history,
            &self.targets(),
            self.contract,
            schema(),
            LIMITS,
            &work(),
        )
    }
    fn population(&self) -> Population {
        match self.begin().unwrap() {
            Begin::Population(p) => p,
            Begin::Resolved(_) => panic!("expected empty attempt"),
        }
    }
    fn resolve(&self) -> Resolution {
        local::resolve(
            &self.history,
            &self.targets(),
            &self.contract,
            &schema(),
            LIMITS,
            &work(),
        )
        .unwrap()
    }
    fn abort(&self) -> Result<(), Error> {
        local::abort(
            &self.history,
            &self.targets(),
            &self.contract,
            &schema(),
            LIMITS,
            &work(),
        )
    }
    fn finish(&self) -> Installed {
        let mut p = self.population();
        p.apply(&batch(&[(1, "a"), (2, "b")], &[]), &work())
            .unwrap();
        p.finish(&work()).unwrap()
    }
    fn activate(&self, installed: &Installed) -> Result<Resolution, Error> {
        local::activate(
            &self.history,
            &self.targets(),
            installed,
            &schema(),
            LIMITS,
            &work(),
        )
    }
    fn target(&self) -> LocalHistory<SchemaDescriptor> {
        let ns =
            TargetNamespace::new(&self.targets(), self.contract.target.incarnation_id).unwrap();
        LocalHistory::open(
            Arc::new(Db::open(&ns.target_dir(), schema(), work()).unwrap()),
            LIMITS,
        )
        .unwrap()
    }
}
fn batch(add: &[(u64, &str)], remove: &[(u64, &str)]) -> ChangeSet {
    let compiled = schema().validate().unwrap();
    let mut builder = ChangeSet::builder(&compiled, work());
    for &(id, body) in add {
        builder
            .insert(RelationId(0), &[Value::U64(id), Value::String(body.into())])
            .unwrap();
    }
    for &(id, body) in remove {
        builder
            .delete(RelationId(0), &[Value::U64(id), Value::String(body.into())])
            .unwrap();
    }
    builder.finish().unwrap()
}
fn submit(history: &LocalHistory<SchemaDescriptor>, id: u8) -> SubmitOutcome {
    let command = Command::seal(
        CommandMetadata {
            identity: history.identity(),
            id: CommandId {
                receipt_epoch: ReceiptEpoch::INITIAL,
                request_id: RequestId::from_core(Uuid::from_bytes([id; 16])),
            },
            condition: Condition::Unconditional,
        },
        batch(&[(u64::from(id), "later")], &[]),
        CommandResult::empty(),
        LIMITS,
        &work(),
    )
    .unwrap();
    history.submit(&command, &work())
}
fn frozen(history: &LocalHistory<SchemaDescriptor>) {
    assert!(matches!(
        submit(history, 88),
        SubmitOutcome::NotSubmitted {
            error: LogError::DatabaseFrozen,
            ..
        }
    ));
}
fn facts(history: &LocalHistory<SchemaDescriptor>) -> Vec<Vec<u8>> {
    let mut facts = vec![];
    history
        .db()
        .read(work(), |read| {
            read.snapshot()
                .export(&work(), &mut |_, row| {
                    facts.push(row.to_vec());
                    Ok(())
                })
                .unwrap();
            Ok(())
        })
        .unwrap();
    facts.sort();
    facts
}

fn raw_commit(
    db: &Db<SchemaDescriptor>,
    changes: &ChangeSet,
    records: &[bumbledb::integration::HostRecordChange<'_>],
) {
    use bumbledb::integration::{AttachmentChange, HostChanges, Preparation};
    let work = work();
    let mut session = db.integration_writer(&work).unwrap();
    let Preparation::Accepted(prepared) = session.prepare(changes).unwrap() else {
        panic!("valid raw mutation")
    };
    prepared
        .seal(HostChanges {
            records,
            attachment: AttachmentChange::Keep,
        })
        .unwrap()
        .commit()
        .unwrap();
}

#[test]
fn ordinary_open_and_checkpoint_refuse_retired_history_even_without_old_authority_tags() {
    use bumbledb_log::checkpointer::{CheckpointError, CheckpointPolicy, capture_into};
    use bumbledb_log::codec::ChunkSink;
    struct NeverExport;
    impl ChunkSink for NeverExport {
        type Error = CheckpointError;
        fn chunk(&mut self, _: &[u8]) -> Result<bumbledb_log::store::ObjectRef, Self::Error> {
            panic!("refuse before export")
        }
    }
    let f = Fixture::new();
    raw_commit(
        f.history.db(),
        &batch(&[], &[]),
        &[bumbledb::integration::HostRecordChange::Put {
            key: b"m-retired-not-sequence-zero",
            value: b"old plan evidence",
        }],
    );
    assert!(matches!(
        f.begin(),
        Err(Error::Log(LogError::UnsupportedArtifact))
    ));
    assert!(matches!(
        capture_into(
            f.history.db(),
            &mut NeverExport,
            0,
            &CheckpointPolicy::DEFAULT,
            &work()
        ),
        Err(CheckpointError::Corruption("retired migration records"))
    ));
    let root = f.root.clone();
    drop(f);
    let db = Arc::new(Db::open(&root.join("source"), schema(), work()).unwrap());
    assert!(matches!(
        LocalHistory::open(db, LIMITS),
        Err(LogError::UnsupportedArtifact)
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ready_resolution_rehashes_content_but_completed_resolution_keeps_original_genesis() {
    let f = Fixture::new();
    let installed = f.finish();
    let target = f.target();
    raw_commit(
        target.db(),
        &batch(&[(99, "changed outside history")], &[]),
        &[],
    );
    drop(target);
    assert!(matches!(
        local::resolve(
            &f.history,
            &f.targets(),
            &f.contract,
            &schema(),
            LIMITS,
            &work()
        ),
        Err(Error::OutputMismatch)
    ));
    assert!(matches!(f.activate(&installed), Err(Error::OutputMismatch)));
    frozen(&f.history);
}

#[test]
fn resolution_repairs_the_activation_marker_crash_window_from_recorded_control() {
    let f = Fixture::new();
    let installed = f.finish();
    let activated = f.activate(&installed).unwrap();
    let marker = f.targets().join("05".repeat(16) + ".activation");
    std::fs::remove_file(&marker).unwrap();
    assert_eq!(f.resolve(), activated);
    assert!(marker.is_file());
    let target = f.target();
    assert!(matches!(submit(&target, 99), SubmitOutcome::Decided { .. }));
    assert_eq!(
        f.resolve(),
        activated,
        "marker supports resolution while advanced target is owned"
    );
}

#[test]
fn capture_install_inspect_activate_and_reopen() {
    let f = Fixture::new();
    assert!(matches!(
        submit(&f.history, 1),
        SubmitOutcome::Decided { .. }
    ));
    let before = facts(&f.history);
    let mut p = f.population();
    assert_eq!(p.captured().contract, f.contract);
    frozen(&f.history);
    p.apply(&batch(&[(1, "later")], &[]), &work()).unwrap();
    let installed = p.finish(&work()).unwrap();
    assert_eq!(f.resolve(), Resolution::Ready(installed));
    let target = f.target();
    frozen(&target);
    assert_eq!(facts(&target), before);
    drop(target);
    assert!(matches!(
        f.activate(&installed).unwrap(),
        Resolution::Activated { .. }
    ));
    let target = f.target();
    assert!(matches!(submit(&target, 2), SubmitOutcome::Decided { .. }));
    drop(target);
    frozen(&f.history);
    assert!(matches!(f.resolve(), Resolution::Activated { .. }));
    assert!(matches!(
        f.begin().unwrap(),
        Begin::Resolved(Resolution::Activated { .. })
    ));
    assert!(matches!(f.abort(), Err(Error::ActivationWon)));
    let reopened = f.target();
    assert_eq!(facts(&reopened).len(), 2);
}

#[test]
fn population_restart_waits_for_owner_and_starts_empty() {
    let f = Fixture::new();
    let mut p = f.population();
    p.apply(&batch(&[(1, "abandoned")], &[]), &work()).unwrap();
    assert!(matches!(
        f.begin(),
        Err(Error::Namespace(NamespaceError::Busy))
    ));
    drop(p);
    frozen(&f.history);
    let mut p = f.population();
    p.apply(&batch(&[(2, "only")], &[]), &work()).unwrap();
    let installed = p.finish(&work()).unwrap();
    assert_eq!(f.resolve(), Resolution::Ready(installed));
    assert_eq!(facts(&f.target()).len(), 1);
}

#[test]
fn invalid_final_state_never_installs_and_consumes_attempt() {
    let f = Fixture::new();
    let mut p = f.population();
    p.apply(&batch(&[(1, "a")], &[]), &work()).unwrap();
    p.apply(&batch(&[(1, "b")], &[]), &work()).unwrap();
    assert!(p.finish(&work()).is_err());
    assert_eq!(f.resolve(), Resolution::Uninstalled);
    frozen(&f.history);
    assert!(f.population().finish(&work()).is_ok());
}

#[test]
fn sequential_deletes_are_not_add_wins_command_composition() {
    let a = Fixture::new();
    let mut p = a.population();
    p.apply(&batch(&[(1, "a")], &[]), &work()).unwrap();
    p.apply(&batch(&[], &[(1, "a")]), &work()).unwrap();
    let empty = p.finish(&work()).unwrap();
    assert_eq!(facts(&a.target()).len(), 0);
    let b = Fixture::new();
    let mut p = b.population();
    p.apply(&batch(&[], &[(1, "a")]), &work()).unwrap();
    p.apply(&batch(&[(1, "a")], &[]), &work()).unwrap();
    let full = p.finish(&work()).unwrap();
    assert_eq!(facts(&b.target()).len(), 1);
    assert_ne!(empty.application_digest, full.application_digest);
}

#[test]
fn digest_is_independent_of_batch_order_and_duplicate_facts() {
    let a = Fixture::new();
    let installed = a.finish();
    let b = Fixture::new();
    let mut p = b.population();
    for row in [(2, "b"), (1, "a"), (2, "b")] {
        p.apply(&batch(&[row], &[]), &work()).unwrap();
    }
    assert_eq!(
        installed.application_digest,
        p.finish(&work()).unwrap().application_digest
    );
}

#[test]
fn abort_fences_live_attempt_before_source_writes_and_is_terminal() {
    let f = Fixture::new();
    let mut p = f.population();
    f.abort().unwrap();
    assert!(matches!(
        submit(&f.history, 1),
        SubmitOutcome::Decided { .. }
    ));
    assert!(matches!(
        p.apply(&batch(&[(1, "late")], &[]), &work()),
        Err(Error::Aborted)
    ));
    assert!(matches!(p.finish(&work()), Err(Error::Aborted)));
    assert_eq!(f.resolve(), Resolution::Aborted);
    assert!(matches!(
        f.begin().unwrap(),
        Begin::Resolved(Resolution::Aborted)
    ));
    f.abort().unwrap();
}

#[test]
fn ready_target_abort_fences_activation_and_thaws_source() {
    let f = Fixture::new();
    let installed = f.finish();
    f.abort().unwrap();
    assert_eq!(f.resolve(), Resolution::Aborted);
    assert!(matches!(f.activate(&installed), Err(Error::Aborted)));
    assert!(matches!(
        submit(&f.history, 1),
        SubmitOutcome::Decided { .. }
    ));
}

#[test]
fn changed_contracts_refuse_before_any_target_fence_or_thaw() {
    let f = Fixture::new();
    let p = f.population();
    for contract in [
        Contract {
            commitment: [7; 32],
            ..f.contract
        },
        Contract {
            target: DatabaseIdentity {
                incarnation_id: inc(9),
                ..f.contract.target
            },
            ..f.contract
        },
        Contract {
            operation: op(9),
            ..f.contract
        },
    ] {
        assert!(matches!(
            local::abort(
                &f.history,
                &f.targets(),
                &contract,
                &schema(),
                LIMITS,
                &work()
            ),
            Err(Error::ContractMismatch)
        ));
        let ns = TargetNamespace::new(&f.targets(), contract.target.incarnation_id).unwrap();
        assert!(ns.read_tombstone(LIMITS.envelope_bytes).unwrap().is_none());
    }
    frozen(&f.history);
    drop(p);
}

#[test]
fn mismatched_ready_and_completed_evidence_never_activates() {
    let f = Fixture::new();
    let installed = f.finish();
    let mut wrong = installed;
    wrong.application_digest[0] ^= 1;
    assert!(matches!(f.activate(&wrong), Err(Error::OutputMismatch)));
    f.activate(&installed).unwrap();
    assert!(matches!(f.activate(&wrong), Err(Error::OutputMismatch)));
    let mut wrong = installed;
    wrong.captured.decision.hash = bumbledb_log::history::DecisionDigest::from_bytes([44; 32]);
    assert!(matches!(f.activate(&wrong), Err(Error::OutputMismatch)));
    let mut changed = f.contract;
    changed.commitment[0] ^= 1;
    assert!(matches!(
        local::resolve(
            &f.history,
            &f.targets(),
            &changed,
            &schema(),
            LIMITS,
            &work()
        ),
        Err(Error::ContractMismatch)
    ));
    // The source's retained operation is authoritative even when a changed
    // target would otherwise look like a fresh, uninstalled namespace.
    changed = Contract {
        target: DatabaseIdentity {
            incarnation_id: inc(9),
            ..f.contract.target
        },
        ..f.contract
    };
    assert!(matches!(
        local::resolve(
            &f.history,
            &f.targets(),
            &changed,
            &schema(),
            LIMITS,
            &work()
        ),
        Err(Error::ContractMismatch)
    ));
    let mut wrong = installed;
    wrong.captured.contract = changed;
    assert!(matches!(f.activate(&wrong), Err(Error::ContractMismatch)));
    assert!(matches!(
        Population::begin(&f.history, &f.targets(), changed, schema(), LIMITS, &work()),
        Err(Error::ContractMismatch)
    ));
}

#[test]
fn abort_before_begin_retains_contract_and_never_freezes_source() {
    let f = Fixture::new();
    f.abort().unwrap();
    assert!(matches!(
        f.history.authority().unwrap().live().unwrap().access,
        Access::Active
    ));
    let changed = Contract {
        target: DatabaseIdentity {
            incarnation_id: inc(9),
            ..f.contract.target
        },
        ..f.contract
    };
    assert!(matches!(
        Population::begin(&f.history, &f.targets(), changed, schema(), LIMITS, &work()),
        Err(Error::ContractMismatch)
    ));
    assert_eq!(f.resolve(), Resolution::Aborted);
}

#[test]
fn schema_mismatch_precedes_freeze_and_cancelled_finalize_loses_owner() {
    let f = Fixture::new();
    let mut wrong = schema();
    wrong.relations[0].name = "Other".into();
    assert_ne!(schema_id(&wrong).unwrap(), f.contract.target.schema_id);
    assert!(matches!(
        Population::begin(&f.history, &f.targets(), f.contract, wrong, LIMITS, &work()),
        Err(Error::ContractMismatch)
    ));
    assert!(matches!(
        f.history.authority().unwrap().live().unwrap().access,
        Access::Active
    ));
    let p = f.population();
    let stopped = work();
    stopped.cancel();
    assert!(matches!(p.finish(&stopped), Err(Error::Work(_))));
    assert!(matches!(f.begin().unwrap(), Begin::Population(_)));
}

#[test]
fn namespace_lock_cannot_authorize_another_target() {
    let f = Fixture::new();
    let a = TargetNamespace::new(&f.targets(), inc(1)).unwrap();
    let held = a.lock().unwrap();
    let b = TargetNamespace::new(&f.targets(), inc(2)).unwrap();
    assert!(
        b.install_target(&held, Path::new("/no-such-stage"), LIMITS.envelope_bytes)
            .is_err()
    );
}

#[test]
fn crash_child() {
    let Ok(root) = std::env::var("BUMBLEDB_TRANSITION_CRASH_ROOT") else {
        return;
    };
    let phase = std::env::var("BUMBLEDB_TRANSITION_CRASH_PHASE").unwrap();
    let f = Fixture::at(root.into(), false);
    let mut p = f.population();
    p.apply(&batch(&[(1, "crashed")], &[]), &work()).unwrap();
    if phase == "population" {
        std::process::exit(71);
    }
    let installed = p.finish(&work()).unwrap();
    if phase == "installed" {
        std::process::exit(71);
    }
    f.activate(&installed).unwrap();
    std::process::exit(71);
}

#[test]
fn process_restart_discards_prefix_or_reuses_durable_terminal_evidence() {
    for phase in ["population", "installed", "activated"] {
        let f = Fixture::new();
        let root = f.root.clone();
        drop(f);
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash_child", "--nocapture"])
            .env("BUMBLEDB_TRANSITION_CRASH_ROOT", &root)
            .env("BUMBLEDB_TRANSITION_CRASH_PHASE", phase)
            .status()
            .unwrap();
        assert_eq!(result.code(), Some(71));
        let f = Fixture::at(root, false);
        match phase {
            "population" => {
                let p = f.population();
                p.finish(&work()).unwrap();
                assert!(facts(&f.target()).is_empty());
            }
            "installed" => {
                assert!(matches!(
                    f.begin().unwrap(),
                    Begin::Resolved(Resolution::Ready(_))
                ));
                assert_eq!(facts(&f.target()).len(), 1);
            }
            "activated" => {
                assert!(matches!(
                    f.begin().unwrap(),
                    Begin::Resolved(Resolution::Activated { .. })
                ));
                assert_eq!(facts(&f.target()).len(), 1);
            }
            _ => unreachable!(),
        }
        frozen(&f.history);
    }
}

#[test]
fn closed_roster_coverage_and_capacity_are_judged_once_at_final_state() {
    use bumbledb::schema::{Bound, Row, Side, Weight};
    for defect in ["none", "missing", "over-capacity"] {
        let f = Fixture::new();
        let mut target = schema();
        target.relations.push(RelationDescriptor {
            name: "Kind".into(),
            fields: vec![],
            extension: Some(Box::new([
                Row {
                    handle: "First".into(),
                    values: Box::new([]),
                },
                Row {
                    handle: "Second".into(),
                    values: Box::new([]),
                },
            ])),
        });
        target.statements = vec![StatementDescriptor::Capacity {
            target: Side {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
            source: Side {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
            weight: Weight::Unit,
            lo: 1,
            hi: Some(Bound::Lit(1)),
        }];
        let c = Contract {
            target: DatabaseIdentity {
                schema_id: schema_id(&target).unwrap(),
                ..f.contract.target
            },
            ..f.contract
        };
        let Begin::Population(mut population) =
            Population::begin(&f.history, &f.targets(), c, target.clone(), LIMITS, &work())
                .unwrap()
        else {
            panic!("new population")
        };
        let compiled = target.clone().validate().unwrap();
        let mut rows = vec![(0, "first")];
        if defect != "missing" {
            rows.push((1, "second"));
        }
        if defect == "over-capacity" {
            rows.push((0, "extra"));
        }
        for (id, body) in rows {
            let mut changes = ChangeSet::builder(&compiled, work());
            changes
                .insert(RelationId(0), &[Value::U64(id), Value::String(body.into())])
                .unwrap();
            population
                .apply(&changes.finish().unwrap(), &work())
                .unwrap();
        }
        let result = population.finish(&work());
        if defect == "none" {
            let installed = result.unwrap();
            assert_eq!(
                local::resolve(&f.history, &f.targets(), &c, &target, LIMITS, &work()).unwrap(),
                Resolution::Ready(installed)
            );
        } else {
            let error = result.unwrap_err();
            assert!(format!("{error:?}").contains("JudgeRefused"), "{error:?}");
            assert_eq!(
                local::resolve(&f.history, &f.targets(), &c, &target, LIMITS, &work()).unwrap(),
                Resolution::Uninstalled
            );
        }
        frozen(&f.history);
    }
}
