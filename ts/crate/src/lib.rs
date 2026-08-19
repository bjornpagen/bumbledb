//! The dumb-bridge law: no logic beyond marshaling will EVER live in this
//! crate. Anything smart belongs in the TypeScript SDK or the engine.
//!
//! # Threading model
//!
//! Store reads and writes invoke the JavaScript callback synchronously
//! inside `Db::read` / `Db::write` on the JavaScript thread. Instance and
//! transaction handles are raw pointers valid only while `alive` is set —
//! they are never transmuted to `'static` and never parked on a worker.
//! `InstanceBuilder::admit` is the surviving async native task (`Send`).
//!
//! # Error taxonomy
//!
//! Domain outcomes are DATA (admission and write tags). Language and
//! shape errors THROW. Engine errors carry a forced [`ErrorFamily`] kind
//! in `tags::error_family`.

use std::cell::{Cell, Ref, RefCell, RefMut, UnsafeCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::schema::{SpecIssue, StatementDescriptor};
use bumbledb::{
    Answers, BindValue, Db, Error, Exhumed, FieldId, FreshRange, InstanceBuilder, OwnedInstance,
    ParamArg, PreparedQuery, Query, RelationId, SchemaDescriptor, StatementId, Theory, Value,
    Violations, Witness, WriteTx, exhume, render_rejection,
};
use napi::bindgen_prelude::{
    Array, AsyncTask, BigInt, Env, External, FnArgs, Function, Object, Task, ToNapiValue, TypeName,
    Unknown, ValueType,
};
use napi::sys;
use napi_derive::napi;

#[cfg(test)]
mod fingerprint_lock;
mod marshal;
mod tags;

use marshal::{ExplainWire, ManifestWire, OwnedParam, StalenessWire, ValueOut, ViolationWire};

#[napi]
#[must_use]
pub fn engine_version() -> String {
    format!(
        "bumbledb-node {} (bumbledb storage format v{})",
        env!("CARGO_PKG_VERSION"),
        bumbledb::STORAGE_FORMAT_VERSION
    )
}

struct Sealed {
    descriptor: SchemaDescriptor,
    statements: Vec<StatementDescriptor>,
}

type Engine = Db<SchemaDescriptor>;

struct WireError {
    kind: Option<&'static str>,
    message: String,
}

fn wire(error: &Error) -> WireError {
    WireError {
        kind: Some(tags::error_family::tag(&error.family())),
        message: marshal::engine_message(error),
    }
}

fn bridge_error(message: String) -> WireError {
    WireError {
        kind: None,
        message,
    }
}

fn thrown(env: Env, error: WireError) -> napi::Error {
    match error.kind {
        Some(kind) => marshal::throw_kind_message(env, kind, error.message),
        None => marshal::err(error.message),
    }
}

fn abort_sentinel() -> Error {
    Error::from(std::io::Error::other("bumbledb-node transaction abort"))
}

fn closed_handle(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: use of a closed {what} handle"))
}

fn reentrant_use(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: re-entrant use of a {what} handle"))
}

fn throw_engine(env: Env, error: &Error) -> napi::Error {
    marshal::throw_engine(env, error)
}

fn take_handle<T>(cell: &RefCell<Option<T>>, what: &str) -> napi::Result<T> {
    let mut borrowed = cell.try_borrow_mut().map_err(|_| reentrant_use(what))?;
    borrowed.take().ok_or_else(|| closed_handle(what))
}

macro_rules! outcome_to_napi {
    ($ty:ty { $( $variant:ident $(( $($tuple:ident),+ ))? $({ $($field:ident),+ })? => { $($key:literal : $value:expr),+ $(,)? } ),+ $(,)? }) => {
        impl ToNapiValue for $ty {
            #[expect(
                unsafe_code,
                reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl only \
                          builds a plain object and delegates to napi's own impls"
            )]
            unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
                let env_handle = napi::Env::from_raw(env);
                let mut obj = Object::new(&env_handle)?;
                match val {
                    $(Self::$variant $(( $($tuple),+ ))? $({ $($field),+ })? => {
                        $(obj.set($key, $value)?;)+
                    })+
                }
                unsafe { Object::to_napi_value(env, obj) }
            }
        }
    };
}

fn live<'a, T>(cell: &'a RefCell<Option<T>>, what: &str) -> napi::Result<Ref<'a, T>> {
    let borrowed = cell.try_borrow().map_err(|_| reentrant_use(what))?;
    Ref::filter_map(borrowed, Option::as_ref).map_err(|_| closed_handle(what))
}

fn live_mut<'a, T>(cell: &'a RefCell<Option<T>>, what: &str) -> napi::Result<RefMut<'a, T>> {
    let borrowed = cell.try_borrow_mut().map_err(|_| reentrant_use(what))?;
    RefMut::filter_map(borrowed, Option::as_mut).map_err(|_| closed_handle(what))
}

fn require_alive(flag: &AtomicBool, what: &str) -> napi::Result<()> {
    if flag.load(Ordering::Acquire) {
        Ok(())
    } else {
        Err(closed_handle(what))
    }
}

fn hex_fingerprint(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn bind_value(value: &Value) -> BindValue<'_> {
    match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::String(text) => BindValue::Str(text),
        Value::FixedBytes(bytes) => BindValue::FixedBytes(bytes),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
    }
}

fn param_args(params: &[OwnedParam]) -> Result<Vec<ParamArg<'_>>, WireError> {
    params
        .iter()
        .map(|param| match param {
            OwnedParam::Set(values) => Ok(ParamArg::Set(values)),
            OwnedParam::Scalar(value) => Ok(ParamArg::Scalar(bind_value(value))),
        })
        .collect()
}

fn violations_wire(descriptor: &SchemaDescriptor, violations: &Violations) -> Vec<ViolationWire> {
    render_rejection(descriptor, violations)
        .into_iter()
        .map(ViolationWire::from_rendered)
        .collect()
}

fn assemble(db: Engine, descriptor: SchemaDescriptor) -> DbHandle {
    let statements = descriptor.materialized_statements();
    DbHandle {
        inner: RefCell::new(Some(DbInner {
            db: Arc::new(db),
            sealed: Arc::new(Sealed {
                descriptor,
                statements,
            }),
            writing: AtomicBool::new(false),
        })),
    }
}

// ---------------------------------------------------------------------------
// Db handle
// ---------------------------------------------------------------------------

pub struct DbHandle {
    inner: RefCell<Option<DbInner>>,
}

struct DbInner {
    db: Arc<Engine>,
    sealed: Arc<Sealed>,
    writing: AtomicBool,
}

pub enum CreateOutcome {
    Accepted(External<DbHandle>),
    Rejected(Vec<ViolationWire>),
    SchemaError(String),
    NewtypeMismatch(String),
}

outcome_to_napi!(CreateOutcome {
    Accepted(handle) => { "tag": tags::admission_tag::ACCEPTED, "db": handle },
    Rejected(violations) => { "tag": tags::admission_tag::REJECTED, "violations": violations },
    SchemaError(message) => { "tag": tags::open_kind::SCHEMA_ERROR, "message": message },
    NewtypeMismatch(message) => { "tag": tags::open_kind::NEWTYPE_MISMATCH, "message": message },
});

pub enum OpenOutcome {
    Ok(External<DbHandle>),
    SchemaError(String),
    NewtypeMismatch(String),
    FingerprintMismatch(String),
}

outcome_to_napi!(OpenOutcome {
    Ok(handle) => { "ok": true, "db": handle },
    SchemaError(message) => { "ok": false, "kind": tags::open_kind::SCHEMA_ERROR, "message": message },
    NewtypeMismatch(message) => { "ok": false, "kind": tags::open_kind::NEWTYPE_MISMATCH, "message": message },
    FingerprintMismatch(message) => { "ok": false, "kind": tags::open_kind::FINGERPRINT_MISMATCH, "message": message },
});

fn descriptor_of(spec: &Object) -> napi::Result<std::result::Result<SchemaDescriptor, OpenOutcome>> {
    let spec = marshal::schema_spec(spec)?;
    match spec.descriptor() {
        Ok(descriptor) => Ok(Ok(descriptor)),
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

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn db_create(env: Env, path: String, spec: Object) -> napi::Result<CreateOutcome> {
    let descriptor = match descriptor_of(&spec)? {
        Ok(descriptor) => descriptor,
        Err(OpenOutcome::SchemaError(message)) => return Ok(CreateOutcome::SchemaError(message)),
        Err(OpenOutcome::NewtypeMismatch(message)) => {
            return Ok(CreateOutcome::NewtypeMismatch(message));
        }
        Err(_) => unreachable!("descriptor_of only mints schema/newtype arms"),
    };
    match Db::create(std::path::Path::new(&path), descriptor.clone()) {
        Ok(bumbledb::Admission::Accepted(db)) => Ok(CreateOutcome::Accepted(External::new(
            assemble(db, descriptor),
        ))),
        Ok(bumbledb::Admission::Rejected(violations)) => Ok(CreateOutcome::Rejected(
            violations_wire(&descriptor, &violations),
        )),
        Err(Error::Schema(error)) => Ok(CreateOutcome::SchemaError(error.to_string())),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn db_open(env: Env, path: String, spec: Object) -> napi::Result<OpenOutcome> {
    let descriptor = match descriptor_of(&spec)? {
        Ok(descriptor) => descriptor,
        Err(outcome) => return Ok(outcome),
    };
    match Db::open(std::path::Path::new(&path), descriptor.clone()) {
        Ok(db) => Ok(OpenOutcome::Ok(External::new(assemble(db, descriptor)))),
        Err(Error::Schema(error)) => Ok(OpenOutcome::SchemaError(error.to_string())),
        Err(error @ Error::SchemaMismatch { .. }) => Ok(OpenOutcome::FingerprintMismatch(
            marshal::engine_message(&error),
        )),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

#[napi]
pub fn db_close(db: &External<DbHandle>) -> napi::Result<()> {
    take_handle(&db.inner, "db")?;
    Ok(())
}

#[napi]
pub fn db_manifest(db: &External<DbHandle>) -> napi::Result<ManifestWire> {
    let inner = live(&db.inner, "db")?;
    Ok(ManifestWire(inner.sealed.descriptor.clone().manifest()))
}

#[napi]
pub fn db_fingerprint(db: &External<DbHandle>) -> napi::Result<String> {
    use bumbledb::schema::ValidateDescriptor as _;
    let inner = live(&db.inner, "db")?;
    let schema = inner
        .sealed
        .descriptor
        .clone()
        .validate()
        .map_err(|error| marshal::err(error.to_string()))?;
    let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
    Ok(hex_fingerprint(&fingerprint.0))
}

#[napi]
pub fn db_generation(env: Env, db: &External<DbHandle>) -> napi::Result<u64> {
    let inner = live(&db.inner, "db")?;
    match inner.db.generation() {
        Ok(generation) => Ok(generation.value()),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn db_from_instance(
    env: Env,
    path: String,
    instance: &External<OwnedHandle>,
) -> napi::Result<External<DbHandle>> {
    let owned = live(&instance.inner, "owned instance")?;
    match Db::from_instance(std::path::Path::new(&path), &owned.instance) {
        Ok(db) => Ok(External::new(assemble(
            db,
            owned.sealed.descriptor.clone(),
        ))),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

// ---------------------------------------------------------------------------
// Exhume
// ---------------------------------------------------------------------------

pub struct ExhumeHandle {
    inner: RefCell<Option<Exhumed>>,
}

pub enum ExhumeOutcome {
    Ok(External<ExhumeHandle>),
    DescriptorMissing(String),
    FormatMismatch(String),
    Corruption(String),
}

outcome_to_napi!(ExhumeOutcome {
    Ok(handle) => { "ok": true, "exhume": handle },
    DescriptorMissing(message) => {
        "ok": false,
        "kind": tags::exhume_kind::DESCRIPTOR_MISSING,
        "message": message,
    },
    FormatMismatch(message) => {
        "ok": false,
        "kind": tags::exhume_kind::FORMAT_MISMATCH,
        "message": message,
    },
    Corruption(message) => {
        "ok": false,
        "kind": tags::exhume_kind::CORRUPTION,
        "message": message,
    },
});

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn db_exhume(env: Env, path: String) -> napi::Result<ExhumeOutcome> {
    match exhume(std::path::Path::new(&path)) {
        Ok(exhumed) => Ok(ExhumeOutcome::Ok(External::new(ExhumeHandle {
            inner: RefCell::new(Some(exhumed)),
        }))),
        Err(error @ Error::DescriptorMissing) => Ok(ExhumeOutcome::DescriptorMissing(
            marshal::engine_message(&error),
        )),
        Err(error @ Error::FormatMismatch { .. }) => Ok(ExhumeOutcome::FormatMismatch(
            marshal::engine_message(&error),
        )),
        Err(error @ Error::Corruption(_)) => {
            Ok(ExhumeOutcome::Corruption(marshal::engine_message(&error)))
        }
        Err(error) => Err(throw_engine(env, &error)),
    }
}

#[napi]
pub fn exhume_descriptor(exhume: &External<ExhumeHandle>) -> napi::Result<ManifestWire> {
    let exhumed = live(&exhume.inner, "exhume")?;
    Ok(ManifestWire(exhumed.descriptor().clone().manifest()))
}

#[napi]
pub fn exhume_close(exhume: &External<ExhumeHandle>) -> napi::Result<()> {
    take_handle(&exhume.inner, "exhume")?;
    Ok(())
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn exhume_scan(
    env: Env,
    exhume: &External<ExhumeHandle>,
    relation_name: String,
) -> napi::Result<Vec<Vec<ValueOut>>> {
    let exhumed = live(&exhume.inner, "exhume")?;
    let Some(relation) = exhumed.relation(&relation_name) else {
        return Err(marshal::err(format!(
            "bumbledb: the exhumed store's descriptor declares no relation `{relation_name}`"
        )));
    };
    let rows = exhumed.read(|snap| {
        let iter = snap.scan(relation)?;
        let mut rows = Vec::new();
        for row in iter {
            rows.push(row?);
        }
        Ok(rows)
    });
    match rows {
        Ok(rows) => marshal::rows_out(rows),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

// ---------------------------------------------------------------------------
// Borrowed instance + witness
// ---------------------------------------------------------------------------

enum InstanceKind {
    Store(*const ()),
    Heap {
        instance: *const OwnedInstance<SchemaDescriptor>,
        accounted: *const Cell<i64>,
    },
}

pub struct InstanceHandle {
    sealed: Arc<Sealed>,
    alive: Arc<AtomicBool>,
    kind: InstanceKind,
}

impl InstanceHandle {
    fn store(sealed: Arc<Sealed>, instance: &bumbledb::ReadInstance<'_, SchemaDescriptor>) -> Self {
        Self {
            sealed,
            alive: Arc::new(AtomicBool::new(true)),
            kind: InstanceKind::Store(std::ptr::from_ref(instance).cast()),
        }
    }

    fn heap(
        sealed: Arc<Sealed>,
        instance: &OwnedInstance<SchemaDescriptor>,
        accounted: &Cell<i64>,
    ) -> Self {
        Self {
            sealed,
            alive: Arc::new(AtomicBool::new(true)),
            kind: InstanceKind::Heap {
                instance: std::ptr::from_ref(instance),
                accounted: std::ptr::from_ref(accounted),
            },
        }
    }

    fn with_instance<R>(
        &self,
        body: impl FnOnce(&dyn InstanceOps) -> napi::Result<R>,
    ) -> napi::Result<R> {
        require_alive(self.alive.as_ref(), "instance")?;
        match self.kind {
            InstanceKind::Store(ptr) => {
                #[expect(
                    unsafe_code,
                    reason = "ptr is the Db::read argument; alive is cleared before that frame returns"
                )]
                let instance = unsafe { &*ptr.cast::<bumbledb::ReadInstance<'_, SchemaDescriptor>>() };
                body(&StoreOps(instance))
            }
            InstanceKind::Heap { instance, .. } => {
                #[expect(
                    unsafe_code,
                    reason = "ptr is the owned instance borrow; alive is cleared before owned_read returns"
                )]
                let instance = unsafe { &*instance };
                body(&HeapOps(instance))
            }
        }
    }

    fn sync_accounted(&self, env: Env) -> napi::Result<()> {
        let InstanceKind::Heap {
            instance,
            accounted,
        } = self.kind
        else {
            return Ok(());
        };
        #[expect(
            unsafe_code,
            reason = "both pointers are the OwnedSlot borrow held for owned_read"
        )]
        let (instance, accounted) = unsafe { (&*instance, &*accounted) };
        let want = i64::try_from(instance.retained_bytes()).unwrap_or(i64::MAX);
        let have = accounted.get();
        let delta = want.saturating_sub(have);
        if delta != 0 {
            env.adjust_external_memory(delta)?;
            accounted.set(want);
        }
        Ok(())
    }

    fn with_instance_accounted<R>(
        &self,
        env: Env,
        body: impl FnOnce(&dyn InstanceOps) -> napi::Result<R>,
    ) -> napi::Result<R> {
        let result = self.with_instance(body);
        self.sync_accounted(env)?;
        result
    }
}

trait InstanceOps {
    fn scan(&self, relation: RelationId) -> bumbledb::Result<Vec<Vec<Value>>>;
    fn contains(&self, relation: RelationId, values: &[Value]) -> bumbledb::Result<bool>;
    fn get(
        &self,
        relation: RelationId,
        key: StatementId,
        values: &[Value],
    ) -> bumbledb::Result<Option<Vec<Value>>>;
    fn prepare(&self, query: &Query) -> bumbledb::Result<PreparedQuery<SchemaDescriptor>>;
    fn execute(
        &self,
        prepared: &mut PreparedQuery<SchemaDescriptor>,
        params: &[OwnedParam],
    ) -> Result<Answers, WireError>;
    fn explain(
        &self,
        prepared: &mut PreparedQuery<SchemaDescriptor>,
        params: &[OwnedParam],
    ) -> Result<bumbledb::ExecutionStats, WireError>;
    fn staleness(
        &self,
        prepared: &PreparedQuery<SchemaDescriptor>,
    ) -> Result<StalenessWire, WireError>;
    fn generation(&self) -> Result<u64, WireError>;
}

struct StoreOps<'a>(&'a bumbledb::ReadInstance<'a, SchemaDescriptor>);
struct HeapOps<'a>(&'a OwnedInstance<SchemaDescriptor>);

fn scan_rows(
    iter: impl Iterator<Item = bumbledb::Result<Vec<Value>>>,
) -> bumbledb::Result<Vec<Vec<Value>>> {
    let mut rows = Vec::new();
    for row in iter {
        rows.push(row?);
    }
    Ok(rows)
}

impl InstanceOps for StoreOps<'_> {
    fn scan(&self, relation: RelationId) -> bumbledb::Result<Vec<Vec<Value>>> {
        scan_rows(self.0.scan(relation)?)
    }
    fn contains(&self, relation: RelationId, values: &[Value]) -> bumbledb::Result<bool> {
        self.0.contains_dyn(relation, values)
    }
    fn get(
        &self,
        relation: RelationId,
        key: StatementId,
        values: &[Value],
    ) -> bumbledb::Result<Option<Vec<Value>>> {
        self.0.get_dyn(relation, key, values)
    }
    fn prepare(&self, query: &Query) -> bumbledb::Result<PreparedQuery<SchemaDescriptor>> {
        self.0.prepare(query)
    }
    fn execute(
        &self,
        prepared: &mut PreparedQuery<SchemaDescriptor>,
        params: &[OwnedParam],
    ) -> Result<Answers, WireError> {
        let args = param_args(params)?;
        self.0.execute_collect(prepared, &args).map_err(|e| wire(&e))
    }
    fn explain(
        &self,
        prepared: &mut PreparedQuery<SchemaDescriptor>,
        params: &[OwnedParam],
    ) -> Result<bumbledb::ExecutionStats, WireError> {
        let args = param_args(params)?;
        let (_, stats) = self.0.profile(prepared, &args).map_err(|e| wire(&e))?;
        Ok(stats)
    }
    fn staleness(
        &self,
        prepared: &PreparedQuery<SchemaDescriptor>,
    ) -> Result<StalenessWire, WireError> {
        staleness_wire(prepared.staleness(self.0).map_err(|e| wire(&e))?)
    }
    fn generation(&self) -> Result<u64, WireError> {
        self.0
            .generation()
            .map(|generation| generation.value())
            .map_err(|error| wire(&error))
    }
}

impl InstanceOps for HeapOps<'_> {
    fn scan(&self, relation: RelationId) -> bumbledb::Result<Vec<Vec<Value>>> {
        scan_rows(self.0.scan(relation)?)
    }
    fn contains(&self, relation: RelationId, values: &[Value]) -> bumbledb::Result<bool> {
        self.0.contains_dyn(relation, values)
    }
    fn get(
        &self,
        relation: RelationId,
        key: StatementId,
        values: &[Value],
    ) -> bumbledb::Result<Option<Vec<Value>>> {
        self.0.get_dyn(relation, key, values)
    }
    fn prepare(&self, query: &Query) -> bumbledb::Result<PreparedQuery<SchemaDescriptor>> {
        self.0.prepare(query)
    }
    fn execute(
        &self,
        prepared: &mut PreparedQuery<SchemaDescriptor>,
        params: &[OwnedParam],
    ) -> Result<Answers, WireError> {
        let args = param_args(params)?;
        let mut answers = Answers::new();
        self.0
            .execute(prepared, &args, &mut answers)
            .map_err(|e| wire(&e))?;
        Ok(answers)
    }
    fn explain(
        &self,
        _prepared: &mut PreparedQuery<SchemaDescriptor>,
        _params: &[OwnedParam],
    ) -> Result<bumbledb::ExecutionStats, WireError> {
        Err(bridge_error(
            "bumbledb: profile is a store-read diagnostic, not an owned-instance method".into(),
        ))
    }
    fn staleness(
        &self,
        _prepared: &PreparedQuery<SchemaDescriptor>,
    ) -> Result<StalenessWire, WireError> {
        Err(bridge_error(
            "bumbledb: staleness is a store-read signal, not an owned-instance method".into(),
        ))
    }
    fn generation(&self) -> Result<u64, WireError> {
        Err(bridge_error(
            "bumbledb: generation is a store-read diagnostic, not an owned-instance method".into(),
        ))
    }
}

fn staleness_wire(staleness: bumbledb::Staleness) -> Result<StalenessWire, WireError> {
    Ok(match staleness {
        bumbledb::Staleness::NoStatistics => StalenessWire {
            per_occurrence: Vec::new(),
            max_ratio: 1.0,
        },
        bumbledb::Staleness::Measured {
            per_occurrence,
            max_ratio,
        } => StalenessWire {
            per_occurrence: per_occurrence
                .iter()
                .map(|drift| (drift.relation.0, drift.pinned, drift.live, drift.ratio))
                .collect(),
            max_ratio,
        },
    })
}

pub struct WitnessHandle {
    inner: RefCell<Option<Witness<SchemaDescriptor>>>,
}

#[napi]
pub fn db_read<'a>(
    env: Env,
    db: &'a External<DbHandle>,
    callback: Function<
        'a,
        FnArgs<(External<InstanceHandle>, External<WitnessHandle>)>,
        Unknown<'a>,
    >,
) -> napi::Result<Unknown<'a>> {
    let inner = live(&db.inner, "db")?;
    let sealed = Arc::clone(&inner.sealed);
    let engine = Arc::clone(&inner.db);
    let mut result = None;
    let mut js_error = None;
    let outcome = engine.read(|instance| {
        let witness = instance.witness()?;
        let instance_handle = InstanceHandle::store(Arc::clone(&sealed), instance);
        let alive = Arc::clone(&instance_handle.alive);
        let witness_handle = WitnessHandle {
            inner: RefCell::new(Some(witness)),
        };
        match callback.call(
            (
                External::new(instance_handle),
                External::new(witness_handle),
            )
                .into(),
        ) {
            Ok(value) => {
                alive.store(false, Ordering::Release);
                result = Some(value);
            }
            Err(error) => {
                alive.store(false, Ordering::Release);
                js_error = Some(error);
                return Err(abort_sentinel());
            }
        }
        Ok(())
    });
    if let Some(error) = js_error {
        return Err(error);
    }
    match outcome {
        Ok(()) => Ok(result.ok_or_else(|| {
            marshal::err("bumbledb: read callback produced no value".into())
        })?),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

#[napi]
pub fn instance_generation(env: Env, instance: &External<InstanceHandle>) -> napi::Result<u64> {
    instance.with_instance(|ops| ops.generation().map_err(|error| thrown(env, error)))
}

#[napi]
pub fn instance_scan(
    env: Env,
    instance: &External<InstanceHandle>,
    relation: u32,
) -> napi::Result<Vec<Vec<ValueOut>>> {
    let rows = instance.with_instance_accounted(env, |ops| {
        ops.scan(RelationId(relation))
            .map_err(|error| throw_engine(env, &error))
    })?;
    marshal::rows_out(rows)
}

#[napi]
pub fn instance_contains(
    env: Env,
    instance: &External<InstanceHandle>,
    relation: u32,
    values: Array,
) -> napi::Result<bool> {
    let row = {
        let sealed = &instance.sealed;
        marshal::fact_row(&sealed.descriptor, relation, &values)?
    };
    instance.with_instance_accounted(env, |ops| {
        ops.contains(row.0, &row.1)
            .map_err(|error| throw_engine(env, &error))
    })
}

#[napi]
pub fn instance_get(
    env: Env,
    instance: &External<InstanceHandle>,
    relation: u32,
    key_statement: u32,
    key_values: Array,
) -> napi::Result<Option<Vec<ValueOut>>> {
    let (rel, key, row) = marshal::key_row(
        &instance.sealed.descriptor,
        &instance.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = instance.with_instance_accounted(env, |ops| {
        ops.get(rel, key, &row)
            .map_err(|error| throw_engine(env, &error))
    })?;
    found
        .map(|values| values.into_iter().map(ValueOut::from_value).collect())
        .transpose()
}

#[napi]
pub fn witness_close(witness: &External<WitnessHandle>) -> napi::Result<()> {
    let _ = take_handle(&witness.inner, "witness")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------

pub struct TxHandle {
    sealed: Arc<Sealed>,
    /// The `Db` that minted this write — valid only while `alive`.
    /// A clone of `Arc<Engine>` here would keep the exclusive lock alive
    /// after the callback returns (JS may retain the External).
    engine: *const Engine,
    alive: Arc<AtomicBool>,
    tx: *mut (),
}

impl TxHandle {
    fn mint(sealed: Arc<Sealed>, engine: &Engine, tx: &mut WriteTx<'_, SchemaDescriptor>) -> Self {
        Self {
            sealed,
            engine: std::ptr::from_ref(engine),
            alive: Arc::new(AtomicBool::new(true)),
            tx: std::ptr::from_mut(tx).cast(),
        }
    }

    fn engine(&self) -> napi::Result<&Engine> {
        require_alive(self.alive.as_ref(), "transaction")?;
        #[expect(
            unsafe_code,
            reason = "ptr is the Db that owns this write; alive is cleared before that frame returns"
        )]
        Ok(unsafe { &*self.engine })
    }

    fn tx(&self) -> napi::Result<&mut WriteTx<'_, SchemaDescriptor>> {
        require_alive(self.alive.as_ref(), "transaction")?;
        #[expect(
            unsafe_code,
            reason = "ptr is the Db::write argument; alive is cleared before that frame returns"
        )]
        Ok(unsafe { &mut *self.tx.cast() })
    }
}

pub enum WriteOutcome {
    Accepted(u64),
    Rejected(Vec<ViolationWire>),
    Aborted,
    Moved { witnessed: u64, current: u64 },
}

outcome_to_napi!(WriteOutcome {
    Accepted(generation) => { "tag": tags::write_tag::ACCEPTED, "generation": generation },
    Rejected(violations) => { "tag": tags::write_tag::REJECTED, "violations": violations },
    Aborted => { "tag": tags::write_tag::ABANDONED },
    Moved { witnessed, current } => {
        "tag": tags::write_tag::MOVED,
        "witnessed": witnessed,
        "current": current,
    },
});

enum WriteExit {
    Proceed,
    Abort,
    Throw(napi::Error),
}

fn run_write(
    env: Env,
    inner: &DbInner,
    witness: Option<&Witness<SchemaDescriptor>>,
    callback: Function<External<TxHandle>, bool>,
) -> napi::Result<WriteOutcome> {
    if inner.writing.swap(true, Ordering::AcqRel) {
        return Err(marshal::err(
            "bumbledb: a write transaction is already open on this db handle \
             (single-writer engine; finish the enclosing write first)"
                .into(),
        ));
    }
    let _guard = WriteFlag(&inner.writing);
    let sealed = Arc::clone(&inner.sealed);
    let engine = Arc::clone(&inner.db);
    let mut exit = WriteExit::Proceed;
    let body = |tx: &mut WriteTx<'_, SchemaDescriptor>| -> bumbledb::Result<()> {
        let handle = TxHandle::mint(Arc::clone(&sealed), engine.as_ref(), tx);
        let alive = Arc::clone(&handle.alive);
        match callback.call(External::new(handle)) {
            Ok(true) => {
                alive.store(false, Ordering::Release);
            }
            Ok(false) => {
                alive.store(false, Ordering::Release);
                exit = WriteExit::Abort;
                return Err(abort_sentinel());
            }
            Err(error) => {
                alive.store(false, Ordering::Release);
                exit = WriteExit::Throw(error);
                return Err(abort_sentinel());
            }
        }
        Ok(())
    };
    let result = match witness {
        None => match inner.db.write(body) {
            Ok(bumbledb::Admission::Accepted(committed)) => {
                Ok(bumbledb::ConditionalWrite::Accepted(committed))
            }
            Ok(bumbledb::Admission::Rejected(violations)) => {
                Ok(bumbledb::ConditionalWrite::Rejected(violations))
            }
            Err(error) => Err(error),
        },
        Some(witness) => inner.db.write_from(witness, body),
    };
    match exit {
        WriteExit::Throw(error) => return Err(error),
        WriteExit::Abort => return Ok(WriteOutcome::Aborted),
        WriteExit::Proceed => {}
    }
    match result {
        Ok(bumbledb::ConditionalWrite::Accepted(committed)) => {
            Ok(WriteOutcome::Accepted(committed.generation.value()))
        }
        Ok(bumbledb::ConditionalWrite::Rejected(violations)) => Ok(WriteOutcome::Rejected(
            violations_wire(&sealed.descriptor, &violations),
        )),
        Ok(bumbledb::ConditionalWrite::Moved { witnessed, current }) => Ok(WriteOutcome::Moved {
            witnessed: witnessed.value(),
            current: current.value(),
        }),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

struct WriteFlag<'a>(&'a AtomicBool);
impl Drop for WriteFlag<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[napi]
pub fn db_write(
    env: Env,
    db: &External<DbHandle>,
    callback: Function<External<TxHandle>, bool>,
) -> napi::Result<WriteOutcome> {
    let inner = live(&db.inner, "db")?;
    run_write(env, &inner, None, callback)
}

#[napi]
pub fn db_write_from(
    env: Env,
    db: &External<DbHandle>,
    witness: &External<WitnessHandle>,
    callback: Function<External<TxHandle>, bool>,
) -> napi::Result<WriteOutcome> {
    let inner = live(&db.inner, "db")?;
    let witness = live(&witness.inner, "witness")?;
    run_write(env, &inner, Some(&witness), callback)
}

pub enum MutationReportWire {
    Report { submitted: u64, changed: u64 },
}

outcome_to_napi!(MutationReportWire {
    Report { submitted, changed } => { "submitted": submitted, "changed": changed },
});

pub enum FreshRangeWire {
    Empty,
    NonEmpty { start: u64, end_exclusive: u64 },
}

outcome_to_napi!(FreshRangeWire {
    Empty => { "empty": true },
    NonEmpty { start, end_exclusive } => { "empty": false, "start": start, "endExclusive": end_exclusive },
});

#[napi]
pub fn tx_insert(
    env: Env,
    tx: &External<TxHandle>,
    relation: u32,
    rows: Array,
) -> napi::Result<MutationReportWire> {
    let facts = marshal::fact_rows(&tx.sealed.descriptor, relation, &rows)?;
    let report = tx
        .tx()?
        .insert_dyn(facts.0, facts.1)
        .map_err(|e| throw_engine(env, &e))?;
    Ok(MutationReportWire::Report {
        submitted: report.submitted(),
        changed: report.changed(),
    })
}

#[napi]
pub fn tx_delete(
    env: Env,
    tx: &External<TxHandle>,
    relation: u32,
    rows: Array,
) -> napi::Result<MutationReportWire> {
    let facts = marshal::fact_rows(&tx.sealed.descriptor, relation, &rows)?;
    let report = tx
        .tx()?
        .delete_dyn(facts.0, facts.1)
        .map_err(|e| throw_engine(env, &e))?;
    Ok(MutationReportWire::Report {
        submitted: report.submitted(),
        changed: report.changed(),
    })
}

#[napi]
pub fn tx_contains(
    env: Env,
    tx: &External<TxHandle>,
    relation: u32,
    values: Array,
) -> napi::Result<bool> {
    let row = marshal::fact_row(&tx.sealed.descriptor, relation, &values)?;
    tx.tx()?
        .contains_dyn(row.0, &row.1)
        .map_err(|e| throw_engine(env, &e))
}

#[napi]
pub fn tx_get(
    env: Env,
    tx: &External<TxHandle>,
    relation: u32,
    key_statement: u32,
    key_values: Array,
) -> napi::Result<Option<Vec<ValueOut>>> {
    let (rel, key, row) = marshal::key_row(
        &tx.sealed.descriptor,
        &tx.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = tx
        .tx()?
        .get_dyn(rel, key, &row)
        .map_err(|e| throw_engine(env, &e))?;
    found
        .map(|values| values.into_iter().map(ValueOut::from_value).collect())
        .transpose()
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn tx_reserve(
    env: Env,
    tx: &External<TxHandle>,
    relation: u32,
    field: u32,
    count: BigInt,
) -> napi::Result<FreshRangeWire> {
    let field = u16::try_from(field)
        .map_err(|_| marshal::err(format!("bumbledb marshal: field id {field} exceeds u16")))?;
    let count = marshal::u64_in(&count, "reserve count")?;
    let fresh = tx
        .engine()?
        .fresh_field(RelationId(relation), FieldId(field))
        .map_err(|error| throw_engine(env, &Error::FactShape(error)))?;
    let range = tx
        .tx()?
        .reserve_at(fresh, count)
        .map_err(|e| throw_engine(env, &e))?;
    Ok(match range {
        FreshRange::Empty => FreshRangeWire::Empty,
        FreshRange::NonEmpty { start, count } => FreshRangeWire::NonEmpty {
            start,
            end_exclusive: start + count.get(),
        },
    })
}

// ---------------------------------------------------------------------------
// Prepared
// ---------------------------------------------------------------------------

pub struct PreparedHandle {
    inner: RefCell<Option<PreparedInner>>,
}

struct PreparedInner {
    prepared: UnsafeCell<PreparedQuery<SchemaDescriptor>>,
}

pub enum PrepareOutcome {
    Ok(External<PreparedHandle>),
    IrError(String),
}

outcome_to_napi!(PrepareOutcome {
    Ok(handle) => { "ok": true, "prepared": handle },
    IrError(message) => { "ok": false, "kind": tags::prepare_kind::IR_ERROR, "message": message },
});

fn wrap_prepared(prepared: PreparedQuery<SchemaDescriptor>) -> External<PreparedHandle> {
    External::new(PreparedHandle {
        inner: RefCell::new(Some(PreparedInner {
            prepared: UnsafeCell::new(prepared),
        })),
    })
}

fn prepared_mut(
    prepared: &External<PreparedHandle>,
) -> napi::Result<RefMut<'_, PreparedQuery<SchemaDescriptor>>> {
    let inner = live_mut(&prepared.inner, "prepared query")?;
    #[expect(
        unsafe_code,
        reason = "execute needs &mut PreparedQuery; JS is single-threaded and the handle is !shared"
    )]
    Ok(RefMut::map(inner, |inner| unsafe {
        &mut *inner.prepared.get()
    }))
}

fn prepared_ref(
    prepared: &External<PreparedHandle>,
) -> napi::Result<Ref<'_, PreparedQuery<SchemaDescriptor>>> {
    let inner = live(&prepared.inner, "prepared query")?;
    #[expect(
        unsafe_code,
        reason = "staleness needs &PreparedQuery; JS is single-threaded"
    )]
    Ok(Ref::map(inner, |inner| unsafe { &*inner.prepared.get() }))
}

fn prepare_outcome(
    env: Env,
    result: bumbledb::Result<PreparedQuery<SchemaDescriptor>>,
) -> napi::Result<PrepareOutcome> {
    match result {
        Ok(prepared) => Ok(PrepareOutcome::Ok(wrap_prepared(prepared))),
        Err(Error::Validation(error)) => Ok(PrepareOutcome::IrError(error.to_string())),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

#[napi]
pub fn instance_prepare(
    env: Env,
    instance: &External<InstanceHandle>,
    query: Object,
) -> napi::Result<PrepareOutcome> {
    let query = marshal::query_in(&query)?;
    let result = instance.with_instance_accounted(env, |ops| Ok(ops.prepare(&query)))?;
    prepare_outcome(env, result)
}

#[napi]
pub fn db_prepare(env: Env, db: &External<DbHandle>, query: Object) -> napi::Result<PrepareOutcome> {
    let inner = live(&db.inner, "db")?;
    let query = marshal::query_in(&query)?;
    prepare_outcome(env, inner.db.prepare(&query))
}

#[napi]
pub fn prepared_execute(
    env: Env,
    prepared: &External<PreparedHandle>,
    instance: &External<InstanceHandle>,
    params: Array,
) -> napi::Result<Vec<Vec<ValueOut>>> {
    let params = marshal::params_in(&params)?;
    let mut prepared = prepared_mut(prepared)?;
    let answers = instance.with_instance_accounted(env, |ops| {
        ops.execute(&mut prepared, &params)
            .map_err(|error| thrown(env, error))
    })?;
    Ok(marshal::answers_out(&answers))
}

#[napi]
pub fn prepared_explain(
    env: Env,
    prepared: &External<PreparedHandle>,
    instance: &External<InstanceHandle>,
    params: Array,
) -> napi::Result<ExplainWire> {
    let params = marshal::params_in(&params)?;
    let mut prepared = prepared_mut(prepared)?;
    let stats = instance.with_instance_accounted(env, |ops| {
        ops.explain(&mut prepared, &params)
            .map_err(|error| thrown(env, error))
    })?;
    Ok(ExplainWire(stats))
}

#[napi]
pub fn prepared_staleness(
    env: Env,
    prepared: &External<PreparedHandle>,
    instance: &External<InstanceHandle>,
) -> napi::Result<StalenessWire> {
    let prepared = prepared_ref(prepared)?;
    instance.with_instance(|ops| {
        ops.staleness(&prepared)
            .map_err(|error| thrown(env, error))
    })
}

#[napi]
pub fn prepared_close(prepared: &External<PreparedHandle>) -> napi::Result<()> {
    take_handle(&prepared.inner, "prepared query")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Builder / owned instance
// ---------------------------------------------------------------------------

pub struct BuilderHandle {
    inner: RefCell<Option<InstanceBuilder<SchemaDescriptor>>>,
    sealed: Arc<Sealed>,
}

pub struct OwnedHandle {
    inner: RefCell<Option<OwnedSlot>>,
}

struct OwnedSlot {
    instance: OwnedInstance<SchemaDescriptor>,
    sealed: Arc<Sealed>,
    accounted: Cell<i64>,
}

fn account_bytes(env: Env, bytes: usize) -> napi::Result<i64> {
    let delta = i64::try_from(bytes).unwrap_or(i64::MAX);
    if delta != 0 {
        env.adjust_external_memory(delta)?;
    }
    Ok(delta)
}

fn release_accounted(env: Env, accounted: i64) -> napi::Result<()> {
    if accounted != 0 {
        env.adjust_external_memory(-accounted)?;
    }
    Ok(())
}

#[napi]
pub fn instance_builder_new(env: Env, spec: Object) -> napi::Result<External<BuilderHandle>> {
    let descriptor = match descriptor_of(&spec)? {
        Ok(descriptor) => descriptor,
        Err(OpenOutcome::SchemaError(message) | OpenOutcome::NewtypeMismatch(message)) => {
            return Err(marshal::throw_kind_message(
                env,
                tags::error_family::SCHEMA,
                message,
            ));
        }
        Err(_) => unreachable!("descriptor_of only mints schema/newtype arms"),
    };
    let statements = descriptor.materialized_statements();
    let builder = InstanceBuilder::new(descriptor.clone())
        .map_err(|error| throw_engine(env, &error))?;
    Ok(External::new(BuilderHandle {
        inner: RefCell::new(Some(builder)),
        sealed: Arc::new(Sealed {
            descriptor,
            statements,
        }),
    }))
}

#[napi]
pub fn instance_builder_load(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    rows: Array,
) -> napi::Result<MutationReportWire> {
    let facts = marshal::fact_rows(&builder.sealed.descriptor, relation, &rows)?;
    let report = live_mut(&builder.inner, "builder")?
        .load_dyn(facts.0, facts.1)
        .map_err(|error| throw_engine(env, &error))?;
    Ok(MutationReportWire::Report {
        submitted: report.submitted(),
        changed: report.changed(),
    })
}

#[napi]
pub fn instance_builder_close(builder: &External<BuilderHandle>) -> napi::Result<()> {
    take_handle(&builder.inner, "builder")?;
    Ok(())
}

pub struct AdmitTask {
    builder: Option<InstanceBuilder<SchemaDescriptor>>,
    sealed: Arc<Sealed>,
}

pub enum AdmitOutput {
    Accepted(OwnedInstance<SchemaDescriptor>),
    Rejected(Vec<ViolationWire>),
    Failed {
        kind: &'static str,
        message: String,
    },
}

impl Task for AdmitTask {
    type Output = AdmitOutput;
    type JsValue = AdmitOutcome;

    fn compute(&mut self) -> napi::Result<AdmitOutput> {
        let builder = self
            .builder
            .take()
            .ok_or_else(|| marshal::err("bumbledb: builder already admitted".into()))?;
        match builder.admit() {
            Ok(bumbledb::Admission::Accepted(instance)) => Ok(AdmitOutput::Accepted(instance)),
            Ok(bumbledb::Admission::Rejected(violations)) => Ok(AdmitOutput::Rejected(
                violations_wire(&self.sealed.descriptor, &violations),
            )),
            Err(error) => Ok(AdmitOutput::Failed {
                kind: tags::error_family::tag(&error.family()),
                message: marshal::engine_message(&error),
            }),
        }
    }

    fn resolve(&mut self, env: Env, output: AdmitOutput) -> napi::Result<AdmitOutcome> {
        match output {
            AdmitOutput::Accepted(instance) => {
                let accounted = account_bytes(env, instance.retained_bytes())?;
                Ok(AdmitOutcome::Accepted(External::new(OwnedHandle {
                    inner: RefCell::new(Some(OwnedSlot {
                        instance,
                        sealed: Arc::clone(&self.sealed),
                        accounted: Cell::new(accounted),
                    })),
                })))
            }
            AdmitOutput::Rejected(violations) => Ok(AdmitOutcome::Rejected(violations)),
            AdmitOutput::Failed { kind, message } => {
                Err(marshal::throw_kind_message(env, kind, message))
            }
        }
    }
}

pub enum AdmitOutcome {
    Accepted(External<OwnedHandle>),
    Rejected(Vec<ViolationWire>),
}

impl TypeName for AdmitOutcome {
    fn type_name() -> &'static str {
        "object"
    }

    fn value_type() -> ValueType {
        ValueType::Object
    }
}

outcome_to_napi!(AdmitOutcome {
    Accepted(handle) => { "tag": tags::admission_tag::ACCEPTED, "value": handle },
    Rejected(violations) => { "tag": tags::admission_tag::REJECTED, "violations": violations },
});

#[napi]
pub fn instance_builder_admit(
    builder: &External<BuilderHandle>,
) -> napi::Result<AsyncTask<AdmitTask>> {
    let taken = take_handle(&builder.inner, "builder")?;
    Ok(AsyncTask::new(AdmitTask {
        builder: Some(taken),
        sealed: Arc::clone(&builder.sealed),
    }))
}

#[napi]
pub fn owned_instance_close(
    env: Env,
    instance: &External<OwnedHandle>,
) -> napi::Result<()> {
    let slot = take_handle(&instance.inner, "owned instance")?;
    release_accounted(env, slot.accounted.get())
}

#[napi]
pub fn owned_read<'a>(
    instance: &'a External<OwnedHandle>,
    callback: Function<'a, External<InstanceHandle>, Unknown<'a>>,
) -> napi::Result<Unknown<'a>> {
    let owned = live(&instance.inner, "owned instance")?;
    let handle = InstanceHandle::heap(
        Arc::clone(&owned.sealed),
        &owned.instance,
        &owned.accounted,
    );
    let alive = Arc::clone(&handle.alive);
    let result = callback.call(External::new(handle));
    alive.store(false, Ordering::Release);
    result
}

#[cfg(test)]
mod handle_lifecycle {
    use super::*;

    #[test]
    fn take_handle_reentrant_borrow_is_typed_error() {
        let cell = RefCell::new(Some(1_u8));
        let error = {
            let _guard = cell.borrow();
            take_handle(&cell, "db").expect_err("re-entrant take must not panic")
        };
        assert!(
            error.reason.contains("re-entrant use of a db handle"),
            "{}",
            error.reason
        );
        assert!(
            cell.borrow().is_some(),
            "re-entrant take must not spend the handle"
        );
    }

    #[test]
    fn take_handle_closed_is_typed_error() {
        let cell = RefCell::new(None::<u8>);
        let error = take_handle(&cell, "instance").expect_err("empty handle");
        assert!(
            error.reason.contains("closed instance handle"),
            "{}",
            error.reason
        );
    }

    #[test]
    fn take_handle_spends_the_value() {
        let cell = RefCell::new(Some(7_u8));
        assert_eq!(take_handle(&cell, "db").expect("live handle"), 7);
        assert!(cell.borrow().is_none());
    }
}
