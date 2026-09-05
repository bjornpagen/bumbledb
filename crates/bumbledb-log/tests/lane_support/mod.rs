//! Shared harness for the P05 backend/retention/recovery lanes.
//!
//! The **hosted mirror** drives real application commands through
//! `LocalHistory` (real LMDB facts/receipts/attachment) and mirrors every
//! decision onto a `ConditionalStore` as the composed hosted layout: one
//! immutable decision object per terminal outcome plus a composed
//! `HeadRecord` CAS. The mirror re-frames each decision from the receipt and
//! the sealed command and CHECKS the recomputed digest against the receipt's
//! stamp — an independent cross-check of the decision framing, not a call
//! into the production hosted writer. Verification: `NotRun` (F1 authors, does
//! not execute).

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bumbledb::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{ChangeSet, Db, ExecutionPolicy, Id128, RelationId, Value, WorkContext};

use bumbledb_log::checkpointer::read_live_head;
use bumbledb_log::history::authority::HeadAuthority;
use bumbledb_log::history::command::{Command, CommandMetadata, Limits, UnverifiedOutcome};
use bumbledb_log::history::decision::{self, DecisionParts};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseId, DatabaseIdentity, DecisionStamp,
    IncarnationId, OperationId, ReceiptEpoch, RequestId, StateStamp, TerminalOutcome,
    TerminalReceipt,
};
use bumbledb_log::manifest::{HeadRecord, TailPolicy, encode_head};
use bumbledb_log::store::{
    BackendError, ConditionalOutcome, ConditionalStore, HeadRead, ObjectKind, head_key,
    put_verified,
};
use bumbledb_log::writer::{LocalHistory, SubmitOutcome};

pub const LIMITS: Limits = Limits {
    envelope_bytes: 1_000_000,
    change_bytes: 900_000,
    evidence_bytes: 10_000,
    result_bytes: 1_000,
};

pub const HEAD_CAP: usize = 1024 * 1024;

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-p05-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// One relation `User(id: u64)`.
pub fn theory() -> SchemaDescriptor {
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

pub fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 100_000_000,
        working_bytes: 100_000_000,
        scratch_bytes: 100_000_000,
        result_bytes: 100_000_000,
        rows: 10_000_000,
        work_units: 1_000_000_000,
        timeout: Duration::from_secs(600),
    }
}

pub fn work() -> WorkContext {
    policy().start().expect("work budget starts")
}

pub fn fresh_db(tag: &str) -> Arc<Db<SchemaDescriptor>> {
    let dir = temp_dir(tag).join("db");
    Arc::new(
        Db::create(&dir, theory(), work())
            .expect("create store")
            .expect("empty store admits"),
    )
}

pub fn test_identity(db: &Db<SchemaDescriptor>) -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(db.schema()),
    }
}

pub fn op(byte: u8) -> OperationId {
    OperationId::from_core(Id128::from_bytes([byte; 16]))
}

/// Seal one command inserting/deleting `User` rows.
pub fn command(
    db: &Db<SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    condition: Condition,
    build: impl FnOnce(&mut bumbledb::ChangeSetBuilder<'_>),
) -> Command {
    let mut draft = ChangeSet::builder(db.schema(), work());
    build(&mut draft);
    let changes = draft.finish().expect("draft finishes");
    let metadata = CommandMetadata {
        identity,
        id: CommandId {
            receipt_epoch: ReceiptEpoch::INITIAL,
            request_id: RequestId::from_core(Id128::from_bytes([request; 16])),
        },
        condition,
    };
    Command::seal(metadata, changes, CommandResult::empty(), LIMITS, &work())
        .expect("command seals")
}

pub fn insert_user(
    db: &Db<SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    id: u64,
) -> Command {
    command(db, identity, request, Condition::Unconditional, |draft| {
        draft
            .insert(RelationId(0), &[Value::U64(id)])
            .expect("insert");
    })
}

pub fn delete_user(
    db: &Db<SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    id: u64,
) -> Command {
    command(db, identity, request, Condition::Unconditional, |draft| {
        draft
            .delete(RelationId(0), &[Value::U64(id)])
            .expect("delete");
    })
}

/// A `LocalHistory` whose every decision is mirrored to one hosted backend as
/// the composed head layout plus verified decision objects.
pub struct Mirror<'b, B> {
    pub history: LocalHistory<SchemaDescriptor>,
    /// A shared handle to the same materialization, for threads that need an
    /// owned `Arc` while the mirror keeps mutating.
    pub db_arc: Arc<Db<SchemaDescriptor>>,
    pub identity: DatabaseIdentity,
    pub backend: &'b B,
    pub prefix: String,
    /// The state stamp before the next decision (for decision framing).
    before_state: StateStamp,
    parent: DecisionStamp,
}

impl<'b, B> Mirror<'b, B>
where
    B: ConditionalStore,
    B::Error: BackendError,
{
    /// Create the local history and install the composed genesis head.
    pub fn create(tag: &str, backend: &'b B, prefix: &str) -> Self {
        let db = fresh_db(tag);
        let identity = test_identity(&db);
        let history = LocalHistory::create(
            Arc::clone(&db),
            identity.database_id,
            identity.incarnation_id,
            op(0xc3),
            LIMITS,
            &work(),
        )
        .expect("local history creates");
        let authority = history.authority().expect("authority reads");
        let position = authority.position().expect("live genesis");
        let record = HeadRecord::genesis(authority, 0).expect("genesis head");
        let body = encode_head(&record, HEAD_CAP).expect("head encodes");
        match backend.create_head(&head_key(prefix), &body) {
            Ok(ConditionalOutcome::Published { .. }) => {}
            other => panic!("genesis head publish: {other:?}"),
        }
        Self {
            history,
            db_arc: db,
            identity,
            backend,
            prefix: prefix.to_string(),
            before_state: position.state,
            parent: position.decision,
        }
    }

    pub fn db(&self) -> &Db<SchemaDescriptor> {
        self.history.db()
    }

    pub fn authority(&self) -> HeadAuthority {
        self.history.authority().expect("authority reads")
    }

    /// Submit locally and mirror the (new) decision to the hosted layout.
    /// Returns the receipt. Retried/known commands mirror nothing.
    pub fn submit(&mut self, command: &Command) -> TerminalReceipt {
        let receipt = match self.history.submit(command, &work()) {
            SubmitOutcome::Decided { receipt, .. } => receipt,
            other => panic!("expected decided, got {other:?}"),
        };
        if receipt.decision_at.seq <= self.parent.seq {
            // A retained retry: nothing new to mirror.
            return receipt;
        }
        assert_eq!(
            receipt.decision_at.seq,
            self.parent.seq + 1,
            "the mirror observes every decision in order"
        );
        // Re-frame the decision independently and check the digest.
        let canonical_command = command.encode(LIMITS).expect("command encodes");
        let result_bytes: Vec<u8> = match &receipt.outcome {
            TerminalOutcome::Committed { result, .. } | TerminalOutcome::NoChange { result } => {
                result.as_bytes().to_vec()
            }
            _ => Vec::new(),
        };
        let evidence_bytes: Vec<u8> = match &receipt.outcome {
            TerminalOutcome::InvariantRejected { evidence } => evidence.as_bytes().to_vec(),
            _ => Vec::new(),
        };
        let outcome = match &receipt.outcome {
            TerminalOutcome::Committed { changed, .. } => UnverifiedOutcome::Committed {
                changed: *changed,
                result: &result_bytes,
            },
            TerminalOutcome::NoChange { .. } => UnverifiedOutcome::NoChange {
                result: &result_bytes,
            },
            TerminalOutcome::PreconditionFailed { expected, observed } => {
                UnverifiedOutcome::PreconditionFailed {
                    expected: *expected,
                    observed: *observed,
                }
            }
            TerminalOutcome::InvariantRejected { .. } => UnverifiedOutcome::InvariantRejected {
                core_evidence: &evidence_bytes,
            },
        };
        let decision_bytes = decision::encode_decision(
            DecisionParts {
                identity: self.identity,
                seq: receipt.decision_at.seq,
                parent: self.parent,
                parent_object: None,
                before_state: self.before_state,
                after_state: receipt.state_at,
                canonical_command: &canonical_command,
                outcome,
            },
            LIMITS,
        )
        .expect("decision frames");
        assert_eq!(
            decision::decision_digest(&decision_bytes),
            receipt.decision_at.hash,
            "independently re-framed decision digest equals the receipt stamp"
        );
        // Publish: object first, then the composed head CAS.
        let (head, version) =
            read_live_head(self.backend, &self.prefix, HEAD_CAP).expect("head reads");
        let decision_ref = put_verified(
            self.backend,
            &self.prefix,
            head.object_epoch,
            ObjectKind::Decision,
            &decision_bytes,
        )
        .expect("decision object stores");
        let new_control = self.authority();
        let proposed = head
            .decided(
                new_control,
                decision_bytes.len() as u64,
                Some(decision_ref),
                &TailPolicy::UNBOUNDED,
            )
            .expect("head composes");
        let body = encode_head(&proposed, HEAD_CAP).expect("head encodes");
        match self
            .backend
            .replace_head(&head_key(&self.prefix), &version, &body)
        {
            Ok(ConditionalOutcome::Published { .. }) => {}
            other => panic!("mirror head publish: {other:?}"),
        }
        self.parent = receipt.decision_at;
        self.before_state = receipt.state_at;
        receipt
    }

    /// The current composed hosted head.
    pub fn head(&self) -> HeadRecord {
        read_live_head(self.backend, &self.prefix, HEAD_CAP)
            .expect("head reads")
            .0
    }
}

/// Read the current hosted head body verbatim.
pub fn raw_head<B>(backend: &B, prefix: &str) -> Vec<u8>
where
    B: ConditionalStore,
    B::Error: BackendError,
{
    match backend.read_head(&head_key(prefix)).expect("head reads") {
        HeadRead::Present { body, .. } => body.into_vec(),
        HeadRead::Absent => panic!("head exists"),
    }
}
