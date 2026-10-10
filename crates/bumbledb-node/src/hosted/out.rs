//! What one machine step hands JavaScript: store requests to perform,
//! settled tickets, and the head after the step.
use std::sync::Arc;

use bumbledb::work::WorkContext;
use bumbledb_log::{
    Bucket, Cache, Done, Evidence, Head, IoBody, IoRequest, MigrationId, Op, Outcome, Receipt,
    Refusal, Replica as _, Settled, Step, Target,
};
use napi_derive::napi;

use crate::marshal::ViolationOut;
use crate::runtime::QueuedBytes;
use crate::schema::{SchemaHandle, hex};

/// The largest evidence the log records.
const EVIDENCE_BYTES: usize = 64 * 1024;

#[napi(string_enum)]
pub enum BucketOut {
    /// The commit bucket (`log/`).
    Log,
    /// Images and migration state (`ckpt/`, `mig/`).
    Checkpoints,
}

/// One store operation. `GetFile` downloads into `path`; `PutFile` uploads
/// the file at `path`; puts are create-only (`If-None-Match: *`).
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum OpOut {
    GetMemory,
    GetFile {
        path: String,
    },
    PutBytes {
        #[napi(ts_type = "Uint8Array")]
        bytes: QueuedBytes,
    },
    PutFile {
        path: String,
    },
    /// Keys under the request key as a prefix, in lexicographic order.
    List {
        start_after: Option<String>,
        max_keys: u32,
    },
    Delete,
}

/// One store request; `key` is relative to the database's prefix.
#[napi(object, object_from_js = false)]
pub struct IoRequestOut {
    pub id: u64,
    pub bucket: BucketOut,
    pub key: String,
    pub op: OpOut,
}

#[napi(object, object_from_js = false)]
pub struct MigrationOut {
    pub name: String,
    pub hash: String,
}

/// The decided state: identity, position, revision, schema fingerprint, the
/// migration ledger, and the store-time end of a freeze.
#[napi(object, object_from_js = false)]
pub struct HeadOut {
    pub database: String,
    pub seq: u64,
    pub revision: u64,
    pub schema: String,
    pub applied: Vec<MigrationOut>,
    pub rejected: Vec<MigrationOut>,
    pub frozen_until: Option<u64>,
}

/// A rejection's evidence, rendered against the schema it was judged under
/// when this code bundles it.
#[napi(object, object_from_js = false)]
pub struct EvidenceOut {
    pub violations: Vec<ViolationOut>,
    #[napi(ts_type = "Uint8Array")]
    pub bytes: QueuedBytes,
}

#[napi(discriminant = "_tag", object_from_js = false)]
pub enum OutcomeOut {
    Committed { added: u64, removed: u64 },
    NoChange,
    PreconditionFailed { expected: u64, observed: u64 },
    InvariantRejected { evidence: EvidenceOut },
}

/// One request's durable decision; `seq` is the read-your-writes bookmark.
#[napi(object, object_from_js = false)]
pub struct ReceiptOut {
    pub request: String,
    pub command: String,
    pub seq: u64,
    pub revision: u64,
    pub outcome: OutcomeOut,
}

/// Why a ticket settled without its effect. Every refusal of a submit except
/// `Unknown` proves the log does not decide the command.
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum RefusalOut {
    NotOpen,
    Closed,
    NotFound,
    Frozen {
        deadline: u64,
    },
    SchemaAdvanced,
    MigrationPending {
        next: u32,
    },
    MigrationRejected {
        migration: MigrationOut,
        evidence: EvidenceOut,
    },
    MigrationsDiverged {
        index: u32,
    },
    NotPending,
    Stale {
        head: u64,
    },
    RequestReused {
        request: String,
        command: String,
    },
    ForeignSchema,
    TooLarge,
    Unknown,
    NotSubmitted,
    Cache {
        message: String,
    },
    Corrupt {
        seq: u64,
    },
}

#[napi(discriminant = "_tag", object_from_js = false)]
pub enum SettledOut {
    Opened { pending: u32 },
    Decided { receipt: ReceiptOut },
    Synced { seq: u64 },
    Resolved { receipt: Option<ReceiptOut> },
    Frozen { seq: u64 },
    Migrated { seq: u64 },
    Refused { refusal: RefusalOut },
}

#[napi(object, object_from_js = false)]
pub struct DoneOut {
    pub ticket: u64,
    pub settled: SettledOut,
}

/// One step: requests to perform now, tickets settled by it, and the head.
#[napi(object, object_from_js = false)]
pub struct StepOut {
    pub io: Vec<IoRequestOut>,
    pub done: Vec<DoneOut>,
    pub head: Option<HeadOut>,
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn path(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

fn bytes(bytes: Vec<u8>) -> QueuedBytes {
    QueuedBytes { bytes }
}

fn migration(id: &MigrationId) -> MigrationOut {
    MigrationOut {
        name: id.name.to_string(),
        hash: hex(&id.hash.0),
    }
}

fn evidence(evidence: &Evidence, schema: Option<&Arc<SchemaHandle>>) -> EvidenceOut {
    let violations = schema
        .and_then(|schema| {
            let decoded =
                bumbledb::schema::evidence::decode(evidence.as_bytes(), EVIDENCE_BYTES).ok()?;
            let violations = decoded
                .to_violations(&schema.schema, &WorkContext::new())
                .ok()?;
            Some(crate::violations_out(&schema.descriptor, &violations))
        })
        .unwrap_or_default();
    EvidenceOut {
        violations,
        bytes: bytes(evidence.as_bytes().to_vec()),
    }
}

fn receipt(receipt: Receipt, schema: Option<&Arc<SchemaHandle>>) -> ReceiptOut {
    ReceiptOut {
        request: hex(&receipt.command.request.0),
        command: hex(&receipt.command.digest.0),
        seq: receipt.seq.get(),
        revision: receipt.revision.0,
        outcome: match receipt.outcome {
            Outcome::Committed(delta) => OutcomeOut::Committed {
                added: delta.added(),
                removed: delta.removed(),
            },
            Outcome::NoChange => OutcomeOut::NoChange,
            Outcome::PreconditionFailed { expected, observed } => OutcomeOut::PreconditionFailed {
                expected: expected.0,
                observed: observed.0,
            },
            Outcome::InvariantRejected(found) => OutcomeOut::InvariantRejected {
                evidence: evidence(&found, schema),
            },
        },
    }
}

fn head(head: &Head) -> HeadOut {
    HeadOut {
        database: hex(&head.database.0),
        seq: head.seq.get(),
        revision: head.revision.0,
        schema: hex(&head.schema.0),
        applied: head.ledger.applied.iter().map(migration).collect(),
        rejected: head
            .ledger
            .rejected
            .iter()
            .map(|rejection| migration(&rejection.migration))
            .collect(),
        frozen_until: head.mode.deadline().map(|deadline| deadline.0),
    }
}

fn request(request: IoRequest) -> IoRequestOut {
    IoRequestOut {
        id: request.id.0,
        bucket: match request.bucket {
            Bucket::Log => BucketOut::Log,
            Bucket::Checkpoints => BucketOut::Checkpoints,
        },
        key: request.key,
        op: match request.op {
            Op::Get(Target::Memory) => OpOut::GetMemory,
            Op::Get(Target::File(file)) => OpOut::GetFile { path: path(&file) },
            Op::PutIfAbsent(IoBody::Bytes(body)) => OpOut::PutBytes { bytes: bytes(body) },
            Op::PutIfAbsent(IoBody::File(file)) => OpOut::PutFile { path: path(&file) },
            Op::List {
                start_after,
                max_keys,
            } => OpOut::List {
                start_after,
                max_keys,
            },
            Op::Delete => OpOut::Delete,
        },
    }
}

/// Convert one step, rendering evidence with the bundled schemas: a
/// decision's against the head's schema, a migration rejection's against
/// the migration's own.
pub(crate) fn step(step: Step, cache: &Cache, schemas: &[Arc<SchemaHandle>]) -> StepOut {
    let steps = cache.bundle().steps();
    let schema_of = |fingerprint: bumbledb::SchemaFingerprint| {
        steps
            .iter()
            .position(|step| step.fingerprint == fingerprint)
            .and_then(|index| schemas.get(index))
    };
    let migration_schema = |id: &MigrationId| {
        steps
            .iter()
            .position(|step| &step.id == id)
            .and_then(|index| schemas.get(index))
    };
    let current = cache.head().and_then(|head| schema_of(head.schema));
    let settled = |settled: Settled| match settled {
        Settled::Opened { pending } => SettledOut::Opened {
            pending: count(pending),
        },
        Settled::Decided(decided) => SettledOut::Decided {
            receipt: receipt(decided, current),
        },
        Settled::Synced(seq) => SettledOut::Synced { seq: seq.get() },
        Settled::Resolved(found) => SettledOut::Resolved {
            receipt: found.map(|found| receipt(found, current)),
        },
        Settled::Frozen(seq) => SettledOut::Frozen { seq: seq.get() },
        Settled::Migrated(seq) => SettledOut::Migrated { seq: seq.get() },
        Settled::Refused(refusal) => SettledOut::Refused {
            refusal: match refusal {
                Refusal::NotOpen => RefusalOut::NotOpen,
                Refusal::Closed => RefusalOut::Closed,
                Refusal::NotFound => RefusalOut::NotFound,
                Refusal::Frozen { deadline } => RefusalOut::Frozen {
                    deadline: deadline.0,
                },
                Refusal::SchemaAdvanced => RefusalOut::SchemaAdvanced,
                Refusal::MigrationPending { next } => {
                    RefusalOut::MigrationPending { next: count(next) }
                }
                Refusal::MigrationRejected(rejection) => RefusalOut::MigrationRejected {
                    evidence: evidence(&rejection.evidence, migration_schema(&rejection.migration)),
                    migration: migration(&rejection.migration),
                },
                Refusal::MigrationsDiverged { index } => RefusalOut::MigrationsDiverged {
                    index: count(index),
                },
                Refusal::NotPending => RefusalOut::NotPending,
                Refusal::Stale { head } => RefusalOut::Stale { head: head.get() },
                Refusal::RequestReused(command) => RefusalOut::RequestReused {
                    request: hex(&command.request.0),
                    command: hex(&command.digest.0),
                },
                Refusal::ForeignSchema => RefusalOut::ForeignSchema,
                Refusal::TooLarge => RefusalOut::TooLarge,
                Refusal::Unknown => RefusalOut::Unknown,
                Refusal::NotSubmitted => RefusalOut::NotSubmitted,
                Refusal::Cache(error) => RefusalOut::Cache {
                    message: error.to_string(),
                },
                Refusal::Corrupt(seq) => RefusalOut::Corrupt { seq: seq.get() },
            },
        },
    };
    StepOut {
        io: step.io.into_iter().map(request).collect(),
        done: step
            .done
            .into_iter()
            .map(
                |Done {
                     ticket,
                     settled: done,
                 }| DoneOut {
                    ticket: ticket.0,
                    settled: settled(done),
                },
            )
            .collect(),
        head: cache.head().map(head),
    }
}
