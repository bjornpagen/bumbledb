//! Shared hosted-history harness for retention and recovery tests.
//! Uses the production writer so every retained decision carries its authenticated
//! parent locator. A local-only history cannot be mirrored by reframing receipts:
//! adding hosted locators changes the decision digest.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{ChangeSet, Db, RelationId, Uuid, Value, WorkContext};

use bumbledb_log::checkpointer::read_live_head;
use bumbledb_log::history::authority::HeadAuthority;
use bumbledb_log::history::command::{Command, CommandMetadata, Limits};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseId, DatabaseIdentity, IncarnationId, OperationId,
    ReceiptEpoch, RequestId, TerminalReceipt,
};
use bumbledb_log::manifest::{HeadRecord, TailPolicy};
use bumbledb_log::store::receive::{
    ObservedError, ReceiveLimits, ReceivedHead, ReceivingStore, TransportContext,
};
use bumbledb_log::store::{BackendError, head_key};
use bumbledb_log::writer::{HostedHistory, SubmitOutcome};

pub const LIMITS: Limits = Limits {
    envelope_bytes: 1_000_000,
    change_bytes: 900_000,
    evidence_bytes: 10_000,
    result_bytes: 1_000,
};

pub const HEAD_CAP: usize = 1024 * 1024;

pub fn completed<T: std::fmt::Debug>(outcome: bumbledb_log::certainty::AdminCertainty<T>) -> T {
    match outcome {
        bumbledb_log::certainty::AdminCertainty::Completed { value } => value,
        other => panic!("administrative transition did not complete: {other:?}"),
    }
}

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

pub fn policy() -> WorkContext {
    WorkContext::new()
}

pub fn work() -> WorkContext {
    policy()
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
        database_id: DatabaseId::from_core(Uuid::from_bytes([0xa1; 16])),
        incarnation_id: IncarnationId::from_core(Uuid::from_bytes([0xb2; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(db.schema()),
    }
}

pub fn op(byte: u8) -> OperationId {
    OperationId::from_core(Uuid::from_bytes([byte; 16]))
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
            request_id: RequestId::from_core(Uuid::from_bytes([request; 16])),
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

/// A hosted history with a shared local materialization.
pub struct Mirror<'b, B> {
    pub history: HostedHistory<SchemaDescriptor, &'b B>,
    pub db_arc: Arc<Db<SchemaDescriptor>>,
    pub identity: DatabaseIdentity,
    pub backend: &'b B,
    pub prefix: String,
}

impl<'b, B> Mirror<'b, B>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    pub fn create(tag: &str, backend: &'b B, prefix: &str) -> Self {
        let db = fresh_db(tag);
        let identity = test_identity(&db);
        let history = HostedHistory::create(
            Arc::clone(&db),
            backend,
            prefix.to_string(),
            0,
            identity.database_id,
            identity.incarnation_id,
            op(0xc3),
            LIMITS,
            &work(),
        )
        .expect("hosted history creates")
        .with_tail_policy(TailPolicy::UNBOUNDED);
        Self {
            history,
            db_arc: db,
            identity,
            backend,
            prefix: prefix.to_string(),
        }
    }

    pub fn db(&self) -> &Db<SchemaDescriptor> {
        self.history.db()
    }

    pub fn authority(&self) -> HeadAuthority {
        bumbledb_log::admin::local_authority(self.db(), LIMITS.envelope_bytes)
            .expect("local authority reads")
    }

    pub fn submit(&mut self, command: &Command) -> TerminalReceipt {
        match self.history.submit(command, &work()) {
            SubmitOutcome::Decided { receipt, .. } => receipt,
            other => panic!("expected decided, got {other:?}"),
        }
    }

    pub fn head(&self) -> HeadRecord {
        read_live_head(self.backend, &self.prefix, HEAD_CAP, &work())
            .expect("head reads")
            .0
    }
}

/// Read the current hosted head body verbatim.
pub fn raw_head<B>(backend: &B, prefix: &str) -> Vec<u8>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    match backend
        .receive_head(
            &head_key(prefix),
            TransportContext::new(&work(), ReceiveLimits::capped(HEAD_CAP as u64)),
        )
        .expect("head reads")
    {
        ReceivedHead::Present { body, .. } => body.as_slice().to_vec(),
        ReceivedHead::Absent => panic!("head exists"),
    }
}
