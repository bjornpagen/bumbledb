//! F3 review finding A — the admin verb family must validate the intended
//! database identity (REP-011 / SDK-016 / ARCH-004; gates G10/G11/G14).
//!
//! These tests drive [`run_admin`] — the exact native dispatcher every
//! `logAdmin` request reaches below the N-API marshal layer — against real
//! runtime registries, kernel fences and LMDB materializations. The pinned
//! defect: `open_admin_db` selected a tenant by DIRECTORY alone (warm
//! registry reuse and cold transient opens alike) with no comparison of the
//! requested binding's database/incarnation/schema identity, so erase,
//! receipt retirement, root release and the migration arms could act on the
//! wrong tenant. Each regression asserts the refusal is typed
//! (`ForeignIdentity` / `WrongLineage` / `CacheIdentityMismatch`, mapping to
//! certainty `not-started`) and that the unintended tenant's facts, receipts,
//! roots and authority remain byte-unchanged.
//!
//! The hosted-prefix double (valid identity paired with another tenant's
//! hosted prefix, MemStore/FsStore) lives in
//! `crates/bumbledb-log/tests/adversarial-admin-identity.rs`: the bridge's
//! hosted arms reach S3 only, and every one of them now goes through the
//! same `bumbledb_log::admin::verify_hosted_identity` gate those tests pin.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use bumbledb::Theory as _;
use bumbledb::work::ExecutionPolicy;
use bumbledb::{RelationId, Value};
use bumbledb_log::history::command::{Command, CommandMetadata};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseId, IncarnationId, ReceiptEpoch, RequestId,
};
use bumbledb_log::certainty::{PublicationPhase, SubmitCertainty};
use bumbledb_log::writer::SubmitOptions;

use super::*;
use crate::runtime::{CloseReport, Options};

use super::super::{HistoryOpened, OpenSpec, open_history};

bumbledb::schema! {
    pub Mini;
    relation Item { a: u64, b: u64 }
    Item(a) -> Item;
}

fn options() -> Options {
    Options {
        workers: 2,
        queue_capacity: 8,
        cleanup_capacity: 8,
        owner_capacity: 8,
        native_handle_capacity: 16,
        aggregate_bytes: [64 << 20; 4],
        chunk_bytes: 1 << 20,
        cleanup_timeout: Duration::from_millis(500),
    }
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 16 << 20,
        working_bytes: 16 << 20,
        scratch_bytes: 16 << 20,
        result_bytes: 16 << 20,
        rows: 1 << 20,
        work_units: 1 << 30,
        timeout: Duration::from_secs(10),
    }
}

fn unique_dir(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "bumbledb-f3a-admin-identity-{tag}-{}-{seq}",
        std::process::id()
    ))
}

fn identity_of(descriptor: &bumbledb::SchemaDescriptor, seed: u8) -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: DatabaseId::from_core(bumbledb::Id128::from_bytes([seed; 16])),
        incarnation_id: IncarnationId::from_core(bumbledb::Id128::from_bytes([seed ^ 0xff; 16])),
        schema_id: bumbledb_log::schema_file::schema_id(descriptor).expect("valid schema"),
    }
}

fn open_spec(directory: &Path, create: bool, seed: u8) -> OpenSpec {
    let descriptor = Mini.descriptor();
    let identity = identity_of(&descriptor, seed);
    let artifact = bumbledb_log::schema_file::render(&descriptor).into_bytes();
    OpenSpec {
        create,
        directory: directory.to_string_lossy().into_owned(),
        identity,
        backend: BackendSpec::Local,
        discard_mismatched: false,
        creation: create.then(|| {
            (
                OperationId::from_core(bumbledb::Id128::from_bytes([seed.wrapping_add(1); 16])),
                artifact,
            )
        }),
        descriptor,
        attrs: Vec::new(),
        tail_policy: bumbledb_log::manifest::TailPolicy::UNBOUNDED,
    }
}

fn drain_runtime(runtime: &Arc<Runtime>) -> CloseReport {
    let (tx, rx) = std::sync::mpsc::channel();
    runtime.drain(
        None,
        Box::new(move |report| {
            tx.send(report).unwrap();
        }),
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("runtime drain")
}

fn drain_resource(resource: &Arc<super::super::HistoryResource>) -> CloseReport {
    let (tx, rx) = std::sync::mpsc::channel();
    resource.drain(Box::new(move |report| {
        tx.send(report).unwrap();
    }));
    rx.recv_timeout(Duration::from_secs(10))
        .expect("history drain")
}

fn op_id(byte: u8) -> OperationId {
    OperationId::from_core(bumbledb::Id128::from_bytes([byte; 16]))
}

/// The requested binding for the local admin verbs under test.
fn local_binding(directory: &Path, identity: DatabaseIdentity, with_schema: bool) -> BindingSpec {
    BindingSpec {
        directory: directory.to_string_lossy().into_owned(),
        identity,
        backend: BackendSpec::Local,
        descriptor: with_schema.then(|| (Mini.descriptor(), Vec::new())),
    }
}

/// Submit one real command (an `Item` insert) through the opened history —
/// facts AND a terminal receipt row now exist for byte-unchanged assertions.
fn submit_fact(opened: &HistoryOpened, request: u8, a: u64, work: &WorkContext) {
    let (kind, lease) = opened
        .resource
        .kind_and_lease()
        .expect("live history for submit");
    drop(lease);
    let descriptor = Mini.descriptor();
    let schema = {
        use bumbledb::schema::ValidateDescriptor as _;
        descriptor.validate().expect("valid schema")
    };
    let mut builder = bumbledb::ChangeSet::builder(&schema, work.clone());
    builder
        .insert(RelationId(0), &[Value::U64(a), Value::U64(a + 1)])
        .expect("insert");
    let changes = builder.finish().expect("change set");
    let command = Command::seal(
        CommandMetadata {
            identity: opened.resource.identity,
            id: CommandId {
                receipt_epoch: ReceiptEpoch::new(1).expect("one"),
                request_id: RequestId::from_core(bumbledb::Id128::from_bytes([request; 16])),
            },
            condition: Condition::Unconditional,
        },
        changes,
        CommandResult::from_canonical_bytes(Vec::new().into_boxed_slice()),
        LIMITS,
        work,
    )
    .expect("seals");
    match kind.submit_certain_with(&command, SubmitOptions::DEFAULT, work) {
        SubmitCertainty::Decided { .. } => {}
        other => panic!("fixture submit decides, got {other:?}"),
    }
}

type LocalSnapshot = (Vec<(Vec<u8>, Vec<u8>)>, Option<Vec<u8>>, Vec<Vec<Value>>);

/// Every byte an admin verb could touch: all host records (receipts, roots
/// registry, origin binding), the authority attachment, and the facts of the
/// one test relation.
fn snapshot_engine(db: &crate::Engine) -> LocalSnapshot {
    let mut records = Vec::new();
    let mut attachment = None;
    let mut facts = Vec::new();
    db.read(|read| {
        read.integration_host_scan(b"", &mut |key: &[u8], value: &[u8]| {
            records.push((key.to_vec(), value.to_vec()));
            Ok(())
        })
        .expect("host scan");
        attachment = read
            .integration_host_attachment()
            .expect("attachment reads")
            .map(<[u8]>::to_vec);
        for row in read.scan(RelationId(0)).expect("facts scan") {
            facts.push(row.expect("fact row"));
        }
        Ok(())
    })
    .expect("read");
    (records, attachment, facts)
}

/// Snapshot a CLOSED tenant directory by opening its materialization
/// directly (nothing else may hold it).
fn snapshot_dir(directory: &Path) -> LocalSnapshot {
    let ready = bumbledb_log::recovery::materialization_path(directory);
    let db = crate::Engine::open(&ready, Mini.descriptor()).expect("snapshot open");
    snapshot_engine(&db)
}

fn expect_completed(result: MachineResult<AdminOwned>) -> AdminValueOwned {
    match result {
        Ok(AdminOwned::Completed(value)) => value,
        Ok(AdminOwned::Report(_)) => panic!("expected Completed, got Report"),
        Ok(AdminOwned::Failed { fail, phase }) => {
            panic!("expected Completed, got Failed({fail:?}, phase={phase:?})")
        }
        Err(fail) => panic!("expected Completed, got Err({fail:?})"),
    }
}

fn expect_refusal(result: MachineResult<AdminOwned>, code: &str) {
    match result {
        Err(LogFail::Protocol { code: got, detail }) => {
            assert_eq!(got, code, "typed refusal code (detail: {detail})");
        }
        Err(other) => panic!("expected {code} protocol refusal, got {other:?}"),
        Ok(AdminOwned::Completed(_)) => panic!("expected {code} refusal, got Completed"),
        Ok(AdminOwned::Report(_)) => panic!("expected {code} refusal, got Report"),
        Ok(AdminOwned::Failed { fail, phase }) => {
            panic!("expected thrown {code} refusal, got Failed({fail:?}, phase={phase:?})")
        }
    }
}

#[test]
fn admin_identity_cold_erase_refuses_a_foreign_database_and_mutates_nothing() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("cold-erase");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let created = open_history(&runtime, &open_spec(&dir, true, 3), &work).expect("creates");
    let identity = created.resource.identity;
    submit_fact(&created, 21, 10, &work);
    assert_eq!(drain_resource(&created.resource), CloseReport::Closed);
    let before = snapshot_dir(&dir);

    // Same schema, valid-looking identity — but a DIFFERENT database. The
    // cold transient open must refuse before erase dispatches anything.
    let mut foreign = identity;
    foreign.database_id = DatabaseId::from_core(bumbledb::Id128::from_bytes([0x99; 16]));
    expect_refusal(
        run_admin(
            &runtime,
            AdminVerb::Erase {
                binding: local_binding(&dir, foreign, true),
                operation: op_id(0x31),
                retain_roots: Vec::new(),
            },
            &work,
        ),
        "ForeignIdentity",
    );
    assert_eq!(
        snapshot_dir(&dir),
        before,
        "facts, receipts and authority are byte-unchanged after the refusal"
    );

    // The exact identity is NOT blocked: the same verb tombstones its own
    // tenant (positive control — the gate blocks cross-tenant aim only).
    match expect_completed(run_admin(
        &runtime,
        AdminVerb::Erase {
            binding: local_binding(&dir, identity, true),
            operation: op_id(0x32),
            retain_roots: Vec::new(),
        },
        &work,
    )) {
        AdminValueOwned::Erase { tombstoned, .. } => assert!(tombstoned),
        _ => panic!("erase answers the erase verb"),
    }

    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn admin_identity_warm_reuse_refuses_a_stale_incarnation_before_retirement() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("warm-retire");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    // The tenant STAYS open in this runtime's registry: the verb takes the
    // warm lease path (no `schema` field supplied — cold open is impossible).
    let opened = open_history(&runtime, &open_spec(&dir, true, 7), &work).expect("creates");
    let identity = opened.resource.identity;
    submit_fact(&opened, 22, 40, &work);
    let (_, lease) = opened.resource.kind_and_lease().expect("live");
    let before = snapshot_engine(lease.db());

    // The stale binding: the same database under its pre-restore/migration
    // incarnation. Receipt retirement must refuse before touching any row.
    let mut stale = identity;
    stale.incarnation_id = IncarnationId::from_core(bumbledb::Id128::from_bytes([0x11; 16]));
    expect_refusal(
        run_admin(
            &runtime,
            AdminVerb::RetireReceipts {
                binding: local_binding(&dir, stale, false),
                operation: op_id(0x41),
                through: 1,
            },
            &work,
        ),
        "WrongLineage",
    );
    assert_eq!(
        snapshot_engine(lease.db()),
        before,
        "the warm tenant's receipts are byte-unchanged after the refusal"
    );

    // The exact identity maintains through the warm lease (positive
    // control): rotate the open epoch past 1, then retire epoch 1.
    match expect_completed(run_admin(
        &runtime,
        AdminVerb::RotateEpoch {
            binding: local_binding(&dir, identity, false),
            operation: op_id(0x42),
        },
        &work,
    )) {
        AdminValueOwned::RotateEpoch { open_epoch } => assert_eq!(open_epoch, 2),
        _ => panic!("rotation answers the rotate verb"),
    }
    match expect_completed(run_admin(
        &runtime,
        AdminVerb::RetireReceipts {
            binding: local_binding(&dir, identity, false),
            operation: op_id(0x43),
            through: 1,
        },
        &work,
    )) {
        AdminValueOwned::RetireReceipts { retired_through } => assert_eq!(retired_through, 1),
        _ => panic!("retirement answers the retire verb"),
    }

    drop(lease);
    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn admin_identity_valid_identity_at_another_tenants_directory_refuses_root_release() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("cross-dir");
    std::fs::create_dir_all(&base).unwrap();
    let dir_a = base.join("tenant-a");
    let dir_b = base.join("tenant-b");
    let work = policy().start().unwrap();

    // Two SAME-SCHEMA tenants; identity A is fully valid — it is just not
    // the tenant living at dir_b.
    let created_a = open_history(&runtime, &open_spec(&dir_a, true, 3), &work).expect("creates a");
    let identity_a = created_a.resource.identity;
    assert_eq!(drain_resource(&created_a.resource), CloseReport::Closed);
    let created_b = open_history(&runtime, &open_spec(&dir_b, true, 5), &work).expect("creates b");
    let identity_b = created_b.resource.identity;
    submit_fact(&created_b, 23, 70, &work);
    assert_eq!(drain_resource(&created_b.resource), CloseReport::Closed);

    // Pin a named root on B (its own restore capability).
    let AdminValueOwned::PinRoot { root: root_hex, .. } = expect_completed(run_admin(
        &runtime,
        AdminVerb::PinRoot {
            binding: local_binding(&dir_b, identity_b, true),
            operation: op_id(0x51),
            label: "keep".to_string(),
        },
        &work,
    )) else {
        panic!("pin-root answers the pin verb");
    };
    let root =
        OperationId::from_core(marshal::id128_in(&root_hex, "test root").expect("root id parses"));
    let before = snapshot_dir(&dir_b);

    // Erase and root release aimed at B's directory under A's identity: both
    // refuse typed; B keeps its facts, receipts, roots and authority.
    expect_refusal(
        run_admin(
            &runtime,
            AdminVerb::ReleaseRoot {
                binding: local_binding(&dir_b, identity_a, true),
                operation: op_id(0x52),
                root,
            },
            &work,
        ),
        "ForeignIdentity",
    );
    expect_refusal(
        run_admin(
            &runtime,
            AdminVerb::Erase {
                binding: local_binding(&dir_b, identity_a, true),
                operation: op_id(0x53),
                retain_roots: Vec::new(),
            },
            &work,
        ),
        "ForeignIdentity",
    );
    assert_eq!(
        snapshot_dir(&dir_b),
        before,
        "tenant b is byte-unchanged after both cross-tenant refusals"
    );

    // B's own identity releases its root (positive control).
    match expect_completed(run_admin(
        &runtime,
        AdminVerb::ReleaseRoot {
            binding: local_binding(&dir_b, identity_b, true),
            operation: op_id(0x54),
            root,
        },
        &work,
    )) {
        AdminValueOwned::ReleaseRoot { .. } => {}
        _ => panic!("release-root answers the release verb"),
    }

    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn admin_identity_stale_binding_after_reincarnation_refuses_epoch_rotation() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("stale-binding");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    // The post-restore/post-migration state: database D reborn under a NEW
    // incarnation in this directory.
    let mut spec = open_spec(&dir, true, 9);
    spec.identity.incarnation_id =
        IncarnationId::from_core(bumbledb::Id128::from_bytes([0xdd; 16]));
    let created = open_history(&runtime, &spec, &work).expect("creates");
    let new_identity = created.resource.identity;
    assert_eq!(drain_resource(&created.resource), CloseReport::Closed);
    let before = snapshot_dir(&dir);

    // The stale binding names the OLD incarnation.
    let mut stale = new_identity;
    stale.incarnation_id = IncarnationId::from_core(bumbledb::Id128::from_bytes([0x11; 16]));
    expect_refusal(
        run_admin(
            &runtime,
            AdminVerb::RotateEpoch {
                binding: local_binding(&dir, stale, true),
                operation: op_id(0x61),
            },
            &work,
        ),
        "WrongLineage",
    );
    assert_eq!(
        snapshot_dir(&dir),
        before,
        "the reborn tenant is byte-unchanged after the stale-binding refusal"
    );

    // The new binding rotates (positive control): epoch advances 1 → 2.
    match expect_completed(run_admin(
        &runtime,
        AdminVerb::RotateEpoch {
            binding: local_binding(&dir, new_identity, true),
            operation: op_id(0x62),
        },
        &work,
    )) {
        AdminValueOwned::RotateEpoch { open_epoch } => assert_eq!(open_epoch, 2),
        _ => panic!("rotation answers the rotate verb"),
    }

    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn admin_identity_copied_directory_refuses_on_the_recorded_origin_binding() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("copied-dir");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let copy = base.join("tenant-copy");
    let work = policy().start().unwrap();

    let created = open_history(&runtime, &open_spec(&dir, true, 3), &work).expect("creates");
    let identity = created.resource.identity;
    submit_fact(&created, 24, 90, &work);
    assert_eq!(drain_resource(&created.resource), CloseReport::Closed);

    // A byte-for-byte copy of the tenant at a NEW location: the identity is
    // exact, but the recorded origin binding still names the original
    // directory — canonical-location provenance (REP-011) refuses adoption.
    copy_dir(&dir, &copy);
    let before = snapshot_dir(&copy);
    expect_refusal(
        run_admin(
            &runtime,
            AdminVerb::RetireReceipts {
                binding: local_binding(&copy, identity, true),
                operation: op_id(0x71),
                through: 1,
            },
            &work,
        ),
        "CacheIdentityMismatch",
    );
    assert_eq!(
        snapshot_dir(&copy),
        before,
        "the copied directory is byte-unchanged after the origin refusal"
    );

    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy root");
    for entry in std::fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        let kind = entry.file_type().expect("file type");
        if kind.is_dir() {
            copy_dir(&entry.path(), &target);
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}
