//! Native hosted transitions: conditional authority, empty restart, and
//! independent recovery of the exact admitted facts and transition evidence.
mod lane_support;
use bumbledb::{ChangeSet, Db, RelationId, SchemaDescriptor, Uuid, Value};
use bumbledb_log::checkpointer::{CheckpointPolicy, read_live_head};
use bumbledb_log::history::authority::Access;
use bumbledb_log::history::{DatabaseIdentity, IncarnationId};
use bumbledb_log::recovery::open_hosted;
use bumbledb_log::store::mem::{Behavior, Gate, MemStore, Op};
use bumbledb_log::transition::hosted::{Begin, Error, Hosted, Population};
use bumbledb_log::transition::local::{self, Resolution};
use bumbledb_log::transition::{Contract, Installed};
use bumbledb_log::writer::{HostedHistory, LogError, SubmitOutcome};
use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, op, temp_dir, theory, work};
use std::sync::{Arc, mpsc};
use std::time::Duration;

fn contract(source: DatabaseIdentity) -> Contract {
    Contract {
        operation: op(4),
        source,
        target: DatabaseIdentity {
            incarnation_id: IncarnationId::from_core(Uuid::from_bytes([5; 16])),
            ..source
        },
        commitment: [6; 32],
    }
}
fn hosted(store: &MemStore) -> Hosted<'_, MemStore> {
    Hosted::new(
        store,
        "source",
        "target",
        1,
        &temp_dir("hosted-transition"),
        LIMITS,
        CheckpointPolicy::DEFAULT,
    )
}
fn population<'a>(
    h: &Hosted<'a, MemStore>,
    source: &Arc<Db<SchemaDescriptor>>,
    c: Contract,
) -> Population<'a, MemStore> {
    match h.begin(source, c, theory(), &work()).unwrap() {
        Begin::Populating {
            source: reader,
            population,
        } => {
            let _ = reader.frame(&work());
            population
        }
        Begin::Resolved(state) => panic!("expected population, got {state:?}"),
    }
}
fn batch(db: &Db<SchemaDescriptor>, ids: &[u64]) -> ChangeSet {
    let mut builder = ChangeSet::builder(db.schema(), work());
    for &id in ids {
        builder.insert(RelationId(0), &[Value::U64(id)]).unwrap();
    }
    builder.finish().unwrap()
}
fn finish(h: &Hosted<'_, MemStore>, source: &Arc<Db<SchemaDescriptor>>, c: Contract) -> Installed {
    let mut p = population(h, source, c);
    p.apply(&batch(source, &[1, 2]), &work()).unwrap();
    match p.finish(&work()).unwrap() {
        Resolution::Ready(installed) => installed,
        other => panic!("{other:?}"),
    }
}
fn frozen(source: &Mirror<'_, MemStore>) {
    assert!(matches!(
        source
            .history
            .submit(&insert_user(source.db(), source.identity, 99, 99), &work()),
        SubmitOutcome::NotSubmitted {
            error: LogError::DatabaseFrozen,
            ..
        }
    ));
}
fn writable(source: &Mirror<'_, MemStore>) {
    assert!(matches!(
        source
            .history
            .submit(&insert_user(source.db(), source.identity, 98, 98), &work()),
        SubmitOutcome::Decided { .. }
    ));
}
fn count(db: &Db<SchemaDescriptor>) -> usize {
    db.read(work(), |read| {
        let mut n = 0;
        for row in read.scan(RelationId(0))? {
            row?;
            n += 1;
        }
        Ok(n)
    })
    .unwrap()
}
fn assert_evidence(db: &Db<SchemaDescriptor>, installed: &Installed) {
    let mut key = vec![b't'];
    key.extend_from_slice(installed.captured.contract.operation.as_core().as_bytes());
    db.read(work(), |read| {
        assert_eq!(
            Installed::decode(
                read.integration_host_record(&key)
                    .unwrap()
                    .expect("installed evidence"),
                LIMITS.envelope_bytes
            )
            .unwrap(),
            *installed
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn ready_cold_recovery_activation_and_completed_retry_after_new_writes() {
    let store = MemStore::new();
    let mut source = Mirror::create("transition-source", &store, "source");
    source.submit(&insert_user(source.db(), source.identity, 1, 1));
    let c = contract(source.identity);
    let h = hosted(&store);
    let installed = finish(&h, &source.db_arc, c);
    frozen(&source);
    assert_eq!(
        h.resolve(&c, &theory(), &work()).unwrap(),
        Resolution::Ready(installed)
    );
    let recovered = open_hosted(
        &temp_dir("transition-cold"),
        theory(),
        &store,
        "mem",
        "target",
        LIMITS,
        CheckpointPolicy::DEFAULT.stream,
        HEAD_CAP,
        &work(),
    )
    .unwrap();
    assert_eq!(count(&recovered.db), 2);
    assert_evidence(&recovered.db, &installed);
    let target = HostedHistory::open(
        Arc::clone(&recovered.db),
        &store,
        "target".into(),
        LIMITS,
        &work(),
    )
    .unwrap();
    assert!(matches!(
        target.submit(&insert_user(&recovered.db, c.target, 2, 3), &work()),
        SubmitOutcome::NotSubmitted {
            error: LogError::DatabaseFrozen,
            ..
        }
    ));
    let activated = h.activate(&installed, &theory(), &work()).unwrap();
    assert!(matches!(activated, Resolution::Activated { .. }));
    assert!(matches!(
        target.submit(&insert_user(&recovered.db, c.target, 2, 3), &work()),
        SubmitOutcome::Decided { .. }
    ));
    assert_eq!(count(&recovered.db), 3);
    assert_eq!(h.resolve(&c, &theory(), &work()).unwrap(), activated);
    assert_eq!(
        h.activate(&installed, &theory(), &work()).unwrap(),
        activated
    );
    assert!(matches!(
        h.begin(&source.db_arc, c, theory(), &work()).unwrap(),
        Begin::Resolved(Resolution::Activated { .. })
    ));
    assert!(matches!(
        h.abort(&c, &theory(), &work()),
        Err(Error::Native(local::Error::ActivationWon))
    ));
    frozen(&source);
    assert_evidence(&recovered.db, &installed);
    for changed in [
        Contract {
            commitment: [9; 32],
            ..c
        },
        Contract {
            target: DatabaseIdentity {
                incarnation_id: IncarnationId::from_core(Uuid::from_bytes([9; 16])),
                ..c.target
            },
            ..c
        },
    ] {
        assert!(h.resolve(&changed, &theory(), &work()).is_err());
        assert!(h.begin(&source.db_arc, changed, theory(), &work()).is_err());
    }
}

#[test]
fn dropped_attempt_restarts_empty_and_abort_revokes_live_population() {
    let store = MemStore::new();
    let source = Mirror::create("transition-restart", &store, "source");
    let c = contract(source.identity);
    let h = hosted(&store);
    let mut first = population(&h, &source.db_arc, c);
    first.apply(&batch(source.db(), &[99]), &work()).unwrap();
    assert!(
        h.begin(&source.db_arc, c, theory(), &work()).is_err(),
        "same scratch excludes another live owner"
    );
    drop(first);
    frozen(&source);
    let second = population(&h, &source.db_arc, c);
    let Resolution::Ready(installed) = second.finish(&work()).unwrap() else {
        panic!("ready")
    };
    assert_eq!(
        h.resolve(&c, &theory(), &work()).unwrap(),
        Resolution::Ready(installed)
    );
    let recovered = open_hosted(
        &temp_dir("transition-empty"),
        theory(),
        &store,
        "mem",
        "target",
        LIMITS,
        CheckpointPolicy::DEFAULT.stream,
        HEAD_CAP,
        &work(),
    )
    .unwrap();
    assert_eq!(count(&recovered.db), 0);
    h.abort(&c, &theory(), &work()).unwrap();
    assert_eq!(
        h.resolve(&c, &theory(), &work()).unwrap(),
        Resolution::Aborted
    );
    assert!(h.activate(&installed, &theory(), &work()).is_err());
    writable(&source);
    let changed = Contract {
        commitment: [9; 32],
        ..c
    };
    assert!(h.abort(&changed, &theory(), &work()).is_err());
    h.abort(&c, &theory(), &work()).unwrap();

    let store = MemStore::new();
    let source = Mirror::create("transition-live-abort", &store, "source");
    let c = contract(source.identity);
    let h = hosted(&store);
    let mut p = population(&h, &source.db_arc, c);
    h.abort(&c, &theory(), &work()).unwrap();
    assert!(p.apply(&batch(source.db(), &[88]), &work()).is_err());
    assert!(p.finish(&work()).is_err());
    writable(&source);
}

#[test]
fn lost_responses_resolve_and_dropped_publications_leave_source_frozen() {
    for behavior in [
        Behavior::IndeterminateApplied,
        Behavior::IndeterminateDropped,
    ] {
        let store = MemStore::new();
        let source = Mirror::create("transition-uncertain", &store, "source");
        let c = contract(source.identity);
        let h = hosted(&store);
        // Source freeze CAS response loss is resolved against the exact intent.
        store.fail_next(Op::ReplaceHead, behavior);
        let p = population(&h, &source.db_arc, c);
        store.fail_next(Op::CreateHead, behavior);
        let result = p.finish(&work());
        frozen(&source);
        if behavior == Behavior::IndeterminateApplied {
            assert!(matches!(result, Ok(Resolution::Ready(_))));
        } else {
            assert!(matches!(result, Err(Error::Unresolved)));
            assert_eq!(
                h.resolve(&c, &theory(), &work()).unwrap(),
                Resolution::Uninstalled
            );
        }
        let installed = match h.begin(&source.db_arc, c, theory(), &work()).unwrap() {
            Begin::Resolved(Resolution::Ready(installed)) => installed,
            Begin::Populating { population, .. } => match population.finish(&work()).unwrap() {
                Resolution::Ready(installed) => installed,
                _ => panic!("ready"),
            },
            Begin::Resolved(_) => panic!("recover ready"),
        };
        store.fail_next(Op::ReplaceHead, behavior);
        assert!(matches!(
            h.activate(&installed, &theory(), &work()).unwrap(),
            Resolution::Activated { .. }
        ));
        frozen(&source);
    }
}

fn pause_once(
    store: &MemStore,
    operation: Op,
    key: &'static str,
) -> (Arc<Gate>, mpsc::Receiver<()>) {
    let gate = Arc::new(Gate::new());
    let blocked = Arc::clone(&gate);
    let (notify, entered) = mpsc::channel();
    let mut used = false;
    store.set_gate(move |op, path| {
        if !used && op == operation && path == key {
            used = true;
            notify.send(()).unwrap();
            Some(Arc::clone(&blocked))
        } else {
            None
        }
    });
    (gate, entered)
}

#[test]
fn abort_beats_a_delayed_install_and_a_delayed_source_freeze() {
    for before_freeze in [false, true] {
        let store = MemStore::new();
        let source = Mirror::create("transition-delayed", &store, "source");
        let c = contract(source.identity);
        let h = hosted(&store);
        let pending = if before_freeze {
            None
        } else {
            Some(population(&h, &source.db_arc, c))
        };
        let (gate, entered) = pause_once(
            &store,
            if before_freeze {
                Op::ReplaceHead
            } else {
                Op::CreateHead
            },
            if before_freeze {
                "source/HEAD"
            } else {
                "target/HEAD"
            },
        );
        std::thread::scope(|scope| {
            let h = &h;
            let source = &source;
            let delayed = scope.spawn(move || {
                if let Some(p) = pending {
                    assert!(p.finish(&work()).is_err());
                } else {
                    assert!(matches!(
                        h.begin(&source.db_arc, c, theory(), &work()).unwrap(),
                        Begin::Resolved(Resolution::Aborted)
                    ));
                }
            });
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
            h.abort(&c, &theory(), &work()).unwrap();
            gate.open();
            delayed.join().unwrap();
        });
        assert_eq!(
            h.resolve(&c, &theory(), &work()).unwrap(),
            Resolution::Aborted
        );
        writable(&source);
    }
}

#[test]
fn activation_and_abort_race_on_one_authority_and_only_the_winner_can_enable_writes() {
    for activation_wins in [false, true] {
        let store = MemStore::new();
        let source = Mirror::create("transition-race", &store, "source");
        let c = contract(source.identity);
        let h = hosted(&store);
        let installed = finish(&h, &source.db_arc, c);
        let (gate, entered) = pause_once(&store, Op::ReplaceHead, "target/HEAD");
        std::thread::scope(|scope| {
            let h = &h;
            let loser = scope.spawn(move || {
                if activation_wins {
                    h.abort(&c, &theory(), &work()).is_err()
                } else {
                    h.activate(&installed, &theory(), &work()).is_err()
                }
            });
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
            if activation_wins {
                h.activate(&installed, &theory(), &work()).unwrap();
            } else {
                h.abort(&c, &theory(), &work()).unwrap();
            }
            gate.open();
            assert!(loser.join().unwrap());
        });
        let target = read_live_head(&store, "target", HEAD_CAP, &work());
        if activation_wins {
            assert_eq!(
                target.unwrap().0.control.live().unwrap().access,
                Access::Active
            );
            frozen(&source);
        } else {
            assert!(
                target.unwrap().0.control.live().is_err(),
                "aborted target is a tombstone"
            );
            writable(&source);
        }
    }
}

#[test]
fn stale_independent_population_cannot_replace_a_different_admitted_output() {
    let store = MemStore::new();
    let source = Mirror::create("transition-two-hosts", &store, "source");
    let c = contract(source.identity);
    let first = hosted(&store);
    let second = hosted(&store);
    let mut a = population(&first, &source.db_arc, c);
    let mut b = population(&second, &source.db_arc, c);
    a.apply(&batch(source.db(), &[1]), &work()).unwrap();
    b.apply(&batch(source.db(), &[2]), &work()).unwrap();
    let Resolution::Ready(installed) = a.finish(&work()).unwrap() else {
        panic!("ready")
    };
    assert!(b.finish(&work()).is_err());
    assert_eq!(
        second.resolve(&c, &theory(), &work()).unwrap(),
        Resolution::Ready(installed)
    );
    frozen(&source);
}

#[test]
fn backup_restore_preserves_installed_transition_evidence_without_the_original_store() {
    use bumbledb_log::backup::{backup_root, read_backup_manifest, relocated_tail, verify_backup};
    use bumbledb_log::restore::restore_writable_with_tail;
    use bumbledb_log::store::{ConditionalStore, ReceiveLimits, TransportContext, get_verified};
    let store = MemStore::new();
    let destination = MemStore::new();
    let source = Mirror::create("transition-backup", &store, "source");
    let c = contract(source.identity);
    let h = hosted(&store);
    let installed = finish(&h, &source.db_arc, c);
    h.activate(&installed, &theory(), &work()).unwrap();
    let head = read_live_head(&store, "target", HEAD_CAP, &work())
        .unwrap()
        .0;
    let report = backup_root(
        &store,
        "target",
        &destination,
        "vault",
        c.target,
        head.control.live().unwrap().state,
        &head.recovery.unwrap(),
        op(22),
        LIMITS,
        CheckpointPolicy::DEFAULT.stream,
        &work(),
    )
    .unwrap();
    verify_backup(
        &destination,
        "vault",
        op(22),
        LIMITS,
        CheckpointPolicy::DEFAULT.stream,
        &work(),
    )
    .unwrap();
    for key in store.object_keys() {
        store.delete_object(&key).unwrap();
    }
    let (manifest, digest) = read_backup_manifest(&destination, "vault", op(22), &work()).unwrap();
    let reference = manifest.checkpoint.unwrap();
    let context = work();
    let bytes = get_verified(
        &destination,
        "vault",
        &reference,
        TransportContext::new(&context, ReceiveLimits::exact(reference.length)),
    )
    .unwrap();
    let checkpoint =
        bumbledb_log::codec::decode_manifest(&bytes, CheckpointPolicy::DEFAULT.stream).unwrap();
    let restored = restore_writable_with_tail(
        &temp_dir("transition-restored").join("db"),
        theory(),
        &checkpoint,
        checkpoint.chunks.iter().map(|reference| {
            get_verified(
                &destination,
                "vault",
                reference,
                TransportContext::new(&context, ReceiveLimits::exact(reference.length)),
            )
            .map_err(bumbledb_log::recovery::RecoveryError::from)
        }),
        relocated_tail(&destination, "vault", &manifest, LIMITS, &context)
            .map(|item| item.map_err(bumbledb_log::recovery::RecoveryError::from)),
        manifest.tip,
        IncarnationId::from_core(Uuid::from_bytes([23; 16])),
        op(24),
        digest,
        "mem",
        "restored",
        LIMITS,
        &CheckpointPolicy::DEFAULT,
        HEAD_CAP,
        &work(),
    )
    .unwrap();
    assert_eq!(count(&restored.db), 2);
    assert_evidence(&restored.db, &installed);
    assert_eq!(restored.source_decision, report.manifest.tip);
    assert_ne!(restored.identity.incarnation_id, c.target.incarnation_id);
    // Permanent operation claims are authority records, outside GC's object
    // epochs. Removing checkpoint objects cannot erase an old operation claim.
    assert!(
        h.resolve(
            &Contract {
                commitment: [99; 32],
                ..c
            },
            &theory(),
            &work()
        )
        .is_err()
    );
    assert!(matches!(
        h.resolve(&c, &theory(), &work()).unwrap(),
        Resolution::Activated { .. }
    ));
}

#[test]
fn schema_cancel_and_upload_refusals_never_publish_partial_target() {
    let store = MemStore::new();
    let source = Mirror::create("transition-refusal", &store, "source");
    let c = contract(source.identity);
    let h = hosted(&store);
    let mut wrong = theory();
    wrong.relations[0].name = "Other".into();
    assert!(h.begin(&source.db_arc, c, wrong, &work()).is_err());
    writable(&source);
    let p = population(&h, &source.db_arc, c);
    let stopped = work();
    stopped.cancel();
    assert!(p.finish(&stopped).is_err());
    assert_eq!(
        h.resolve(&c, &theory(), &work()).unwrap(),
        Resolution::Uninstalled
    );
    let p = population(&h, &source.db_arc, c);
    store.fail_next(Op::PutObject, Behavior::Error);
    assert!(p.finish(&work()).is_err());
    assert_eq!(
        h.resolve(&c, &theory(), &work()).unwrap(),
        Resolution::Uninstalled
    );
    frozen(&source);
    let installed = finish(&h, &source.db_arc, c);
    let mut changed = installed;
    changed.application_digest[0] ^= 1;
    assert!(h.activate(&changed, &theory(), &work()).is_err());
    h.activate(&installed, &theory(), &work()).unwrap();
    assert!(h.activate(&changed, &theory(), &work()).is_err());
}
