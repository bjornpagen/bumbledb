//! The dumb-bridge law: no logic beyond marshaling will EVER live in this
//! crate. Anything smart belongs in the TypeScript SDK or the engine.
//!
//! Every database resource is owned by the ONE runtime registry
//! (`runtime_wire.rs`): databases live behind kernel-held directory
//! owners, `!Send` engine transactions and prepared queries live inside
//! worker-affine sessions (`runtime/session.rs`), and every operation is
//! registered, cancellable and drainable. The historical raw-pointer
//! `InstanceHandle`/`TxHandle` scoped-borrow surface — a JavaScript
//! callback executing inside a native transaction frame — is deleted, as
//! are the libuv `AsyncTask` entrypoints and the fresh/reserve issuance
//! verbs (the successor has application-owned `Uuid` identity only).
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bumbledb::schema::{SpecIssue, StatementDescriptor};
use bumbledb::{
    BindValue, Db, ParamArg, SchemaDescriptor, Theory as _, Value, Violations, render_rejection,
};
use napi::bindgen_prelude::{Buffer, Env, Object};
use napi_derive::napi;

pub mod db_wire;
#[cfg(test)]
mod fingerprint_lock;
pub mod log;
pub mod log_wire;
mod marshal;
mod migration_wire;
mod runtime;
pub mod runtime_wire;
pub use runtime::publication::runtime_arm_publication_cancel;
mod tags;

use marshal::{DescriptorWire, OwnedParam, ViolationWire};

/// Per-relation, sealed-order spec attribute rows (host newtype names) —
/// the spec-only half of the field vocabulary the descriptor drops.
type FieldAttrsTable = Vec<Vec<marshal::FieldAttrs>>;

#[napi]
#[must_use]
pub fn engine_version() -> String {
    format!(
        "bumbledb-node {} (bumbledb storage format v{})",
        env!("CARGO_PKG_VERSION"),
        bumbledb::STORAGE_FORMAT_VERSION
    )
}

/// The engine's own blake3 (`bumbledb::digest::Digest`), lent to the
/// replication driver so the SDK ships exactly one hash implementation.
/// Internal surface: not part of the SDK's documented API. Bounded bulk
/// hashing belongs on the executor (`runtime_hash`); this synchronous verb
/// is for small identity-sized inputs only.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn blake3_hash(data: Buffer) -> Buffer {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(&data);
    Buffer::from(digest.finalize().to_vec())
}

/// The engine's own sealed descriptor as data, lent to the
/// replication driver so one authority seals the theory.
/// Internal surface: not part of the SDK's documented API.
#[napi]
#[doc(hidden)]
#[allow(clippy::needless_pass_by_value)]
pub fn descriptor(env: Env, spec: Object) -> napi::Result<DescriptorWire> {
    use bumbledb::schema::ValidateDescriptor as _;
    let (descriptor, attrs) = match descriptor_of(&spec)? {
        Ok(parsed) => parsed,
        Err(OpenOutcome::SchemaError(message) | OpenOutcome::NewtypeMismatch(message)) => {
            return Err(marshal::throw_kind_message(
                env,
                tags::error_family::SCHEMA,
                message,
            ));
        }
    };
    let sealed = seal(descriptor, attrs);
    let schema = sealed.descriptor.clone().validate().map_err(|error| {
        marshal::throw_kind_message(env, tags::error_family::SCHEMA, error.to_string())
    })?;
    let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
    Ok(DescriptorWire {
        manifest: sealed.descriptor.manifest(),
        statements: sealed.statements,
        fingerprint: hex_fingerprint(&fingerprint.0),
        attrs: sealed.attrs,
    })
}

pub struct Sealed {
    pub(crate) descriptor: SchemaDescriptor,
    pub(crate) statements: Vec<StatementDescriptor>,
    /// The resident sealed field rosters, index = `RelationId` ordinal —
    /// computed once here, borrowed by every fact-lane call; the bridge
    /// re-derives nothing.
    pub(crate) rosters: Vec<marshal::SealedRoster>,
    /// The spec-only field attributes in the same sealed order — carried
    /// so the manifest wire speaks the spec's whole field vocabulary.
    pub(crate) attrs: FieldAttrsTable,
}

pub(crate) fn seal(descriptor: SchemaDescriptor, attrs: FieldAttrsTable) -> Sealed {
    let statements = descriptor.materialized_statements();
    let rosters = marshal::sealed_rosters(&descriptor);
    Sealed {
        descriptor,
        statements,
        rosters,
        attrs,
    }
}

pub(crate) type Engine = Db<SchemaDescriptor>;

pub(crate) fn hex_fingerprint(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

pub(crate) fn bind_value(value: &Value) -> BindValue<'_> {
    match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::F64(v) => BindValue::F64(*v),
        Value::Uuid(v) => BindValue::Uuid(*v),
        Value::String(text) => BindValue::Str(text),
        Value::FixedBytes(bytes) => BindValue::FixedBytes(bytes),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
        Value::IntervalF64(interval) => BindValue::IntervalF64(*interval),
    }
}

pub(crate) fn param_args(params: &[OwnedParam]) -> Vec<ParamArg<'_>> {
    params
        .iter()
        .map(|param| match param {
            OwnedParam::Set(values) => ParamArg::Set(values),
            OwnedParam::Scalar(value) => ParamArg::Scalar(bind_value(value)),
        })
        .collect()
}

pub(crate) fn violations_wire(
    descriptor: &SchemaDescriptor,
    violations: &Violations,
) -> Vec<ViolationWire> {
    render_rejection(descriptor, violations)
        .into_iter()
        .map(ViolationWire::from_rendered)
        .collect()
}

pub(crate) fn assemble_inner(
    db: Engine,
    descriptor: SchemaDescriptor,
    attrs: FieldAttrsTable,
) -> DbInner {
    DbInner {
        db: Arc::new(db),
        sealed: Arc::new(seal(descriptor, attrs)),
        writing: AtomicBool::new(false),
    }
}

/// The one database owner: a registry-held [`runtime::owners::ManagedDb`].
/// Every native DB lives in the one runtime registry behind a kernel-held
/// directory lock, so a retained JS wrapper can never keep an engine,
/// mapping, FD or directory lock alive after a completed close, and the
/// directory lock always belongs to the same native owner as its
/// environment and active operations.
pub struct DbHandle {
    inner: runtime::owners::ManagedDb,
}

impl DbHandle {
    pub(crate) fn managed(owner: runtime::owners::ManagedDb) -> Self {
        Self { inner: owner }
    }

    pub(crate) fn owner(&self) -> &runtime::owners::ManagedDb {
        &self.inner
    }
}

pub(crate) struct DbInner {
    pub(crate) db: Arc<Engine>,
    pub(crate) sealed: Arc<Sealed>,
    /// The single-writer admission flag: a live write session owns the
    /// engine writer; a second open refuses (`WriterBusy`) instead of
    /// parking a session thread on the writer mutex.
    pub(crate) writing: AtomicBool,
}

/// The two spec-resolution refusals `descriptor_of` can surface. This is
/// an internal error carrier only: database creation/open is the managed
/// runtime path (`runtime_directory_db_open` → `runtime_db_take`), which
/// renders these as its own `refused` wire arms.
pub enum OpenOutcome {
    SchemaError(String),
    NewtypeMismatch(String),
}

pub(crate) fn descriptor_of(
    spec: &Object,
) -> napi::Result<std::result::Result<(SchemaDescriptor, FieldAttrsTable), OpenOutcome>> {
    let spec = marshal::schema_spec(spec)?;
    let attrs = marshal::field_attrs(&spec);
    match spec.descriptor() {
        Ok(descriptor) => Ok(Ok((descriptor, attrs))),
        Err(error) => {
            let mismatched = error
                .issues()
                .iter()
                .any(|issue| matches!(issue, SpecIssue::StatementNewtypeMismatch { .. }));
            Ok(Err(if mismatched {
                OpenOutcome::NewtypeMismatch(error.to_string())
            } else {
                OpenOutcome::SchemaError(error.to_string())
            }))
        }
    }
}
