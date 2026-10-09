//! The Node bridge: marshaling between JavaScript and the engine, nothing
//! smarter. Every native resource is owned by the one runtime registry
//! (`runtime_wire.rs`); databases live behind kernel-held directory owners,
//! `!Send` engine state lives in worker-affine tables, and every operation is
//! registered, cancellable and drainable.
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bumbledb::{BindValue, Db, ParamArg, SchemaDescriptor, Value, Violations, render_rejection};
use napi_derive::napi;

mod bindings;
pub mod db_wire;
pub mod hosted;
pub mod input;
pub mod marshal;
pub mod query;
mod runtime;
pub mod runtime_wire;
pub mod schema;
pub use runtime::publication::runtime_arm_publication_cancel;
mod tags;

use marshal::{OwnedParam, ViolationOut};

#[napi]
#[must_use]
pub fn engine_version() -> String {
    format!(
        "bumbledb-node {} (bumbledb storage format v{})",
        env!("CARGO_PKG_VERSION"),
        bumbledb::STORAGE_FORMAT_VERSION
    )
}

pub(crate) type Engine = Db<SchemaDescriptor>;

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

pub(crate) fn violations_out(
    descriptor: &SchemaDescriptor,
    violations: &Violations,
) -> Vec<ViolationOut> {
    render_rejection(descriptor, violations)
        .into_iter()
        .map(Into::into)
        .collect()
}

pub(crate) fn assemble_inner(db: Engine, schema: Arc<schema::SchemaHandle>) -> DbInner {
    DbInner {
        db: Arc::new(db),
        schema,
        writing: AtomicBool::new(false),
    }
}

/// The one database owner: a registry-held [`runtime::owners::ManagedDb`].
/// Every native database lives in the runtime registry behind a kernel-held
/// directory lock, so a retained JS wrapper never keeps an engine, mapping,
/// file descriptor or lock alive after a completed close.
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
    pub(crate) schema: Arc<schema::SchemaHandle>,
    /// The single-writer admission flag: a live write owns the engine
    /// writer, and a second write refuses (`WriterBusy`) instead of parking
    /// a worker on the writer mutex.
    pub(crate) writing: AtomicBool,
}
