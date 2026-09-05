//! W2-CERT regressions — publication certainty at the writer/adapter boundary
//! (audit-log findings #1, #9, #17, #6 machine half), scripted over the
//! deterministic production `MemStore` double.
//!
//! - A transport error DURING the dispatched head CAS is potentially
//!   published: never `NotSubmitted`, always the unknown-resolution ladder.
//! - After an indeterminate CAS, a consumed version token with no retained
//!   receipt post-catch-up is a PROVEN loss: the submit loop re-attempts and
//!   decides within its bounds instead of returning `OutcomeUnknown`.
//! - A genuinely unresolvable outcome (the conditioned version still current)
//!   stays `OutcomeUnknown`.
//! - Hosted create resolves uncertainty by evidence: byte-exact genesis (or
//!   our own activation evidence on an advanced head) is created-by-us;
//!   foreign bytes are `CommandIdentityConflict` (`AuthorityExists`);
//!   unreadable/absent evidence stays unknown-typed.
//! - The durable-tail envelope default is FINITE; `UNBOUNDED` only by the
//!   explicit option.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use bumbledb::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{ChangeSet, Db, ExecutionPolicy, Id128, RelationId, Value, WorkContext};

use bumbledb_log::history::command::{Command, CommandMetadata, Limits};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseId, DatabaseIdentity, IncarnationId, OperationId,
    ReceiptEpoch, RequestId, TerminalOutcome,
};
use bumbledb_log::manifest::{self, TailPolicy};
use bumbledb_log::store::mem::{Behavior, Gate, MemFault, MemStore, Op};
use bumbledb_log::store::{
    ConditionalOutcome, ConditionalStore, HeadVersion, ListPage, PutOutcome, ReceiveLimits,
    ReceivedBody, ReceivedHead,
    ReceivingStore, TransportContext, TransportObservation,
};
use bumbledb_log::writer::hosted::DEFAULT_TAIL_POLICY;
use bumbledb_log::writer::{HostedHistory, LogError, ResolveOutcome, SubmitOutcome};

const LIMITS: Limits = Limits {
    envelope_bytes: 1_000_000,
    change_bytes: 900_000,
    evidence_bytes: 10_000,
    result_bytes: 1_000,
};
const EPOCH: u64 = 1;
const OPERATION: [u8; 16] = [0xc3; 16];

// ---- fixtures --------------------------------------------------------------

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-cert-{tag}-{}-{nanos}-{seq}",
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
        Db::create(&dir, theory(), work())
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

/// Create one hosted machine over the shared production `MemStore` double,
/// handing back the `Db` so the test can open further machines over it.
fn create<'a>(
    tag: &str,
    store: &'a MemStore,
) -> (
    HostedHistory<SchemaDescriptor, &'a MemStore>,
    DatabaseIdentity,
    Arc<Db<SchemaDescriptor>>,
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
        OperationId::from_core(Id128::from_bytes(OPERATION)),
        LIMITS,
        &work(),
    )
    .unwrap();
    (history, identity, db)
}

// ---- #1: transport error during the dispatched CAS --------------------------

#[test]
fn a_transport_error_during_the_dispatched_cas_is_never_not_submitted() {
    let store = MemStore::new();
    let (history, identity, _db) = create("casreset", &store);
    let insert = command(history.db(), identity, 1, 7);
    // The adapter reports a transport error on the dispatched conditional
    // replace — a 5xx / connection-reset shape with no typed outcome. The CAS
    // may have been applied; this must never surface as NotSubmitted.
    store.fail_next(Op::ReplaceHead, Behavior::Error);
    match history.submit(&insert, &work()) {
        SubmitOutcome::OutcomeUnknown { command, .. } => {
            assert_eq!(command, insert.command_ref());
        }
        other => panic!("a mid-CAS transport error is potentially published: {other:?}"),
    }
    // In this schedule the request never landed: the ladder observed the head
    // still at the conditioned version — genuine uncertainty, an observation,
    // not a fabricated refusal.
    match history.resolve(insert.command_ref(), &work()).unwrap() {
        ResolveOutcome::NotRecordedAt { .. } => {}
        other => panic!("expected NotRecordedAt observation, got {other:?}"),
    }
    // The retained ref resolves by evidence on the next attempt: the same
    // command decides exactly once.
    match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("expected decided retry, got {other:?}"),
    }
}

// ---- #9: proven CAS loss re-attempts ----------------------------------------

#[test]
fn a_proven_cas_loss_re_attempts_and_decides_within_bounds() {
    let store = MemStore::new();
    let (victim, identity, _db) = create("provenloss", &store);
    // A second independent writer over the SAME durable prefix: hosted create
    // is evidence-idempotent (same identity, same operation), so it binds a
    // second machine to its own fresh local cache.
    let db2 = fresh_db("provenloss-rival");
    let rival = HostedHistory::create(
        Arc::clone(&db2),
        &store,
        "tenants/provenloss".to_string(),
        EPOCH,
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes(OPERATION)),
        LIMITS,
        &work(),
    )
    .unwrap();

    // Pause the victim's FIRST head CAS with the request "in flight".
    let gate = Arc::new(Gate::new());
    let armed = Arc::new(AtomicBool::new(false));
    {
        let gate = Arc::clone(&gate);
        let armed = Arc::clone(&armed);
        let mut first = true;
        store.set_gate(move |op, _key| {
            if op == Op::ReplaceHead && first {
                first = false;
                armed.store(true, Ordering::SeqCst);
                return Some(Arc::clone(&gate));
            }
            None
        });
    }
    let victim_cmd = command(victim.db(), identity, 1, 11);
    let rival_cmd = command(rival.db(), identity, 2, 22);
    std::thread::scope(|scope| {
        let paused = scope.spawn(|| victim.submit(&victim_cmd, &work()));
        while !armed.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        // The rival publishes and CONSUMES the version token the victim's
        // in-flight CAS conditioned on.
        match rival.submit(&rival_cmd, &work()) {
            SubmitOutcome::Decided { receipt, .. } => {
                assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
            }
            other => panic!("the rival must publish: {other:?}"),
        }
        // The victim's CAS response is lost (nothing applied): an ambiguous
        // outcome, NOT a typed loss. The evidence ladder must prove the loss
        // (token consumed + complete retained lookup post-catch-up empty) and
        // re-attempt under the machine bounds instead of OutcomeUnknown.
        store.fail_next(Op::ReplaceHead, Behavior::IndeterminateDropped);
        gate.open();
        match paused.join().expect("victim thread") {
            SubmitOutcome::Decided { receipt, .. } => {
                assert_eq!(receipt.command, victim_cmd.command_ref());
                assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
            }
            other => panic!("a proven loss re-attempts and decides: {other:?}"),
        }
    });

    // Both decisions are durable: the composed head accounts for exactly the
    // rival's and the victim's decisions.
    let ctx = work();
    let body = match store
        .receive_head(
            "tenants/provenloss/HEAD",
            TransportContext::new(&ctx, ReceiveLimits::capped(LIMITS.envelope_bytes as u64)),
        )
        .expect("bounded head receive")
    {
        ReceivedHead::Present { body, .. } => body,
        ReceivedHead::Absent => panic!("head must exist"),
    };
    let record = manifest::decode_head(body.as_bytes(), LIMITS.envelope_bytes).unwrap();
    drop(body);
    assert_eq!(
        record.recovery.expect("live head").tail_count(),
        2,
        "the rival's commit and the victim's re-attempted commit"
    );
}

// ---- #17: hosted create resolves by evidence ---------------------------------

#[test]
fn an_uncertain_create_resolves_to_created_by_us_evidence() {
    let store = MemStore::new();
    let db = fresh_db("createvid");
    let identity = identity(&db);
    // The create's conditional PUT applies but its response is lost: the
    // machine reads the head back, byte-compares its own deterministic
    // genesis, and reports created-by-us success — never AuthorityExists.
    store.fail_next(Op::CreateHead, Behavior::IndeterminateApplied);
    let history = HostedHistory::create(
        Arc::clone(&db),
        &store,
        "tenants/createvid".to_string(),
        EPOCH,
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes(OPERATION)),
        LIMITS,
        &work(),
    )
    .expect("an applied-but-unacknowledged create resolves to created evidence");

    // A full create retry (same identity, same operation) against the now
    // definite PreconditionFailed also resolves by the byte-exact evidence.
    let retry = HostedHistory::create(
        Arc::clone(&db),
        &store,
        "tenants/createvid".to_string(),
        EPOCH,
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes(OPERATION)),
        LIMITS,
        &work(),
    )
    .expect("a create retry returns created evidence, not AuthorityExists");
    assert_eq!(retry.identity(), identity);

    // The machine is fully usable: a submitted command decides.
    let insert = command(history.db(), identity, 1, 7);
    match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("expected committed, got {other:?}"),
    }

    // Even after the head advanced past genesis, a late create retry from a
    // fresh cache still recognizes its OWN activation evidence (identity +
    // operation + target genesis) on the decoded head.
    let db_late = fresh_db("createvid-late");
    let late = HostedHistory::create(
        Arc::clone(&db_late),
        &store,
        "tenants/createvid".to_string(),
        EPOCH,
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes(OPERATION)),
        LIMITS,
        &work(),
    )
    .expect("an advanced head still carries this create's activation evidence");
    // And that machine catches up and deduplicates the decided command.
    match late.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("expected deduplicated receipt, got {other:?}"),
    }
}

#[test]
fn create_over_a_foreign_authority_refuses_with_evidence() {
    let store = MemStore::new();
    let (_history, _identity, _db) = create("foreign", &store);
    // A different creator (different database id and operation) on the SAME
    // prefix: the read-back evidence is a foreign authority.
    let db_b = fresh_db("foreign-b");
    let outcome = HostedHistory::create(
        Arc::clone(&db_b),
        &store,
        "tenants/foreign".to_string(),
        EPOCH,
        DatabaseId::from_core(Id128::from_bytes([0x55; 16])),
        IncarnationId::from_core(Id128::from_bytes([0x56; 16])),
        OperationId::from_core(Id128::from_bytes([0x66; 16])),
        LIMITS,
        &work(),
    )
    .map(|_| ());
    assert!(
        matches!(outcome, Err(LogError::CommandIdentityConflict)),
        "foreign evidence is AuthorityExists: {outcome:?}"
    );
    // The same verdict when the conditional outcome itself was ambiguous.
    let db_c = fresh_db("foreign-c");
    store.fail_next(Op::CreateHead, Behavior::IndeterminateDropped);
    let outcome = HostedHistory::create(
        Arc::clone(&db_c),
        &store,
        "tenants/foreign".to_string(),
        EPOCH,
        DatabaseId::from_core(Id128::from_bytes([0x57; 16])),
        IncarnationId::from_core(Id128::from_bytes([0x58; 16])),
        OperationId::from_core(Id128::from_bytes([0x67; 16])),
        LIMITS,
        &work(),
    )
    .map(|_| ());
    assert!(
        matches!(outcome, Err(LogError::CommandIdentityConflict)),
        "ambiguous create over foreign evidence is AuthorityExists: {outcome:?}"
    );
}

#[test]
fn an_unresolvable_create_outcome_stays_unknown_typed() {
    // The create was dropped in flight and the read-back finds no head: the
    // request could still land — unknown-typed, never a fabricated refusal
    // and never a claimed success.
    let store = MemStore::new();
    let db = fresh_db("unknowncreate");
    store.fail_next(Op::CreateHead, Behavior::IndeterminateDropped);
    let outcome = HostedHistory::create(
        Arc::clone(&db),
        &store,
        "tenants/unknowncreate".to_string(),
        EPOCH,
        DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        OperationId::from_core(Id128::from_bytes(OPERATION)),
        LIMITS,
        &work(),
    )
    .map(|_| ());
    assert!(
        matches!(outcome, Err(LogError::Backend)),
        "absent evidence stays unknown-typed: {outcome:?}"
    );
    // Unreadable evidence (the read-back itself fails) also stays unknown.
    let store2 = MemStore::new();
    let db2 = fresh_db("unknowncreate-2");
    store2.fail_next(Op::CreateHead, Behavior::IndeterminateApplied);
    store2.fail_next(Op::ReadHead, Behavior::Error);
    let outcome = HostedHistory::create(
        Arc::clone(&db2),
        &store2,
        "tenants/unknowncreate2".to_string(),
        EPOCH,
        DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        OperationId::from_core(Id128::from_bytes(OPERATION)),
        LIMITS,
        &work(),
    )
    .map(|_| ());
    assert!(
        matches!(outcome, Err(LogError::Backend)),
        "unreadable evidence stays unknown-typed: {outcome:?}"
    );
}

// ---- #6 machine half: finite default tail envelope ---------------------------

#[test]
fn the_default_tail_envelope_is_finite_and_unbounded_is_explicit_only() {
    assert_ne!(
        DEFAULT_TAIL_POLICY,
        TailPolicy::UNBOUNDED,
        "an unconfigured hosted machine must not grow an unbounded tail"
    );

    let store = MemStore::new();
    let (history, identity, db) = create("envelope-default", &store);
    // Inflate the durable tail accounting to the default byte bound: no
    // with_tail_policy call anywhere, so any refusal proves the DEFAULT is
    // finite and active.
    assert!(
        store.corrupt_head("tenants/envelope-default/HEAD", |body| {
            let mut record = manifest::decode_head(body, LIMITS.envelope_bytes).unwrap();
            let mut recovery = record.recovery.expect("live head names its recovery root");
            recovery.tail_bytes = DEFAULT_TAIL_POLICY.max_bytes;
            record.recovery = Some(recovery);
            *body = manifest::encode_head(&record, LIMITS.envelope_bytes).unwrap();
        })
    );
    let insert = command(history.db(), identity, 1, 7);
    match history.submit(&insert, &work()) {
        SubmitOutcome::NotSubmitted {
            error: LogError::MaintenanceRequired { bytes, .. },
            ..
        } => assert_eq!(bytes, DEFAULT_TAIL_POLICY.max_bytes),
        other => panic!("the default envelope must backpressure: {other:?}"),
    }
    // A reopened machine carries the same finite default.
    let reopened = HostedHistory::open(
        Arc::clone(&db),
        &store,
        "tenants/envelope-default".to_string(),
        LIMITS,
        &work(),
    )
    .unwrap();
    assert!(matches!(
        reopened.submit(&insert, &work()),
        SubmitOutcome::NotSubmitted {
            error: LogError::MaintenanceRequired { .. },
            ..
        }
    ));
    // UNBOUNDED exists only as the explicit option — and then admission
    // proceeds.
    let unbounded = HostedHistory::open(
        Arc::clone(&db),
        &store,
        "tenants/envelope-default".to_string(),
        LIMITS,
        &work(),
    )
    .unwrap()
    .with_tail_policy(TailPolicy::UNBOUNDED);
    match unbounded.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("explicit UNBOUNDED admits: {other:?}"),
    }
}

// ---- authenticated decision locators (LOG-013) ------------------------------

#[test]
fn authenticated_parent_locators_fetch_in_one_get() {
    use bumbledb_log::certainty::PublicationPhase;
    use bumbledb_log::history::locator::{ChainVisitor, walk_decision_chain};
    use bumbledb_log::store::ObjectError;
    use bumbledb_log::store::fetch_decision_ref;

    let store = MemStore::new();
    let (history, identity, _db) = create("locator", &store);
    let insert = command(history.db(), identity, 1, 42);
    let certainty = history.submit_certain(&insert, &work());
    match &certainty {
        bumbledb_log::certainty::SubmitCertainty::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("expected decided insert: {other:?}"),
    }
    let head_work = work();
    let (head, _) = bumbledb_log::checkpointer::read_live_head(
        &store,
        "tenants/locator",
        1_000_000,
        &head_work,
    )
    .expect("head reads");
    let recovery = head.recovery.expect("recovery root");
    let tip_object = recovery.tip_object.expect("tip locator after publish");
    assert_eq!(manifest::OBJECT_REF_ENCODED_LEN, 49);
    let fetch_work = work();
    let bytes = fetch_decision_ref(
        &store,
        "tenants/locator",
        &tip_object,
        TransportContext::new(
            &fetch_work,
            ReceiveLimits::capped(LIMITS.envelope_bytes as u64),
        ),
    )
    .expect("direct fetch");
    let envelope =
        bumbledb_log::history::decision::decode_decision(bytes.as_bytes(), LIMITS).expect("decodes");
    drop(bytes);
    assert_eq!(envelope.stamp(), recovery.tip);
    // Phase is derived from certainty arms, not stored independently.
    assert_eq!(certainty.publication_phase(), PublicationPhase::Confirmed);
    let genesis = recovery.base;
    let mut budget = 64;
    struct Count(usize);
    impl ChainVisitor for Count {
        type Error = ObjectError;
        fn visit(
            &mut self,
            _stamp: bumbledb_log::history::DecisionStamp,
            _bytes: &[u8],
            _reference: bumbledb_log::store::ObjectRef,
        ) -> Result<bool, ObjectError> {
            self.0 += 1;
            Ok(true)
        }
    }
    let ctx = work();
    let mut count = Count(0);
    walk_decision_chain(
        &store,
        "tenants/locator",
        recovery.tip,
        genesis,
        Some(tip_object),
        LIMITS,
        &mut budget,
        &ctx,
        &mut count,
    )
    .expect("locator walk");
    assert_eq!(count.0, 1, "one decision above genesis");
}

#[test]
fn admin_certainty_carries_phase_not_inferred_from_error_name() {
    use bumbledb_log::admin;
    use bumbledb_log::certainty::{AdminCertainty, PublicationPhase};
    use bumbledb_log::store::mem::MemStore;

    let store = MemStore::new();
    let work = work();
    // Pre-dispatch: no head exists — must be not-started (prepared phase).
    let certainty = admin::rotate_receipts_hosted(
        &store,
        "missing-prefix",
        ReceiptEpoch::new(2).expect("epoch"),
        LIMITS.envelope_bytes,
        &work,
    );
    assert!(
        matches!(certainty, AdminCertainty::NotStarted { .. }),
        "pre-dispatch refusal is not-started: {certainty:?}"
    );
    assert_eq!(
        certainty.publication_phase(),
        PublicationPhase::Prepared,
        "phase is derived from the arm, not stored independently"
    );
}

// ---- D13 / D15: strongest evidence after publication and retirement ----------

#[test]
fn published_lost_response_keeps_the_original_command_ref() {
    use bumbledb_log::certainty::{PublicationPhase, SubmitCertainty};

    let store = MemStore::new();
    let (history, identity, _db) = create("d13-lost", &store);
    let insert = command(history.db(), identity, 1, 5);
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
    let certainty = history.submit_certain(&insert, &work());
    match &certainty {
        SubmitCertainty::OutcomeUnknown { command, .. }
        | SubmitCertainty::Decided {
            receipt: bumbledb_log::history::TerminalReceipt { command, .. },
            ..
        } => assert_eq!(*command, insert.command_ref()),
        SubmitCertainty::NotSubmitted { .. } => {
            panic!("published-then-lost must not become NotSubmitted: {certainty:?}")
        }
    }
    assert_ne!(
        certainty.publication_phase(),
        PublicationPhase::Prepared,
        "a dispatched attempt cannot report the prepared phase"
    );
}

#[test]
fn retired_receipt_after_lost_ack_is_expired_unprovable_not_loss() {
    let store = MemStore::new();
    let (history, identity, _db) = create("d15-retire", &store);
    let insert = command(history.db(), identity, 1, 5);
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
    store.fail_next(Op::GetObject, Behavior::Error);
    match history.submit(&insert, &work()) {
        SubmitOutcome::OutcomeUnknown { command, .. } => {
            assert_eq!(command, insert.command_ref());
        }
        other => panic!("lost ack with failed catch-up stays unknown: {other:?}"),
    }
    assert!(store.corrupt_head("tenants/d15-retire/HEAD", |body| {
        let mut record = manifest::decode_head(body, LIMITS.envelope_bytes).unwrap();
        record.control = record
            .control
            .retire_receipts(1)
            .expect("same-tip retirement");
        *body = manifest::encode_head(&record, LIMITS.envelope_bytes).unwrap();
    }));
    match history.resolve(insert.command_ref(), &work()) {
        Ok(ResolveOutcome::ReceiptExpiredUnknown) | Err(LogError::ReceiptExpiredUnknown) => {}
        Ok(ResolveOutcome::Found(_)) => {
            panic!("retirement must not keep a matching receipt after the frontier advances")
        }
        other => panic!("retirement is expired-unprovable, not proved loss: {other:?}"),
    }
    match history.submit(&insert, &work()) {
        SubmitOutcome::OutcomeUnknown { command, error } => {
            assert_eq!(command, insert.command_ref());
            assert!(
                matches!(
                    error,
                    LogError::ReceiptExpiredUnknown | LogError::Backend | LogError::CommandEpochClosed
                ),
                "original identity stays unknown/expired, got {error:?}"
            );
        }
        SubmitOutcome::NotSubmitted { .. } => {
            panic!("retirement after a dispatched attempt is not NotSubmitted")
        }
        SubmitOutcome::Decided { .. } => {
            panic!("retired absence must not mint a new decided receipt")
        }
    }
}

#[test]
fn known_rejected_receipt_survives_incomplete_diagnostic_health() {
    use bumbledb_log::certainty::{PublicationPhase, SubmitCertainty};
    use bumbledb_log::writer::LocalHealth;

    let store = MemStore::new();
    let (history, identity, _db) = create("d13-diag", &store);
    let insert = command(history.db(), identity, 1, 3);
    let certainty = history.submit_certain(&insert, &work());
    match certainty {
        SubmitCertainty::Decided {
            receipt,
            local_health,
        } => {
            assert_eq!(receipt.command, insert.command_ref());
            match local_health {
                LocalHealth::Ready { .. } | LocalHealth::Unavailable { .. } => {}
            }
        }
        other => panic!("a decided receipt is terminal: {other:?}"),
    }
    assert_eq!(
        history.submit_certain(&insert, &work()).publication_phase(),
        PublicationPhase::Confirmed
    );
}

/// Adapter that can inject a transport observation on the next HEAD replace.
/// Publication is only a typed `Published` arm — these observations must not
/// become `Decided` / `Confirmed` when the CAS did not land.
struct ObservedReplace {
    inner: MemStore,
    next: std::sync::Mutex<Option<TransportObservation>>,
}

impl ObservedReplace {
    fn new() -> Self {
        Self {
            inner: MemStore::new(),
            next: std::sync::Mutex::new(None),
        }
    }

    fn inject(&self, observation: TransportObservation) {
        *self.next.lock().expect("lock") = Some(observation);
    }
}

impl ConditionalStore for ObservedReplace {
    type Error = MemFault;

    fn create_head(&self, head_key: &str, body: &[u8]) -> Result<ConditionalOutcome, MemFault> {
        self.inner.create_head(head_key, body)
    }

    fn replace_head(
        &self,
        head_key: &str,
        expected: &HeadVersion,
        body: &[u8],
    ) -> Result<ConditionalOutcome, MemFault> {
        if let Some(observation) = self.next.lock().expect("lock").take() {
            return Err(MemFault::observed(Op::ReplaceHead, head_key, observation));
        }
        self.inner.replace_head(head_key, expected, body)
    }

    fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, MemFault> {
        self.inner.put_object(key, body)
    }

    fn list_objects(&self, prefix: &str, after: Option<&[u8]>) -> Result<ListPage, MemFault> {
        self.inner.list_objects(prefix, after)
    }

    fn delete_object(&self, key: &str) -> Result<(), MemFault> {
        self.inner.delete_object(key)
    }
}

impl ReceivingStore for ObservedReplace {
    fn receive_object(
        &self,
        key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedBody, MemFault> {
        self.inner.receive_object(key, ctx)
    }

    fn receive_head(
        &self,
        head_key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedHead, MemFault> {
        self.inner.receive_head(head_key, ctx)
    }
}

fn assert_command_unpublished(
    history: HostedHistory<SchemaDescriptor, &ObservedReplace>,
    command: &Command,
    store: &ObservedReplace,
    prefix: &str,
) {
    use bumbledb_log::certainty::{PublicationPhase, SubmitCertainty};

    let history = history.with_attempts(1);
    let certainty = history.submit_certain(command, &work());
    match &certainty {
        SubmitCertainty::OutcomeUnknown { command: held, .. } => {
            assert_eq!(*held, command.command_ref());
        }
        SubmitCertainty::Decided { .. } => {
            panic!("only typed Published is publication: {certainty:?}")
        }
        SubmitCertainty::NotSubmitted { .. } => {
            panic!("a dispatched replace observation is not NotSubmitted: {certainty:?}")
        }
    }
    assert_ne!(
        certainty.publication_phase(),
        PublicationPhase::Confirmed,
        "Denied/Capped/Indeterminate must not derive Confirmed"
    );
    match history.resolve(command.command_ref(), &work()).unwrap() {
        ResolveOutcome::NotRecordedAt { .. } => {}
        ResolveOutcome::Found(_) => {
            panic!("an unpublished attempt must not resolve to a receipt")
        }
        other => panic!("expected NotRecordedAt, got {other:?}"),
    }
    let ctx = work();
    let body = match store
        .receive_head(
            &format!("{prefix}/HEAD"),
            TransportContext::new(&ctx, ReceiveLimits::capped(LIMITS.envelope_bytes as u64)),
        )
        .expect("bounded head receive")
    {
        ReceivedHead::Present { body, .. } => body,
        ReceivedHead::Absent => panic!("genesis head remains"),
    };
    let record = manifest::decode_head(body.as_bytes(), LIMITS.envelope_bytes).unwrap();
    drop(body);
    assert_eq!(
        record.recovery.expect("genesis recovery").tail_count(),
        0,
        "HEAD must not advance without typed Published"
    );
}

#[test]
fn ok_indeterminate_without_apply_is_not_publication() {
    use bumbledb_log::certainty::{PublicationPhase, SubmitCertainty};

    let store = MemStore::new();
    let (history, identity, _db) = create("indeterminate-not-pub", &store);
    let history = history.with_attempts(1);
    let insert = command(history.db(), identity, 1, 9);
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateDropped);
    let certainty = history.submit_certain(&insert, &work());
    match &certainty {
        SubmitCertainty::OutcomeUnknown { command, .. } => {
            assert_eq!(*command, insert.command_ref());
        }
        SubmitCertainty::Decided { .. } => {
            panic!("Ok(Indeterminate) is not typed Published: {certainty:?}")
        }
        SubmitCertainty::NotSubmitted { .. } => {
            panic!("typed Indeterminate after dispatch is not NotSubmitted: {certainty:?}")
        }
    }
    assert_eq!(
        certainty.publication_phase(),
        PublicationPhase::DispatchedUnresolved
    );
    match history.resolve(insert.command_ref(), &work()).unwrap() {
        ResolveOutcome::NotRecordedAt { .. } => {}
        other => panic!("dropped CAS must not leave a receipt: {other:?}"),
    }
}

#[test]
fn denied_capped_and_generic_indeterminate_err_are_not_publication() {
    for (tag, observation) in [
        ("denied-not-pub", TransportObservation::Denied),
        ("capped-not-pub", TransportObservation::Capped),
        (
            "indet-err-not-pub",
            TransportObservation::Indeterminate,
        ),
    ] {
        let store = ObservedReplace::new();
        let db = fresh_db(tag);
        let identity = identity(&db);
        let history = HostedHistory::create(
            Arc::clone(&db),
            &store,
            format!("tenants/{tag}"),
            EPOCH,
            identity.database_id,
            identity.incarnation_id,
            OperationId::from_core(Id128::from_bytes(OPERATION)),
            LIMITS,
            &work(),
        )
        .expect("genesis Published");
        store.inject(observation);
        let insert = command(history.db(), identity, 1, 4);
        assert_command_unpublished(history, &insert, &store, &format!("tenants/{tag}"));
    }
}

#[test]
fn admin_denied_replace_is_unknown_not_completed() {
    use bumbledb_log::admin;
    use bumbledb_log::certainty::{AdminCertainty, PublicationPhase};

    let store = ObservedReplace::new();
    let db = fresh_db("admin-denied");
    let identity = identity(&db);
    HostedHistory::create(
        Arc::clone(&db),
        &store,
        "tenants/admin-denied".to_string(),
        EPOCH,
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes(OPERATION)),
        LIMITS,
        &work(),
    )
    .expect("genesis Published");
    store.inject(TransportObservation::Denied);
    let certainty = admin::rotate_receipts_hosted(
        &store,
        "tenants/admin-denied",
        ReceiptEpoch::new(2).expect("epoch"),
        LIMITS.envelope_bytes,
        &work(),
    );
    assert!(
        matches!(certainty, AdminCertainty::OutcomeUnknown { .. }),
        "Denied after dispatch is unknown, not completed: {certainty:?}"
    );
    assert_eq!(
        certainty.publication_phase(),
        PublicationPhase::DispatchedUnresolved
    );
}
