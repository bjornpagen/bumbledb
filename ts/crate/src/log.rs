//! The bumbledb-log grammar bridge onto the SUCCESSOR history machine.
//!
//! One implementation reads and writes the protocol's bytes —
//! `crates/bumbledb-log/src/history` (C06) — and this module only carries
//! payloads across: sealed canonical commands, retained receipt rows,
//! immutable decision/genesis records and the head-authority control
//! projection. Bytes in, plain tagged payloads out, grammar only — no
//! store verb, no fd, no clock crosses (the conditional-store verbs are
//! C07/P05 and ride `runtime_fs`/the TS transport). The deleted 0.x
//! braids/codec/manifest/sidecar/ckpt-scratch lanes are gone whole.
//!
//! Refusal identities cross exactly as `bumbledb_log::identities` spells
//! them: the [`frame_kind`]/[`receipt_kind`]/[`chain_kind`]/
//! [`command_kind`] spellers are exhaustive matches, so a new core
//! variant fails compile HERE, and `log_identities()` hands TypeScript
//! the complete emitted table (the committed `log-identities.json` golden
//! is pinned to `identities::emit()` by an authored test).
//!
//! Small fixed-size frames (receipt rows, authority control, genesis)
//! marshal synchronously; command/decision lanes embed whole canonical
//! change payloads and charged hashing, so they run on the ONE bounded
//! executor (`runtime_log_*` verbs + `runtime_log_take`).

use std::sync::Arc;

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{Id128, Schema, SchemaFingerprint, WorkContext};
use bumbledb_log::history::authority::{
    Access, Activation, ActivationCause, DeletedReason, FreezeIntent, HeadAuthority, Lifecycle,
    LiveAuthority, decode_control, encode_control,
};
use bumbledb_log::history::command::{Command, CommandError, CommandMetadata, Limits};
use bumbledb_log::history::decision::{
    ChainError, DecisionParts, GenesisProvenance, GenesisRecord, blank_initial_digests,
    decode_decision, decode_genesis, encode_decision, encode_genesis, genesis_stamp, verify_step,
};
use bumbledb_log::history::receipt::{
    ReceiptRowError, decode_receipt_row, decode_receipt_row_at, encode_receipt_row, receipt_key,
};
use bumbledb_log::history::{
    ChangeSummary, CommandDigest, CommandId, CommandRef, CommandResult, Condition, DatabaseId,
    DatabaseIdentity, DecisionDigest, DecisionStamp, FrameError, HeadRevision, IncarnationId,
    OperationId, ReceiptEpoch, ReceiptPolicy, RejectionEvidence, RequestId, StateStamp,
    TerminalOutcome, TerminalReceipt,
};
use napi::bindgen_prelude::{BigInt, Buffer, Env, External, Function, Object, Uint8Array, Unknown};
use napi::sys;
use napi_derive::napi;

use crate::marshal;
use crate::runtime::{Output, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, PolicyWire, RuntimeHandle, notification, operation_handle, owner, take_output,
    thrown, unshared_input,
};

// ---------------------------------------------------------------------------
// Refusal spellers: exhaustive over the core enums, spelled exactly as
// `bumbledb_log::identities` spells them (the one speller). A new core
// variant refuses to compile here.
// ---------------------------------------------------------------------------

pub(crate) fn frame_kind(error: &FrameError) -> &'static str {
    match error {
        FrameError::LimitExceeded => "limitExceeded",
        FrameError::LengthOverflow => "lengthOverflow",
        FrameError::Allocation => "allocation",
        FrameError::Truncated { .. } => "truncated",
        FrameError::Family => "family",
        FrameError::Layout { .. } => "layout",
        FrameError::Kind { .. } => "kind",
        FrameError::Tag { .. } => "tag",
        FrameError::InvalidEpoch => "invalidEpoch",
        FrameError::StateIdentityMismatch => "stateIdentityMismatch",
        FrameError::EmptyChangeSummary => "emptyChangeSummary",
        FrameError::EmptyEvidence => "emptyEvidence",
        FrameError::InvalidTerminalStamp => "invalidTerminalStamp",
        FrameError::InvalidPreconditionEvidence => "invalidPreconditionEvidence",
        FrameError::InvalidPolicy => "invalidPolicy",
        FrameError::InvalidSequence => "invalidSequence",
        FrameError::InvalidCount => "invalidCount",
        FrameError::TrailingBytes { .. } => "trailingBytes",
    }
}

/// Command-lane refusal kinds: the frame family plus the two command-only
/// arms. `Work` propagates as the operation's own typed work error, never
/// a domain refusal.
fn command_kind(error: &CommandError) -> Result<&'static str, RuntimeError> {
    match error {
        CommandError::Frame(frame) => Ok(frame_kind(frame)),
        CommandError::Core(_) => Ok("core"),
        CommandError::SchemaMismatch => Ok("schemaMismatch"),
        CommandError::Work(work) => Err(RuntimeError::Work(*work)),
    }
}

fn receipt_kind(error: &ReceiptRowError) -> &'static str {
    match error {
        ReceiptRowError::Frame(frame) => frame_kind(frame),
        ReceiptRowError::ForeignRow => "foreignRow",
    }
}

fn chain_kind(error: &ChainError) -> &'static str {
    match error {
        ChainError::WrongParent { .. } => "wrongParent",
        ChainError::WrongSequence { .. } => "wrongSequence",
    }
}

/// A grammar lane's domain outcome: the payload, or a refusal row
/// `{ ok: false, kind, message }` whose kind is an identity-table entry.
pub enum LogOutcome<T> {
    Value(T),
    Refused { kind: &'static str, message: String },
}

impl<T: napi::bindgen_prelude::ToNapiValue> napi::bindgen_prelude::ToNapiValue for LogOutcome<T> {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds a plain object and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut obj = Object::new(&env_handle)?;
        match val {
            Self::Value(value) => {
                obj.set("ok", true)?;
                obj.set("value", value)?;
            }
            Self::Refused { kind, message } => {
                obj.set("ok", false)?;
                obj.set("kind", kind)?;
                obj.set("message", message)?;
            }
        }
        unsafe { Object::to_napi_value(env, obj) }
    }
}

/// The complete successor refusal-identity and outcome-tag table, emitted
/// by the one speller (`bumbledb_log::identities::emit`). The TypeScript
/// driver pins its own table against this at load; the committed
/// `log-identities.json` golden is pinned by the test below.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[must_use]
pub fn log_identities() -> String {
    bumbledb_log::identities::emit()
}

// ---------------------------------------------------------------------------
// Inbound wire parsing (JS thread, small owned values).
// ---------------------------------------------------------------------------

fn digest_in(bytes: &Uint8Array, ctx: &str) -> napi::Result<[u8; 32]> {
    let raw: &[u8] = bytes;
    <[u8; 32]>::try_from(raw).map_err(|_| {
        marshal::err(format!(
            "bumbledb-log marshal: {ctx}: expected 32 bytes, got {}",
            raw.len()
        ))
    })
}

fn fingerprint_in(hex: &str) -> napi::Result<[u8; 32]> {
    let refuse = || {
        marshal::err(format!(
            "bumbledb-log marshal: fingerprint is not 64 hex characters: `{hex}`"
        ))
    };
    if hex.len() != 64 || !hex.is_ascii() {
        return Err(refuse());
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(hex.as_bytes().as_chunks::<2>().0) {
        let text = std::str::from_utf8(pair).map_err(|_| refuse())?;
        *slot = u8::from_str_radix(text, 16).map_err(|_| refuse())?;
    }
    Ok(out)
}

fn id128_field(obj: &Object, key: &str, ctx: &str) -> napi::Result<Id128> {
    marshal::id128_in(&marshal::req::<String>(obj, key, ctx)?, ctx)
}

fn identity_in(obj: &Object, ctx: &str) -> napi::Result<DatabaseIdentity> {
    Ok(DatabaseIdentity {
        database_id: DatabaseId::from_core(id128_field(obj, "databaseId", ctx)?),
        incarnation_id: IncarnationId::from_core(id128_field(obj, "incarnationId", ctx)?),
        schema_id: SchemaFingerprint(fingerprint_in(&marshal::req::<String>(
            obj, "schemaId", ctx,
        )?)?),
    })
}

fn epoch_in(obj: &Object, key: &str, ctx: &str) -> napi::Result<ReceiptEpoch> {
    let raw = marshal::u64_in(&marshal::req::<BigInt>(obj, key, ctx)?, ctx)?;
    ReceiptEpoch::new(raw)
        .ok_or_else(|| marshal::err(format!("bumbledb-log marshal: {ctx}: receipt epoch 0")))
}

fn command_id_in(obj: &Object, ctx: &str) -> napi::Result<CommandId> {
    Ok(CommandId {
        receipt_epoch: epoch_in(obj, "receiptEpoch", ctx)?,
        request_id: RequestId::from_core(id128_field(obj, "requestId", ctx)?),
    })
}

fn state_in(obj: &Object, ctx: &str) -> napi::Result<StateStamp> {
    Ok(StateStamp {
        incarnation: IncarnationId::from_core(id128_field(obj, "incarnation", ctx)?),
        data_revision: marshal::u64_in(&marshal::req::<BigInt>(obj, "dataRevision", ctx)?, ctx)?,
    })
}

fn stamp_in(obj: &Object, ctx: &str) -> napi::Result<DecisionStamp> {
    Ok(DecisionStamp {
        seq: marshal::u64_in(&marshal::req::<BigInt>(obj, "seq", ctx)?, ctx)?,
        hash: DecisionDigest::from_bytes(digest_in(
            &marshal::req::<Uint8Array>(obj, "hash", ctx)?,
            ctx,
        )?),
    })
}

fn condition_in(obj: &Object, ctx: &str) -> napi::Result<Condition> {
    let kind: String = marshal::req(obj, "kind", ctx)?;
    match kind.as_str() {
        "unconditional" => Ok(Condition::Unconditional),
        "exactState" => Ok(Condition::ExactState(state_in(
            &marshal::req::<Object>(obj, "state", ctx)?,
            ctx,
        )?)),
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: unknown condition kind `{other}`"
        ))),
    }
}

fn metadata_in(obj: &Object, ctx: &str) -> napi::Result<CommandMetadata> {
    Ok(CommandMetadata {
        identity: identity_in(&marshal::req::<Object>(obj, "identity", ctx)?, ctx)?,
        id: command_id_in(&marshal::req::<Object>(obj, "id", ctx)?, ctx)?,
        condition: condition_in(&marshal::req::<Object>(obj, "condition", ctx)?, ctx)?,
    })
}

fn size_in(obj: &Object, key: &str, ctx: &str) -> napi::Result<usize> {
    let raw = marshal::u64_in(&marshal::req::<BigInt>(obj, key, ctx)?, ctx)?;
    usize::try_from(raw)
        .map_err(|_| marshal::err(format!("bumbledb-log marshal: {ctx}: {key} exceeds usize")))
}

fn limits_in(obj: &Object) -> napi::Result<Limits> {
    let ctx = "log limits";
    Ok(Limits {
        envelope_bytes: size_in(obj, "envelopeBytes", ctx)?,
        change_bytes: size_in(obj, "changeBytes", ctx)?,
        evidence_bytes: size_in(obj, "evidenceBytes", ctx)?,
        result_bytes: size_in(obj, "resultBytes", ctx)?,
    })
}

fn outcome_in(obj: &Object, ctx: &str) -> napi::Result<OwnedOutcome> {
    let kind: String = marshal::req(obj, "kind", ctx)?;
    match kind.as_str() {
        "committed" => {
            let added = marshal::u64_in(&marshal::req::<BigInt>(obj, "added", ctx)?, ctx)?;
            let removed = marshal::u64_in(&marshal::req::<BigInt>(obj, "removed", ctx)?, ctx)?;
            let result: Option<Uint8Array> = obj.get("result")?;
            Ok(OwnedOutcome::Committed {
                added,
                removed,
                result: result.map(|bytes| bytes.to_vec()).unwrap_or_default(),
            })
        }
        "noChange" => {
            let result: Option<Uint8Array> = obj.get("result")?;
            Ok(OwnedOutcome::NoChange {
                result: result.map(|bytes| bytes.to_vec()).unwrap_or_default(),
            })
        }
        "preconditionFailed" => Ok(OwnedOutcome::PreconditionFailed {
            expected: state_in(&marshal::req::<Object>(obj, "expected", ctx)?, ctx)?,
            observed: state_in(&marshal::req::<Object>(obj, "observed", ctx)?, ctx)?,
        }),
        "invariantRejected" => Ok(OwnedOutcome::InvariantRejected {
            evidence: marshal::req::<Uint8Array>(obj, "evidence", ctx)?.to_vec(),
        }),
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: unknown outcome kind `{other}`"
        ))),
    }
}

/// An owned terminal-outcome payload that can cross threads and lower to
/// either the borrowed frame grammar or the owned receipt grammar.
pub enum OwnedOutcome {
    Committed {
        added: u64,
        removed: u64,
        result: Vec<u8>,
    },
    NoChange {
        result: Vec<u8>,
    },
    PreconditionFailed {
        expected: StateStamp,
        observed: StateStamp,
    },
    InvariantRejected {
        evidence: Vec<u8>,
    },
}

impl OwnedOutcome {
    fn terminal(&self) -> Result<TerminalOutcome, &'static str> {
        Ok(match self {
            Self::Committed {
                added,
                removed,
                result,
            } => TerminalOutcome::Committed {
                changed: ChangeSummary::new(*added, *removed).ok_or("emptyChangeSummary")?,
                result: CommandResult::from_canonical_bytes(result.clone().into_boxed_slice()),
            },
            Self::NoChange { result } => TerminalOutcome::NoChange {
                result: CommandResult::from_canonical_bytes(result.clone().into_boxed_slice()),
            },
            Self::PreconditionFailed { expected, observed } => {
                TerminalOutcome::PreconditionFailed {
                    expected: *expected,
                    observed: *observed,
                }
            }
            Self::InvariantRejected { evidence } => TerminalOutcome::InvariantRejected {
                evidence: RejectionEvidence::from_canonical_bytes(
                    evidence.clone().into_boxed_slice(),
                )
                .ok_or("emptyEvidence")?,
            },
        })
    }

    fn unverified(
        &self,
    ) -> Result<bumbledb_log::history::command::UnverifiedOutcome<'_>, &'static str> {
        use bumbledb_log::history::command::UnverifiedOutcome;
        Ok(match self {
            Self::Committed {
                added,
                removed,
                result,
            } => UnverifiedOutcome::Committed {
                changed: ChangeSummary::new(*added, *removed).ok_or("emptyChangeSummary")?,
                result,
            },
            Self::NoChange { result } => UnverifiedOutcome::NoChange { result },
            Self::PreconditionFailed { expected, observed } => {
                UnverifiedOutcome::PreconditionFailed {
                    expected: *expected,
                    observed: *observed,
                }
            }
            Self::InvariantRejected { evidence } => UnverifiedOutcome::InvariantRejected {
                core_evidence: evidence,
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Outbound wire rendering.
// ---------------------------------------------------------------------------

fn hex32(id: Id128) -> String {
    marshal::id128_hex(id)
}

fn hex64(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn identity_out(env: &Env, identity: DatabaseIdentity) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env)?;
    obj.set("databaseId", hex32(identity.database_id.as_core()))?;
    obj.set("incarnationId", hex32(identity.incarnation_id.as_core()))?;
    obj.set("schemaId", hex64(&identity.schema_id.0))?;
    Ok(obj)
}

fn state_out(env: &Env, state: StateStamp) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env)?;
    obj.set("incarnation", hex32(state.incarnation.as_core()))?;
    obj.set("dataRevision", BigInt::from(state.data_revision))?;
    Ok(obj)
}

fn stamp_out(env: &Env, stamp: DecisionStamp) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env)?;
    obj.set("seq", BigInt::from(stamp.seq))?;
    obj.set("hash", Buffer::from(stamp.hash.as_bytes().to_vec()))?;
    Ok(obj)
}

fn reference_out(env: &Env, reference: CommandRef) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env)?;
    obj.set("identity", identity_out(env, reference.identity)?)?;
    obj.set(
        "receiptEpoch",
        BigInt::from(reference.id.receipt_epoch.get()),
    )?;
    obj.set("requestId", hex32(reference.id.request_id.as_core()))?;
    obj.set("digest", Buffer::from(reference.digest.as_bytes().to_vec()))?;
    Ok(obj)
}

fn owned_outcome_out<'env>(env: &'env Env, outcome: &OwnedOutcome) -> napi::Result<Object<'env>> {
    let mut obj = Object::new(env)?;
    match outcome {
        OwnedOutcome::Committed {
            added,
            removed,
            result,
        } => {
            obj.set("kind", "committed")?;
            obj.set("added", BigInt::from(*added))?;
            obj.set("removed", BigInt::from(*removed))?;
            obj.set("result", Buffer::from(result.clone()))?;
        }
        OwnedOutcome::NoChange { result } => {
            obj.set("kind", "noChange")?;
            obj.set("result", Buffer::from(result.clone()))?;
        }
        OwnedOutcome::PreconditionFailed { expected, observed } => {
            obj.set("kind", "preconditionFailed")?;
            obj.set("expected", state_out(env, *expected)?)?;
            obj.set("observed", state_out(env, *observed)?)?;
        }
        OwnedOutcome::InvariantRejected { evidence } => {
            obj.set("kind", "invariantRejected")?;
            obj.set("evidence", Buffer::from(evidence.clone()))?;
        }
    }
    Ok(obj)
}

fn owned_outcome_of(outcome: &TerminalOutcome) -> OwnedOutcome {
    match outcome {
        TerminalOutcome::Committed { changed, result } => OwnedOutcome::Committed {
            added: changed.added(),
            removed: changed.removed(),
            result: result.as_bytes().to_vec(),
        },
        TerminalOutcome::NoChange { result } => OwnedOutcome::NoChange {
            result: result.as_bytes().to_vec(),
        },
        TerminalOutcome::PreconditionFailed { expected, observed } => {
            OwnedOutcome::PreconditionFailed {
                expected: *expected,
                observed: *observed,
            }
        }
        TerminalOutcome::InvariantRejected { evidence } => OwnedOutcome::InvariantRejected {
            evidence: evidence.as_bytes().to_vec(),
        },
    }
}

// ---------------------------------------------------------------------------
// The sealed schema handle: the one validated Schema shared by the
// command lanes (executor jobs hold an Arc; plain immutable data).
// ---------------------------------------------------------------------------

pub struct LogSchemaHandle {
    schema: Arc<Schema>,
    fingerprint: SchemaFingerprint,
}

/// Seals the validated core `Schema` for the log lanes from the same
/// `SchemaSpec` object every other lane speaks.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_schema(env: Env, spec: Object) -> napi::Result<External<LogSchemaHandle>> {
    let (descriptor, _attrs) = match crate::descriptor_of(&spec)? {
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
    let schema = descriptor.validate().map_err(|error| {
        marshal::throw_kind_message(env, crate::tags::error_family::SCHEMA, error.to_string())
    })?;
    let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
    Ok(External::new(LogSchemaHandle {
        schema: Arc::new(schema),
        fingerprint,
    }))
}

#[napi]
#[doc(hidden)]
#[must_use]
pub fn log_schema_fingerprint(handle: &External<LogSchemaHandle>) -> String {
    hex64(&handle.fingerprint.0)
}

// ---------------------------------------------------------------------------
// Synchronous small-frame lanes: receipt rows, authority control, genesis.
// ---------------------------------------------------------------------------

/// The 25-byte retained-receipt row key for one command id.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_receipt_key(id: Object) -> napi::Result<Buffer> {
    let id = command_id_in(&id, "receipt key")?;
    Ok(Buffer::from(receipt_key(id).to_vec()))
}

/// Encodes one durable receipt row from an owned terminal receipt.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_receipt_encode(receipt: Object, limits: Object) -> napi::Result<LogOutcome<Buffer>> {
    let ctx = "receipt row";
    let limits = limits_in(&limits)?;
    let command = {
        let reference: Object = marshal::req(&receipt, "command", ctx)?;
        CommandRef {
            identity: identity_in(&marshal::req::<Object>(&reference, "identity", ctx)?, ctx)?,
            id: command_id_in(&reference, ctx)?,
            digest: CommandDigest::from_bytes(digest_in(
                &marshal::req::<Uint8Array>(&reference, "digest", ctx)?,
                ctx,
            )?),
        }
    };
    let outcome = outcome_in(&marshal::req::<Object>(&receipt, "outcome", ctx)?, ctx)?;
    let terminal = match outcome.terminal() {
        Ok(terminal) => terminal,
        Err(kind) => {
            return Ok(LogOutcome::Refused {
                kind,
                message: format!("bumbledb-log receipt refusal: {kind}"),
            });
        }
    };
    let owned = TerminalReceipt {
        command,
        decision_at: stamp_in(&marshal::req::<Object>(&receipt, "decisionAt", ctx)?, ctx)?,
        state_at: state_in(&marshal::req::<Object>(&receipt, "stateAt", ctx)?, ctx)?,
        outcome: terminal,
    };
    match encode_receipt_row(&owned, limits) {
        Ok(bytes) => Ok(LogOutcome::Value(Buffer::from(bytes))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: frame_kind(&refusal),
            message: format!("bumbledb-log receipt refusal: {refusal:?}"),
        }),
    }
}

/// An owned receipt wire shell rendered at the boundary.
pub struct ReceiptWire(TerminalReceipt);

impl napi::bindgen_prelude::ToNapiValue for ReceiptWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds plain objects and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let receipt = val.0;
        let mut obj = Object::new(&env_handle)?;
        obj.set("command", reference_out(&env_handle, receipt.command)?)?;
        obj.set("decisionAt", stamp_out(&env_handle, receipt.decision_at)?)?;
        obj.set("stateAt", state_out(&env_handle, receipt.state_at)?)?;
        obj.set(
            "outcome",
            owned_outcome_out(&env_handle, &owned_outcome_of(&receipt.outcome))?,
        )?;
        unsafe { Object::to_napi_value(env, obj) }
    }
}

/// Decodes one stored receipt row against the exact expected command
/// reference (identity + id must match; a wrong-scope row is `foreignRow`,
/// never a receipt).
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_receipt_decode(
    expected: Object,
    bytes: Uint8Array,
    limits: Object,
) -> napi::Result<LogOutcome<ReceiptWire>> {
    let ctx = "receipt decode";
    let limits = limits_in(&limits)?;
    let reference = CommandRef {
        identity: identity_in(&marshal::req::<Object>(&expected, "identity", ctx)?, ctx)?,
        id: command_id_in(&expected, ctx)?,
        digest: CommandDigest::from_bytes(
            expected
                .get::<Uint8Array>("digest")?
                .map(|digest| digest_in(&digest, ctx))
                .transpose()?
                .unwrap_or([0; 32]),
        ),
    };
    match decode_receipt_row(reference, &bytes, limits) {
        Ok(receipt) => Ok(LogOutcome::Value(ReceiptWire(receipt))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: receipt_kind(&refusal),
            message: format!("bumbledb-log receipt refusal: {refusal:?}"),
        }),
    }
}

/// Decodes a receipt row when only the storage key is known (retention
/// walks/inspection), verifying the key binding.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_receipt_decode_at(
    id: Object,
    bytes: Uint8Array,
    limits: Object,
) -> napi::Result<LogOutcome<ReceiptWire>> {
    let limits = limits_in(&limits)?;
    let id = command_id_in(&id, "receipt key")?;
    match decode_receipt_row_at(id, &bytes, limits) {
        Ok(receipt) => Ok(LogOutcome::Value(ReceiptWire(receipt))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: receipt_kind(&refusal),
            message: format!("bumbledb-log receipt refusal: {refusal:?}"),
        }),
    }
}

fn intent_in(obj: &Object, ctx: &str) -> napi::Result<FreezeIntent> {
    let kind: String = marshal::req(obj, "kind", ctx)?;
    match kind.as_str() {
        "erasure" => Ok(FreezeIntent::Erasure),
        "migration" => Ok(FreezeIntent::Migration {
            plan_set_digest: digest_in(
                &marshal::req::<Uint8Array>(obj, "planSetDigest", ctx)?,
                ctx,
            )?,
            target: IncarnationId::from_core(id128_field(obj, "target", ctx)?),
        }),
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: unknown freeze intent `{other}`"
        ))),
    }
}

fn access_in(obj: &Object, ctx: &str) -> napi::Result<Access> {
    let kind: String = marshal::req(obj, "kind", ctx)?;
    match kind.as_str() {
        "active" => Ok(Access::Active),
        "frozen" => Ok(Access::Frozen {
            operation: OperationId::from_core(id128_field(obj, "operation", ctx)?),
            intent: intent_in(&marshal::req::<Object>(obj, "intent", ctx)?, ctx)?,
        }),
        other => Err(marshal::err(format!(
            "bumbledb-log marshal: {ctx}: unknown access kind `{other}`"
        ))),
    }
}

#[allow(clippy::too_many_lines)]
fn authority_in(obj: &Object) -> napi::Result<HeadAuthority> {
    let ctx = "head authority";
    let lifecycle_obj: Object = marshal::req(obj, "lifecycle", ctx)?;
    let lifecycle_kind: String = marshal::req(&lifecycle_obj, "kind", ctx)?;
    let lifecycle = match lifecycle_kind.as_str() {
        "live" => {
            let receipts: Object = marshal::req(&lifecycle_obj, "receipts", ctx)?;
            let policy = ReceiptPolicy::new(
                epoch_in(&receipts, "openEpoch", ctx)?,
                marshal::u64_in(
                    &marshal::req::<BigInt>(&receipts, "retiredThrough", ctx)?,
                    ctx,
                )?,
            )
            .map_err(|error| marshal::err(format!("bumbledb-log marshal: {ctx}: {error:?}")))?;
            Lifecycle::Live(LiveAuthority {
                access: access_in(&marshal::req::<Object>(&lifecycle_obj, "access", ctx)?, ctx)?,
                decision: stamp_in(
                    &marshal::req::<Object>(&lifecycle_obj, "decision", ctx)?,
                    ctx,
                )?,
                state: state_in(&marshal::req::<Object>(&lifecycle_obj, "state", ctx)?, ctx)?,
                receipts: policy,
            })
        }
        "deleted" => {
            let reason_obj: Object = marshal::req(&lifecycle_obj, "reason", ctx)?;
            let reason_kind: String = marshal::req(&reason_obj, "kind", ctx)?;
            let reason = match reason_kind.as_str() {
                "erasure" => DeletedReason::Erasure,
                "migrationAborted" => DeletedReason::MigrationAborted {
                    source_database: DatabaseId::from_core(id128_field(
                        &reason_obj,
                        "sourceDatabase",
                        ctx,
                    )?),
                    source_incarnation: IncarnationId::from_core(id128_field(
                        &reason_obj,
                        "sourceIncarnation",
                        ctx,
                    )?),
                    plan_set_digest: digest_in(
                        &marshal::req::<Uint8Array>(&reason_obj, "planSetDigest", ctx)?,
                        ctx,
                    )?,
                },
                other => {
                    return Err(marshal::err(format!(
                        "bumbledb-log marshal: {ctx}: unknown deletion reason `{other}`"
                    )));
                }
            };
            Lifecycle::Deleted {
                operation: OperationId::from_core(id128_field(&lifecycle_obj, "operation", ctx)?),
                reason,
            }
        }
        other => {
            return Err(marshal::err(format!(
                "bumbledb-log marshal: {ctx}: unknown lifecycle kind `{other}`"
            )));
        }
    };
    let activation_obj: Object = marshal::req(obj, "activation", ctx)?;
    let activation_kind: String = marshal::req(&activation_obj, "kind", ctx)?;
    let activation = match activation_kind.as_str() {
        "notActivated" => Activation::NotActivated,
        "activated" => {
            let cause_obj: Object = marshal::req(&activation_obj, "cause", ctx)?;
            let cause_kind: String = marshal::req(&cause_obj, "kind", ctx)?;
            let cause = match cause_kind.as_str() {
                "create" => ActivationCause::Create,
                "restore" => ActivationCause::Restore,
                "migration" => ActivationCause::Migration {
                    plan_set_digest: digest_in(
                        &marshal::req::<Uint8Array>(&cause_obj, "planSetDigest", ctx)?,
                        ctx,
                    )?,
                },
                other => {
                    return Err(marshal::err(format!(
                        "bumbledb-log marshal: {ctx}: unknown activation cause `{other}`"
                    )));
                }
            };
            Activation::Activated {
                operation: OperationId::from_core(id128_field(&activation_obj, "operation", ctx)?),
                target_genesis: DecisionDigest::from_bytes(digest_in(
                    &marshal::req::<Uint8Array>(&activation_obj, "targetGenesis", ctx)?,
                    ctx,
                )?),
                cause,
            }
        }
        other => {
            return Err(marshal::err(format!(
                "bumbledb-log marshal: {ctx}: unknown activation kind `{other}`"
            )));
        }
    };
    Ok(HeadAuthority {
        identity: identity_in(&marshal::req::<Object>(obj, "identity", ctx)?, ctx)?,
        revision: HeadRevision(marshal::u64_in(
            &marshal::req::<BigInt>(obj, "revision", ctx)?,
            ctx,
        )?),
        lifecycle,
        activation,
    })
}

fn authority_out<'env>(env: &'env Env, authority: &HeadAuthority) -> napi::Result<Object<'env>> {
    let mut obj = Object::new(env)?;
    obj.set("identity", identity_out(env, authority.identity)?)?;
    obj.set("revision", BigInt::from(authority.revision.0))?;
    let mut lifecycle = Object::new(env)?;
    match &authority.lifecycle {
        Lifecycle::Live(live) => {
            lifecycle.set("kind", "live")?;
            let mut access = Object::new(env)?;
            match live.access {
                Access::Active => access.set("kind", "active")?,
                Access::Frozen { operation, intent } => {
                    access.set("kind", "frozen")?;
                    access.set("operation", hex32(operation.as_core()))?;
                    let mut intent_obj = Object::new(env)?;
                    match intent {
                        FreezeIntent::Erasure => intent_obj.set("kind", "erasure")?,
                        FreezeIntent::Migration {
                            plan_set_digest,
                            target,
                        } => {
                            intent_obj.set("kind", "migration")?;
                            intent_obj
                                .set("planSetDigest", Buffer::from(plan_set_digest.to_vec()))?;
                            intent_obj.set("target", hex32(target.as_core()))?;
                        }
                    }
                    access.set("intent", intent_obj)?;
                }
            }
            lifecycle.set("access", access)?;
            lifecycle.set("decision", stamp_out(env, live.decision)?)?;
            lifecycle.set("state", state_out(env, live.state)?)?;
            let mut receipts = Object::new(env)?;
            receipts.set("openEpoch", BigInt::from(live.receipts.open_epoch().get()))?;
            receipts.set(
                "retiredThrough",
                BigInt::from(live.receipts.retired_through()),
            )?;
            lifecycle.set("receipts", receipts)?;
        }
        Lifecycle::Deleted { operation, reason } => {
            lifecycle.set("kind", "deleted")?;
            lifecycle.set("operation", hex32(operation.as_core()))?;
            let mut reason_obj = Object::new(env)?;
            match reason {
                DeletedReason::Erasure => reason_obj.set("kind", "erasure")?,
                DeletedReason::MigrationAborted {
                    source_database,
                    source_incarnation,
                    plan_set_digest,
                } => {
                    reason_obj.set("kind", "migrationAborted")?;
                    reason_obj.set("sourceDatabase", hex32(source_database.as_core()))?;
                    reason_obj.set("sourceIncarnation", hex32(source_incarnation.as_core()))?;
                    reason_obj.set("planSetDigest", Buffer::from(plan_set_digest.to_vec()))?;
                }
            }
            lifecycle.set("reason", reason_obj)?;
        }
    }
    obj.set("lifecycle", lifecycle)?;
    let mut activation = Object::new(env)?;
    match &authority.activation {
        Activation::NotActivated => activation.set("kind", "notActivated")?,
        Activation::Activated {
            operation,
            target_genesis,
            cause,
        } => {
            activation.set("kind", "activated")?;
            activation.set("operation", hex32(operation.as_core()))?;
            activation.set(
                "targetGenesis",
                Buffer::from(target_genesis.as_bytes().to_vec()),
            )?;
            let mut cause_obj = Object::new(env)?;
            match cause {
                ActivationCause::Create => cause_obj.set("kind", "create")?,
                ActivationCause::Restore => cause_obj.set("kind", "restore")?,
                ActivationCause::Migration { plan_set_digest } => {
                    cause_obj.set("kind", "migration")?;
                    cause_obj.set("planSetDigest", Buffer::from(plan_set_digest.to_vec()))?;
                }
            }
            activation.set("cause", cause_obj)?;
        }
    }
    obj.set("activation", activation)?;
    Ok(obj)
}

/// Renders the head-authority control projection — the bytes P05 wraps
/// with its retention fields in the hosted HEAD body (C07/C08).
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_control_encode(authority: Object, cap: BigInt) -> napi::Result<LogOutcome<Buffer>> {
    let authority = authority_in(&authority)?;
    let cap = usize::try_from(marshal::u64_in(&cap, "control cap")?)
        .map_err(|_| marshal::err("bumbledb-log marshal: control cap exceeds usize".into()))?;
    match encode_control(&authority, cap) {
        Ok(bytes) => Ok(LogOutcome::Value(Buffer::from(bytes))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: frame_kind(&refusal),
            message: format!("bumbledb-log control refusal: {refusal:?}"),
        }),
    }
}

/// An owned head-authority wire shell rendered at the boundary.
pub struct AuthorityWire(HeadAuthority);

impl napi::bindgen_prelude::ToNapiValue for AuthorityWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds plain objects and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let obj = authority_out(&env_handle, &val.0)?;
        unsafe { Object::to_napi_value(env, obj) }
    }
}

/// Parses the head-authority control projection.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_control_decode(
    bytes: Uint8Array,
    cap: BigInt,
) -> napi::Result<LogOutcome<AuthorityWire>> {
    let cap = usize::try_from(marshal::u64_in(&cap, "control cap")?)
        .map_err(|_| marshal::err("bumbledb-log marshal: control cap exceeds usize".into()))?;
    match decode_control(&bytes, cap) {
        Ok(authority) => Ok(LogOutcome::Value(AuthorityWire(authority))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: frame_kind(&refusal),
            message: format!("bumbledb-log control refusal: {refusal:?}"),
        }),
    }
}

fn genesis_in(obj: &Object) -> napi::Result<GenesisRecord> {
    let ctx = "genesis record";
    let provenance_obj: Object = marshal::req(obj, "provenance", ctx)?;
    let kind: String = marshal::req(&provenance_obj, "kind", ctx)?;
    let provenance = match kind.as_str() {
        "create" => GenesisProvenance::Create,
        "restore" => GenesisProvenance::Restore {
            source_evidence: digest_in(
                &marshal::req::<Uint8Array>(&provenance_obj, "sourceEvidence", ctx)?,
                ctx,
            )?,
        },
        "migration" => GenesisProvenance::Migration {
            source_database: DatabaseId::from_core(id128_field(
                &provenance_obj,
                "sourceDatabase",
                ctx,
            )?),
            source_incarnation: IncarnationId::from_core(id128_field(
                &provenance_obj,
                "sourceIncarnation",
                ctx,
            )?),
            plan_set_digest: digest_in(
                &marshal::req::<Uint8Array>(&provenance_obj, "planSetDigest", ctx)?,
                ctx,
            )?,
        },
        other => {
            return Err(marshal::err(format!(
                "bumbledb-log marshal: {ctx}: unknown provenance kind `{other}`"
            )));
        }
    };
    Ok(GenesisRecord {
        identity: identity_in(&marshal::req::<Object>(obj, "identity", ctx)?, ctx)?,
        initial_application_digest: digest_in(
            &marshal::req::<Uint8Array>(obj, "initialApplicationDigest", ctx)?,
            ctx,
        )?,
        initial_system_digest: digest_in(
            &marshal::req::<Uint8Array>(obj, "initialSystemDigest", ctx)?,
            ctx,
        )?,
        provenance,
    })
}

/// Encodes the versioned genesis preimage record.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_genesis_encode(record: Object, cap: BigInt) -> napi::Result<LogOutcome<Buffer>> {
    let record = genesis_in(&record)?;
    let cap = usize::try_from(marshal::u64_in(&cap, "genesis cap")?)
        .map_err(|_| marshal::err("bumbledb-log marshal: genesis cap exceeds usize".into()))?;
    match encode_genesis(&record, cap) {
        Ok(bytes) => Ok(LogOutcome::Value(Buffer::from(bytes))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: frame_kind(&refusal),
            message: format!("bumbledb-log genesis refusal: {refusal:?}"),
        }),
    }
}

/// An owned genesis-record wire shell rendered at the boundary.
pub struct GenesisWire(GenesisRecord);

impl napi::bindgen_prelude::ToNapiValue for GenesisWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds plain objects and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let record = val.0;
        let mut obj = Object::new(&env_handle)?;
        obj.set("identity", identity_out(&env_handle, record.identity)?)?;
        obj.set(
            "initialApplicationDigest",
            Buffer::from(record.initial_application_digest.to_vec()),
        )?;
        obj.set(
            "initialSystemDigest",
            Buffer::from(record.initial_system_digest.to_vec()),
        )?;
        let mut provenance = Object::new(&env_handle)?;
        match record.provenance {
            GenesisProvenance::Create => provenance.set("kind", "create")?,
            GenesisProvenance::Restore { source_evidence } => {
                provenance.set("kind", "restore")?;
                provenance.set("sourceEvidence", Buffer::from(source_evidence.to_vec()))?;
            }
            GenesisProvenance::Migration {
                source_database,
                source_incarnation,
                plan_set_digest,
            } => {
                provenance.set("kind", "migration")?;
                provenance.set("sourceDatabase", hex32(source_database.as_core()))?;
                provenance.set("sourceIncarnation", hex32(source_incarnation.as_core()))?;
                provenance.set("planSetDigest", Buffer::from(plan_set_digest.to_vec()))?;
            }
        }
        obj.set("provenance", provenance)?;
        unsafe { Object::to_napi_value(env, obj) }
    }
}

/// Parses a genesis preimage record.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_genesis_decode(bytes: Uint8Array, cap: BigInt) -> napi::Result<LogOutcome<GenesisWire>> {
    let cap = usize::try_from(marshal::u64_in(&cap, "genesis cap")?)
        .map_err(|_| marshal::err("bumbledb-log marshal: genesis cap exceeds usize".into()))?;
    match decode_genesis(&bytes, cap) {
        Ok(record) => Ok(LogOutcome::Value(GenesisWire(record))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: frame_kind(&refusal),
            message: format!("bumbledb-log genesis refusal: {refusal:?}"),
        }),
    }
}

/// An owned decision-stamp wire shell rendered at the boundary.
pub struct StampWire(DecisionStamp);

impl napi::bindgen_prelude::ToNapiValue for StampWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds a plain object and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let obj = stamp_out(&env_handle, val.0)?;
        unsafe { Object::to_napi_value(env, obj) }
    }
}

/// The sequence-zero genesis decision stamp for a record.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn log_genesis_stamp(record: Object, cap: BigInt) -> napi::Result<LogOutcome<StampWire>> {
    let record = genesis_in(&record)?;
    let cap = usize::try_from(marshal::u64_in(&cap, "genesis cap")?)
        .map_err(|_| marshal::err("bumbledb-log marshal: genesis cap exceeds usize".into()))?;
    match genesis_stamp(&record, cap) {
        Ok(stamp) => Ok(LogOutcome::Value(StampWire(stamp))),
        Err(refusal) => Ok(LogOutcome::Refused {
            kind: frame_kind(&refusal),
            message: format!("bumbledb-log genesis refusal: {refusal:?}"),
        }),
    }
}

/// Canonical digests of the empty initial state (ordinary blank creation).
pub struct BlankDigests {
    application: [u8; 32],
    system: [u8; 32],
}

impl napi::bindgen_prelude::ToNapiValue for BlankDigests {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                  builds a plain object and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut obj = Object::new(&env_handle)?;
        obj.set("application", Buffer::from(val.application.to_vec()))?;
        obj.set("system", Buffer::from(val.system.to_vec()))?;
        unsafe { Object::to_napi_value(env, obj) }
    }
}

/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[must_use]
pub fn log_blank_digests() -> BlankDigests {
    let (application, system) = blank_initial_digests();
    BlankDigests {
        application,
        system,
    }
}

// ---------------------------------------------------------------------------
// Executor lanes: command seal/parse and decision encode/decode/verify.
// These embed whole canonical change payloads and charged hashing, so they
// are bounded operations on the one executor, never synchronous JS work.
// ---------------------------------------------------------------------------

/// An owned command reference crossing back from an executor job.
pub struct OwnedRef {
    identity: DatabaseIdentity,
    epoch: u64,
    request: Id128,
    digest: [u8; 32],
}

impl OwnedRef {
    fn of(reference: CommandRef) -> Self {
        Self {
            identity: reference.identity,
            epoch: reference.id.receipt_epoch.get(),
            request: reference.id.request_id.as_core(),
            digest: *reference.digest.as_bytes(),
        }
    }
}

/// An owned decoded command envelope.
pub struct OwnedCommand {
    metadata_identity: DatabaseIdentity,
    epoch: u64,
    request: Id128,
    condition: Condition,
    changes: Vec<u8>,
    result: Vec<u8>,
    reference: OwnedRef,
}

/// An owned decoded decision envelope.
pub struct OwnedDecision {
    identity: DatabaseIdentity,
    seq: u64,
    parent: DecisionStamp,
    before_state: StateStamp,
    after_state: StateStamp,
    command_bytes: Vec<u8>,
    command: OwnedRef,
    outcome: OwnedOutcome,
    digest: [u8; 32],
}

/// The executor's grammar-lane payloads (all owned, `Send`).
pub enum LogOutput {
    /// Command sealed: the canonical envelope bytes plus its reference.
    Sealed { bytes: Vec<u8>, reference: OwnedRef },
    /// Command parsed through the core's strict change decoder.
    Command(Box<OwnedCommand>),
    /// Decision framed: envelope bytes, digest, stamp sequence.
    Decision { bytes: Vec<u8>, digest: [u8; 32] },
    /// Decision decoded (and, on the verify lane, chain-checked).
    Decoded(Box<OwnedDecision>),
    /// A domain refusal from the grammar (identity-table kind).
    Refused { kind: &'static str, message: String },
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "the Ok-wrap IS the helper: every caller returns this value \
              directly from a Result-typed job body"
)]
fn refused(kind: &'static str, message: String) -> Result<Output, RuntimeError> {
    Ok(Output::Log(LogOutput::Refused { kind, message }))
}

fn seal_command(
    schema: &Schema,
    metadata: CommandMetadata,
    changes: &[u8],
    result: Vec<u8>,
    limits: Limits,
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    let changes = match bumbledb::ChangeSet::parse(schema, changes, work) {
        Ok(changes) => changes,
        Err(bumbledb::ChangeError::Work(error)) => return Err(RuntimeError::Work(error)),
        Err(error) => return refused("core", format!("bumbledb-log command refusal: {error:?}")),
    };
    let command = match Command::seal(
        metadata,
        changes,
        CommandResult::from_canonical_bytes(result.into_boxed_slice()),
        limits,
        work,
    ) {
        Ok(command) => command,
        Err(error) => {
            let kind = command_kind(&error)?;
            return refused(kind, format!("bumbledb-log command refusal: {error:?}"));
        }
    };
    let reference = command.command_ref();
    match command.encode(limits) {
        Ok(bytes) => Ok(Output::Log(LogOutput::Sealed {
            bytes,
            reference: OwnedRef::of(reference),
        })),
        Err(error) => refused(
            frame_kind(&error),
            format!("bumbledb-log command refusal: {error:?}"),
        ),
    }
}

/// Seals one canonical command envelope on the executor: metadata + the
/// core's canonical change bytes (+ optional declared result bytes) in,
/// envelope bytes + command reference out.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn runtime_log_command_seal(
    env: Env,
    handle: &External<RuntimeHandle>,
    schema: &External<LogSchemaHandle>,
    policy: PolicyWire,
    metadata: Object,
    changes: Unknown,
    result: Option<Unknown>,
    limits: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let schema = Arc::clone(&schema.schema);
    let metadata = metadata_in(&metadata, "command metadata")?;
    let limits = limits_in(&limits)?;
    let changes = unshared_input(env, changes, runtime.options.chunk_bytes)?;
    let result = result
        .map(|value| unshared_input(env, value, runtime.options.chunk_bytes))
        .transpose()?;
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            |context| {
                let total = changes.len() as u64 + result.as_ref().map_or(0, |r| r.len() as u64);
                context.input(total)?;
                let changes = changes.to_vec();
                let result = result.map(|bytes| bytes.to_vec()).unwrap_or_default();
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    seal_command(&schema, metadata, &changes, result, limits, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// Parses one canonical command envelope through the core's strict change
/// decoder on the executor.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_log_command_parse(
    env: Env,
    handle: &External<RuntimeHandle>,
    schema: &External<LogSchemaHandle>,
    policy: PolicyWire,
    bytes: Unknown,
    limits: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let schema = Arc::clone(&schema.schema);
    let limits = limits_in(&limits)?;
    let bytes = unshared_input(env, bytes, runtime.options.chunk_bytes)?;
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            |context| {
                context.input(bytes.len() as u64)?;
                let bytes = bytes.to_vec();
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    let command = match Command::parse(&schema, &bytes, limits, context) {
                        Ok(command) => command,
                        Err(CommandError::Work(error)) => return Err(RuntimeError::Work(error)),
                        Err(error) => {
                            let kind = command_kind(&error)?;
                            return refused(
                                kind,
                                format!("bumbledb-log command refusal: {error:?}"),
                            );
                        }
                    };
                    let metadata = command.metadata();
                    Ok(Output::Log(LogOutput::Command(Box::new(OwnedCommand {
                        metadata_identity: metadata.identity,
                        epoch: metadata.id.receipt_epoch.get(),
                        request: metadata.id.request_id.as_core(),
                        condition: metadata.condition,
                        changes: command.changes().as_bytes().to_vec(),
                        result: command.result().as_bytes().to_vec(),
                        reference: OwnedRef::of(command.command_ref()),
                    }))))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// Frames one immutable decision envelope on the executor.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn runtime_log_decision_encode(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    parts: Object,
    command_bytes: Unknown,
    limits: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let ctx = "decision parts";
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let limits = limits_in(&limits)?;
    let identity = identity_in(&marshal::req::<Object>(&parts, "identity", ctx)?, ctx)?;
    let seq = marshal::u64_in(&marshal::req::<BigInt>(&parts, "seq", ctx)?, ctx)?;
    let parent = stamp_in(&marshal::req::<Object>(&parts, "parent", ctx)?, ctx)?;
    let before_state = state_in(&marshal::req::<Object>(&parts, "beforeState", ctx)?, ctx)?;
    let after_state = state_in(&marshal::req::<Object>(&parts, "afterState", ctx)?, ctx)?;
    let outcome = outcome_in(&marshal::req::<Object>(&parts, "outcome", ctx)?, ctx)?;
    let command_bytes = unshared_input(env, command_bytes, runtime.options.chunk_bytes)?;
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            |context| {
                context.input(command_bytes.len() as u64)?;
                let command_bytes = command_bytes.to_vec();
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    let unverified = match outcome.unverified() {
                        Ok(unverified) => unverified,
                        Err(kind) => {
                            return refused(kind, format!("bumbledb-log decision refusal: {kind}"));
                        }
                    };
                    let parts = DecisionParts {
                        identity,
                        seq,
                        parent,
                        before_state,
                        after_state,
                        canonical_command: &command_bytes,
                        outcome: unverified,
                    };
                    match encode_decision(parts, limits) {
                        Ok(bytes) => {
                            let digest = *bumbledb_log::history::decision::decision_digest(&bytes)
                                .as_bytes();
                            Ok(Output::Log(LogOutput::Decision { bytes, digest }))
                        }
                        Err(error) => refused(
                            frame_kind(&error),
                            format!("bumbledb-log decision refusal: {error:?}"),
                        ),
                    }
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

fn decode_owned_decision(bytes: &[u8], limits: Limits) -> Result<Output, RuntimeError> {
    match decode_decision(bytes, limits) {
        Ok(envelope) => {
            let outcome = match envelope.outcome {
                bumbledb_log::history::command::UnverifiedOutcome::Committed {
                    changed,
                    result,
                } => OwnedOutcome::Committed {
                    added: changed.added(),
                    removed: changed.removed(),
                    result: result.to_vec(),
                },
                bumbledb_log::history::command::UnverifiedOutcome::NoChange { result } => {
                    OwnedOutcome::NoChange {
                        result: result.to_vec(),
                    }
                }
                bumbledb_log::history::command::UnverifiedOutcome::PreconditionFailed {
                    expected,
                    observed,
                } => OwnedOutcome::PreconditionFailed { expected, observed },
                bumbledb_log::history::command::UnverifiedOutcome::InvariantRejected {
                    core_evidence,
                } => OwnedOutcome::InvariantRejected {
                    evidence: core_evidence.to_vec(),
                },
            };
            Ok(Output::Log(LogOutput::Decoded(Box::new(OwnedDecision {
                identity: envelope.identity,
                seq: envelope.seq,
                parent: envelope.parent,
                before_state: envelope.before_state,
                after_state: envelope.after_state,
                command_bytes: envelope.canonical_command.to_vec(),
                command: OwnedRef::of(envelope.command),
                outcome,
                digest: *envelope.digest().as_bytes(),
            }))))
        }
        Err(error) => refused(
            frame_kind(&error),
            format!("bumbledb-log decision refusal: {error:?}"),
        ),
    }
}

/// Decodes one decision envelope on the executor; with a parent stamp it
/// also verifies the exact chain step (wrongParent/wrongSequence refusals).
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_log_decision_decode(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    bytes: Unknown,
    parent: Option<Object>,
    limits: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let limits = limits_in(&limits)?;
    let parent = parent
        .map(|stamp| stamp_in(&stamp, "chain parent"))
        .transpose()?;
    let bytes = unshared_input(env, bytes, runtime.options.chunk_bytes)?;
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            |context| {
                context.input(bytes.len() as u64)?;
                let bytes = bytes.to_vec();
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    if let Some(parent) = parent {
                        // Decode first (borrowed), verify, then rebuild owned.
                        match decode_decision(&bytes, limits) {
                            Ok(envelope) => {
                                if let Err(error) = verify_step(parent, &envelope) {
                                    return refused(
                                        chain_kind(&error),
                                        format!("bumbledb-log chain refusal: {error:?}"),
                                    );
                                }
                            }
                            Err(error) => {
                                return refused(
                                    frame_kind(&error),
                                    format!("bumbledb-log decision refusal: {error:?}"),
                                );
                            }
                        }
                    }
                    decode_owned_decision(&bytes, limits)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

fn reference_wire<'env>(env: &'env Env, reference: &OwnedRef) -> napi::Result<Object<'env>> {
    let mut obj = Object::new(env)?;
    obj.set("identity", identity_out(env, reference.identity)?)?;
    obj.set("receiptEpoch", BigInt::from(reference.epoch))?;
    obj.set("requestId", hex32(reference.request))?;
    obj.set("digest", Buffer::from(reference.digest.to_vec()))?;
    Ok(obj)
}

/// Takes an executor grammar-lane outcome: `{ ok: true, ... }` payloads or
/// `{ ok: false, kind, message }` identity-table refusals.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
pub fn runtime_log_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(&env)?;
    match take_output(env, handle)? {
        Output::Log(LogOutput::Sealed { bytes, reference }) => {
            obj.set("ok", true)?;
            obj.set("bytes", Buffer::from(bytes))?;
            obj.set("ref", reference_wire(&env, &reference)?)?;
        }
        Output::Log(LogOutput::Command(command)) => {
            obj.set("ok", true)?;
            obj.set("identity", identity_out(&env, command.metadata_identity)?)?;
            obj.set("receiptEpoch", BigInt::from(command.epoch))?;
            obj.set("requestId", hex32(command.request))?;
            let mut condition = Object::new(&env)?;
            match command.condition {
                Condition::Unconditional => condition.set("kind", "unconditional")?,
                Condition::ExactState(state) => {
                    condition.set("kind", "exactState")?;
                    condition.set("state", state_out(&env, state)?)?;
                }
            }
            obj.set("condition", condition)?;
            obj.set("changes", Buffer::from(command.changes))?;
            obj.set("result", Buffer::from(command.result))?;
            obj.set("ref", reference_wire(&env, &command.reference)?)?;
        }
        Output::Log(LogOutput::Decision { bytes, digest }) => {
            obj.set("ok", true)?;
            obj.set("bytes", Buffer::from(bytes))?;
            obj.set("digest", Buffer::from(digest.to_vec()))?;
        }
        Output::Log(LogOutput::Decoded(decision)) => {
            obj.set("ok", true)?;
            obj.set("identity", identity_out(&env, decision.identity)?)?;
            obj.set("seq", BigInt::from(decision.seq))?;
            obj.set("parent", stamp_out(&env, decision.parent)?)?;
            obj.set("beforeState", state_out(&env, decision.before_state)?)?;
            obj.set("afterState", state_out(&env, decision.after_state)?)?;
            obj.set("commandBytes", Buffer::from(decision.command_bytes))?;
            obj.set("command", reference_wire(&env, &decision.command)?)?;
            obj.set("outcome", owned_outcome_out(&env, &decision.outcome)?)?;
            obj.set("digest", Buffer::from(decision.digest.to_vec()))?;
        }
        Output::Log(LogOutput::Refused { kind, message }) => {
            obj.set("ok", false)?;
            obj.set("kind", kind)?;
            obj.set("message", message)?;
        }
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    }
    Ok(obj)
}

#[cfg(test)]
mod identity_table {
    /// The committed golden is the byte-exact `identities::emit()` output;
    /// the emitter is the one speller, and this pin makes drift loud. The
    /// old braided mint-table golden is deleted with the 0.x protocol.
    #[test]
    fn log_identities_json_is_the_emitted_table() {
        assert_eq!(
            include_str!("../log-identities.json"),
            bumbledb_log::identities::emit(),
            "ts/crate/log-identities.json drifted from bumbledb_log::identities::emit() — \
             regenerate the golden from the emitter (never the reverse without a core change)"
        );
    }

    /// The frame speller is pinned to the identities table row-for-row:
    /// the emitted `frame` family must spell exactly what `frame_kind`
    /// spells for every variant (both are exhaustive compile locks).
    #[test]
    fn frame_kinds_match_the_emitted_family() {
        let emitted = bumbledb_log::identities::emit();
        for kind in [
            "limitExceeded",
            "lengthOverflow",
            "allocation",
            "truncated",
            "family",
            "layout",
            "kind",
            "tag",
            "invalidEpoch",
            "stateIdentityMismatch",
            "emptyChangeSummary",
            "emptyEvidence",
            "invalidTerminalStamp",
            "invalidPreconditionEvidence",
            "invalidPolicy",
            "invalidSequence",
            "invalidCount",
            "trailingBytes",
        ] {
            assert!(
                emitted.contains(&format!("\"{kind}\"")),
                "frame kind `{kind}` missing from the emitted identity table"
            );
        }
    }
}
