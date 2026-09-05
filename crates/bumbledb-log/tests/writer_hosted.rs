//! `HostedHistory` over the deterministic C07 conditional store: one-HEAD
//! publication of composed head bodies, decision objects staged under the
//! parent head's object epoch, retry deduplication, `TailPolicy` envelope
//! backpressure, and — the crux — a lost/unknown CAS response that resolves to
//! the durable receipt via catch-up materialization rather than a fabricated
//! success/rejection. Maps to PROTO-04/06/12, the hosted arm of PROTO-17 and
//! the C08 `MaintenanceRequired` refusal. Verification: `NotRun`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bumbledb::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{ChangeSet, Db, ExecutionPolicy, Id128, RelationId, Value, WorkContext};

use bumbledb_log::history::authority::{DeleteOutcome, DeletedReason};
use bumbledb_log::history::command::{Command, CommandMetadata, Limits};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseId, DatabaseIdentity, IncarnationId, OperationId,
    ReceiptEpoch, RequestId, TerminalOutcome,
};
use bumbledb_log::manifest::{self, RecoveryRoot, TailPolicy};
use bumbledb_log::store::mem::{Behavior, MemStore, Op};
use bumbledb_log::store::{ObjectKind, ObjectRef, parse_object_key};
use bumbledb_log::writer::verbs::{ConditionalStore as _, HeadRead};
use bumbledb_log::writer::{HostedHistory, LogError, ResolveOutcome, SubmitOptions, SubmitOutcome};

const LIMITS: Limits = Limits {
    envelope_bytes: 1_000_000,
    change_bytes: 900_000,
    evidence_bytes: 10_000,
    result_bytes: 1_000,
};
const EPOCH: u64 = 1;

// ---- fixtures --------------------------------------------------------------

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-wh-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

fn theory() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "User".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
            }],
            extension: None,
        }],
        statements: vec![],
    }
}

fn fresh_db(tag: &str) -> Arc<Db<SchemaDescriptor>> {
    let dir = temp_dir(tag).join("db");
    Arc::new(
        Db::create(&dir, theory())
            .expect("create store")
            .expect("empty store admits"),
    )
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 1_000_000,
        result_bytes: 1_000_000,
        rows: 100_000,
        work_units: 10_000_000,
        timeout: Duration::from_secs(60),
    }
}

fn work() -> WorkContext {
    policy().start().unwrap()
}

fn identity(db: &Db<SchemaDescriptor>) -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(db.schema()),
    }
}

fn command(
    db: &Db<SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    value: u64,
) -> Command {
    let mut draft = ChangeSet::builder(db.schema(), work());
    draft.insert(RelationId(0), &[Value::U64(value)]).unwrap();
    let changes = draft.finish().unwrap();
    Command::seal(
        CommandMetadata {
            identity,
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
    .unwrap()
}

/// The machine borrows the shared production `MemStore` double, so the test
/// keeps direct fault-injection/inspection access to the same instance.
fn create<'a>(
    tag: &str,
    store: &'a MemStore,
) -> (
    HostedHistory<SchemaDescriptor, &'a MemStore>,
    DatabaseIdentity,
) {
    let db = fresh_db(tag);
    let identity = identity(&db);
    let history = HostedHistory::create(
        Arc::clone(&db),
        store,
        format!("tenants/{tag}"),
        EPOCH,
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes([0xc3; 16])),
        LIMITS,
        &work(),
    )
    .unwrap();
    (history, identity)
}

// ---- tests -----------------------------------------------------------------

#[test]
fn single_writer_publishes_composed_heads_and_deduplicates_retries() {
    let store = MemStore::new();
    let (history, identity) = create("happy", &store);
    let insert = command(history.db(), identity, 1, 7);
    let receipt = match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
            receipt
        }
        other => panic!("expected committed, got {other:?}"),
    };
    // Retry the identical command: the retained receipt, not a second commit.
    match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt: retry, .. } => assert_eq!(retry, receipt),
        other => panic!("expected decided retry, got {other:?}"),
    }
    // Re-inserting the same fact under a new command is a durable no-op.
    let again = command(history.db(), identity, 2, 7);
    assert!(matches!(
        history.submit(&again, &work()),
        SubmitOutcome::Decided {
            receipt: bumbledb_log::history::TerminalReceipt {
                outcome: TerminalOutcome::NoChange { .. },
                ..
            },
            ..
        }
    ));
    // Resolve returns the retained receipt.
    match history.resolve(receipt.command, &work()).unwrap() {
        ResolveOutcome::Found(found) => assert_eq!(found, receipt),
        other => panic!("expected found, got {other:?}"),
    }

    // The published HEAD body is the composed C08 head record — not the bare
    // control projection: it decodes through the manifest grammar, embeds the
    // advanced control, preserves the recovery root at the new tip and grows
    // the tail accounting (2 decisions: the commit and the no-op).
    let body = match store.read_head("tenants/happy/HEAD").unwrap() {
        HeadRead::Present { body, .. } => body,
        HeadRead::Absent => panic!("head must exist"),
    };
    let record = manifest::decode_head(&body, LIMITS.envelope_bytes).unwrap();
    assert_eq!(record.object_epoch, EPOCH);
    let recovery = record.recovery.expect("live head names its recovery root");
    assert_eq!(recovery.tail_count(), 2, "commit + durable no-op");
    assert!(recovery.tail_bytes > 0, "decision bytes are accounted");
    let live = record.control.live().unwrap();
    assert!(live.decision.seq >= receipt.decision_at.seq);

    // Every decision object is staged under the PARENT head's object epoch in
    // the canonical key grammar (the GC reference-introduction rule).
    let keys = store.object_keys();
    assert_eq!(keys.len(), 2, "one immutable object per decision");
    for key in keys {
        let (epoch, kind, _) = parse_object_key("tenants/happy", &key)
            .expect("staged keys parse in the canonical grammar");
        assert_eq!(epoch, EPOCH);
        assert_eq!(kind, ObjectKind::Decision);
    }
}

#[test]
fn lost_cas_response_resolves_to_the_durable_receipt_without_re_execution() {
    let store = MemStore::new();
    let (history, identity) = create("lost", &store);
    let insert = command(history.db(), identity, 1, 5);
    // The CAS applies remotely but its response is lost.
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
    match history.submit(&insert, &work()) {
        // The publication machine cannot establish the outcome in this
        // invocation, so it returns uncertainty with the retained ref.
        SubmitOutcome::OutcomeUnknown { command, .. } => {
            assert_eq!(command, insert.command_ref());
        }
        // If the machine resolved it inline via catch-up, that is also correct
        // as long as it is the true decided receipt.
        SubmitOutcome::Decided { receipt, .. } => {
            assert_eq!(receipt.command, insert.command_ref());
        }
        other @ SubmitOutcome::NotSubmitted { .. } => {
            panic!("lost CAS must never be NotSubmitted: {other:?}")
        }
    }
    // A subsequent resolve proves the decision published, from the retained
    // receipt materialized by catch-up — never a re-execution.
    match history.resolve(insert.command_ref(), &work()).unwrap() {
        ResolveOutcome::Found(receipt) => {
            assert_eq!(receipt.command, insert.command_ref());
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("expected found after lost CAS, got {other:?}"),
    }
    // Re-submitting the identical command now returns the same receipt.
    match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("expected decided on resubmit, got {other:?}"),
    }
}

#[test]
fn exhausted_tail_envelope_refuses_new_admission_but_still_resolves_receipts() {
    let store = MemStore::new();
    let (history, identity) = create("envelope", &store);
    let history = history.with_tail_policy(TailPolicy {
        max_count: 1,
        max_bytes: u64::MAX,
    });
    // The first decision fits the one-decision envelope exactly.
    let first = command(history.db(), identity, 1, 5);
    let receipt = match history.submit(&first, &work()) {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("expected committed, got {other:?}"),
    };
    // A second NEW command — even one that would net a no-op decision — is
    // refused with typed backpressure before any work is dispatched.
    let second = command(history.db(), identity, 2, 5);
    match history.submit(&second, &work()) {
        SubmitOutcome::NotSubmitted {
            error: LogError::MaintenanceRequired { count, bytes },
            ..
        } => {
            assert_eq!(count, 1);
            assert!(bytes > 0);
        }
        other => panic!("expected MaintenanceRequired, got {other:?}"),
    }
    // The retained receipt still resolves and retries still deduplicate:
    // backpressure refuses NEW admission, not lookup.
    match history.resolve(receipt.command, &work()).unwrap() {
        ResolveOutcome::Found(found) => assert_eq!(found, receipt),
        other => panic!("expected found under backpressure, got {other:?}"),
    }
    match history.submit(&first, &work()) {
        SubmitOutcome::Decided { receipt: retry, .. } => assert_eq!(retry, receipt),
        other => panic!("expected deduplicated retry under backpressure, got {other:?}"),
    }
}

#[test]
fn open_on_missing_head_never_initializes() {
    let db = fresh_db("missing");
    let store = MemStore::new();
    assert!(matches!(
        HostedHistory::open(db, &store, "tenants/missing".into(), LIMITS, &work()),
        Err(LogError::NotInitialized)
    ));
}

// ---- read-side catch-up (the public P06R2 verb) ------------------------------

/// Leave the machine with a genuinely stale LOCAL materialization: the CAS
/// applies remotely but its response is lost, and the resolve-time decision
/// fetch fails transiently, so the submit ends `OutcomeUnknown` with the
/// local cache still at genesis while the verified head is one decision ahead.
fn submit_leaving_local_stale(
    history: &HostedHistory<SchemaDescriptor, &MemStore>,
    store: &MemStore,
    command: &Command,
) {
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
    store.fail_next(Op::GetObject, Behavior::Error);
    match history.submit(command, &work()) {
        SubmitOutcome::OutcomeUnknown {
            command: reference, ..
        } => {
            assert_eq!(reference, command.command_ref());
        }
        other => panic!("scripted lost CAS + failed fetch must be Unknown: {other:?}"),
    }
}

#[test]
fn read_side_catch_up_advances_a_stale_local_materialization() {
    let store = MemStore::new();
    let (history, identity) = create("catchup", &store);
    let insert = command(history.db(), identity, 1, 9);
    submit_leaving_local_stale(&history, &store, &insert);

    // The public read-side verb advances the local materialization to the
    // verified tip WITHOUT submitting, initializing or thawing anything.
    let version_before = match store.read_head("tenants/catchup/HEAD").unwrap() {
        HeadRead::Present { version, .. } => version,
        HeadRead::Absent => panic!("head must exist"),
    };
    let tip = history.catch_up(&work()).unwrap();
    let record = match store.read_head("tenants/catchup/HEAD").unwrap() {
        HeadRead::Present { version, body } => {
            assert_eq!(version, version_before, "catch-up never writes the head");
            manifest::decode_head(&body, LIMITS.envelope_bytes).unwrap()
        }
        HeadRead::Absent => panic!("head must exist"),
    };
    assert_eq!(tip, record.control.live().unwrap().decision);
    assert_eq!(tip.seq, 1, "genesis plus exactly one decision");
    // The decided fact is now locally materialized (readable at latest).
    let mut rows = 0;
    history
        .db()
        .read(|read| {
            rows = read.count(RelationId(0))?;
            Ok(())
        })
        .unwrap();
    assert_eq!(rows, 1, "the committed fact reached the local cache");
    // The retained receipt is locally visible too: resolve finds it.
    match history.resolve(insert.command_ref(), &work()).unwrap() {
        ResolveOutcome::Found(receipt) => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("expected found after catch-up, got {other:?}"),
    }
    // Already at the tip: idempotent, same stamp, still no head write.
    assert_eq!(history.catch_up(&work()).unwrap(), tip);
}

#[test]
fn read_side_catch_up_routes_stale_caches_and_tombstones_typed() {
    // A warm cache OLDER than the durable tail's checkpoint base cannot walk
    // to the tip (the gap may be legitimately collected): typed
    // MaterializationStale for the recovery lane, never corruption or an
    // empty fallback.
    let store = MemStore::new();
    let (history, identity) = create("stalecache", &store);
    let insert = command(history.db(), identity, 1, 5);
    submit_leaving_local_stale(&history, &store, &insert);
    // Simulate a checkpoint published exactly at the tip (base == tip): the
    // stale local (still at genesis) is now behind the checkpoint base.
    assert!(store.corrupt_head("tenants/stalecache/HEAD", |body| {
        let mut record = manifest::decode_head(body, LIMITS.envelope_bytes).unwrap();
        let tip = record.control.live().unwrap().decision;
        record.recovery = Some(RecoveryRoot {
            checkpoint: Some(ObjectRef {
                epoch: EPOCH,
                kind: ObjectKind::Checkpoint,
                digest: [7; 32],
                length: 9,
            }),
            base: tip,
            tip,
            tail_bytes: 0,
            epoch_floor: EPOCH,
        });
        *body = manifest::encode_head(&record, LIMITS.envelope_bytes).unwrap();
    }));
    assert!(matches!(
        history.catch_up(&work()),
        Err(LogError::MaterializationStale)
    ));

    // A terminal tombstone head refuses typed: nothing to materialize, and
    // the verb performs no lifecycle transition of its own.
    let store2 = MemStore::new();
    let (history2, _) = create("deadhead", &store2);
    assert!(store2.corrupt_head("tenants/deadhead/HEAD", |body| {
        let record = manifest::decode_head(body, LIMITS.envelope_bytes).unwrap();
        let deleted = match record
            .control
            .delete(
                OperationId::from_core(Id128::from_bytes([0xee; 16])),
                DeletedReason::Erasure,
            )
            .unwrap()
        {
            DeleteOutcome::Deleted(deleted) => deleted,
            DeleteOutcome::AlreadyDeleted { .. } => unreachable!("live head"),
        };
        *body =
            manifest::encode_head(&record.with_control(deleted), LIMITS.envelope_bytes).unwrap();
    }));
    assert!(matches!(
        history2.catch_up(&work()),
        Err(LogError::DatabaseDeleted)
    ));
}

// ---- per-call submit options (the P04R attempts/backoff seam) ----------------

#[test]
fn per_call_submit_options_never_consume_or_widen_the_machine() {
    let store = MemStore::new();
    let (history, identity) = create("options", &store);
    // A narrowed per-call budget with a bounded backoff request: the happy
    // path decides on the first attempt (backoff applies only after a
    // definite CAS loss, so none of it delays this call).
    let options = SubmitOptions {
        attempts: Some(1),
        backoff_base: Some(Duration::from_millis(1)),
        backoff_cap: Some(Duration::from_millis(4)),
    };
    let first = command(history.db(), identity, 1, 7);
    let receipt = match history.submit_with(&first, options, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
            receipt
        }
        other => panic!("expected committed, got {other:?}"),
    };
    // The machine was NOT consumed or reconfigured: the same instance keeps
    // submitting under its own defaults, and a retry still deduplicates.
    match history.submit(&first, &work()) {
        SubmitOutcome::Decided { receipt: retry, .. } => assert_eq!(retry, receipt),
        other => panic!("expected deduplicated retry, got {other:?}"),
    }
    // A hostile wide request (u32::MAX attempts, hour-long backoff) is
    // clamped to the machine's own bounds — the shape is asserted directly
    // by the writer::hosted unit tests; here it must simply still decide.
    let wide = SubmitOptions {
        attempts: Some(u32::MAX),
        backoff_base: Some(Duration::from_secs(3_600)),
        backoff_cap: Some(Duration::from_secs(3_600)),
    };
    let second = command(history.db(), identity, 2, 8);
    assert!(matches!(
        history.submit_with(&second, wide, &work()),
        SubmitOutcome::Decided { .. }
    ));
}
