//! The dumb-bridge law: no logic beyond marshaling will EVER live in this
//! crate. Anything smart belongs in the TypeScript SDK or the engine.
use std::cell::{Cell, Ref, RefCell, RefMut, UnsafeCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::schema::{SpecIssue, StatementDescriptor};
use bumbledb::{
    Answers, BindValue, Db, Error, FieldId, FreshRange, InstanceBuilder, MutationReport,
    OwnedInstance, ParamArg, PreparedQuery, Query, RelationId, SchemaDescriptor, StatementId,
    Theory, Value, Violations, Witness, WriteTx, render_rejection,
};
use napi::bindgen_prelude::{
    Array, AsyncTask, BigInt, Buffer, Env, External, FnArgs, Function, Object, Task, ToNapiValue,
    TypeName, Unknown, ValueType,
};
use napi::sys;
use napi_derive::napi;

#[cfg(test)]
mod fingerprint_lock;
pub mod log;
mod marshal;
mod tags;

use marshal::{DescriptorWire, FieldAttrs, ManifestWire, OwnedParam, ValueOut, ViolationWire};

/// Per-relation, sealed-order spec attribute rows (`fresh` + newtype) —
/// the spec-only half of the field vocabulary the descriptor drops.
type FieldAttrsTable = Vec<Vec<FieldAttrs>>;

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
/// Internal surface: not part of the SDK's documented API.
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
        Err(_) => unreachable!("descriptor_of only mints schema/newtype arms"),
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

struct Sealed {
    descriptor: SchemaDescriptor,
    statements: Vec<StatementDescriptor>,
    /// The resident sealed field rosters, index = `RelationId` ordinal —
    /// computed once here, borrowed by every fact-lane call; the bridge
    /// re-derives nothing.
    rosters: Vec<marshal::SealedRoster>,
    /// The spec-only field attributes in the same sealed order — carried
    /// so the manifest wire speaks the spec's whole field vocabulary.
    attrs: FieldAttrsTable,
}

fn seal(descriptor: SchemaDescriptor, attrs: FieldAttrsTable) -> Sealed {
    let statements = descriptor.materialized_statements();
    let rosters = marshal::sealed_rosters(&descriptor);
    Sealed {
        descriptor,
        statements,
        rosters,
        attrs,
    }
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

fn leased_handle(what: &str) -> napi::Error {
    marshal::err(format!("bumbledb: {what} is leased for publish"))
}

fn engine_failed(error: &Error) -> (&'static str, String) {
    (
        tags::error_family::tag(&error.family()),
        marshal::engine_message(error),
    )
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

fn assemble(db: Engine, descriptor: SchemaDescriptor, attrs: FieldAttrsTable) -> DbHandle {
    DbHandle {
        inner: RefCell::new(Some(DbInner {
            db: Arc::new(db),
            sealed: Arc::new(seal(descriptor, attrs)),
            writing: AtomicBool::new(false),
        })),
    }
}

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

impl TypeName for CreateOutcome {
    fn type_name() -> &'static str {
        "object"
    }

    fn value_type() -> ValueType {
        ValueType::Object
    }
}

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

impl TypeName for OpenOutcome {
    fn type_name() -> &'static str {
        "object"
    }

    fn value_type() -> ValueType {
        ValueType::Object
    }
}

fn descriptor_of(
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

enum CreatePrep {
    Descriptor(SchemaDescriptor, FieldAttrsTable),
    SchemaError(String),
    NewtypeMismatch(String),
}

#[allow(
    clippy::large_enum_variant,
    reason = "the Task output owns the Engine handle until resolve; boxing adds a hop without shrinking the JS wire type"
)]
pub enum CreateOutput {
    Accepted(Engine, SchemaDescriptor, FieldAttrsTable),
    Rejected(Vec<ViolationWire>),
    SchemaError(String),
    NewtypeMismatch(String),
    Failed { kind: &'static str, message: String },
}

pub struct CreateTask {
    path: String,
    prep: Option<CreatePrep>,
}

impl Task for CreateTask {
    type Output = CreateOutput;
    type JsValue = CreateOutcome;

    fn compute(&mut self) -> napi::Result<CreateOutput> {
        let prep = self
            .prep
            .take()
            .ok_or_else(|| marshal::err("bumbledb: create already computed".into()))?;
        let (descriptor, attrs) = match prep {
            CreatePrep::SchemaError(message) => return Ok(CreateOutput::SchemaError(message)),
            CreatePrep::NewtypeMismatch(message) => {
                return Ok(CreateOutput::NewtypeMismatch(message));
            }
            CreatePrep::Descriptor(descriptor, attrs) => (descriptor, attrs),
        };
        match Db::create(std::path::Path::new(&self.path), descriptor.clone()) {
            Ok(bumbledb::Admission::Accepted(db)) => {
                Ok(CreateOutput::Accepted(db, descriptor, attrs))
            }
            Ok(bumbledb::Admission::Rejected(violations)) => Ok(CreateOutput::Rejected(
                violations_wire(&descriptor, &violations),
            )),
            Err(Error::Schema(error)) => Ok(CreateOutput::SchemaError(error.to_string())),
            Err(error) => {
                let (kind, message) = engine_failed(&error);
                Ok(CreateOutput::Failed { kind, message })
            }
        }
    }

    fn resolve(&mut self, _env: Env, output: CreateOutput) -> napi::Result<CreateOutcome> {
        match output {
            CreateOutput::Accepted(db, descriptor, attrs) => Ok(CreateOutcome::Accepted(
                External::new(assemble(db, descriptor, attrs)),
            )),
            CreateOutput::Rejected(violations) => Ok(CreateOutcome::Rejected(violations)),
            CreateOutput::SchemaError(message) => Ok(CreateOutcome::SchemaError(message)),
            CreateOutput::NewtypeMismatch(message) => Ok(CreateOutcome::NewtypeMismatch(message)),
            CreateOutput::Failed { kind: _, message } => Err(napi::Error::from_reason(message)),
        }
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn db_create(path: String, spec: Object) -> napi::Result<AsyncTask<CreateTask>> {
    let prep = match descriptor_of(&spec)? {
        Ok((descriptor, attrs)) => CreatePrep::Descriptor(descriptor, attrs),
        Err(OpenOutcome::SchemaError(message)) => CreatePrep::SchemaError(message),
        Err(OpenOutcome::NewtypeMismatch(message)) => CreatePrep::NewtypeMismatch(message),
        Err(_) => unreachable!("descriptor_of only mints schema/newtype arms"),
    };
    Ok(AsyncTask::new(CreateTask {
        path,
        prep: Some(prep),
    }))
}

enum OpenPrep {
    Descriptor(SchemaDescriptor, FieldAttrsTable),
    SchemaError(String),
    NewtypeMismatch(String),
}

#[allow(
    clippy::large_enum_variant,
    reason = "the Task output owns the Engine handle until resolve; boxing adds a hop without shrinking the JS wire type"
)]
pub enum OpenOutput {
    Ok(Engine, SchemaDescriptor, FieldAttrsTable),
    SchemaError(String),
    NewtypeMismatch(String),
    FingerprintMismatch(String),
    Failed { kind: &'static str, message: String },
}

pub struct OpenTask {
    path: String,
    prep: Option<OpenPrep>,
}

impl Task for OpenTask {
    type Output = OpenOutput;
    type JsValue = OpenOutcome;

    fn compute(&mut self) -> napi::Result<OpenOutput> {
        let prep = self
            .prep
            .take()
            .ok_or_else(|| marshal::err("bumbledb: open already computed".into()))?;
        let (descriptor, attrs) = match prep {
            OpenPrep::SchemaError(message) => return Ok(OpenOutput::SchemaError(message)),
            OpenPrep::NewtypeMismatch(message) => return Ok(OpenOutput::NewtypeMismatch(message)),
            OpenPrep::Descriptor(descriptor, attrs) => (descriptor, attrs),
        };
        match Db::open(std::path::Path::new(&self.path), descriptor.clone()) {
            Ok(db) => Ok(OpenOutput::Ok(db, descriptor, attrs)),
            Err(Error::Schema(error)) => Ok(OpenOutput::SchemaError(error.to_string())),
            Err(error @ Error::SchemaMismatch { .. }) => Ok(OpenOutput::FingerprintMismatch(
                marshal::engine_message(&error),
            )),
            Err(error) => {
                let (kind, message) = engine_failed(&error);
                Ok(OpenOutput::Failed { kind, message })
            }
        }
    }

    fn resolve(&mut self, _env: Env, output: OpenOutput) -> napi::Result<OpenOutcome> {
        match output {
            OpenOutput::Ok(db, descriptor, attrs) => Ok(OpenOutcome::Ok(External::new(assemble(
                db, descriptor, attrs,
            )))),
            OpenOutput::SchemaError(message) => Ok(OpenOutcome::SchemaError(message)),
            OpenOutput::NewtypeMismatch(message) => Ok(OpenOutcome::NewtypeMismatch(message)),
            OpenOutput::FingerprintMismatch(message) => {
                Ok(OpenOutcome::FingerprintMismatch(message))
            }
            OpenOutput::Failed { kind: _, message } => Err(napi::Error::from_reason(message)),
        }
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn db_open(path: String, spec: Object) -> napi::Result<AsyncTask<OpenTask>> {
    let prep = match descriptor_of(&spec)? {
        Ok((descriptor, attrs)) => OpenPrep::Descriptor(descriptor, attrs),
        Err(OpenOutcome::SchemaError(message)) => OpenPrep::SchemaError(message),
        Err(OpenOutcome::NewtypeMismatch(message)) => OpenPrep::NewtypeMismatch(message),
        Err(_) => unreachable!("descriptor_of only mints schema/newtype arms"),
    };
    Ok(AsyncTask::new(OpenTask {
        path,
        prep: Some(prep),
    }))
}

#[napi]
pub fn db_close(db: &External<DbHandle>) -> napi::Result<()> {
    take_handle(&db.inner, "db")?;
    Ok(())
}

#[napi]
pub fn db_manifest(db: &External<DbHandle>) -> napi::Result<ManifestWire> {
    let inner = live(&db.inner, "db")?;
    Ok(ManifestWire {
        manifest: inner.sealed.descriptor.clone().manifest(),
        attrs: inner.sealed.attrs.clone(),
    })
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

/// blake3 over the canonical catalog enumeration — the replication
/// equality oracle: equal digests imply identical judged content
/// regardless of page layout or allocation history.
#[napi]
pub fn db_catalog_digest(env: Env, db: &External<DbHandle>) -> napi::Result<Buffer> {
    let inner = live(&db.inner, "db")?;
    match inner.db.catalog_digest() {
        Ok(digest) => Ok(Buffer::from(digest.to_vec())),
        Err(error) => Err(throw_engine(env, &error)),
    }
}

#[allow(
    clippy::large_enum_variant,
    reason = "the Task output owns the Engine handle until resolve; boxing adds a hop without shrinking the JS wire type"
)]
pub enum PublishOutput {
    Ok(Engine),
    Failed { kind: &'static str, message: String },
}

pub struct PublishTask {
    path: String,
    instance: Arc<OwnedInstance<SchemaDescriptor>>,
    sealed: Arc<Sealed>,
    leased: Arc<AtomicBool>,
}

pub struct PublishedHandle(External<DbHandle>);

impl TypeName for PublishedHandle {
    fn type_name() -> &'static str {
        "external"
    }

    fn value_type() -> ValueType {
        ValueType::External
    }
}

impl ToNapiValue for PublishedHandle {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; this forwards the External"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        unsafe { External::to_napi_value(env, val.0) }
    }
}

impl Task for PublishTask {
    type Output = PublishOutput;
    type JsValue = PublishedHandle;

    fn compute(&mut self) -> napi::Result<PublishOutput> {
        match Db::from_instance(std::path::Path::new(&self.path), self.instance.as_ref()) {
            Ok(db) => Ok(PublishOutput::Ok(db)),
            Err(error) => {
                let (kind, message) = engine_failed(&error);
                Ok(PublishOutput::Failed { kind, message })
            }
        }
    }

    fn resolve(&mut self, _env: Env, output: PublishOutput) -> napi::Result<PublishedHandle> {
        match output {
            PublishOutput::Ok(db) => Ok(PublishedHandle(External::new(assemble(
                db,
                self.sealed.descriptor.clone(),
                self.sealed.attrs.clone(),
            )))),
            PublishOutput::Failed { kind: _, message } => Err(napi::Error::from_reason(message)),
        }
    }
}

impl Drop for PublishTask {
    fn drop(&mut self) {
        self.leased.store(false, Ordering::Release);
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn db_from_instance(
    path: String,
    instance: &External<OwnedHandle>,
) -> napi::Result<AsyncTask<PublishTask>> {
    let owned = live(&instance.inner, "owned instance")?;
    if owned.leased.swap(true, Ordering::AcqRel) {
        return Err(leased_handle("owned instance"));
    }
    Ok(AsyncTask::new(PublishTask {
        path,
        instance: Arc::clone(&owned.instance),
        sealed: Arc::clone(&owned.sealed),
        leased: Arc::clone(&owned.leased),
    }))
}

pub struct InstanceHandle {
    sealed: Arc<Sealed>,
    alive: Arc<AtomicBool>,
    instance: *const (),
}

impl InstanceHandle {
    fn store(sealed: Arc<Sealed>, instance: &bumbledb::ReadInstance<'_, SchemaDescriptor>) -> Self {
        Self {
            sealed,
            alive: Arc::new(AtomicBool::new(true)),
            instance: std::ptr::from_ref(instance).cast(),
        }
    }

    fn with_instance<R>(
        &self,
        body: impl FnOnce(&dyn InstanceOps) -> napi::Result<R>,
    ) -> napi::Result<R> {
        require_alive(self.alive.as_ref(), "instance")?;
        #[expect(
            unsafe_code,
            reason = "ptr is the Db::read argument; alive is cleared before that frame returns"
        )]
        let instance = unsafe {
            &*self
                .instance
                .cast::<bumbledb::ReadInstance<'_, SchemaDescriptor>>()
        };
        body(&StoreOps(instance))
    }
}

trait InstanceOps {
    fn scan(&self, relation: RelationId) -> bumbledb::Result<Vec<Vec<Value>>>;
    fn count(&self, relation: RelationId) -> bumbledb::Result<u64>;
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
    fn count(&self, relation: RelationId) -> bumbledb::Result<u64> {
        self.0.count(relation)
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
        self.0
            .execute_collect(prepared, &args)
            .map_err(|e| wire(&e))
    }
    fn generation(&self) -> Result<u64, WireError> {
        self.0
            .generation()
            .map(bumbledb::GenerationId::value)
            .map_err(|error| wire(&error))
    }
}

impl InstanceOps for HeapOps<'_> {
    fn scan(&self, relation: RelationId) -> bumbledb::Result<Vec<Vec<Value>>> {
        scan_rows(self.0.scan(relation)?)
    }
    fn count(&self, relation: RelationId) -> bumbledb::Result<u64> {
        self.0.count(relation)
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
    fn generation(&self) -> Result<u64, WireError> {
        Err(bridge_error(
            "bumbledb: generation is a store-read diagnostic, not an owned-instance method".into(),
        ))
    }
}

pub struct WitnessHandle {
    inner: RefCell<Option<Witness<SchemaDescriptor>>>,
}

#[napi]
#[allow(
    clippy::needless_pass_by_value,
    clippy::type_complexity,
    reason = "napi Function is the callback token; the generic is the JS arity, not a rustc type we own"
)]
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
        Ok(()) => Ok(result
            .ok_or_else(|| marshal::err("bumbledb: read callback produced no value".into()))?),
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
    let rows = instance.with_instance(|ops| {
        ops.scan(RelationId(relation))
            .map_err(|error| throw_engine(env, &error))
    })?;
    Ok(marshal::rows_out(rows))
}

#[napi]
pub fn instance_count(
    env: Env,
    instance: &External<InstanceHandle>,
    relation: u32,
) -> napi::Result<u64> {
    instance.with_instance(|ops| {
        ops.count(RelationId(relation))
            .map_err(|error| throw_engine(env, &error))
    })
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
        marshal::fact_row(&sealed.rosters, relation, &values)?
    };
    instance.with_instance(|ops| {
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
        &instance.sealed.rosters,
        &instance.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = instance.with_instance(|ops| {
        ops.get(rel, key, &row)
            .map_err(|error| throw_engine(env, &error))
    })?;
    Ok(found.map(|values| values.into_iter().map(ValueOut::from_value).collect()))
}

#[napi]
pub fn witness_close(witness: &External<WitnessHandle>) -> napi::Result<()> {
    let _ = take_handle(&witness.inner, "witness")?;
    Ok(())
}

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

    #[allow(
        clippy::mut_from_ref,
        reason = "the write token is a raw pointer into the Db::write frame; alive is cleared before that frame returns"
    )]
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

#[allow(
    clippy::needless_pass_by_value,
    reason = "napi Function is the callback token; the JS thread consumes it by value"
)]
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
#[allow(
    clippy::needless_pass_by_value,
    reason = "napi Function is the callback token; the JS thread consumes it by value"
)]
pub fn db_write(
    env: Env,
    db: &External<DbHandle>,
    callback: Function<External<TxHandle>, bool>,
) -> napi::Result<WriteOutcome> {
    let inner = live(&db.inner, "db")?;
    run_write(env, &inner, None, callback)
}

#[napi]
#[allow(
    clippy::needless_pass_by_value,
    reason = "napi Function is the callback token; the JS thread consumes it by value"
)]
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

fn mutation_report_wire(report: MutationReport) -> MutationReportWire {
    MutationReportWire::Report {
        submitted: report.submitted(),
        changed: report.changed(),
    }
}

fn fresh_range_wire(range: FreshRange<u64>) -> FreshRangeWire {
    match range {
        FreshRange::Empty => FreshRangeWire::Empty,
        FreshRange::NonEmpty { start, count } => FreshRangeWire::NonEmpty {
            start,
            end_exclusive: start + count.get(),
        },
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn tx_insert(
    env: Env,
    tx: &External<TxHandle>,
    relation: u32,
    rows: BigInt,
    cells: Array,
) -> napi::Result<MutationReportWire> {
    let rows = marshal::u64_in(&rows, "collection rows")?;
    let collection = marshal::accepted_collection(env, &tx.sealed.rosters, relation, rows, &cells)?;
    let report = tx
        .tx()?
        .insert_accepted(&collection)
        .map_err(|e| throw_engine(env, &e))?;
    Ok(mutation_report_wire(report))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn tx_delete(
    env: Env,
    tx: &External<TxHandle>,
    relation: u32,
    rows: BigInt,
    cells: Array,
) -> napi::Result<MutationReportWire> {
    let rows = marshal::u64_in(&rows, "collection rows")?;
    let collection = marshal::accepted_collection(env, &tx.sealed.rosters, relation, rows, &cells)?;
    let report = tx
        .tx()?
        .delete_accepted(&collection)
        .map_err(|e| throw_engine(env, &e))?;
    Ok(mutation_report_wire(report))
}

#[napi]
pub fn tx_contains(
    env: Env,
    tx: &External<TxHandle>,
    relation: u32,
    values: Array,
) -> napi::Result<bool> {
    let row = marshal::fact_row(&tx.sealed.rosters, relation, &values)?;
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
        &tx.sealed.rosters,
        &tx.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = tx
        .tx()?
        .get_dyn(rel, key, &row)
        .map_err(|e| throw_engine(env, &e))?;
    Ok(found.map(|values| values.into_iter().map(ValueOut::from_value).collect()))
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
    Ok(fresh_range_wire(range))
}

pub struct PreparedHandle {
    inner: RefCell<Option<PreparedInner>>,
}

struct PreparedInner {
    prepared: UnsafeCell<PreparedQuery<SchemaDescriptor>>,
}

#[allow(
    clippy::large_enum_variant,
    reason = "the prepared handle is the Ok payload; boxing would add a hop on the one success path"
)]
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
    let result = instance.with_instance(|ops| Ok(ops.prepare(&query)))?;
    prepare_outcome(env, result)
}

#[napi]
pub fn db_prepare(
    env: Env,
    db: &External<DbHandle>,
    query: Object,
) -> napi::Result<PrepareOutcome> {
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
    let answers = instance.with_instance(|ops| {
        ops.execute(&mut prepared, &params)
            .map_err(|error| thrown(env, error))
    })?;
    Ok(marshal::answers_out(&answers))
}

#[napi]
pub fn prepared_close(prepared: &External<PreparedHandle>) -> napi::Result<()> {
    take_handle(&prepared.inner, "prepared query")?;
    Ok(())
}

pub struct BuilderHandle {
    inner: RefCell<Option<InstanceBuilder<SchemaDescriptor>>>,
    sealed: Arc<Sealed>,
}

pub struct OwnedHandle {
    inner: RefCell<Option<OwnedSlot>>,
}

struct OwnedSlot {
    instance: Arc<OwnedInstance<SchemaDescriptor>>,
    sealed: Arc<Sealed>,
    accounted: Cell<i64>,
    leased: Arc<AtomicBool>,
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

fn sync_owned_accounted(env: Env, slot: &OwnedSlot) -> napi::Result<()> {
    let want = i64::try_from(slot.instance.retained_bytes()).unwrap_or(i64::MAX);
    let have = slot.accounted.get();
    let delta = want.saturating_sub(have);
    if delta != 0 {
        env.adjust_external_memory(delta)?;
        slot.accounted.set(want);
    }
    Ok(())
}

fn owned_ops<R>(
    env: Env,
    instance: &External<OwnedHandle>,
    body: impl FnOnce(&HeapOps<'_>, &Sealed) -> napi::Result<R>,
) -> napi::Result<R> {
    let slot = live(&instance.inner, "owned instance")?;
    let result = body(&HeapOps(slot.instance.as_ref()), &slot.sealed);
    sync_owned_accounted(env, &slot)?;
    result
}

#[napi]
pub fn instance_builder_new(env: Env, spec: Object) -> napi::Result<External<BuilderHandle>> {
    let (descriptor, attrs) = match descriptor_of(&spec)? {
        Ok(parsed) => parsed,
        Err(OpenOutcome::SchemaError(message) | OpenOutcome::NewtypeMismatch(message)) => {
            return Err(marshal::throw_kind_message(
                env,
                tags::error_family::SCHEMA,
                message,
            ));
        }
        Err(_) => unreachable!("descriptor_of only mints schema/newtype arms"),
    };
    let builder =
        InstanceBuilder::new(descriptor.clone()).map_err(|error| throw_engine(env, &error))?;
    Ok(External::new(BuilderHandle {
        inner: RefCell::new(Some(builder)),
        sealed: Arc::new(seal(descriptor, attrs)),
    }))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn instance_builder_load(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    rows: BigInt,
    cells: Array,
) -> napi::Result<MutationReportWire> {
    let rows = marshal::u64_in(&rows, "collection rows")?;
    let collection =
        marshal::accepted_collection(env, &builder.sealed.rosters, relation, rows, &cells)?;
    let report = live_mut(&builder.inner, "builder")?
        .load_accepted(&collection)
        .map_err(|error| throw_engine(env, &error))?;
    Ok(mutation_report_wire(report))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn instance_builder_delete(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    rows: BigInt,
    cells: Array,
) -> napi::Result<MutationReportWire> {
    let rows = marshal::u64_in(&rows, "collection rows")?;
    let collection =
        marshal::accepted_collection(env, &builder.sealed.rosters, relation, rows, &cells)?;
    let report = live_mut(&builder.inner, "builder")?
        .delete_accepted(&collection)
        .map_err(|error| throw_engine(env, &error))?;
    Ok(mutation_report_wire(report))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn instance_builder_reserve(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    field: u32,
    count: BigInt,
) -> napi::Result<FreshRangeWire> {
    let field = u16::try_from(field)
        .map_err(|_| marshal::err(format!("bumbledb marshal: field id {field} exceeds u16")))?;
    let count = marshal::u64_in(&count, "reserve count")?;
    let mut inner = live_mut(&builder.inner, "builder")?;
    let fresh = inner
        .fresh_field(RelationId(relation), FieldId(field))
        .map_err(|error| throw_engine(env, &Error::FactShape(error)))?;
    let range = inner
        .reserve_at(fresh, count)
        .map_err(|error| throw_engine(env, &error))?;
    Ok(fresh_range_wire(range))
}

#[napi]
pub fn instance_builder_contains(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    values: Array,
) -> napi::Result<bool> {
    let row = marshal::fact_row(&builder.sealed.rosters, relation, &values)?;
    live_mut(&builder.inner, "builder")?
        .contains_dyn(row.0, &row.1)
        .map_err(|error| throw_engine(env, &error))
}

#[napi]
pub fn instance_builder_get(
    env: Env,
    builder: &External<BuilderHandle>,
    relation: u32,
    key_statement: u32,
    key_values: Array,
) -> napi::Result<Option<Vec<ValueOut>>> {
    let (rel, key, row) = marshal::key_row(
        &builder.sealed.rosters,
        &builder.sealed.statements,
        relation,
        key_statement,
        &key_values,
    )?;
    let found = live_mut(&builder.inner, "builder")?
        .get_dyn(rel, key, &row)
        .map_err(|error| throw_engine(env, &error))?;
    Ok(found.map(|values| values.into_iter().map(ValueOut::from_value).collect()))
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
    Failed { kind: &'static str, message: String },
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
                        instance: Arc::new(instance),
                        sealed: Arc::clone(&self.sealed),
                        accounted: Cell::new(accounted),
                        leased: Arc::new(AtomicBool::new(false)),
                    })),
                })))
            }
            AdmitOutput::Rejected(violations) => Ok(AdmitOutcome::Rejected(violations)),
            AdmitOutput::Failed { kind: _, message } => Err(napi::Error::from_reason(message)),
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
pub fn owned_instance_close(env: Env, instance: &External<OwnedHandle>) -> napi::Result<()> {
    {
        let slot = live(&instance.inner, "owned instance")?;
        if slot.leased.load(Ordering::Acquire) {
            return Err(leased_handle("owned instance"));
        }
    }
    let slot = take_handle(&instance.inner, "owned instance")?;
    release_accounted(env, slot.accounted.get())
}

#[napi]
pub fn owned_scan(
    env: Env,
    instance: &External<OwnedHandle>,
    relation: u32,
) -> napi::Result<Vec<Vec<ValueOut>>> {
    let rows = owned_ops(env, instance, |ops, _sealed| {
        ops.scan(RelationId(relation))
            .map_err(|error| throw_engine(env, &error))
    })?;
    Ok(marshal::rows_out(rows))
}

#[napi]
pub fn owned_count(env: Env, instance: &External<OwnedHandle>, relation: u32) -> napi::Result<u64> {
    owned_ops(env, instance, |ops, _sealed| {
        ops.count(RelationId(relation))
            .map_err(|error| throw_engine(env, &error))
    })
}

#[napi]
pub fn owned_contains(
    env: Env,
    instance: &External<OwnedHandle>,
    relation: u32,
    values: Array,
) -> napi::Result<bool> {
    owned_ops(env, instance, |ops, sealed| {
        let row = marshal::fact_row(&sealed.rosters, relation, &values)?;
        ops.contains(row.0, &row.1)
            .map_err(|error| throw_engine(env, &error))
    })
}

#[napi]
pub fn owned_get(
    env: Env,
    instance: &External<OwnedHandle>,
    relation: u32,
    key_statement: u32,
    key_values: Array,
) -> napi::Result<Option<Vec<ValueOut>>> {
    let found = owned_ops(env, instance, |ops, sealed| {
        let (rel, key, row) = marshal::key_row(
            &sealed.rosters,
            &sealed.statements,
            relation,
            key_statement,
            &key_values,
        )?;
        ops.get(rel, key, &row)
            .map_err(|error| throw_engine(env, &error))
    })?;
    Ok(found.map(|values| values.into_iter().map(ValueOut::from_value).collect()))
}

#[napi]
pub fn owned_prepare(
    env: Env,
    instance: &External<OwnedHandle>,
    query: Object,
) -> napi::Result<PrepareOutcome> {
    let query = marshal::query_in(&query)?;
    let result = owned_ops(env, instance, |ops, _sealed| Ok(ops.prepare(&query)))?;
    prepare_outcome(env, result)
}

#[napi]
pub fn owned_execute(
    env: Env,
    prepared: &External<PreparedHandle>,
    instance: &External<OwnedHandle>,
    params: Array,
) -> napi::Result<Vec<Vec<ValueOut>>> {
    let params = marshal::params_in(&params)?;
    let mut prepared = prepared_mut(prepared)?;
    let answers = owned_ops(env, instance, |ops, _sealed| {
        ops.execute(&mut prepared, &params)
            .map_err(|error| thrown(env, error))
    })?;
    Ok(marshal::answers_out(&answers))
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
