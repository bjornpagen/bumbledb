//! Exercise the native integration facade, including the writer mutex across
//! rejected/dropped candidates. Channel/barrier handshakes establish ordering;
//! timeout bounds detect a stuck test, never establish correctness by sleeping.

use std::sync::{Barrier, mpsc};
use std::time::Duration;

use super::*;
use crate::integration::{ApplicationChanges, AttachmentChange, HostRecordChange};
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, StatementDescriptor,
    ValueType,
};
use crate::testutil::TempDir;
use crate::{ExecutionPolicy, FieldId, Value};

const ROW: RelationId = RelationId(0);
const RECEIPT: &[u8] = b"named/1";

fn descriptor(name: &str) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: name.into(),
            fields: ["key", "value"]
                .map(|name| FieldDescriptor {
                    name: name.into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                })
                .to_vec(),
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: ROW,
            projection: Box::new([FieldId(0)]),
        }],
    }
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 10_000,
        work_units: 1_000_000,
        timeout: Duration::from_secs(30),
    }
}

fn work() -> WorkContext {
    policy().start().unwrap()
}

fn changes(
    db: &Db<SchemaDescriptor>,
    additions: &[(u64, u64)],
    removals: &[(u64, u64)],
) -> ChangeSet {
    let mut draft = ChangeSet::builder(&db.schema, work());
    for &(key, value) in additions {
        draft
            .insert(ROW, &[Value::U64(key), Value::U64(value)])
            .unwrap();
    }
    for &(key, value) in removals {
        draft
            .delete(ROW, &[Value::U64(key), Value::U64(value)])
            .unwrap();
    }
    draft.finish().unwrap()
}

fn no_host() -> HostChanges<'static> {
    HostChanges {
        records: &[],
        attachment: AttachmentChange::Keep,
    }
}

fn seed(db: &Db<SchemaDescriptor>) {
    let initial = changes(db, &[(1, 10)], &[]);
    let mut owner = db.integration_writer(&work()).unwrap();
    owner
        .prepare(&initial)
        .unwrap()
        .expect("admitted")
        .seal(no_host())
        .unwrap()
        .commit()
        .unwrap();
}

fn assert_writer_owned(db: &Db<SchemaDescriptor>) {
    assert!(
        matches!(db.writer.try_lock(), Err(TryLockError::WouldBlock)),
        "session must still own the actual writer mutex"
    );
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    facts: Vec<Vec<Value>>,
    receipt: Option<Vec<u8>>,
    attachment: Option<Vec<u8>>,
    generation: u64,
}

fn capture(view: &ReadInstance<'_, SchemaDescriptor>) -> crate::Result<Snapshot> {
    Ok(Snapshot {
        facts: view.scan(ROW)?.collect::<crate::Result<_>>()?,
        receipt: view
            .integration_host_record(RECEIPT)
            .unwrap()
            .map(<[u8]>::to_vec),
        attachment: view.integration_host_attachment()?.map(<[u8]>::to_vec),
        generation: view.generation()?.value(),
    })
}

#[test]
fn rejection_and_empty_receipt_keep_one_session_until_the_waiter_can_observe_both() {
    let dir = TempDir::new("integration-rejection-session");
    let db = Db::create(dir.path(), descriptor("Account"))
        .unwrap()
        .expect("schema");
    seed(&db);
    let invalid = changes(&db, &[(1, 99)], &[]);
    let empty = changes(&db, &[], &[]);
    std::thread::scope(|scope| {
        // Session local to the scope closure: unwinding releases it before
        // scope joins the waiting child, so a failed assertion cannot deadlock.
        let mut owner = db.integration_writer(&work()).unwrap();
        let (probed, observed_probe) = mpsc::channel();
        let (acquired, observed_acquire) = mpsc::channel();
        let contender_db = &db;
        let contender = scope.spawn(move || {
            assert_writer_owned(contender_db);
            probed.send(()).unwrap();
            let waiter_work = ExecutionPolicy {
                timeout: Duration::from_secs(5),
                ..policy()
            }
            .start()
            .unwrap();
            let _next = contender_db.integration_writer(&waiter_work).unwrap();
            acquired.send(contender_db.read(capture).unwrap()).unwrap();
        });
        observed_probe.recv_timeout(Duration::from_secs(5)).unwrap();
        let rejection = match owner.prepare(&invalid).unwrap() {
            Admission::Rejected(violations) => violations,
            Admission::Accepted(_) => panic!("declared key law must reject"),
        };
        assert_eq!(rejection.len(), 1);
        assert!(!rejection.cited_facts(0).is_empty());
        assert_writer_owned(&db);
        assert_eq!(owner.generation().unwrap().value(), 1);
        let prepared = owner.prepare(&empty).unwrap().expect("empty admitted");
        assert_eq!(
            prepared.application_changes(),
            ApplicationChanges {
                added: 0,
                removed: 0
            }
        );
        let records = [HostRecordChange::Put {
            key: RECEIPT,
            value: b"rejected",
        }];
        let sealed = prepared
            .seal(HostChanges {
                records: &records,
                attachment: AttachmentChange::Put(b"decision/1-state/0"),
            })
            .unwrap();
        assert_writer_owned(&db);
        assert_eq!(db.read(capture).unwrap().receipt, None);
        let committed = sealed.commit().unwrap();
        assert_eq!(
            committed.application,
            ApplicationChanges {
                added: 0,
                removed: 0
            }
        );
        assert_eq!(committed.generation.value(), 2);
        assert_writer_owned(&db); // Commit releases LMDB txn, not the session.
        drop(owner);
        let next = observed_acquire
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert_eq!(next.facts, vec![vec![Value::U64(1), Value::U64(10)]]);
        assert_eq!(next.receipt.as_deref(), Some(b"rejected".as_slice()));
        assert_eq!(
            next.attachment.as_deref(),
            Some(b"decision/1-state/0".as_slice())
        );
        assert_eq!(next.generation, 2);
        contender.join().unwrap();
    });
}

#[test]
fn dropped_prepared_and_sealed_candidates_release_native_txn_not_writer_session() {
    let dir = TempDir::new("integration-drop-session");
    let db = Db::create(dir.path(), descriptor("Account"))
        .unwrap()
        .expect("schema");
    let add = changes(&db, &[(1, 10)], &[]);
    let empty = changes(&db, &[], &[]);
    let mut owner = db.integration_writer(&work()).unwrap();
    drop(owner.prepare(&add).unwrap().expect("admitted"));
    assert_writer_owned(&db);
    assert_eq!(owner.generation().unwrap().value(), 0);
    assert!(db.read(capture).unwrap().facts.is_empty());
    let records = [HostRecordChange::Put {
        key: RECEIPT,
        value: b"must-abort",
    }];
    let sealed = owner
        .prepare(&add)
        .unwrap()
        .expect("admitted")
        .seal(HostChanges {
            records: &records,
            attachment: AttachmentChange::Put(b"must-abort"),
        })
        .unwrap();
    drop(sealed);
    assert_writer_owned(&db);
    let untouched = db.read(capture).unwrap();
    assert!(untouched.facts.is_empty());
    assert_eq!(
        (
            untouched.receipt,
            untouched.attachment,
            untouched.generation
        ),
        (None, None, 0)
    );
    // Obtaining another private LMDB transaction proves drop released that
    // resource, while the deterministic try_lock above proves session retention.
    let noop = owner
        .prepare(&empty)
        .unwrap()
        .expect("admitted")
        .seal(no_host())
        .unwrap()
        .commit()
        .unwrap();
    assert!(!noop.changed);
    assert_writer_owned(&db);
    drop(owner);
    assert!(db.writer.try_lock().is_ok());
    let _next = db.integration_writer(&work()).unwrap();
}

#[test]
fn old_and_new_read_instances_bind_application_rows_and_host_records_together() {
    let dir = TempDir::new("integration-coherent-snapshot");
    let db = Db::create(dir.path(), descriptor("Account"))
        .unwrap()
        .expect("schema");
    seed(&db);
    let replacement = changes(&db, &[(1, 20)], &[(1, 10)]);
    let barrier = Barrier::new(2);
    let (finish, may_finish) = mpsc::channel();
    std::thread::scope(|scope| {
        let reader_db = &db;
        let rendezvous = &barrier;
        let old = scope.spawn(move || {
            reader_db
                .read(|snapshot| {
                    let before = capture(snapshot)?;
                    rendezvous.wait();
                    may_finish.recv_timeout(Duration::from_secs(5)).unwrap();
                    assert_eq!(
                        capture(snapshot)?,
                        before,
                        "old application and host snapshot survives publication"
                    );
                    Ok(before)
                })
                .unwrap()
        });
        barrier.wait();
        let mut owner = db.integration_writer(&work()).unwrap();
        let prepared = owner.prepare(&replacement).unwrap().expect("admitted");
        assert_eq!(
            prepared.application_changes(),
            ApplicationChanges {
                added: 1,
                removed: 1
            }
        );
        let records = [HostRecordChange::Put {
            key: RECEIPT,
            value: b"committed",
        }];
        let sealed = prepared
            .seal(HostChanges {
                records: &records,
                attachment: AttachmentChange::Put(b"state/1"),
            })
            .unwrap();
        let before_commit = db.read(capture).unwrap();
        assert_eq!(
            before_commit.facts,
            vec![vec![Value::U64(1), Value::U64(10)]]
        );
        assert_eq!(before_commit.receipt, None);
        assert_eq!(sealed.commit().unwrap().generation.value(), 2);
        let after = db.read(capture).unwrap();
        assert_eq!(after.facts, vec![vec![Value::U64(1), Value::U64(20)]]);
        assert_eq!(after.receipt.as_deref(), Some(b"committed".as_slice()));
        assert_eq!(after.attachment.as_deref(), Some(b"state/1".as_slice()));
        assert_eq!(after.generation, 2);
        finish.send(()).unwrap();
        assert_eq!(old.join().unwrap(), before_commit);
    });
}

#[test]
fn cancelled_waiter_never_steals_or_releases_the_existing_session() {
    let dir = TempDir::new("integration-cancelled-waiter");
    let db = Db::create(dir.path(), descriptor("Account"))
        .unwrap()
        .expect("schema");
    let context = work();
    std::thread::scope(|scope| {
        let owner = db.integration_writer(&work()).unwrap();
        let (probed, observed_probe) = mpsc::channel();
        let contender_db = &db;
        let waiter_context = context.clone();
        let waiter = scope.spawn(move || {
            assert_writer_owned(contender_db);
            probed.send(()).unwrap();
            match contender_db.integration_writer(&waiter_context) {
                Err(IntegrationError::Work(WorkError::Cancelled)) => {}
                _ => panic!("cancelled contender must not acquire the held session"),
            }
        });
        observed_probe.recv_timeout(Duration::from_secs(5)).unwrap();
        context.cancel();
        waiter.join().unwrap();
        assert_writer_owned(&db);
        assert_eq!(owner.generation().unwrap().value(), 0);
        drop(owner);
    });
    let _next = db.integration_writer(&work()).unwrap();
}

#[test]
fn same_thread_reentrant_acquisition_and_foreign_change_refuse_without_losing_owner() {
    let dir = TempDir::new("integration-reentrant");
    let db = Db::create(dir.path(), descriptor("Account"))
        .unwrap()
        .expect("schema");
    let other_dir = TempDir::new("integration-foreign");
    let other = Db::create(other_dir.path(), descriptor("OtherAccount"))
        .unwrap()
        .expect("schema");
    let foreign = changes(&other, &[(1, 10)], &[]);
    let empty = changes(&db, &[], &[]);
    let mut owner = db.integration_writer(&work()).unwrap();
    assert!(matches!(
        db.integration_writer(&work()),
        Err(IntegrationError::ReentrantWriter)
    ));
    assert!(matches!(
        owner.prepare(&foreign),
        Err(IntegrationError::ForeignSchema)
    ));
    assert_writer_owned(&db);
    drop(owner.prepare(&empty).unwrap().expect("admitted"));
    assert!(matches!(
        db.integration_writer(&work()),
        Err(IntegrationError::ReentrantWriter)
    ));
    drop(owner);
    let _next = db.integration_writer(&work()).unwrap();
}

#[test]
fn cancellation_during_seal_aborts_private_data_but_session_lives_until_drop() {
    let dir = TempDir::new("integration-cancel-seal");
    let db = Db::create(dir.path(), descriptor("Account"))
        .unwrap()
        .expect("schema");
    let add = changes(&db, &[(1, 10)], &[]);
    let context = work();
    let mut owner = db.integration_writer(&context).unwrap();
    let prepared = owner.prepare(&add).unwrap().expect("admitted");
    context.cancel();
    assert!(matches!(
        prepared.seal(no_host()),
        Err(IntegrationError::Host(HostSealError::Work(
            WorkError::Cancelled
        )))
    ));
    assert_writer_owned(&db);
    let snapshot = db.read(capture).unwrap();
    assert!(snapshot.facts.is_empty());
    assert_eq!(snapshot.generation, 0);
    drop(owner);
    let _next = db.integration_writer(&work()).unwrap();
}
