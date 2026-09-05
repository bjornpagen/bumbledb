//! The `LogNative` roster (C10, `ts-log/src/native.ts`) over the internal Rust
//! machine: history open/call/close (`LocalHistory` in one LMDB transaction,
//! `HostedHistory` over one S3 HEAD), published snapshots wrapping the exact
//! core reader capability, sealed commands over registered core `ChangeSets`,
//! the native-backed typed tenant cache, and the one `logAdmin` verb family
//! (maintenance, retention, backup/restore/erase, migration workflow).
//!
//! Every operation registers in the ONE runtime registry (charged,
//! cancellable, drained at shutdown); take-functions throw the typed
//! `{ source: "core" | "protocol", reason }` error frame; close verbs are
//! join-idempotent. No protocol transition, CAS loop, lock, TTL or timer
//! exists on the JS side — this module IS the one implementation boundary.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bumbledb::canonical::result::{ResultError, decode_result, encode_result};
use bumbledb::work::WorkContext;
use bumbledb::{F64, Id128, SchemaDescriptor, SchemaFingerprint, Value};
use bumbledb_log::history::authority::{Access, HeadAuthority, Lifecycle};
use bumbledb_log::history::command::{Command, CommandMetadata, Limits};
use bumbledb_log::history::{
    CommandDigest, CommandId, CommandRef, CommandResult, Condition, DatabaseId, DatabaseIdentity,
    DecisionStamp, IncarnationId, OperationId, ReceiptEpoch, RequestId, StateStamp,
    TerminalOutcome, TerminalReceipt,
};
use bumbledb_log::recovery::{self, OriginBinding, RecoveryError, RecoveryRefusal};
use bumbledb_log::store::s3::{S3Config, S3Credentials, S3Store};
use bumbledb_log::tenants::{
    Acquire, CompletedOpen, Release, TenantBinding, TenantBorrow, TenantOptions, TenantRefusal,
    TenantRegistry,
};
use bumbledb_log::writer::{
    HostedHistory, LocalHealth, LocalHistory, LogError, ResolveOutcome, SubmitOptions,
    SubmitOutcome,
};
use napi::bindgen_prelude::{BigInt, Buffer, Env, External, Function, Object, Uint8Array, Unknown};
use napi_derive::napi;

use crate::db_wire::{SnapshotHandle, spawn_teardown};
use crate::marshal;
use crate::runtime::owners::{DbLease, DirectoryOwner, ManagedDb};
use crate::runtime::{Output, RetainedNative, Runtime, RuntimeError};
use crate::runtime_wire::{
    CloseWire, OperationHandle, PolicyWire, RuntimeHandle, notification, operation_handle,
    owner as runtime_owner, reason_object, reporter, take_output, thrown,
};

mod admin;

pub(crate) use admin::{AdminOwned, admin_verb};

// ---------------------------------------------------------------------------
// The protocol error-code roster: MUST equal ts-log/src/codes.ts exactly
// (order and spelling) — the authored roster test pins both sides.
// ---------------------------------------------------------------------------

pub(crate) const PROTOCOL_CODES: &[&str] = &[
    "ForeignIdentity",
    "CommandIdentityConflict",
    "DatabaseDeleted",
    "DatabaseFrozen",
    "CommandEpochClosed",
    "ReceiptExpiredUnknown",
    "NotInitialized",
    "DatabaseMissing",
    "AuthorityExists",
    "CacheIdentityMismatch",
    "WrongLineage",
    "NotYetAvailable",
    "WitnessUnavailable",
    "SnapshotExpired",
    "MaintenanceRequired",
    "MaterializationStale",
    "RootCapacityExceeded",
    "SlotBorrowed",
    "Contention",
    "IncompleteRejectionEvidence",
    "MigrationRequired",
    "MigrationDrift",
    "MigrationIntentRequired",
    "MigrationUnsupported",
    "MigrationRepository",
    "DatabaseAhead",
    "MigrationOutputMismatch",
    "OperationConflict",
    "InsufficientLocalDisk",
    "UnsupportedArtifact",
    "Corruption",
    "Backend",
    "Misuse",
];

#[napi]
#[doc(hidden)]
#[must_use]
pub fn log_error_codes() -> Vec<String> {
    PROTOCOL_CODES.iter().map(ToString::to_string).collect()
}

/// The protocol command/frame limits — one bridge-wide envelope,
/// provisional until the F3 format freeze (C12) qualifies deployment caps.
pub(crate) const LIMITS: Limits = Limits {
    envelope_bytes: 4 << 20,
    change_bytes: 64 << 20,
    evidence_bytes: 4 << 20,
    result_bytes: 1 << 20,
};

// ---------------------------------------------------------------------------
// The typed failure that crosses take-functions as `{ source, reason }`.
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum LogFail {
    /// A core failure: the exact `DbError` reason roster, unchanged.
    Core(RuntimeError),
    /// A protocol refusal whose TS reason schema declares only the tag (the
    /// bounded `detail` is diagnostic data for every plain reason, and the
    /// REQUIRED field of the detail-structured reasons `MaterializationStale`/
    /// MigrationDrift/MigrationUnsupported).
    Protocol { code: &'static str, detail: String },
    /// A protocol refusal whose TS `ProtocolReason` schema declares fields
    /// beyond `_tag`/`detail`: the frame renderer carries every declared
    /// field (`BigInt` for bigint-typed fields), or the TS decode refuses the
    /// frame into a core `Internal` ("a malformed reason never forges a
    /// code").
    Structured(StructuredReason),
}

/// The reasons `ts-log/src/errors.ts` declares with fields beyond the tag
/// and detail. Field spellings here MUST match the TS schema exactly; the
/// renderer is the one place they cross the wire.
#[derive(Debug, Clone)]
pub enum StructuredReason {
    /// `{attempts: number}` — bounded attempts consumed before refusing.
    Contention { attempts: u32, detail: String },
    /// `{requestedSeq: bigint, capturedSeq: bigint}`.
    NotYetAvailable {
        requested_seq: u64,
        captured_seq: u64,
        detail: String,
    },
    /// `{requiredBytes: bigint, availableBytes: bigint}` — rostered; no
    /// bridge lane measures local disk yet, but any future emission must
    /// come through this arm so the frame carries its declared fields.
    #[allow(dead_code)] // rostered reason with no native producer yet
    InsufficientLocalDisk {
        required_bytes: u64,
        available_bytes: u64,
        detail: String,
    },
    /// `{count: bigint, bytes: bigint}` — the C08 tail-envelope payload.
    MaintenanceRequired {
        count: u64,
        bytes: u64,
        detail: String,
    },
    /// `{path: string, detail: string}` — produced by the TS generator lane
    /// today; the native arm exists so a native emission is well-formed.
    #[allow(dead_code)] // rostered reason with no native producer yet
    MigrationRepository { path: String, detail: String },
    /// `{requirements: [...], truncated: boolean}` — produced by the TS
    /// generator lane today; the native arm exists so a native emission is
    /// well-formed.
    #[allow(dead_code)] // rostered reason with no native producer yet
    MigrationIntentRequired {
        requirements: Vec<IntentRequirement>,
        truncated: bool,
    },
}

/// One row of `MigrationIntentRequired.requirements`, spelled exactly as the
/// TS schema declares it (`field` is `string | null`, never absent).
#[derive(Debug, Clone)]
#[allow(dead_code)] // constructed only with MigrationIntentRequired (no native producer yet)
pub struct IntentRequirement {
    /// One of the TS literal roster: ambiguous / destructive /
    /// missing-backfill / type-change / unsupported / stale-intent /
    /// conflicting-intent.
    pub(crate) code: &'static str,
    pub(crate) relation: String,
    pub(crate) field: Option<String>,
    pub(crate) detail: String,
}

impl StructuredReason {
    fn code(&self) -> &'static str {
        match self {
            Self::Contention { .. } => "Contention",
            Self::NotYetAvailable { .. } => "NotYetAvailable",
            Self::InsufficientLocalDisk { .. } => "InsufficientLocalDisk",
            Self::MaintenanceRequired { .. } => "MaintenanceRequired",
            Self::MigrationRepository { .. } => "MigrationRepository",
            Self::MigrationIntentRequired { .. } => "MigrationIntentRequired",
        }
    }

    /// Render the reason object with every field the TS schema declares.
    /// bigint-typed TS fields cross as `BigInt`; `detail` rides along as
    /// bounded diagnostic data wherever the schema does not forbid it
    /// (Effect struct decode ignores excess properties).
    fn render<'e>(&self, env: &'e Env) -> napi::Result<Object<'e>> {
        let mut reason = Object::new(env)?;
        reason.set("_tag", self.code())?;
        match self {
            Self::Contention { attempts, detail } => {
                reason.set("attempts", *attempts)?;
                reason.set("detail", detail.as_str())?;
            }
            Self::NotYetAvailable {
                requested_seq,
                captured_seq,
                detail,
            } => {
                reason.set("requestedSeq", BigInt::from(*requested_seq))?;
                reason.set("capturedSeq", BigInt::from(*captured_seq))?;
                reason.set("detail", detail.as_str())?;
            }
            Self::InsufficientLocalDisk {
                required_bytes,
                available_bytes,
                detail,
            } => {
                reason.set("requiredBytes", BigInt::from(*required_bytes))?;
                reason.set("availableBytes", BigInt::from(*available_bytes))?;
                reason.set("detail", detail.as_str())?;
            }
            Self::MaintenanceRequired {
                count,
                bytes,
                detail,
            } => {
                reason.set("count", BigInt::from(*count))?;
                reason.set("bytes", BigInt::from(*bytes))?;
                reason.set("detail", detail.as_str())?;
            }
            Self::MigrationRepository { path, detail } => {
                reason.set("path", path.as_str())?;
                reason.set("detail", detail.as_str())?;
            }
            Self::MigrationIntentRequired {
                requirements,
                truncated,
            } => {
                let mut rows = Vec::with_capacity(requirements.len());
                for requirement in requirements {
                    let mut row = Object::new(env)?;
                    row.set("code", requirement.code)?;
                    row.set("relation", requirement.relation.as_str())?;
                    // `field` is `string | null` in the TS schema — an
                    // absent property would refuse; None crosses as null.
                    row.set("field", requirement.field.clone())?;
                    row.set("detail", requirement.detail.as_str())?;
                    rows.push(row);
                }
                reason.set("requirements", rows)?;
                reason.set("truncated", *truncated)?;
            }
        }
        Ok(reason)
    }
}

impl From<RuntimeError> for LogFail {
    fn from(error: RuntimeError) -> Self {
        Self::Core(error)
    }
}

/// The codes whose TS schema requires fields beyond `_tag`/`detail`; the
/// plain `protocol()` constructor refuses them — they must be emitted
/// through [`StructuredReason`] so the frame is decodable by construction.
const FIELD_STRUCTURED_CODES: &[&str] = &[
    "Contention",
    "NotYetAvailable",
    "InsufficientLocalDisk",
    "MaintenanceRequired",
    "MigrationRepository",
    "MigrationIntentRequired",
];

pub(crate) fn protocol(code: &'static str, detail: impl Into<String>) -> LogFail {
    debug_assert!(PROTOCOL_CODES.contains(&code), "unrostered protocol code");
    debug_assert!(
        !FIELD_STRUCTURED_CODES.contains(&code),
        "structured reason emitted without its declared fields"
    );
    LogFail::Protocol {
        code,
        detail: detail.into(),
    }
}

/// The total `LogError` → frame mapping: protocol arms keep their roster
/// code; core causes cross as the exact core reason (never respelled).
pub(crate) fn fail_of_log(error: LogError) -> LogFail {
    match error {
        LogError::Identity => protocol("ForeignIdentity", "identity/schema mismatch"),
        LogError::CommandIdentityConflict => protocol(
            "CommandIdentityConflict",
            "same command id, different digest",
        ),
        LogError::DatabaseDeleted => protocol("DatabaseDeleted", "terminal tombstone"),
        LogError::DatabaseFrozen => protocol("DatabaseFrozen", "admission frozen"),
        LogError::CommandEpochClosed => protocol("CommandEpochClosed", "epoch closed"),
        LogError::ReceiptExpiredUnknown => {
            protocol("ReceiptExpiredUnknown", "receipt epoch retired")
        }
        LogError::NotInitialized => protocol("NotInitialized", "open never initializes"),
        LogError::Corruption => protocol("Corruption", "malformed or foreign frame"),
        LogError::Work(error) => LogFail::Core(RuntimeError::Work(error)),
        LogError::Core(error) => LogFail::Core(RuntimeError::Engine {
            kind: "command",
            message: format!("{error:?}"),
        }),
        LogError::Storage(error) => LogFail::Core(crate::runtime::session::engine_error(&error)),
        LogError::HostSeal(error) => LogFail::Core(RuntimeError::Engine {
            kind: "hostSeal",
            message: format!("{error:?}"),
        }),
        LogError::Misuse => protocol("Misuse", "reentrant/foreign-schema writer refusal"),
        LogError::IncompleteRejectionEvidence => protocol(
            "IncompleteRejectionEvidence",
            "rejection diagnostic exceeded its budget",
        ),
        LogError::Backend => protocol("Backend", "backend transport failure"),
        // C08 tail-envelope backpressure: the STRUCTURED payload {count,
        // bytes} crosses as BigInt fields (the TS schema requires both).
        LogError::MaintenanceRequired { count, bytes } => {
            LogFail::Structured(StructuredReason::MaintenanceRequired {
                count,
                bytes,
                detail: "the durable tail exceeded its retention envelope (count and bytes); \
                         checkpoint before new admission"
                    .into(),
            })
        }
        // The detail IS the hydration routing: native owns recovery, which
        // runs on the next history open / tenant acquire — the TS layer
        // surfaces the typed refusal and never repairs the cache itself.
        LogError::MaterializationStale => protocol(
            "MaterializationStale",
            "the local materialization predates the durable tail's checkpoint base; close and \
             reopen the history (or re-acquire the tenant) so recovery hydration rebuilds it \
             natively",
        ),
    }
}

pub(crate) fn fail_of_recovery(error: RecoveryError) -> LogFail {
    match error {
        RecoveryError::Refused(RecoveryRefusal::AlreadyOwned) => {
            LogFail::Core(RuntimeError::DirectoryBusy)
        }
        RecoveryError::Refused(RecoveryRefusal::DatabaseMissing) => protocol(
            "DatabaseMissing",
            "no materialization; creation is explicit",
        ),
        RecoveryError::Refused(RecoveryRefusal::DatabaseDeleted) => {
            protocol("DatabaseDeleted", "terminal tombstone")
        }
        RecoveryError::Refused(RecoveryRefusal::ForeignCache { .. }) => protocol(
            "CacheIdentityMismatch",
            "the cache's recorded binding disagrees with the configured origin",
        ),
        RecoveryError::Refused(RecoveryRefusal::UnidentifiedCache) => protocol(
            "CacheIdentityMismatch",
            "the cache carries no binding record",
        ),
        RecoveryError::Corrupt(detail) => protocol("Corruption", detail),
        RecoveryError::Object(error) => protocol("Corruption", format!("{error:?}")),
        RecoveryError::Frame(error) => protocol("Corruption", format!("{error:?}")),
        RecoveryError::Storage(error) => {
            LogFail::Core(crate::runtime::session::engine_error(&error))
        }
        RecoveryError::Host(error) => LogFail::Core(RuntimeError::Engine {
            kind: "hostSeal",
            message: format!("{error:?}"),
        }),
        RecoveryError::Work(error) => LogFail::Core(RuntimeError::Work(error)),
        RecoveryError::Io(error) => LogFail::Core(crate::runtime::owners::io_error(error)),
        RecoveryError::Apply(error) => protocol("Corruption", format!("{error:?}")),
        RecoveryError::Changes(error) => protocol("Corruption", format!("{error:?}")),
        RecoveryError::InvariantViolation => protocol(
            "Corruption",
            "imported state violates the schema's laws; activation refused",
        ),
    }
}

/// The ONE `{ source, reason }` frame renderer — thrown frames and embedded
/// DATA frames spell every reason identically, structured fields included.
fn set_frame_fields(env: Env, frame: &mut Object, fail: &LogFail) -> napi::Result<()> {
    match fail {
        LogFail::Core(error) => {
            frame.set("source", "core")?;
            frame.set("reason", reason_object(&env, error.clone())?)?;
        }
        LogFail::Protocol { code, detail } => {
            frame.set("source", "protocol")?;
            let mut reason = Object::new(&env)?;
            reason.set("_tag", *code)?;
            reason.set("detail", detail.as_str())?;
            frame.set("reason", reason)?;
        }
        LogFail::Structured(structured) => {
            frame.set("source", "protocol")?;
            frame.set("reason", structured.render(&env)?)?;
        }
    }
    Ok(())
}

/// Throw the typed `{ source, reason }` frame from a take-function.
pub(crate) fn throw_frame(env: Env, fail: &LogFail) -> napi::Error {
    let make = |fail: &LogFail| -> napi::Result<()> {
        let mut frame = Object::new(&env)?;
        set_frame_fields(env, &mut frame, fail)?;
        env.throw(frame)
    };
    make(fail)
        .err()
        .unwrap_or_else(|| napi::Error::from_status(napi::Status::PendingException))
}

/// The `{ source, reason }` frame as DATA (embedded in submit/admin arms).
fn frame_object<'e>(env: &'e Env, fail: &LogFail) -> napi::Result<Object<'e>> {
    let mut frame = Object::new(env)?;
    set_frame_fields(*env, &mut frame, fail)?;
    Ok(frame)
}

/// A machine job's domain result: the payload, or the owned typed frame the
/// take-function throws (never a JS exception fabricated off-thread).
pub(crate) type MachineResult<T> = Result<T, LogFail>;

// ---------------------------------------------------------------------------
// Hex / identity marshalling.
// ---------------------------------------------------------------------------

pub(crate) fn hex16(id: Id128) -> String {
    marshal::id128_hex(id)
}

pub(crate) fn hex32(bytes: &[u8; 32]) -> String {
    crate::hex_fingerprint(bytes)
}

pub(crate) fn fingerprint_of_hex(text: &str) -> napi::Result<SchemaFingerprint> {
    if text.len() != 64
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(marshal::err(format!(
            "bumbledb-log marshal: not 64 lowercase hex characters: `{text}`"
        )));
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        let hex = std::str::from_utf8(pair).map_err(|_| marshal::err("hex".into()))?;
        *slot = u8::from_str_radix(hex, 16).map_err(|_| marshal::err("hex".into()))?;
    }
    Ok(SchemaFingerprint(out))
}

pub(crate) fn identity_in(obj: &Object, ctx: &str) -> napi::Result<DatabaseIdentity> {
    Ok(DatabaseIdentity {
        database_id: DatabaseId::from_core(marshal::id128_in(
            &marshal::req::<String>(obj, "databaseId", ctx)?,
            ctx,
        )?),
        incarnation_id: IncarnationId::from_core(marshal::id128_in(
            &marshal::req::<String>(obj, "incarnationId", ctx)?,
            ctx,
        )?),
        schema_id: fingerprint_of_hex(&marshal::req::<String>(obj, "schemaId", ctx)?)?,
    })
}

pub(crate) fn identity_wire(env: &Env, identity: DatabaseIdentity) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env)?;
    obj.set("databaseId", hex16(identity.database_id.as_core()))?;
    obj.set("incarnationId", hex16(identity.incarnation_id.as_core()))?;
    obj.set("schemaId", hex32(&identity.schema_id.0))?;
    Ok(obj)
}

pub(crate) fn stamp_wire(env: &Env, stamp: DecisionStamp) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env)?;
    obj.set("seq", BigInt::from(stamp.seq))?;
    obj.set("hash", hex32(stamp.hash.as_bytes()))?;
    Ok(obj)
}

pub(crate) fn state_wire(env: &Env, state: StateStamp) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env)?;
    obj.set("incarnation", hex16(state.incarnation.as_core()))?;
    obj.set("dataRevision", BigInt::from(state.data_revision))?;
    Ok(obj)
}

pub(crate) fn command_ref_wire<'e>(
    env: &'e Env,
    reference: &CommandRef,
) -> napi::Result<Object<'e>> {
    let mut obj = Object::new(env)?;
    obj.set("identity", identity_wire(env, reference.identity)?)?;
    obj.set(
        "receiptEpoch",
        BigInt::from(reference.id.receipt_epoch.get()),
    )?;
    obj.set("requestId", hex16(reference.id.request_id.as_core()))?;
    obj.set("digest", hex32(reference.digest.as_bytes()))?;
    Ok(obj)
}

pub(crate) fn command_ref_in(obj: &Object, ctx: &str) -> napi::Result<CommandRef> {
    let identity = identity_in(&marshal::req::<Object>(obj, "identity", ctx)?, ctx)?;
    let epoch = marshal::u64_in(&marshal::req::<BigInt>(obj, "receiptEpoch", ctx)?, ctx)?;
    let epoch = ReceiptEpoch::new(epoch)
        .ok_or_else(|| marshal::err(format!("bumbledb-log marshal: {ctx}: receipt epoch 0")))?;
    let request = marshal::id128_in(&marshal::req::<String>(obj, "requestId", ctx)?, ctx)?;
    let digest = fingerprint_of_hex(&marshal::req::<String>(obj, "digest", ctx)?)?;
    Ok(CommandRef {
        identity,
        id: CommandId {
            receipt_epoch: epoch,
            request_id: RequestId::from_core(request),
        },
        digest: CommandDigest::from_bytes(digest.0),
    })
}

// ---------------------------------------------------------------------------
// The declared-result record: the ONE canonical codec is the core authority
// `bumbledb::canonical::result` (family `bumbledb.result.v1\0` layout 1 —
// kind byte, u32 count, u16 name lengths, scalar tags 0/1/2/3/4/5/8). The
// command digest covers these bytes verbatim, so no local byte twin may
// exist (the wave-D C12 defect this re-point closes); this module only
// converts JS values to/from core `Value` scalars. TS never emits tag 8
// (a 32-hex string is deliberately tag 4 — no magic string sniffing); an
// Id128 cell decoded from a Rust-sealed command crosses to JS as its
// canonical 32-lowercase-hex text.
// ---------------------------------------------------------------------------

fn fail_of_result(error: ResultError) -> LogFail {
    match error {
        ResultError::Work(work) => LogFail::Core(RuntimeError::Work(work)),
        // Caller-input refusals at encode (the seal boundary).
        ResultError::NonScalar { .. }
        | ResultError::InvalidName { .. }
        | ResultError::DuplicateName { .. }
        | ResultError::Budget { .. } => protocol("Misuse", format!("declared result: {error:?}")),
        // Strict-decode refusals over stored/wire bytes.
        other => protocol(
            "Corruption",
            format!("malformed declared-result record: {other:?}"),
        ),
    }
}

/// Encode through the core codec — the exact bytes the command digest covers.
pub(crate) fn encode_result_record(
    entries: &[(String, Value)],
    work: &WorkContext,
) -> MachineResult<Vec<u8>> {
    let borrowed: Vec<(&str, &Value)> = entries
        .iter()
        .map(|(name, value)| (name.as_str(), value))
        .collect();
    encode_result(&borrowed, LIMITS.result_bytes, work).map_err(fail_of_result)
}

/// Strict decode through the core codec (tag 8 Id128 included).
pub(crate) fn decode_result_record(
    bytes: &[u8],
    work: &WorkContext,
) -> MachineResult<Vec<(Box<str>, Value)>> {
    decode_result(bytes, LIMITS.result_bytes, work).map_err(fail_of_result)
}

fn result_record_in(obj: &Object, ctx: &str) -> napi::Result<Vec<(String, Value)>> {
    let mut out = Vec::new();
    let names = Object::keys(obj)?;
    for name in names {
        let value: Unknown = marshal::req(obj, &name, ctx)?;
        let cell = result_cell_in(&value, ctx)?;
        out.push((name, cell));
    }
    Ok(out)
}

/// The TS-confirmed `CommandScalar` mapping: boolean→0 Bool; bigint splits
/// at the sign (0 ≤ v < 2^64 → 1 U64, −2^63 ≤ v < 0 → 2 I64, wider refuses);
/// number→3 through the canonical F64 quotient; string→4; Uint8Array→5.
/// TS never emits tag 8.
#[expect(
    unsafe_code,
    reason = "napi declares `Unknown::cast` unsafe; every cast is fenced by the get_type check in its own arm"
)]
fn result_cell_in(value: &Unknown, ctx: &str) -> napi::Result<Value> {
    use napi::ValueType as JsType;
    // SAFETY (each cast): the arm's guard proved the exact JS type.
    match value.get_type()? {
        JsType::Boolean => Ok(Value::Bool(unsafe { value.cast::<bool>()? })),
        JsType::BigInt => {
            let raw = unsafe { value.cast::<BigInt>()? };
            let (negative, word, lossless) = raw.get_u64();
            if !negative && lossless {
                return Ok(Value::U64(word));
            }
            let (word, lossless) = raw.get_i64();
            if !lossless {
                return Err(marshal::err(format!(
                    "bumbledb-log marshal: {ctx}: declared-result bigint out of range"
                )));
            }
            Ok(Value::I64(word))
        }
        JsType::Number => Ok(Value::F64(F64::from(unsafe { value.cast::<f64>()? }))),
        JsType::String => Ok(Value::String(
            unsafe { value.cast::<String>()? }.into_boxed_str(),
        )),
        JsType::Object => {
            let bytes = unsafe { value.cast::<Uint8Array>()? };
            Ok(Value::FixedBytes(bytes.to_vec().into_boxed_slice()))
        }
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: declared results are scalar-only, got {other:?}"
        ))),
    }
}

/// A bounded local budget for take-side result decodes: take-functions run
/// on the JS thread outside any registered job, and the record is already
/// capped by `LIMITS.result_bytes`, so the charge is local by construction.
fn result_take_work() -> Option<WorkContext> {
    let cap = LIMITS.result_bytes as u64;
    bumbledb::work::ExecutionPolicy {
        input_bytes: cap,
        working_bytes: cap,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 1,
        work_units: cap.saturating_mul(4).max(1 << 20),
        timeout: std::time::Duration::from_secs(10),
    }
    .start()
    .ok()
}

fn result_record_wire<'e>(env: &'e Env, bytes: &[u8]) -> napi::Result<Object<'e>> {
    let mut obj = Object::new(env)?;
    // A malformed stored record is surfaced empty rather than forging
    // cells; the receipt itself remains intact evidence.
    let entries = result_take_work()
        .and_then(|work| decode_result_record(bytes, &work).ok())
        .unwrap_or_default();
    for (key, cell) in entries {
        match cell {
            Value::Bool(value) => obj.set(&*key, value)?,
            Value::U64(value) => obj.set(&*key, BigInt::from(value))?,
            Value::I64(value) => obj.set(&*key, BigInt::from(value))?,
            Value::F64(value) => obj.set(&*key, value.to_f64())?,
            Value::String(value) => obj.set(&*key, value.as_ref())?,
            Value::FixedBytes(value) => obj.set(&*key, Buffer::from(value.into_vec()))?,
            // Tag 8 (Id128) exists only in Rust-sealed commands: it crosses
            // to JS as its canonical 32-lowercase-hex text — the only
            // spelling `CommandScalar` carries. Re-sealing such a decoded
            // record from TS respells the cell as tag 4: a NEW command with
            // its own digest, never a mutation of the recorded one.
            Value::Id128(value) => obj.set(&*key, hex16(value))?,
            // The strict core decode never yields intervals (non-scalar
            // tags refuse), so these arms are unreachable data-wise; they
            // render nothing rather than forging a cell.
            Value::IntervalU64(_) | Value::IntervalI64(_) | Value::IntervalF64(_) => {}
        }
    }
    Ok(obj)
}

// ---------------------------------------------------------------------------
// History resources and handles.
// ---------------------------------------------------------------------------

/// One opened history's backend machine (both expose the same command
/// semantics; neither simulates the other).
pub(crate) enum HistoryKind {
    Local(Arc<LocalHistory<SchemaDescriptor>>),
    Hosted {
        history: Arc<HostedHistory<SchemaDescriptor, Arc<S3Store>>>,
        backend: Arc<S3Store>,
        prefix: String,
    },
}

impl HistoryKind {
    fn clone_kind(&self) -> Self {
        match self {
            Self::Local(history) => Self::Local(Arc::clone(history)),
            Self::Hosted {
                history,
                backend,
                prefix,
            } => Self::Hosted {
                history: Arc::clone(history),
                backend: Arc::clone(backend),
                prefix: prefix.clone(),
            },
        }
    }

    /// Submit under the per-call C09 bounds. The hosted machine consumes the
    /// options and clamps them to its OWN attempt/backoff bounds
    /// ([`bumbledb_log::writer::HostedHistory::submit_with`]); the bridge
    /// never re-clamps beyond the wire's type bounds. `LocalHistory` accepts
    /// and ignores them — one LMDB transaction has no CAS loop to bound (the
    /// recorded P04R2 contract, P04.md hub request 5).
    fn submit_with(
        &self,
        command: &Command,
        options: SubmitOptions,
        work: &WorkContext,
    ) -> SubmitOutcome {
        match self {
            Self::Local(history) => {
                let _ = options;
                history.submit(command, work)
            }
            Self::Hosted { history, .. } => history.submit_with(command, options, work),
        }
    }

    fn resolve(
        &self,
        reference: CommandRef,
        work: &WorkContext,
    ) -> Result<ResolveOutcome, LogError> {
        match self {
            Self::Local(history) => history.resolve(reference, work),
            Self::Hosted { history, .. } => history.resolve(reference, work),
        }
    }

    fn identity(&self) -> DatabaseIdentity {
        match self {
            Self::Local(history) => history.identity(),
            Self::Hosted { history, .. } => history.identity(),
        }
    }
}

/// One opened history: registry-held directory owner + managed database +
/// a persistent lease (teardown waits for close), plus the machine.
pub(crate) struct HistoryResource {
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) owner: DirectoryOwner,
    pub(crate) managed: ManagedDb,
    pub(crate) identity: DatabaseIdentity,
    state: Mutex<Option<HistoryState>>,
}

struct HistoryState {
    kind: HistoryKind,
    /// The persistent engine lease: held so managed teardown waits for the
    /// history close, released when the state is spent — never read.
    _lease: DbLease,
    _retained: RetainedNative,
}

impl HistoryResource {
    /// The machine plus a transient engine lease for one registered job; a
    /// closed history refuses `ClosedHandle` before any dispatch.
    fn kind_and_lease(&self) -> Result<(HistoryKind, DbLease), RuntimeError> {
        let state = lock_state(&self.state);
        let Some(state) = state.as_ref() else {
            return Err(RuntimeError::ClosedHandle);
        };
        let lease = self.managed.access()?;
        Ok((state.kind.clone_kind(), lease))
    }

    /// Join-idempotent close: spend the machine state (releasing the
    /// persistent lease), then drain the managed database and the directory
    /// owner in order — the kernel lock releases LAST.
    fn drain(self: &Arc<Self>, report: crate::runtime::Report) {
        {
            let mut state = lock_state(&self.state);
            drop(state.take());
        }
        let shared = Arc::clone(self);
        self.managed.drain(Box::new(move |db_report| {
            let owner_report = db_report;
            shared.owner.drain(Box::new(move |directory_report| {
                report(match owner_report {
                    crate::runtime::CloseReport::Closed => directory_report,
                    other => other,
                });
            }));
        }));
    }
}

fn lock_state(
    state: &Mutex<Option<HistoryState>>,
) -> std::sync::MutexGuard<'_, Option<HistoryState>> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The JS capability: an owner history, or one independently spent cache
/// borrow over the shared owner (release frees ONLY the borrow).
pub struct LogHistoryHandle {
    identity: usize,
    inner: Arc<HistoryResource>,
    borrow: Option<BorrowToken>,
}

struct BorrowToken {
    cache: Arc<CacheShared>,
    borrow: TenantBorrow,
    released: AtomicBool,
}

fn history_handle(handle: &LogHistoryHandle) -> Result<&Arc<HistoryResource>, RuntimeError> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    if let Some(borrow) = &handle.borrow
        && borrow.released.load(Ordering::Acquire)
    {
        return Err(RuntimeError::ClosedHandle);
    }
    Ok(&handle.inner)
}

// ---------------------------------------------------------------------------
// Open (local + hosted), shared by logHistoryOpen and the cache's opens.
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) enum BackendSpec {
    Local,
    Hosted {
        bucket: String,
        prefix: String,
        region: Option<String>,
        credentials: CredentialsSpec,
    },
}

#[derive(Clone)]
pub(crate) enum CredentialsSpec {
    ProviderChain,
    Static {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
}

#[derive(Clone)]
pub(crate) struct OpenSpec {
    pub(crate) create: bool,
    pub(crate) directory: String,
    pub(crate) identity: DatabaseIdentity,
    pub(crate) backend: BackendSpec,
    pub(crate) discard_mismatched: bool,
    /// `(operation, artifact)`: required for create; the artifact is the
    /// checked canonical schema snapshot (`schema_file::render`).
    pub(crate) creation: Option<(OperationId, Vec<u8>)>,
    pub(crate) descriptor: SchemaDescriptor,
    pub(crate) attrs: crate::FieldAttrsTable,
}

/// Optional wire fields cross as ABSENT or NULL interchangeably (the TS
/// machine spells "none" as `null`); both decode to `None`, and any other
/// type must convert or refuse.
#[expect(
    unsafe_code,
    reason = "napi declares `Unknown::cast` unsafe; the cast is fenced by the \
              exact get_type check in its own arm"
)]
pub(crate) fn optional_object<'e>(obj: &Object<'e>, key: &str) -> napi::Result<Option<Object<'e>>> {
    use napi::ValueType as JsType;
    let Some(value) = obj.get::<Unknown>(key)? else {
        return Ok(None);
    };
    match value.get_type()? {
        JsType::Null | JsType::Undefined => Ok(None),
        // SAFETY: the object arm is the only cast and was just type-checked.
        JsType::Object => Ok(Some(unsafe { value.cast::<Object>()? })),
        _ => Err(marshal::err(format!(
            "bumbledb-log marshal: `{key}` must be an object or null"
        ))),
    }
}

#[expect(
    unsafe_code,
    reason = "napi declares `Unknown::cast` unsafe; the cast is fenced by the \
              exact get_type check in its own arm"
)]
pub(crate) fn optional_string(obj: &Object, key: &str) -> napi::Result<Option<String>> {
    use napi::ValueType as JsType;
    let Some(value) = obj.get::<Unknown>(key)? else {
        return Ok(None);
    };
    match value.get_type()? {
        JsType::Null | JsType::Undefined => Ok(None),
        // SAFETY: the string arm is the only cast and was just type-checked.
        JsType::String => Ok(Some(unsafe { value.cast::<String>()? })),
        _ => Err(marshal::err(format!(
            "bumbledb-log marshal: `{key}` must be a string or null"
        ))),
    }
}

/// An optional non-negative integer JS number (absent/null ⇒ `None`); the
/// only bounds applied here are TYPE bounds (finite, integral, `u32` range —
/// the TS wire caps every such field at `0xffffffff` or narrower). Semantic
/// clamping belongs to the consuming machine, never the bridge.
#[expect(
    unsafe_code,
    reason = "napi declares `Unknown::cast` unsafe; the cast is fenced by the \
              exact get_type check in its own arm"
)]
pub(crate) fn optional_number(obj: &Object, key: &str, ctx: &str) -> napi::Result<Option<u32>> {
    use napi::ValueType as JsType;
    let Some(value) = obj.get::<Unknown>(key)? else {
        return Ok(None);
    };
    match value.get_type()? {
        JsType::Null | JsType::Undefined => Ok(None),
        // SAFETY: the number arm is the only cast and was just type-checked.
        JsType::Number => Ok(Some(marshal::ordinal(
            unsafe { value.cast::<f64>()? },
            ctx,
        )?)),
        _ => Err(marshal::err(format!(
            "bumbledb-log marshal: `{key}` must be a number or null"
        ))),
    }
}

pub(crate) fn binding_spec_in(
    obj: &Object,
    ctx: &str,
) -> napi::Result<(String, DatabaseIdentity, BackendSpec)> {
    let kind: String = marshal::req(obj, "kind", ctx)?;
    let directory: String = marshal::req(obj, "directory", ctx)?;
    let identity = identity_in(&marshal::req::<Object>(obj, "identity", ctx)?, ctx)?;
    match kind.as_str() {
        "local" => Ok((directory, identity, BackendSpec::Local)),
        "hosted" => {
            let credentials: Object = marshal::req(obj, "credentials", ctx)?;
            let credentials_kind: String = marshal::req(&credentials, "kind", ctx)?;
            let credentials = match credentials_kind.as_str() {
                "provider-chain" => CredentialsSpec::ProviderChain,
                "static" => CredentialsSpec::Static {
                    access_key_id: marshal::req(&credentials, "accessKeyId", ctx)?,
                    secret_access_key: marshal::req(&credentials, "secretAccessKey", ctx)?,
                    session_token: optional_string(&credentials, "sessionToken")?,
                },
                other => {
                    return Err(marshal::err(format!(
                        "bumbledb-log marshal: {ctx}: unknown credentials kind `{other}`"
                    )));
                }
            };
            Ok((
                directory,
                identity,
                BackendSpec::Hosted {
                    bucket: marshal::req(obj, "bucket", ctx)?,
                    prefix: marshal::req(obj, "prefix", ctx)?,
                    region: optional_string(obj, "region")?,
                    credentials,
                },
            ))
        }
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: unknown binding kind `{other}`"
        ))),
    }
}

pub(crate) fn s3_store(
    bucket: &str,
    region: Option<&str>,
    credentials: &CredentialsSpec,
) -> MachineResult<Arc<S3Store>> {
    let credentials = match credentials {
        CredentialsSpec::ProviderChain => {
            // The provider-chain default rides the environment/instance
            // credentials through a refresh callback resolved per request.
            S3Credentials::Refresh(Arc::new(|| {
                let access = std::env::var("AWS_ACCESS_KEY_ID").map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "AWS_ACCESS_KEY_ID is not configured",
                    )
                })?;
                let secret = std::env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "AWS_SECRET_ACCESS_KEY is not configured",
                    )
                })?;
                Ok(bumbledb_log::store::s3::StaticKeys {
                    access_key_id: access,
                    secret_access_key: secret,
                    session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
                })
            }))
        }
        CredentialsSpec::Static {
            access_key_id,
            secret_access_key,
            session_token,
        } => S3Credentials::Static {
            access_key_id: access_key_id.clone(),
            secret_access_key: secret_access_key.clone(),
            session_token: session_token.clone(),
        },
    };
    let endpoint = std::env::var("BUMBLEDB_S3_ENDPOINT").ok();
    let config = S3Config {
        endpoint,
        region: region.unwrap_or("us-east-1").to_string(),
        bucket: bucket.to_string(),
        credentials,
    };
    S3Store::new(&config)
        .map(Arc::new)
        .map_err(|error| protocol("Backend", error.to_string()))
}

/// Validates the creation artifact: the checked canonical schema snapshot
/// must parse through the ONE native grammar and fingerprint to exactly the
/// created identity's schema. Creation never fabricates applied migration
/// history from an artifact.
fn check_artifact(artifact: &[u8], expected: SchemaFingerprint) -> MachineResult<()> {
    if artifact.is_empty() {
        return Err(protocol(
            "UnsupportedArtifact",
            "creation requires the checked canonical schema snapshot",
        ));
    }
    let text = std::str::from_utf8(artifact)
        .map_err(|_| protocol("UnsupportedArtifact", "artifact is not UTF-8"))?;
    let descriptor = bumbledb_log::schema_file::parse(text)
        .map_err(|error| protocol("UnsupportedArtifact", format!("{error:?}")))?;
    let fingerprint = bumbledb_log::schema_file::schema_id(&descriptor)
        .map_err(|error| protocol("UnsupportedArtifact", error.to_string()))?;
    if fingerprint != expected {
        return Err(protocol(
            "UnsupportedArtifact",
            "artifact schema disagrees with the creation identity",
        ));
    }
    Ok(())
}

/// The whole open/create machine, run INSIDE one registered job: reserve the
/// owner slot, run the recovery machine (its kernel fence installs into the
/// slot), attach the engine to the registry, construct the history machine
/// and verify identity. Every failure releases the reserved slot.
pub(crate) fn open_history(
    runtime: &Arc<Runtime>,
    spec: &OpenSpec,
    context: &WorkContext,
) -> MachineResult<HistoryOpened> {
    context.checkpoint().map_err(RuntimeError::from)?;
    let owner_id = runtime.reserve_owner_slot(spec.directory.len())?;
    match open_history_at(runtime, owner_id, spec, context) {
        Ok(opened) => Ok(opened),
        Err(fail) => {
            runtime.abandon_owner_slot(owner_id);
            Err(fail)
        }
    }
}

pub struct HistoryOpened {
    pub(crate) resource: Arc<HistoryResource>,
    pub(crate) receipt_epoch: u64,
}

impl HistoryOpened {
    fn assemble(
        runtime: &Arc<Runtime>,
        owner: DirectoryOwner,
        managed: ManagedDb,
        kind: HistoryKind,
        lease: DbLease,
        receipt_epoch: u64,
    ) -> MachineResult<Self> {
        let identity = kind.identity();
        let retained = runtime.retain_native(0)?;
        Ok(Self {
            resource: Arc::new(HistoryResource {
                runtime: Arc::clone(runtime),
                owner,
                managed,
                identity,
                state: Mutex::new(Some(HistoryState {
                    kind,
                    _lease: lease,
                    _retained: retained,
                })),
            }),
            receipt_epoch,
        })
    }
}

fn open_history_at(
    runtime: &Arc<Runtime>,
    owner_id: u64,
    spec: &OpenSpec,
    context: &WorkContext,
) -> MachineResult<HistoryOpened> {
    let directory = Path::new(&spec.directory);
    match &spec.backend {
        BackendSpec::Local => open_local(runtime, owner_id, directory, spec, context),
        BackendSpec::Hosted {
            bucket,
            prefix,
            region,
            credentials,
        } => open_hosted(
            runtime,
            owner_id,
            directory,
            spec,
            bucket,
            prefix,
            region.as_deref(),
            credentials,
            context,
        ),
    }
}

fn epoch_of_local(history: &LocalHistory<SchemaDescriptor>) -> MachineResult<u64> {
    let authority = history.authority().map_err(fail_of_log)?;
    Ok(match &authority.lifecycle {
        Lifecycle::Live(live) => live.receipts.open_epoch().get(),
        Lifecycle::Deleted { .. } => 0,
    })
}

fn open_local(
    runtime: &Arc<Runtime>,
    owner_id: u64,
    directory: &Path,
    spec: &OpenSpec,
    context: &WorkContext,
) -> MachineResult<HistoryOpened> {
    if spec.create {
        if let Some(parent) = directory.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| LogFail::Core(crate::runtime::owners::io_error(error)))?;
        }
        std::fs::create_dir_all(directory)
            .map_err(|error| LogFail::Core(crate::runtime::owners::io_error(error)))?;
    }
    let binding = OriginBinding {
        origin: "local".into(),
        prefix: spec.directory.as_str().into(),
        identity: spec.identity,
    };
    if spec.create {
        let (operation, artifact) = spec
            .creation
            .as_ref()
            .ok_or_else(|| protocol("UnsupportedArtifact", "creation options are required"))?;
        check_artifact(artifact, spec.identity.schema_id)?;
        match recovery::create_local(directory, spec.descriptor.clone(), &binding, context) {
            Ok((held, db)) => {
                let owner = runtime.install_owner_lock(owner_id, held)?;
                let sealed = Arc::new(crate::seal(spec.descriptor.clone(), spec.attrs.clone()));
                let inner = crate::DbInner {
                    db: Arc::clone(&db),
                    sealed: Arc::clone(&sealed),
                    writing: AtomicBool::new(false),
                };
                let managed = owner.attach_db(inner)?;
                let lease = managed.access()?;
                let history = LocalHistory::create(
                    db,
                    spec.identity.database_id,
                    spec.identity.incarnation_id,
                    *operation,
                    LIMITS,
                    context,
                )
                .map_err(fail_of_log)?;
                let epoch = epoch_of_local(&history)?;
                HistoryOpened::assemble(
                    runtime,
                    owner,
                    managed,
                    HistoryKind::Local(Arc::new(history)),
                    lease,
                    epoch,
                )
            }
            Err(RecoveryError::Corrupt("materialization already exists")) => {
                // A retry after uncertain creation: validate the stable
                // identity and complete instead of adopting a stranger.
                let opened = open_local_existing(runtime, owner_id, directory, spec, context)?;
                if opened.resource.identity == spec.identity {
                    Ok(opened)
                } else {
                    Err(protocol(
                        "AuthorityExists",
                        "an unrelated database already owns this directory",
                    ))
                }
            }
            Err(error) => Err(fail_of_recovery(error)),
        }
    } else {
        open_local_existing(runtime, owner_id, directory, spec, context)
    }
}

fn open_local_existing(
    runtime: &Arc<Runtime>,
    owner_id: u64,
    directory: &Path,
    spec: &OpenSpec,
    context: &WorkContext,
) -> MachineResult<HistoryOpened> {
    let (held, db) = recovery::open_local(directory, spec.descriptor.clone(), context)
        .map_err(fail_of_recovery)?;
    let owner = runtime.install_owner_lock(owner_id, held)?;
    // Reopen-time owner-scoped root scratch collection (chapter 21 local
    // specialization).
    bumbledb_log::local_roots::clean_roots(&db, directory)
        .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
    let sealed = Arc::new(crate::seal(spec.descriptor.clone(), spec.attrs.clone()));
    let inner = crate::DbInner {
        db: Arc::clone(&db),
        sealed: Arc::clone(&sealed),
        writing: AtomicBool::new(false),
    };
    let managed = owner.attach_db(inner)?;
    let lease = managed.access()?;
    let history = LocalHistory::open(db, LIMITS).map_err(fail_of_log)?;
    let identity = history.identity();
    if identity.database_id != spec.identity.database_id {
        return Err(protocol(
            "ForeignIdentity",
            "the materialization belongs to a different database",
        ));
    }
    if identity.incarnation_id != spec.identity.incarnation_id {
        return Err(protocol(
            "WrongLineage",
            "the materialization is a different incarnation of this database",
        ));
    }
    let epoch = epoch_of_local(&history)?;
    HistoryOpened::assemble(
        runtime,
        owner,
        managed,
        HistoryKind::Local(Arc::new(history)),
        lease,
        epoch,
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
fn open_hosted(
    runtime: &Arc<Runtime>,
    owner_id: u64,
    directory: &Path,
    spec: &OpenSpec,
    bucket: &str,
    prefix: &str,
    region: Option<&str>,
    credentials: &CredentialsSpec,
    context: &WorkContext,
) -> MachineResult<HistoryOpened> {
    let backend = s3_store(bucket, region, credentials)?;
    let origin = format!("s3:{bucket}");
    if spec.create {
        std::fs::create_dir_all(directory)
            .map_err(|error| LogFail::Core(crate::runtime::owners::io_error(error)))?;
        let (operation, artifact) = spec
            .creation
            .as_ref()
            .ok_or_else(|| protocol("UnsupportedArtifact", "creation options are required"))?;
        check_artifact(artifact, spec.identity.schema_id)?;
        let binding = OriginBinding {
            origin: origin.as_str().into(),
            prefix: prefix.into(),
            identity: spec.identity,
        };
        let (held, db) =
            recovery::create_local(directory, spec.descriptor.clone(), &binding, context)
                .map_err(fail_of_recovery)?;
        let owner = runtime.install_owner_lock(owner_id, held)?;
        let sealed = Arc::new(crate::seal(spec.descriptor.clone(), spec.attrs.clone()));
        let inner = crate::DbInner {
            db: Arc::clone(&db),
            sealed: Arc::clone(&sealed),
            writing: AtomicBool::new(false),
        };
        let managed = owner.attach_db(inner)?;
        let lease = managed.access()?;
        let history = HostedHistory::create(
            db,
            Arc::clone(&backend),
            prefix.to_string(),
            1,
            spec.identity.database_id,
            spec.identity.incarnation_id,
            *operation,
            LIMITS,
            context,
        )
        .map_err(|error| match error {
            LogError::CommandIdentityConflict => {
                protocol("AuthorityExists", "a HEAD already exists under this prefix")
            }
            other => fail_of_log(other),
        })?;
        let epoch = 1;
        HistoryOpened::assemble(
            runtime,
            owner,
            managed,
            HistoryKind::Hosted {
                history: Arc::new(history),
                backend,
                prefix: prefix.to_string(),
            },
            lease,
            epoch,
        )
    } else {
        let recovered = match recovery::open_hosted(
            directory,
            spec.descriptor.clone(),
            &backend,
            &origin,
            prefix,
            LIMITS,
            bumbledb_log::codec::StreamLimits::DEFAULT,
            LIMITS.envelope_bytes,
            context,
        ) {
            Ok(recovered) => recovered,
            Err(RecoveryError::Refused(RecoveryRefusal::ForeignCache { .. }))
                if spec.discard_mismatched =>
            {
                // Explicit policy: quarantine the mismatched cache (never
                // delete, never submit its pending work) and rebuild.
                quarantine_cache(directory)?;
                recovery::open_hosted(
                    directory,
                    spec.descriptor.clone(),
                    &backend,
                    &origin,
                    prefix,
                    LIMITS,
                    bumbledb_log::codec::StreamLimits::DEFAULT,
                    LIMITS.envelope_bytes,
                    context,
                )
                .map_err(fail_of_recovery)?
            }
            Err(error) => return Err(fail_of_recovery(error)),
        };
        let recovery::Recovered {
            db,
            head,
            lock: held,
        } = recovered;
        let owner = runtime.install_owner_lock(owner_id, held)?;
        let sealed = Arc::new(crate::seal(spec.descriptor.clone(), spec.attrs.clone()));
        let inner = crate::DbInner {
            db: Arc::clone(&db),
            sealed: Arc::clone(&sealed),
            writing: AtomicBool::new(false),
        };
        let managed = owner.attach_db(inner)?;
        let lease = managed.access()?;
        let history = HostedHistory::open(
            db,
            Arc::clone(&backend),
            prefix.to_string(),
            LIMITS,
            context,
        )
        .map_err(fail_of_log)?;
        let identity = history.identity();
        if identity.database_id != spec.identity.database_id {
            return Err(protocol(
                "ForeignIdentity",
                "the HEAD belongs to a different database",
            ));
        }
        if identity.incarnation_id != spec.identity.incarnation_id {
            return Err(protocol(
                "WrongLineage",
                "the HEAD is a different incarnation of this database",
            ));
        }
        let epoch = match head.control.live() {
            Ok(live) => live.receipts.open_epoch().get(),
            Err(_) => 0,
        };
        HistoryOpened::assemble(
            runtime,
            owner,
            managed,
            HistoryKind::Hosted {
                history: Arc::new(history),
                backend,
                prefix: prefix.to_string(),
            },
            lease,
            epoch,
        )
    }
}

/// Quarantine (never delete) a verified-mismatched cache: rename the
/// materialization aside under the held tenant directory so a rebuild owns a
/// fresh location and the old bytes remain inspectable evidence.
fn quarantine_cache(directory: &Path) -> MachineResult<()> {
    let ready = recovery::materialization_path(directory);
    if !ready.exists() {
        return Ok(());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |age| age.as_millis());
    let aside = directory.join(format!("quarantine-{stamp}"));
    std::fs::rename(&ready, &aside)
        .map_err(|error| LogFail::Core(crate::runtime::owners::io_error(error)))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Machine outputs and the shared submit/take plumbing.
// ---------------------------------------------------------------------------

/// The executor's log-machine payloads — all owned, `Send`.
pub enum MachineOutput {
    History(HistoryOpened),
    Submit(SubmitOwned),
    Resolve(ResolveOutcome),
    Inspect(Box<InspectionOwned>),
    Snapshot(Box<SnapshotOwned>),
    Command(CommandOwned),
    Bytes(Vec<u8>),
    Cache(CacheOpened),
    Borrow(BorrowOwned),
    CacheReport(CacheReportOwned),
    Evicted(crate::runtime::CloseReport),
    Admin(AdminOwned),
}

impl MachineOutput {
    /// Dispatched-mutation evidence (never rewritten into a cancellation).
    pub(crate) fn mutation_evidence(&self) -> bool {
        match self {
            Self::Submit(SubmitOwned::Decided { .. }) => true,
            Self::Admin(owned) => owned.mutation_evidence(),
            _ => false,
        }
    }
}

pub enum SubmitOwned {
    Decided {
        receipt: TerminalReceipt,
        health: LocalHealth,
    },
    NotSubmitted {
        reference: CommandRef,
        fail: LogFail,
    },
    OutcomeUnknown {
        reference: CommandRef,
        fail: LogFail,
    },
}

pub struct InspectionOwned {
    pub identity: DatabaseIdentity,
    pub access: &'static str,
    pub head_revision: u64,
    pub decision: DecisionStamp,
    pub state: StateStamp,
    pub open_epoch: u64,
    pub retired_through: u64,
    pub tail_count: u64,
    pub tail_bytes: u64,
    pub root_count: u32,
    pub root_capacity: u32,
    pub gc: &'static str,
    pub disk_bytes: u64,
    pub working_bytes: u64,
    pub queued: u64,
    pub active: u64,
}

pub struct SnapshotOwned {
    pub(crate) session: Arc<crate::runtime::session::SnapshotSession>,
    pub(crate) sealed: Arc<crate::Sealed>,
    pub(crate) identity: DatabaseIdentity,
    pub(crate) decision: DecisionStamp,
    pub(crate) state: StateStamp,
    pub(crate) freshness: FreshnessOwned,
}

pub(crate) enum FreshnessOwned {
    Cached,
    Latest,
    AtLeast { requested: DecisionStamp },
}

pub struct CommandOwned {
    pub(crate) command: Arc<Command>,
    pub(crate) reference: CommandRef,
}

pub struct CacheOpened {
    pub(crate) shared: Arc<CacheShared>,
}

pub struct BorrowOwned {
    pub(crate) resource: Arc<HistoryResource>,
    pub(crate) receipt_epoch: u64,
    pub(crate) cache: Arc<CacheShared>,
    pub(crate) borrow: TenantBorrow,
}

pub struct CacheReportOwned {
    pub open_count: usize,
    pub opening: usize,
    pub budget_bytes: u64,
    pub max_open: usize,
    pub evictions: u64,
    pub slots: Vec<(String, &'static str, usize, u64)>,
}

/// One refusal frame carried as a job OUTPUT (the take throws it): protocol
/// refusals are DOMAIN data of this machine, not runtime faults, so they
/// must survive the pool's post-work cancellation rewrite untouched.
pub(crate) fn fail_output(fail: LogFail) -> Output {
    match fail {
        LogFail::Core(error) => Output::Machine(MachineOutput::Admin(AdminOwned::Failed {
            fail: LogFail::Core(error),
            dispatched: false,
        })),
        other => Output::Machine(MachineOutput::Admin(AdminOwned::Failed {
            fail: other,
            dispatched: false,
        })),
    }
}

// ---------------------------------------------------------------------------
// logHistoryOpen / logHistoryTake.
// ---------------------------------------------------------------------------

fn open_spec_in(env: Env, request: &Object) -> napi::Result<OpenSpec> {
    let ctx = "history open";
    let mode: String = marshal::req(request, "mode", ctx)?;
    let create = match mode.as_str() {
        "open" => false,
        "create" => true,
        other => {
            return Err(marshal::err(format!(
                "bumbledb-log marshal: unknown open mode `{other}`"
            )));
        }
    };
    let binding: Object = marshal::req(request, "binding", ctx)?;
    let (directory, identity, backend) = binding_spec_in(&binding, ctx)?;
    let discard_mismatched: bool = marshal::req(request, "discardMismatchedCache", ctx)?;
    let creation = match optional_object(request, "creation")? {
        None => None,
        Some(creation) => {
            let operation = OperationId::from_core(marshal::id128_in(
                &marshal::req::<String>(&creation, "operationId", ctx)?,
                ctx,
            )?);
            let artifact: Uint8Array = marshal::req(&creation, "artifact", ctx)?;
            Some((operation, artifact.to_vec()))
        }
    };
    let spec_object: Object = marshal::req(request, "schema", ctx)?;
    let (descriptor, attrs) = match crate::descriptor_of(&spec_object)? {
        Ok(parsed) => parsed,
        Err(
            crate::OpenOutcome::SchemaError(message) | crate::OpenOutcome::NewtypeMismatch(message),
        ) => {
            return Err(marshal::throw_kind_message(
                env,
                crate::tags::error_family::SCHEMA,
                message,
            ));
        }
    };
    Ok(OpenSpec {
        create,
        directory,
        identity,
        backend,
        discard_mismatched,
        creation,
        descriptor,
        attrs,
    })
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_history_open(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    request: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = runtime_owner(handle).map_err(|error| thrown(env, error))?;
    let spec = open_spec_in(env, &request)?;
    let shared = Arc::clone(runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |context| {
                context.input(spec.directory.len() as u64)?;
                Ok(Box::new(move |context| {
                    match open_history(&shared, &spec, context) {
                        Ok(opened) => Ok(Output::Machine(MachineOutput::History(opened))),
                        Err(LogFail::Core(error)) => Err(error),
                        Err(fail) => Ok(fail_output(fail)),
                    }
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

// The brand lifetime is deliberately free of the `&Env` borrow (see
// `admin::admin_wire`): the take verbs hold `Env` by value.
fn history_wire<'e>(
    env: Env,
    resource: Arc<HistoryResource>,
    receipt_epoch: u64,
    borrow: Option<BorrowToken>,
) -> napi::Result<Object<'e>> {
    let identity = resource.identity;
    let mut meta = Object::new(&env)?;
    meta.set("identity", identity_wire(&env, identity)?)?;
    meta.set("receiptEpoch", BigInt::from(receipt_epoch))?;
    let mut wire = Object::new(&env)?;
    wire.set(
        "history",
        External::new(LogHistoryHandle {
            identity: crate::runtime_wire::addon_identity(),
            inner: resource,
            borrow,
        }),
    )?;
    wire.set("meta", meta)?;
    Ok(wire)
}

#[napi]
pub fn log_history_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Object<'_>> {
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::History(opened)) => {
            history_wire(env, opened.resource, opened.receipt_epoch, None)
        }
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            Err(throw_frame(env, &fail))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn log_history_close(
    env: Env,
    handle: &External<LogHistoryHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    if handle.borrow.is_some() {
        // A borrow is released, never owner-closed, through this surface:
        // route to the borrow release so a mistaken close cannot tear down
        // the shared owner.
        return log_borrow_release(env, handle, callback);
    }
    handle.inner.drain(reporter(callback)?);
    Ok(())
}

#[napi]
pub fn log_borrow_release(
    env: Env,
    handle: &External<LogHistoryHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let report = reporter(callback)?;
    let Some(borrow) = &handle.borrow else {
        // Releasing an owner is a misuse; the owner's obligations are
        // untouched (release only frees borrows).
        return Err(thrown(env, RuntimeError::InvalidArgument));
    };
    if borrow.released.swap(true, Ordering::AcqRel) {
        report(crate::runtime::CloseReport::Closed);
        return Ok(());
    }
    let cache = Arc::clone(&borrow.cache);
    let token = borrow.borrow;
    let outcome = {
        let mut registry = cache.lock_registry();
        registry.release(token)
    };
    let _ = matches!(outcome, Release::Released);
    report(crate::runtime::CloseReport::Closed);
    Ok(())
}

// ---------------------------------------------------------------------------
// logHistoryCall / logHistoryResult / logSnapshotClose.
// ---------------------------------------------------------------------------

enum HistoryVerb {
    Submit {
        command: Arc<Command>,
        reference: CommandRef,
        options: SubmitOptions,
    },
    Resolve(CommandRef),
    Inspect,
    Snapshot(ConsistencySpec),
}

/// The pure wire→machine mapping for the C09 per-call submit bounds: absent
/// fields fall back to the machine defaults (`SubmitOptions::DEFAULT`
/// fields); present values cross verbatim — the hosted machine clamps
/// attempts to `1..=configured` and every delay to its `MAX_BACKOFF`, and
/// the bridge NEVER re-clamps beyond the wire's u32 type bounds.
fn submit_options_of(
    attempts: Option<u32>,
    backoff_base_millis: Option<u32>,
    backoff_cap_millis: Option<u32>,
) -> SubmitOptions {
    SubmitOptions {
        attempts,
        backoff_base: backoff_base_millis
            .map(|millis| std::time::Duration::from_millis(u64::from(millis))),
        backoff_cap: backoff_cap_millis
            .map(|millis| std::time::Duration::from_millis(u64::from(millis))),
    }
}

/// Reads the submit arm's wire fields `attempts` / `backoffBaseMillis` /
/// `backoffCapMillis` (JS numbers per `ts-log/src/native.ts`; the TS machine
/// always sends all three, but absent/null decodes to the machine default
/// per the file's optional-field discipline).
fn submit_options_in(request: &Object, ctx: &str) -> napi::Result<SubmitOptions> {
    Ok(submit_options_of(
        optional_number(request, "attempts", ctx)?,
        optional_number(request, "backoffBaseMillis", ctx)?,
        optional_number(request, "backoffCapMillis", ctx)?,
    ))
}

#[derive(Clone, Copy)]
enum ConsistencySpec {
    Cached,
    Latest,
    AtLeast(DecisionStamp),
}

fn consistency_in(obj: &Object, ctx: &str) -> napi::Result<ConsistencySpec> {
    let kind: String = marshal::req(obj, "kind", ctx)?;
    match kind.as_str() {
        "cached" => Ok(ConsistencySpec::Cached),
        "latest" => Ok(ConsistencySpec::Latest),
        "at-least" => {
            let seq = marshal::u64_in(&marshal::req::<BigInt>(obj, "seq", ctx)?, ctx)?;
            let hash = fingerprint_of_hex(&marshal::req::<String>(obj, "hash", ctx)?)?;
            Ok(ConsistencySpec::AtLeast(DecisionStamp {
                seq,
                hash: bumbledb_log::history::DecisionDigest::from_bytes(hash.0),
            }))
        }
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: unknown consistency `{other}`"
        ))),
    }
}

fn history_verb_in(request: &Object) -> napi::Result<HistoryVerb> {
    let ctx = "history call";
    let verb: String = marshal::req(request, "verb", ctx)?;
    match verb.as_str() {
        "submit" => {
            // napi3: an `External` read back OUT of an Object property crosses
            // as `ExternalRef` (napi holds a JS reference for the read; it is
            // dropped here, on the JS thread, before dispatch). Only the owned
            // `Arc<Command>` inside crosses to the job.
            let command: napi::bindgen_prelude::ExternalRef<LogCommandHandle> =
                marshal::req(request, "command", ctx)?;
            let (owned, reference) = command_entry(&command)
                .map_err(|error| marshal::err(format!("bumbledb-log: {error:?}")))?;
            Ok(HistoryVerb::Submit {
                command: owned,
                reference,
                options: submit_options_in(request, ctx)?,
            })
        }
        "resolve" => Ok(HistoryVerb::Resolve(command_ref_in(
            &marshal::req::<Object>(request, "ref", ctx)?,
            ctx,
        )?)),
        "inspect" => Ok(HistoryVerb::Inspect),
        "snapshot" => Ok(HistoryVerb::Snapshot(consistency_in(
            &marshal::req::<Object>(request, "consistency", ctx)?,
            ctx,
        )?)),
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: unknown history verb `{other}`"
        ))),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_history_call(
    env: Env,
    handle: &External<LogHistoryHandle>,
    policy: PolicyWire,
    request: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let resource = Arc::clone(history_handle(handle).map_err(|error| thrown(env, error))?);
    let verb = history_verb_in(&request)?;
    // A cache borrow holds one counted operation lease per in-flight call so
    // eviction/close cannot tear the owner down underneath the job.
    let op_guard = match &handle.borrow {
        None => None,
        Some(borrow) => Some(begin_cache_operation(borrow).map_err(|error| thrown(env, error))?),
    };
    let runtime = Arc::clone(&resource.runtime);
    let job_resource = Arc::clone(&resource);
    let operation = runtime
        .submit_db(
            &resource.managed,
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    let _lease_guard = op_guard;
                    run_history_verb(&job_resource, verb, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

struct CacheOpGuard {
    cache: Arc<CacheShared>,
    lease: bumbledb_log::tenants::OperationLease,
}

impl Drop for CacheOpGuard {
    fn drop(&mut self) {
        let mut registry = self.cache.lock_registry();
        registry.end_operation(self.lease);
    }
}

fn begin_cache_operation(borrow: &BorrowToken) -> Result<CacheOpGuard, RuntimeError> {
    if borrow.released.load(Ordering::Acquire) {
        return Err(RuntimeError::ClosedHandle);
    }
    let mut registry = borrow.cache.lock_registry();
    let Some((lease, _owner)) = registry.begin_operation(borrow.borrow) else {
        return Err(RuntimeError::ClosedHandle);
    };
    drop(registry);
    Ok(CacheOpGuard {
        cache: Arc::clone(&borrow.cache),
        lease,
    })
}

fn run_history_verb(
    resource: &Arc<HistoryResource>,
    verb: HistoryVerb,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    let (kind, lease) = resource.kind_and_lease()?;
    match verb {
        HistoryVerb::Submit {
            command,
            reference,
            options,
        } => {
            let outcome = kind.submit_with(&command, options, context);
            let owned = match outcome {
                SubmitOutcome::Decided {
                    receipt,
                    local_health,
                } => SubmitOwned::Decided {
                    receipt,
                    health: local_health,
                },
                SubmitOutcome::NotSubmitted { command, error } => SubmitOwned::NotSubmitted {
                    reference: command,
                    fail: fail_of_log(error),
                },
                SubmitOutcome::OutcomeUnknown { command, error } => SubmitOwned::OutcomeUnknown {
                    reference: command,
                    fail: fail_of_log(error),
                },
            };
            let _ = reference;
            drop(lease);
            Ok(Output::Machine(MachineOutput::Submit(owned)))
        }
        HistoryVerb::Resolve(reference) => match kind.resolve(reference, context) {
            Ok(outcome) => Ok(Output::Machine(MachineOutput::Resolve(outcome))),
            Err(error) => match fail_of_log(error) {
                LogFail::Core(core) => Err(core),
                fail => Ok(fail_output(fail)),
            },
        },
        HistoryVerb::Inspect => match inspect_history(resource, &kind, &lease, context) {
            Ok(owned) => Ok(Output::Machine(MachineOutput::Inspect(Box::new(owned)))),
            Err(LogFail::Core(core)) => Err(core),
            Err(fail) => Ok(fail_output(fail)),
        },
        HistoryVerb::Snapshot(consistency) => {
            match open_published_snapshot(resource, &kind, lease, consistency, context) {
                Ok(owned) => Ok(Output::Machine(MachineOutput::Snapshot(Box::new(owned)))),
                Err(LogFail::Core(core)) => Err(core),
                Err(fail) => Ok(fail_output(fail)),
            }
        }
    }
}

fn access_tag(authority: &HeadAuthority) -> &'static str {
    match &authority.lifecycle {
        Lifecycle::Live(live) => match live.access {
            Access::Active => "active",
            Access::Frozen { .. } => "frozen",
        },
        Lifecycle::Deleted { .. } => "deleted",
    }
}

fn inspect_history(
    resource: &Arc<HistoryResource>,
    kind: &HistoryKind,
    lease: &DbLease,
    context: &WorkContext,
) -> MachineResult<InspectionOwned> {
    let authority = local_authority_of(kind)?;
    let live = authority.live().ok();
    let (decision, state, open_epoch, retired_through) = match live {
        Some(live) => (
            live.decision,
            live.state,
            live.receipts.open_epoch().get(),
            live.receipts.retired_through(),
        ),
        None => (
            DecisionStamp {
                seq: 0,
                hash: bumbledb_log::history::DecisionDigest::from_bytes([0; 32]),
            },
            StateStamp {
                incarnation: resource.identity.incarnation_id,
                data_revision: 0,
            },
            0,
            0,
        ),
    };
    let (tail_count, tail_bytes, root_count, gc) = match kind {
        HistoryKind::Local(_) => (0, 0, 0, "idle"),
        HistoryKind::Hosted {
            backend, prefix, ..
        } => match bumbledb_log::checkpointer::read_live_head(
            backend.as_ref(),
            prefix,
            LIMITS.envelope_bytes,
        ) {
            Ok((head, _version)) => {
                let (count, bytes) = head
                    .recovery
                    .as_ref()
                    .map_or((0, 0), |root| (root.tail_count(), root.tail_bytes));
                let gc = match head.gc {
                    bumbledb_log::manifest::GcPhase::Idle => "idle",
                    bumbledb_log::manifest::GcPhase::Marking { .. } => "marking",
                    bumbledb_log::manifest::GcPhase::Sweeping { .. } => "sweeping",
                };
                (count, bytes, saturating_u32(head.roots.len()), gc)
            }
            Err(_) => (0, 0, 0, "idle"),
        },
    };
    let report = lease
        .db()
        .integration_store()
        .map_report(context)
        .map_err(|error| {
            LogFail::Core(crate::runtime::session::engine_error(
                &bumbledb::Error::Store(Box::new(error)),
            ))
        })?;
    let (owner_id, database_id) = resource.managed.ids();
    let queued = resource.runtime.database_operations(owner_id, database_id);
    Ok(InspectionOwned {
        identity: authority.identity,
        access: access_tag(&authority),
        head_revision: authority.revision.0,
        decision,
        state,
        open_epoch,
        retired_through,
        tail_count,
        tail_bytes,
        root_count,
        root_capacity: saturating_u32(bumbledb_log::manifest::RootPolicy::DEFAULT.max_roots),
        gc,
        disk_bytes: report.populated_file_bytes,
        working_bytes: report.non_free_page_bytes,
        queued,
        active: 0,
    })
}

fn local_authority_of(kind: &HistoryKind) -> MachineResult<HeadAuthority> {
    match kind {
        HistoryKind::Local(history) => history.authority().map_err(fail_of_log),
        HistoryKind::Hosted { history, .. } => {
            // The local materialization's committed authority: the coherent
            // basis of the published snapshot and inspection stamps.
            bumbledb_log::admin::local_authority(history.db(), LIMITS.envelope_bytes)
                .map_err(|error| protocol("Corruption", format!("{error:?}")))
        }
    }
}

/// One pinned coherent frame plus the provenance decoded from the authority
/// attachment captured INSIDE it (never a racing later commit's).
struct PinnedFrame {
    opened: crate::runtime::session::SessionOpened,
    identity: DatabaseIdentity,
    decision: DecisionStamp,
    state: StateStamp,
}

/// Pin the coherent generation on its owning thread and decode the committed
/// authority attachment read inside the pinned frame.
fn pin_authority_frame(
    resource: &Arc<HistoryResource>,
    lease: DbLease,
) -> MachineResult<PinnedFrame> {
    let opened = match resource
        .runtime
        .spawn_read_session_for(&resource.managed, lease)
    {
        Ok(Output::Session(opened)) => opened,
        Ok(_) => return Err(LogFail::Core(RuntimeError::Internal)),
        Err(error) => return Err(LogFail::Core(error)),
    };
    let control = opened
        .attachment
        .as_deref()
        .ok_or_else(|| protocol("NotInitialized", "no committed authority attachment"))?;
    let authority =
        bumbledb_log::history::authority::decode_control(control, LIMITS.envelope_bytes)
            .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
    let live = authority
        .live()
        .map_err(|_| protocol("DatabaseDeleted", "terminal tombstone"))?;
    let (decision, state) = (live.decision, live.state);
    Ok(PinnedFrame {
        opened,
        identity: authority.identity,
        decision,
        state,
    })
}

/// The post-catch-up freshness judgment (pure): the retaken local stamp must
/// have reached the tip the ONE catch-up attempt reported. Behind it is the
/// typed structured `NotYetAvailable` — never a second attempt or a hidden
/// repair loop; a diverging hash at the caught-up height is `Corruption`
/// (the materialization disagreeing with the chain it just applied).
fn latest_reached(reached: DecisionStamp, retaken: DecisionStamp) -> Result<(), LogFail> {
    if retaken.seq < reached.seq {
        return Err(LogFail::Structured(StructuredReason::NotYetAvailable {
            requested_seq: reached.seq,
            captured_seq: retaken.seq,
            detail: "the local materialization is still behind the verified head after one \
                     catch-up attempt"
                .into(),
        }));
    }
    if retaken.seq == reached.seq && retaken.hash != reached.hash {
        return Err(protocol(
            "Corruption",
            "the retaken local stamp names a different decision at the caught-up height",
        ));
    }
    Ok(())
}

fn open_published_snapshot(
    resource: &Arc<HistoryResource>,
    kind: &HistoryKind,
    lease: DbLease,
    consistency: ConsistencySpec,
    context: &WorkContext,
) -> MachineResult<SnapshotOwned> {
    let mut frame = pin_authority_frame(resource, lease)?;
    let freshness = match consistency {
        ConsistencySpec::Cached => match kind {
            HistoryKind::Local(_) => FreshnessOwned::Latest,
            HistoryKind::Hosted { .. } => FreshnessOwned::Cached,
        },
        ConsistencySpec::Latest => match kind {
            HistoryKind::Local(_) => FreshnessOwned::Latest,
            HistoryKind::Hosted {
                history,
                backend,
                prefix,
            } => {
                let (head, _) = bumbledb_log::checkpointer::read_live_head(
                    backend.as_ref(),
                    prefix,
                    LIMITS.envelope_bytes,
                )
                .map_err(|error| protocol("Backend", format!("{error:?}")))?;
                let tip = head
                    .control
                    .position()
                    .ok_or_else(|| protocol("DatabaseDeleted", "terminal tombstone"))?;
                if tip.decision != frame.decision {
                    // The local materialization is behind the verified head:
                    // ONE read-side catch-up (P04R2's public
                    // `HostedHistory::catch_up`) under THIS operation's
                    // WorkContext, then ONE retake of the pinned frame.
                    // `MaterializationStale` (warm cache older than the
                    // durable tail's checkpoint base) surfaces as its typed
                    // reason — recovery hydration stays native-owned on the
                    // next open; no hidden repair loop exists here.
                    begin_snapshot_teardown(&frame.opened.session);
                    let reached = history.catch_up(context).map_err(fail_of_log)?;
                    let lease = resource.managed.access().map_err(LogFail::Core)?;
                    frame = pin_authority_frame(resource, lease)?;
                    if let Err(fail) = latest_reached(reached, frame.decision) {
                        begin_snapshot_teardown(&frame.opened.session);
                        return Err(fail);
                    }
                }
                FreshnessOwned::Latest
            }
        },
        ConsistencySpec::AtLeast(requested) => {
            if frame.decision.seq < requested.seq {
                begin_snapshot_teardown(&frame.opened.session);
                return Err(LogFail::Structured(StructuredReason::NotYetAvailable {
                    requested_seq: requested.seq,
                    captured_seq: frame.decision.seq,
                    detail: "the requested decision is not yet locally materialized".into(),
                }));
            }
            if frame.decision.seq == requested.seq && frame.decision.hash != requested.hash {
                begin_snapshot_teardown(&frame.opened.session);
                return Err(protocol(
                    "WrongLineage",
                    "the requested stamp names a different decision at this height",
                ));
            }
            FreshnessOwned::AtLeast { requested }
        }
    };
    Ok(SnapshotOwned {
        session: Arc::new(frame.opened.session),
        sealed: frame.opened.sealed,
        identity: frame.identity,
        decision: frame.decision,
        state: frame.state,
        freshness,
    })
}

fn begin_snapshot_teardown(session: &crate::runtime::session::SnapshotSession) {
    session.begin_close();
}

fn violations_wire_placeholder() -> Vec<Object<'static>> {
    // The receipt's rejection evidence is P04's canonical bytes; decoding it
    // into typed Violation rows awaits P01R's canonical Violations codec
    // (recorded cross-lane dependency in implementation/packets/P06.md).
    Vec::new()
}

fn receipt_wire<'e>(env: Env, receipt: &TerminalReceipt) -> napi::Result<Object<'e>> {
    let mut obj = Object::new(&env)?;
    obj.set("command", command_ref_wire(&env, &receipt.command)?)?;
    obj.set("decisionAt", stamp_wire(&env, receipt.decision_at)?)?;
    obj.set("stateAt", state_wire(&env, receipt.state_at)?)?;
    let mut outcome = Object::new(&env)?;
    match &receipt.outcome {
        TerminalOutcome::Committed { changed, result } => {
            outcome.set("kind", "committed")?;
            outcome.set("added", BigInt::from(changed.added()))?;
            outcome.set("removed", BigInt::from(changed.removed()))?;
            outcome.set("result", result_record_wire(&env, result.as_bytes())?)?;
        }
        TerminalOutcome::NoChange { result } => {
            outcome.set("kind", "no-change")?;
            outcome.set("result", result_record_wire(&env, result.as_bytes())?)?;
        }
        TerminalOutcome::PreconditionFailed { expected, observed } => {
            outcome.set("kind", "precondition-failed")?;
            outcome.set("expected", state_wire(&env, *expected)?)?;
            outcome.set("observed", state_wire(&env, *observed)?)?;
        }
        TerminalOutcome::InvariantRejected { .. } => {
            outcome.set("kind", "invariant-rejected")?;
            outcome.set("violations", violations_wire_placeholder())?;
        }
    }
    obj.set("outcome", outcome)?;
    Ok(obj)
}

fn health_wire<'e>(env: &'e Env, health: &LocalHealth) -> napi::Result<Object<'e>> {
    let mut obj = Object::new(env)?;
    match health {
        LocalHealth::Ready { at } => {
            obj.set("kind", "ready")?;
            obj.set("at", stamp_wire(env, *at)?)?;
        }
        LocalHealth::Unavailable { error } => {
            obj.set("kind", "unavailable")?;
            obj.set("error", frame_object(env, &fail_of_log(error.clone()))?)?;
        }
    }
    Ok(obj)
}

pub struct LogSnapshotHandle {
    identity: usize,
    session: Arc<crate::runtime::session::SnapshotSession>,
}

#[napi]
#[allow(clippy::too_many_lines)]
pub fn log_history_result(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Object<'_>> {
    let mut wire = Object::new(&env)?;
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::Submit(owned)) => {
            wire.set("verb", "submit")?;
            let mut outcome = Object::new(&env)?;
            match owned {
                SubmitOwned::Decided { receipt, health } => {
                    outcome.set("kind", "decided")?;
                    outcome.set("receipt", receipt_wire(env, &receipt)?)?;
                    outcome.set("localHealth", health_wire(&env, &health)?)?;
                }
                SubmitOwned::NotSubmitted { reference, fail } => {
                    outcome.set("kind", "not-submitted")?;
                    outcome.set("error", frame_object(&env, &fail)?)?;
                    let _ = reference;
                }
                SubmitOwned::OutcomeUnknown { reference, fail } => {
                    outcome.set("kind", "outcome-unknown")?;
                    outcome.set("error", frame_object(&env, &fail)?)?;
                    let _ = reference;
                }
            }
            wire.set("outcome", outcome)?;
        }
        Output::Machine(MachineOutput::Resolve(outcome)) => {
            wire.set("verb", "resolve")?;
            let mut resolved = Object::new(&env)?;
            match outcome {
                ResolveOutcome::Found(receipt) => {
                    resolved.set("kind", "found")?;
                    resolved.set("receipt", receipt_wire(env, &receipt)?)?;
                }
                ResolveOutcome::NotRecordedAt { decision_at } => {
                    resolved.set("kind", "not-recorded-at")?;
                    resolved.set("decisionAt", stamp_wire(&env, decision_at)?)?;
                }
                ResolveOutcome::CommandEpochClosed => {
                    resolved.set("kind", "command-epoch-closed")?;
                }
                ResolveOutcome::ReceiptExpiredUnknown => {
                    resolved.set("kind", "receipt-expired-unknown")?;
                }
            }
            wire.set("outcome", resolved)?;
        }
        Output::Machine(MachineOutput::Inspect(owned)) => {
            wire.set("verb", "inspect")?;
            let mut inspection = Object::new(&env)?;
            inspection.set("identity", identity_wire(&env, owned.identity)?)?;
            inspection.set("accessMode", owned.access)?;
            inspection.set("headRevision", BigInt::from(owned.head_revision))?;
            inspection.set("decision", stamp_wire(&env, owned.decision)?)?;
            inspection.set("state", state_wire(&env, owned.state)?)?;
            inspection.set("openEpoch", BigInt::from(owned.open_epoch))?;
            inspection.set("retiredThrough", BigInt::from(owned.retired_through))?;
            inspection.set("tailCount", BigInt::from(owned.tail_count))?;
            inspection.set("tailBytes", BigInt::from(owned.tail_bytes))?;
            inspection.set("unknownCount", BigInt::from(0u64))?;
            inspection.set("unknownOldestMillis", Option::<f64>::None)?;
            inspection.set("rootCount", owned.root_count)?;
            inspection.set("rootCapacity", owned.root_capacity)?;
            inspection.set("gc", owned.gc)?;
            inspection.set("lastMaintenanceError", Option::<String>::None)?;
            inspection.set("diskBytes", BigInt::from(owned.disk_bytes))?;
            inspection.set("workingBytes", BigInt::from(owned.working_bytes))?;
            inspection.set("queued", BigInt::from(owned.queued))?;
            inspection.set("active", BigInt::from(owned.active))?;
            wire.set("inspection", inspection)?;
        }
        Output::Machine(MachineOutput::Snapshot(owned)) => {
            wire.set("verb", "snapshot")?;
            wire.set(
                "snapshot",
                External::new(LogSnapshotHandle {
                    identity: crate::runtime_wire::addon_identity(),
                    session: Arc::clone(&owned.session),
                }),
            )?;
            wire.set(
                "core",
                External::new(SnapshotHandle::assemble(
                    Arc::clone(&owned.session),
                    Arc::clone(&owned.sealed),
                )),
            )?;
            let mut provenance = Object::new(&env)?;
            provenance.set("identity", identity_wire(&env, owned.identity)?)?;
            provenance.set("decision", stamp_wire(&env, owned.decision)?)?;
            provenance.set("state", state_wire(&env, owned.state)?)?;
            let mut freshness = Object::new(&env)?;
            match owned.freshness {
                FreshnessOwned::Cached => freshness.set("kind", "cached")?,
                FreshnessOwned::Latest => freshness.set("kind", "latest")?,
                FreshnessOwned::AtLeast { requested } => {
                    freshness.set("kind", "at-least")?;
                    freshness.set("requested", stamp_wire(&env, requested)?)?;
                }
            }
            provenance.set("freshness", freshness)?;
            wire.set("provenance", provenance)?;
        }
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            return Err(throw_frame(env, &fail));
        }
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    }
    Ok(wire)
}

#[napi]
pub fn log_snapshot_close(
    env: Env,
    handle: &External<LogSnapshotHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    handle.session.drain(reporter(callback)?);
    Ok(())
}

// ---------------------------------------------------------------------------
// Command lanes: seal / encode / decode / close.
// ---------------------------------------------------------------------------

pub struct LogCommandHandle {
    identity: usize,
    inner: Arc<Mutex<Option<CommandEntry>>>,
    runtime: Arc<Runtime>,
}

struct CommandEntry {
    command: Arc<Command>,
    reference: CommandRef,
    _retained: RetainedNative,
}

fn command_entry(handle: &LogCommandHandle) -> Result<(Arc<Command>, CommandRef), RuntimeError> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    let slot = handle
        .inner
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = slot.as_ref().ok_or(RuntimeError::ClosedHandle)?;
    Ok((Arc::clone(&entry.command), entry.reference))
}

fn precondition_in(obj: &Object, ctx: &str) -> napi::Result<Condition> {
    let kind: String = marshal::req(obj, "kind", ctx)?;
    match kind.as_str() {
        "blind" => Ok(Condition::Unconditional),
        "exact-state" => Ok(Condition::ExactState(StateStamp {
            incarnation: IncarnationId::from_core(marshal::id128_in(
                &marshal::req::<String>(obj, "incarnation", ctx)?,
                ctx,
            )?),
            data_revision: marshal::u64_in(
                &marshal::req::<BigInt>(obj, "dataRevision", ctx)?,
                ctx,
            )?,
        })),
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: unknown precondition `{other}`"
        ))),
    }
}

/// Seals over the ALREADY-REGISTERED native change: the runtime derives from
/// the change handle (chapter 35: seal "retains the change's captured
/// runtime, never loads a second one").
#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_command_seal(
    env: Env,
    change: &External<crate::db_wire::ChangesHandle>,
    policy: PolicyWire,
    request: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let ctx = "command seal";
    let (change_set, schema, fingerprint, runtime) =
        crate::db_wire::changes_entry(change).map_err(|error| thrown(env, error))?;
    let scope = identity_in(&marshal::req::<Object>(&request, "scope", ctx)?, ctx)?;
    let epoch = marshal::u64_in(&marshal::req::<BigInt>(&request, "receiptEpoch", ctx)?, ctx)?;
    let Some(epoch) = ReceiptEpoch::new(epoch) else {
        return Err(marshal::err("bumbledb-log marshal: receipt epoch 0".into()));
    };
    let request_id = RequestId::from_core(marshal::id128_in(
        &marshal::req::<String>(&request, "requestId", ctx)?,
        ctx,
    )?);
    let condition = precondition_in(&marshal::req::<Object>(&request, "precondition", ctx)?, ctx)?;
    let result = result_record_in(&marshal::req::<Object>(&request, "result", ctx)?, ctx)?;
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |context| {
                context.input(change_set.as_bytes().len() as u64)?;
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    // The scope's schema must be the change's schema —
                    // re-judged natively regardless of the host's claim.
                    if hex32(&scope.schema_id.0) != fingerprint {
                        return Ok(fail_output(protocol(
                            "ForeignIdentity",
                            "the command scope names a different schema",
                        )));
                    }
                    let result_bytes = match encode_result_record(&result, context) {
                        Ok(bytes) => bytes,
                        Err(LogFail::Core(core)) => return Err(core),
                        Err(fail) => return Ok(fail_output(fail)),
                    };
                    let metadata = CommandMetadata {
                        identity: scope,
                        id: CommandId {
                            receipt_epoch: epoch,
                            request_id,
                        },
                        condition,
                    };
                    let _ = &schema;
                    match Command::seal(
                        metadata,
                        change_set.clone(),
                        CommandResult::from_canonical_bytes(result_bytes.into_boxed_slice()),
                        LIMITS,
                        context,
                    ) {
                        Ok(command) => {
                            let reference = command.command_ref();
                            Ok(Output::Machine(MachineOutput::Command(CommandOwned {
                                command: Arc::new(command),
                                reference,
                            })))
                        }
                        Err(error) => match fail_of_command(error) {
                            LogFail::Core(core) => Err(core),
                            fail => Ok(fail_output(fail)),
                        },
                    }
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

fn fail_of_command(error: bumbledb_log::history::command::CommandError) -> LogFail {
    use bumbledb_log::history::command::CommandError;
    match error {
        CommandError::SchemaMismatch => {
            protocol("ForeignIdentity", "the change's schema is not the scope's")
        }
        CommandError::Frame(frame) => protocol("Corruption", format!("{frame:?}")),
        CommandError::Core(core) => LogFail::Core(RuntimeError::Engine {
            kind: "command",
            message: format!("{core:?}"),
        }),
        CommandError::Work(work) => LogFail::Core(RuntimeError::Work(work)),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_command_decode(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    bytes: Unknown,
    schema: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = runtime_owner(handle).map_err(|error| thrown(env, error))?;
    let bytes = crate::runtime_wire::unshared_input(env, bytes, runtime.options.chunk_bytes)?;
    let (descriptor, _attrs) = match crate::descriptor_of(&schema)? {
        Ok(parsed) => parsed,
        Err(
            crate::OpenOutcome::SchemaError(message) | crate::OpenOutcome::NewtypeMismatch(message),
        ) => {
            return Err(marshal::throw_kind_message(
                env,
                crate::tags::error_family::SCHEMA,
                message,
            ));
        }
    };
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |context| {
                context.input(bytes.len() as u64)?;
                let owned = bytes.to_vec();
                Ok(Box::new(move |context| {
                    use bumbledb::schema::ValidateDescriptor as _;
                    context.checkpoint()?;
                    let schema = match descriptor.clone().validate() {
                        Ok(schema) => schema,
                        Err(error) => {
                            return Err(RuntimeError::Engine {
                                kind: crate::tags::error_family::SCHEMA,
                                message: error.to_string(),
                            });
                        }
                    };
                    match Command::parse(&schema, &owned, LIMITS, context) {
                        Ok(command) => {
                            let reference = command.command_ref();
                            Ok(Output::Machine(MachineOutput::Command(CommandOwned {
                                command: Arc::new(command),
                                reference,
                            })))
                        }
                        Err(error) => match fail_of_command(error) {
                            LogFail::Core(core) => Err(core),
                            fail => Ok(fail_output(fail)),
                        },
                    }
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn log_command_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Object<'_>> {
    let runtime = crate::runtime_wire::operation_runtime(handle);
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::Command(owned)) => {
            let retained = runtime
                .retain_native(owned.command.changes().as_bytes().len() as u64)
                .map_err(|error| thrown(env, error))?;
            let mut wire = Object::new(&env)?;
            wire.set(
                "command",
                External::new(LogCommandHandle {
                    identity: crate::runtime_wire::addon_identity(),
                    inner: Arc::new(Mutex::new(Some(CommandEntry {
                        command: owned.command,
                        reference: owned.reference,
                        _retained: retained,
                    }))),
                    runtime,
                }),
            )?;
            wire.set("ref", command_ref_wire(&env, &owned.reference)?)?;
            Ok(wire)
        }
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            Err(throw_frame(env, &fail))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_command_encode(
    env: Env,
    handle: &External<LogCommandHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let (command, _reference) = command_entry(handle).map_err(|error| thrown(env, error))?;
    let runtime = Arc::clone(&handle.runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    match command.encode(LIMITS) {
                        Ok(bytes) => Ok(Output::Machine(MachineOutput::Bytes(bytes))),
                        Err(error) => Ok(fail_output(protocol("Corruption", format!("{error:?}")))),
                    }
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn log_bytes_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Buffer> {
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::Bytes(bytes)) => Ok(Buffer::from(bytes)),
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            Err(throw_frame(env, &fail))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn log_command_close(
    env: Env,
    handle: &External<LogCommandHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let report = reporter(callback)?;
    if let Ok(mut slot) = handle.inner.try_lock() {
        let taken = slot.take();
        drop(slot);
        drop(taken);
        report(crate::runtime::CloseReport::Closed);
        return Ok(());
    }
    let inner = Arc::clone(&handle.inner);
    spawn_teardown(&handle.runtime, report, move || {
        let taken = inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        drop(taken);
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// The tenant cache (chapter 31): TenantRegistry bookkeeping + real opens.
// ---------------------------------------------------------------------------

pub(crate) struct CacheShared {
    runtime: Arc<Runtime>,
    registry: Mutex<TenantRegistry<Arc<HistoryResource>>>,
    descriptor: SchemaDescriptor,
    attrs: crate::FieldAttrsTable,
    expected: Option<(SchemaFingerprint, [u8; 32])>,
    budget_bytes: u64,
    max_open: usize,
    evictions: std::sync::atomic::AtomicU64,
    closing: AtomicBool,
    retained: Mutex<Option<RetainedNative>>,
}

impl CacheShared {
    pub(crate) fn lock_registry(
        &self,
    ) -> std::sync::MutexGuard<'_, TenantRegistry<Arc<HistoryResource>>> {
        self.registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

pub struct LogCacheHandle {
    identity: usize,
    shared: Arc<CacheShared>,
}

fn cache_shared(handle: &LogCacheHandle) -> Result<&Arc<CacheShared>, RuntimeError> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    Ok(&handle.shared)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_cache_make(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    request: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let ctx = "cache make";
    let runtime = runtime_owner(handle).map_err(|error| thrown(env, error))?;
    let max_open = marshal::ordinal(marshal::req::<f64>(&request, "maxOpen", ctx)?, ctx)? as usize;
    let budget_bytes =
        marshal::u64_in(&marshal::req::<BigInt>(&request, "budgetBytes", ctx)?, ctx)?;
    let expected = match optional_object(&request, "expected")? {
        None => None,
        Some(expected) => {
            let schema_id =
                fingerprint_of_hex(&marshal::req::<String>(&expected, "schemaId", ctx)?)?;
            let prefix = fingerprint_of_hex(&marshal::req::<String>(
                &expected,
                "appliedPrefixDigest",
                ctx,
            )?)?;
            Some((schema_id, prefix.0))
        }
    };
    let spec_object: Object = marshal::req(&request, "schema", ctx)?;
    let (descriptor, attrs) = match crate::descriptor_of(&spec_object)? {
        Ok(parsed) => parsed,
        Err(
            crate::OpenOutcome::SchemaError(message) | crate::OpenOutcome::NewtypeMismatch(message),
        ) => {
            return Err(marshal::throw_kind_message(
                env,
                crate::tags::error_family::SCHEMA,
                message,
            ));
        }
    };
    let shared = Arc::clone(runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    if max_open == 0 {
                        return Err(RuntimeError::InvalidArgument);
                    }
                    let retained = shared.retain_native(0)?;
                    Ok(Output::Machine(MachineOutput::Cache(CacheOpened {
                        shared: Arc::new(CacheShared {
                            runtime: Arc::clone(&shared),
                            registry: Mutex::new(TenantRegistry::new(TenantOptions { max_open })),
                            descriptor,
                            attrs,
                            expected,
                            budget_bytes,
                            max_open,
                            evictions: std::sync::atomic::AtomicU64::new(0),
                            closing: AtomicBool::new(false),
                            retained: Mutex::new(Some(retained)),
                        }),
                    })))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn log_cache_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<LogCacheHandle>> {
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::Cache(opened)) => Ok(External::new(LogCacheHandle {
            identity: crate::runtime_wire::addon_identity(),
            shared: opened.shared,
        })),
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            Err(throw_frame(env, &fail))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

fn tenant_binding_of(
    identity: DatabaseIdentity,
    backend: &BackendSpec,
    directory: &str,
) -> TenantBinding {
    let location = match backend {
        BackendSpec::Local => format!("local:{directory}"),
        BackendSpec::Hosted { bucket, prefix, .. } => format!("s3:{bucket}/{prefix}"),
    };
    TenantBinding {
        identity,
        layout: 1,
        location: location.into(),
    }
}

/// The applied migration prefix of one opened database, recomputed from its
/// authoritative chain records — for the cache's `RuntimeExpectation` check.
fn applied_prefix_of(
    resource: &HistoryResource,
    context: &WorkContext,
) -> MachineResult<Option<[u8; 32]>> {
    use bumbledb_log::migration::history::{HistoryRecord, decode_record, history_key};
    use bumbledb_log::migration::manifest::{
        ManifestEntry, base_prefix_digest, next_prefix_digest,
    };
    let lease = resource.managed.access()?;
    let cap = LIMITS.envelope_bytes;
    let mut records: Vec<HistoryRecord> = Vec::new();
    let mut index = 0u64;
    loop {
        context.checkpoint().map_err(RuntimeError::from)?;
        let key = history_key(index);
        let mut found: Option<Vec<u8>> = None;
        let mut host_error = None;
        lease
            .db()
            .read(|read| {
                match read.integration_host_record(&key) {
                    Ok(record) => found = record.map(<[u8]>::to_vec),
                    Err(error) => host_error = Some(error),
                }
                Ok(())
            })
            .map_err(|error| LogFail::Core(crate::runtime::session::engine_error(&error)))?;
        if let Some(error) = host_error {
            return Err(LogFail::Core(RuntimeError::Engine {
                kind: "hostSeal",
                message: format!("{error:?}"),
            }));
        }
        let Some(bytes) = found else { break };
        let record = decode_record(&bytes, cap)
            .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
        records.push(record);
        index += 1;
    }
    if records.is_empty() {
        return Ok(None);
    }
    let mut prefix: Option<[u8; 32]> = None;
    for record in &records {
        match record {
            HistoryRecord::Baseline(baseline) => {
                prefix = Some(baseline.validated_prefix);
            }
            HistoryRecord::Applied(applied) => {
                let mut current = if let Some(current) = prefix {
                    current
                } else {
                    let base = match &applied.source {
                        bumbledb_log::migration::history::AppliedSource::EmptyBase {
                            base_schema,
                        } => *base_schema,
                        bumbledb_log::migration::history::AppliedSource::Database { .. } => applied
                            .steps
                            .first()
                            .map(|step| step.from_schema)
                            .ok_or_else(|| {
                                protocol("Corruption", "applied record with no steps")
                            })?,
                    };
                    base_prefix_digest(&base, cap)
                        .map_err(|error| protocol("Corruption", format!("{error:?}")))?
                };
                for step in &applied.steps {
                    let entry = ManifestEntry {
                        sequence: step.sequence,
                        label: step.label.clone(),
                        from_schema: step.from_schema,
                        to_schema: step.to_schema,
                        plan_digest: step.plan_digest,
                        prefix_digest: [0; 32],
                    };
                    current = next_prefix_digest(&current, &entry, cap)
                        .map_err(|error| protocol("Corruption", format!("{error:?}")))?;
                }
                prefix = Some(current);
            }
        }
    }
    Ok(prefix)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_cache_acquire(
    env: Env,
    handle: &External<LogCacheHandle>,
    policy: PolicyWire,
    request: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let ctx = "cache acquire";
    let shared = Arc::clone(cache_shared(handle).map_err(|error| thrown(env, error))?);
    let binding: Object = marshal::req(&request, "binding", ctx)?;
    let (directory, identity, backend) = binding_spec_in(&binding, ctx)?;
    let runtime = Arc::clone(&shared.runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |context| {
                context.input(directory.len() as u64)?;
                Ok(Box::new(move |context| {
                    match acquire_borrow(&shared, &directory, identity, &backend, context) {
                        Ok(owned) => Ok(Output::Machine(MachineOutput::Borrow(owned))),
                        Err(LogFail::Core(core)) => Err(core),
                        Err(fail) => Ok(fail_output(fail)),
                    }
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[allow(clippy::too_many_lines)]
fn acquire_borrow(
    shared: &Arc<CacheShared>,
    directory: &str,
    identity: DatabaseIdentity,
    backend: &BackendSpec,
    context: &WorkContext,
) -> MachineResult<BorrowOwned> {
    if shared.closing.load(Ordering::Acquire) {
        return Err(LogFail::Core(RuntimeError::ClosedHandle));
    }
    if let Some((expected_schema, _)) = &shared.expected
        && identity.schema_id != *expected_schema
    {
        return Err(protocol(
            "MigrationRequired",
            "the binding's schema is not the deployment's expected schema",
        ));
    }
    let binding = tenant_binding_of(identity, backend, directory);
    loop {
        context.checkpoint().map_err(RuntimeError::from)?;
        let step = {
            let mut registry = shared.lock_registry();
            registry.acquire(&binding)
        };
        match step {
            Acquire::Ready(borrow) => {
                let resource = {
                    let mut registry = shared.lock_registry();
                    match registry.begin_operation(borrow) {
                        Some((lease, owner)) => {
                            let resource = Arc::clone(owner);
                            registry.end_operation(lease);
                            resource
                        }
                        None => return Err(LogFail::Core(RuntimeError::ClosedHandle)),
                    }
                };
                let epoch = current_epoch(&resource)?;
                return Ok(BorrowOwned {
                    resource,
                    receipt_epoch: epoch,
                    cache: Arc::clone(shared),
                    borrow,
                });
            }
            Acquire::Joined { .. } => {
                // Join the one in-flight open: bounded poll under this
                // operation's own deadline; the opener installs or fails.
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            Acquire::Open(ticket) => {
                let spec = OpenSpec {
                    create: false,
                    directory: directory.to_string(),
                    identity,
                    backend: backend.clone(),
                    discard_mismatched: false,
                    creation: None,
                    descriptor: shared.descriptor.clone(),
                    attrs: shared.attrs.clone(),
                };
                match open_history(&shared.runtime, &spec, context) {
                    Ok(opened) => {
                        if let Some((_, expected_prefix)) = &shared.expected {
                            let applied = applied_prefix_of(&opened.resource, context)?;
                            let matches = applied.is_some_and(|prefix| prefix == *expected_prefix);
                            if !matches {
                                let resource = opened.resource;
                                {
                                    let mut registry = shared.lock_registry();
                                    registry.fail_open(ticket);
                                }
                                teardown_resource(&resource);
                                return Err(protocol(
                                    "MigrationRequired",
                                    "the database's applied migration prefix is not the \
                                     deployment's expected prefix",
                                ));
                            }
                        }
                        let epoch = opened.receipt_epoch;
                        let resource = opened.resource;
                        let mut registry = shared.lock_registry();
                        match registry.complete_open(ticket, Arc::clone(&resource)) {
                            CompletedOpen::Installed(borrow) => {
                                drop(registry);
                                return Ok(BorrowOwned {
                                    resource,
                                    receipt_epoch: epoch,
                                    cache: Arc::clone(shared),
                                    borrow,
                                });
                            }
                            CompletedOpen::ClosedDuringOpen(owner) => {
                                drop(registry);
                                teardown_resource(&owner);
                                return Err(LogFail::Core(RuntimeError::ClosedHandle));
                            }
                        }
                    }
                    Err(fail) => {
                        let mut registry = shared.lock_registry();
                        registry.fail_open(ticket);
                        drop(registry);
                        return Err(fail);
                    }
                }
            }
            Acquire::Refused(refusal) => {
                return Err(match refusal {
                    TenantRefusal::BindingMismatch => protocol(
                        "CacheIdentityMismatch",
                        "the slot's recorded binding is not byte-identical",
                    ),
                    TenantRefusal::Capacity => {
                        // Pressure: evict idle slots, then retry once.
                        let evicted = {
                            let mut registry = shared.lock_registry();
                            registry.evict_idle(shared.max_open.saturating_sub(1))
                        };
                        if evicted.is_empty() {
                            // One acquisition attempt was consumed; the TS
                            // schema's `attempts` is that bounded count.
                            LogFail::Structured(StructuredReason::Contention {
                                attempts: 1,
                                detail: "every cache slot is borrowed or leased".into(),
                            })
                        } else {
                            for (_, owner) in evicted {
                                shared.evictions.fetch_add(1, Ordering::Relaxed);
                                teardown_resource(&owner);
                            }
                            continue;
                        }
                    }
                    TenantRefusal::Closing => LogFail::Structured(StructuredReason::Contention {
                        attempts: 1,
                        detail: "the slot is closing".into(),
                    }),
                    TenantRefusal::Faulted => protocol(
                        "Corruption",
                        "the slot faulted; close and reopen explicitly",
                    ),
                });
            }
        }
    }
}

fn current_epoch(resource: &Arc<HistoryResource>) -> MachineResult<u64> {
    let state = lock_state(&resource.state);
    let Some(state) = state.as_ref() else {
        return Err(LogFail::Core(RuntimeError::ClosedHandle));
    };
    match &state.kind {
        HistoryKind::Local(history) => epoch_of_local(history),
        HistoryKind::Hosted { history, .. } => {
            match bumbledb_log::admin::local_authority(history.db(), LIMITS.envelope_bytes) {
                Ok(authority) => Ok(match authority.live() {
                    Ok(live) => live.receipts.open_epoch().get(),
                    Err(_) => 0,
                }),
                Err(error) => Err(protocol("Corruption", format!("{error:?}"))),
            }
        }
    }
}

/// Begin native teardown of one cache-owned history (evicted / failed /
/// closed): the registry handed the owner OUT; the runtime's cleanup lanes
/// join the drains under their own accounting.
fn teardown_resource(resource: &Arc<HistoryResource>) {
    {
        let mut state = lock_state(&resource.state);
        drop(state.take());
    }
    resource.managed.begin_close();
    resource.owner.begin_close();
}

#[napi]
pub fn log_borrow_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Object<'_>> {
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::Borrow(owned)) => history_wire(
            env,
            owned.resource,
            owned.receipt_epoch,
            Some(BorrowToken {
                cache: owned.cache,
                borrow: owned.borrow,
                released: AtomicBool::new(false),
            }),
        ),
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            Err(throw_frame(env, &fail))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_cache_inspect(
    env: Env,
    handle: &External<LogCacheHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let shared = Arc::clone(cache_shared(handle).map_err(|error| thrown(env, error))?);
    let runtime = Arc::clone(&shared.runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    let report = {
                        let registry = shared.lock_registry();
                        registry.report()
                    };
                    let opening = report.iter().filter(|slot| slot.state == "opening").count();
                    let slots = report
                        .into_iter()
                        .map(|slot| {
                            (
                                slot.binding.location.to_string(),
                                slot.state,
                                slot.borrows,
                                0u64,
                            )
                        })
                        .collect();
                    Ok(Output::Machine(MachineOutput::CacheReport(
                        CacheReportOwned {
                            open_count: {
                                let registry = shared.lock_registry();
                                registry.open_count()
                            },
                            opening,
                            budget_bytes: shared.budget_bytes,
                            max_open: shared.max_open,
                            evictions: shared.evictions.load(Ordering::Relaxed),
                            slots,
                        },
                    )))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn log_cache_inspect_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Object<'_>> {
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::CacheReport(owned)) => {
            let mut wire = Object::new(&env)?;
            wire.set("openCount", saturating_u32(owned.open_count))?;
            wire.set("opening", saturating_u32(owned.opening))?;
            wire.set("budgetBytes", BigInt::from(owned.budget_bytes))?;
            wire.set("maxOpen", saturating_u32(owned.max_open))?;
            wire.set("evictions", BigInt::from(owned.evictions))?;
            let mut slots = Vec::with_capacity(owned.slots.len());
            for (binding, state, borrows, disk) in owned.slots {
                let mut slot = Object::new(&env)?;
                slot.set("binding", binding)?;
                slot.set("state", state)?;
                slot.set("borrows", saturating_u32(borrows))?;
                slot.set("diskBytes", BigInt::from(disk))?;
                slots.push(slot);
            }
            wire.set("slots", slots)?;
            Ok(wire)
        }
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            Err(throw_frame(env, &fail))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_cache_evict(
    env: Env,
    handle: &External<LogCacheHandle>,
    policy: PolicyWire,
    request: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let ctx = "cache evict";
    let shared = Arc::clone(cache_shared(handle).map_err(|error| thrown(env, error))?);
    let binding: Object = marshal::req(&request, "binding", ctx)?;
    let (directory, identity, backend) = binding_spec_in(&binding, ctx)?;
    let runtime = Arc::clone(&shared.runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    let binding = tenant_binding_of(identity, &backend, &directory);
                    let owner = {
                        let mut registry = shared.lock_registry();
                        match registry.counts(&binding) {
                            None => None,
                            Some((borrows, leases)) if borrows > 0 || leases > 0 => {
                                return Ok(fail_output(protocol(
                                    "SlotBorrowed",
                                    "eviction never revokes a live borrow or operation",
                                )));
                            }
                            Some(_) => {
                                registry.begin_close(&binding);
                                registry.finish_close(&binding).ok()
                            }
                        }
                    };
                    if let Some(owner) = owner {
                        shared.evictions.fetch_add(1, Ordering::Relaxed);
                        teardown_resource(&owner);
                    }
                    Ok(Output::Machine(MachineOutput::Evicted(
                        crate::runtime::CloseReport::Closed,
                    )))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn log_cache_evict_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<CloseWire> {
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::Evicted(report)) => Ok(report.into()),
        Output::Machine(MachineOutput::Admin(AdminOwned::Failed { fail, .. })) => {
            Err(throw_frame(env, &fail))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn log_cache_close(
    env: Env,
    handle: &External<LogCacheHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    let shared = cache_shared(handle).map_err(|error| thrown(env, error))?;
    let report = reporter(callback)?;
    shared.closing.store(true, Ordering::Release);
    let inner = Arc::clone(shared);
    spawn_teardown(&shared.runtime, report, move || {
        // Revoke every idle capability and hand the owners out; slots with
        // in-flight operation leases drain through the runtime's own
        // operation accounting (their jobs hold the owner Arcs).
        let owners = {
            let mut registry = inner.lock_registry();
            let bindings: Vec<_> = registry
                .report()
                .into_iter()
                .map(|slot| slot.binding)
                .collect();
            let mut owners = Vec::new();
            for binding in bindings {
                registry.begin_close(&binding);
                if let Ok(owner) = registry.finish_close(&binding) {
                    owners.push(owner);
                }
            }
            owners
        };
        for owner in owners {
            teardown_resource(&owner);
        }
        let taken = inner
            .retained
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        drop(taken);
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// logAdmin / logAdminTake (implementation in `admin.rs`).
// ---------------------------------------------------------------------------

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_admin(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    request: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    admin_verb(env, handle, policy, &request, callback)
}

#[napi]
pub fn log_admin_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Object<'_>> {
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::Admin(owned)) => admin::admin_wire(env, owned),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

/// Diagnostic counts cross as u32 wire numbers; a count past `u32::MAX` (a
/// diagnostic, never a budget) saturates instead of truncating.
fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// The stable target-namespace root for migration targets under one tenant
/// directory (the executor's `targets_root`).
pub(crate) fn targets_root(directory: &str) -> PathBuf {
    Path::new(directory).join("targets")
}

/// The deterministic planned target incarnation for one migration
/// operation: a stable ref exists BEFORE dispatch and a retry of the same
/// operation resumes the same target (provisional domain until C12).
pub(crate) fn planned_target_incarnation(operation: OperationId) -> IncarnationId {
    // Domain-prefixed engine hash (the addon carries no second hash
    // implementation); the derivation is provisional until C12.
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(b"bumbledb.migration.v1/target-incarnation\0");
    digest.update(operation.as_core().as_bytes());
    let word = digest.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&word[..16]);
    IncarnationId::from_core(Id128::from_bytes(bytes))
}

#[cfg(test)]
mod tests;
