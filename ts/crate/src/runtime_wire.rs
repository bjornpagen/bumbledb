//! Exact-version Node ownership boundary for the shared native executor.
#![allow(
    clippy::needless_pass_by_value,
    reason = "N-API owns value argument conversion"
)]
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bumbledb::work::{ByteKind, ExecutionPolicy, Resource, WorkError};
use napi::bindgen_prelude::{
    BigInt, Buffer, Env, External, FromNapiValue, Function, JsValue, Object, Uint8Array, Unknown,
};
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_derive::napi;

use crate::runtime::{
    CloseReport, Inspection, Operation, Options, Output, Phase, Runtime, RuntimeError,
};

static LIVE: Mutex<Option<Arc<Runtime>>> = Mutex::new(None);
static ADDON_IDENTITY: u8 = 0;

pub struct RuntimeHandle {
    identity: usize,
    runtime: Arc<Runtime>,
}
pub struct OperationHandle {
    identity: usize,
    runtime: Arc<Runtime>,
    operation: Arc<Operation>,
}
pub struct DirectoryHandle {
    identity: usize,
    owner: crate::runtime::owners::DirectoryOwner,
}

fn identity() -> usize {
    std::ptr::from_ref(&ADDON_IDENTITY) as usize
}

impl Drop for RuntimeHandle {
    fn drop(&mut self) {
        self.runtime.begin_close();
    }
}

impl Drop for OperationHandle {
    fn drop(&mut self) {
        self.runtime.drain(Some(&self.operation), Box::new(|_| {}));
    }
}

fn owner(handle: &RuntimeHandle) -> Result<&Arc<Runtime>, RuntimeError> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    Ok(&handle.runtime)
}

#[napi(object)]
pub struct RuntimeOptionsWire {
    pub workers: f64,
    pub queue_capacity: f64,
    pub cleanup_capacity: f64,
    pub owner_capacity: f64,
    pub native_handle_capacity: f64,
    pub input_bytes: BigInt,
    pub working_bytes: BigInt,
    pub scratch_bytes: BigInt,
    pub result_bytes: BigInt,
    pub chunk_bytes: BigInt,
    pub cleanup_timeout_ms: f64,
}

#[napi(object)]
pub struct PolicyWire {
    pub input_bytes: BigInt,
    pub working_bytes: BigInt,
    pub scratch_bytes: BigInt,
    pub result_bytes: BigInt,
    pub rows: BigInt,
    pub work_units: BigInt,
    pub timeout_ms: f64,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "validated exact unsigned 32-bit integer before cast"
)]
fn unsigned(value: f64) -> Result<u32, RuntimeError> {
    if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) || value.fract() != 0.0 {
        return Err(RuntimeError::InvalidArgument);
    }
    Ok(value as u32)
}

fn integer(value: &BigInt) -> Result<u64, RuntimeError> {
    let (negative, value, lossless) = value.get_u64();
    if negative || !lossless {
        return Err(RuntimeError::InvalidArgument);
    }
    Ok(value)
}

impl PolicyWire {
    fn parse(&self) -> Result<ExecutionPolicy, RuntimeError> {
        Ok(ExecutionPolicy {
            input_bytes: integer(&self.input_bytes)?,
            working_bytes: integer(&self.working_bytes)?,
            scratch_bytes: integer(&self.scratch_bytes)?,
            result_bytes: integer(&self.result_bytes)?,
            rows: integer(&self.rows)?,
            work_units: integer(&self.work_units)?,
            timeout: Duration::from_millis(u64::from(unsigned(self.timeout_ms)?)),
        })
    }
}

pub const ERROR_CODES: &[&str] = &[
    "RuntimeAlreadyLive",
    "ForeignRuntime",
    "ClosedHandle",
    "SpentHandle",
    "QueueFull",
    "InvalidArgument",
    "Internal",
    "DirectoryBusy",
    "InvalidPath",
    "Io",
    "ResourceLimit",
    "Cancelled",
    "DeadlineExceeded",
];

fn error_code(error: &RuntimeError) -> &'static str {
    match error {
        RuntimeError::RuntimeAlreadyLive => "RuntimeAlreadyLive",
        RuntimeError::ForeignRuntime => "ForeignRuntime",
        RuntimeError::ClosedHandle => "ClosedHandle",
        RuntimeError::SpentHandle => "SpentHandle",
        RuntimeError::QueueFull => "QueueFull",
        RuntimeError::InvalidArgument | RuntimeError::Work(WorkError::InvalidTimeout) => {
            "InvalidArgument"
        }
        RuntimeError::Internal => "Internal",
        RuntimeError::DirectoryBusy => "DirectoryBusy",
        RuntimeError::InvalidPath => "InvalidPath",
        RuntimeError::Io { .. } => "Io",
        RuntimeError::ResourceLimit { .. } | RuntimeError::Work(WorkError::Exhausted { .. }) => {
            "ResourceLimit"
        }
        RuntimeError::Work(WorkError::Cancelled) => "Cancelled",
        RuntimeError::Work(WorkError::DeadlineExceeded) => "DeadlineExceeded",
    }
}

fn resource_name(resource: Resource) -> &'static str {
    match resource {
        Resource::InputBytes => "inputBytes",
        Resource::WorkingBytes => "workingBytes",
        Resource::ScratchBytes => "scratchBytes",
        Resource::ResultBytes => "resultBytes",
        Resource::Rows => "rows",
        Resource::WorkUnits => "workUnits",
    }
}

pub(crate) fn thrown(env: Env, error: RuntimeError) -> napi::Error {
    let make = || -> napi::Result<()> {
        let mut object = Object::new(&env)?;
        object.set("_tag", error_code(&error))?;
        if let RuntimeError::Io { kind, code } = error {
            object.set("kind", format!("{kind:?}"))?;
            object.set("osCode", code)?;
        }
        let details = match error {
            RuntimeError::ResourceLimit {
                dimension,
                used,
                requested,
                limit,
            } => Some((dimension, used, requested, limit)),
            RuntimeError::Work(WorkError::Exhausted {
                resource,
                used,
                requested,
                limit,
            }) => Some((resource_name(resource), used, requested, limit)),
            _ => None,
        };
        if let Some((dimension, used, requested, limit)) = details {
            object.set("dimension", dimension)?;
            object.set("used", BigInt::from(used))?;
            object.set("requested", BigInt::from(requested))?;
            object.set("limit", BigInt::from(limit))?;
        }
        env.throw(object)
    };
    make()
        .err()
        .unwrap_or_else(|| napi::Error::from_status(napi::Status::PendingException))
}

#[napi]
pub fn runtime_error_codes() -> Vec<String> {
    ERROR_CODES.iter().map(ToString::to_string).collect()
}

#[napi]
pub fn runtime_open(
    env: Env,
    options: RuntimeOptionsWire,
) -> napi::Result<External<RuntimeHandle>> {
    let parse = || -> Result<Options, RuntimeError> {
        Ok(Options {
            workers: unsigned(options.workers)? as usize,
            queue_capacity: unsigned(options.queue_capacity)? as usize,
            cleanup_capacity: unsigned(options.cleanup_capacity)? as usize,
            owner_capacity: unsigned(options.owner_capacity)? as usize,
            native_handle_capacity: unsigned(options.native_handle_capacity)? as usize,
            aggregate_bytes: [
                integer(&options.input_bytes)?,
                integer(&options.working_bytes)?,
                integer(&options.scratch_bytes)?,
                integer(&options.result_bytes)?,
            ],
            chunk_bytes: integer(&options.chunk_bytes)?,
            cleanup_timeout: Duration::from_millis(u64::from(unsigned(
                options.cleanup_timeout_ms,
            )?)),
        })
    };
    let options = parse().map_err(|error| thrown(env, error))?;
    let mut live = LIVE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if live
        .as_ref()
        .is_some_and(|runtime| runtime.inspect().phase != Phase::Closed)
    {
        return Err(thrown(env, RuntimeError::RuntimeAlreadyLive));
    }
    let runtime = Runtime::start(options).map_err(|error| thrown(env, error))?;
    *live = Some(Arc::clone(&runtime));
    Ok(External::new(RuntimeHandle {
        identity: identity(),
        runtime,
    }))
}

#[napi(object)]
pub struct InspectionWire {
    pub phase: String,
    pub queued: BigInt,
    pub active: BigInt,
    pub retained: BigInt,
    pub owners: BigInt,
    pub databases: BigInt,
    pub input_bytes: BigInt,
    pub working_bytes: BigInt,
    pub scratch_bytes: BigInt,
    pub result_bytes: BigInt,
}

impl From<Inspection> for InspectionWire {
    fn from(value: Inspection) -> Self {
        Self {
            phase: match value.phase {
                Phase::Open => "open",
                Phase::Closing => "closing",
                Phase::Closed => "closed",
            }
            .into(),
            queued: BigInt::from(value.queued as u64),
            active: BigInt::from(value.active as u64),
            retained: BigInt::from(value.retained as u64),
            owners: BigInt::from(value.owners as u64),
            databases: BigInt::from(value.databases as u64),
            input_bytes: BigInt::from(value.reserved[0]),
            working_bytes: BigInt::from(value.reserved[1]),
            scratch_bytes: BigInt::from(value.reserved[2]),
            result_bytes: BigInt::from(value.reserved[3]),
        }
    }
}

#[napi(object)]
pub struct CloseWire {
    pub kind: String,
    pub outstanding: Option<InspectionWire>,
}

impl From<CloseReport> for CloseWire {
    fn from(value: CloseReport) -> Self {
        match value {
            CloseReport::Closed => Self {
                kind: "closed".into(),
                outstanding: None,
            },
            CloseReport::Incomplete(inspection) => Self {
                kind: "incomplete".into(),
                outstanding: Some(inspection.into()),
            },
            CloseReport::Failed => Self {
                kind: "failed".into(),
                outstanding: None,
            },
        }
    }
}

#[napi]
pub fn runtime_close(
    env: Env,
    handle: &External<RuntimeHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let callback = callback
        .build_threadsafe_function()
        .callee_handled::<false>()
        .max_queue_size::<1>()
        .build()?;
    runtime.drain(
        None,
        Box::new(move |report| {
            let _ = callback.call(report.into(), ThreadsafeFunctionCallMode::NonBlocking);
        }),
    );
    Ok(())
}

#[napi]
pub fn runtime_cancel(
    env: Env,
    handle: &External<OperationHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let callback = callback
        .build_threadsafe_function()
        .callee_handled::<false>()
        .max_queue_size::<1>()
        .build()?;
    handle.runtime.drain(
        Some(&handle.operation),
        Box::new(move |report| {
            let _ = callback.call(report.into(), ThreadsafeFunctionCallMode::NonBlocking);
        }),
    );
    Ok(())
}

#[napi]
pub fn runtime_inspect(env: Env, handle: &External<RuntimeHandle>) -> napi::Result<InspectionWire> {
    Ok(owner(handle)
        .map_err(|error| thrown(env, error))?
        .inspect()
        .into())
}

#[napi]
pub fn runtime_ready(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let callback = callback
        .build_threadsafe_function()
        .callee_handled::<false>()
        .max_queue_size::<1>()
        .build()?;
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            Box::new(move || {
                let _ = callback.call((), ThreadsafeFunctionCallMode::NonBlocking);
            }),
            |_| {
                Ok(Box::new(|context| {
                    context.step(1)?;
                    Ok(Output::Ready)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(External::new(OperationHandle {
        identity: identity(),
        runtime: Arc::clone(runtime),
        operation,
    }))
}

#[expect(
    unsafe_code,
    reason = "N-API exposes the actual typed-array backing only through its raw API; validate before constructing a Rust slice"
)]
fn unshared_input(env: Env, value: Unknown, maximum: u64) -> napi::Result<Uint8Array> {
    let mut kind = 0;
    let mut length = 0;
    let mut data = std::ptr::null_mut();
    let mut backing = std::ptr::null_mut();
    let mut offset = 0;
    // SAFETY: env/value are live for this synchronous N-API invocation. This
    // reads metadata only; it never dereferences potentially shared bytes.
    let status = unsafe {
        napi::sys::napi_get_typedarray_info(
            env.raw(),
            value.raw(),
            &raw mut kind,
            &raw mut length,
            &raw mut data,
            &raw mut backing,
            &raw mut offset,
        )
    };
    if status != napi::sys::Status::napi_ok || kind != napi::sys::TypedarrayType::uint8_array {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    let mut ordinary = false;
    let mut detached = false;
    // SAFETY: backing was returned by N-API, not a user-overridable .buffer
    // property. SharedArrayBuffer is not an ArrayBuffer under this predicate.
    let valid = unsafe {
        napi::sys::napi_is_arraybuffer(env.raw(), backing, &raw mut ordinary)
            == napi::sys::Status::napi_ok
            && ordinary
            && napi::sys::napi_is_detached_arraybuffer(env.raw(), backing, &raw mut detached)
                == napi::sys::Status::napi_ok
            && !detached
    };
    if !valid {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    if length as u64 > maximum {
        return Err(thrown(
            env,
            RuntimeError::ResourceLimit {
                dimension: "chunkBytes",
                used: 0,
                requested: length as u64,
                limit: maximum,
            },
        ));
    }
    // SAFETY: exact Uint8 kind, attached unshared backing, bounded length. No
    // application callback/JS runs between this check and the owned copy.
    unsafe { Uint8Array::from_napi_value(env.raw(), value.raw()) }
}

#[napi]
pub fn runtime_hash(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    bytes: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let bytes = unshared_input(env, bytes, runtime.options.chunk_bytes)?;
    let length = bytes.len() as u64;
    if length > runtime.options.chunk_bytes {
        return Err(thrown(
            env,
            RuntimeError::ResourceLimit {
                dimension: "chunkBytes",
                used: 0,
                requested: length,
                limit: runtime.options.chunk_bytes,
            },
        ));
    }
    let callback = callback
        .build_threadsafe_function()
        .callee_handled::<false>()
        .max_queue_size::<1>()
        .build()?;
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            Box::new(move || {
                let _ = callback.call((), ThreadsafeFunctionCallMode::NonBlocking);
            }),
            |context| {
                context.input(length)?;
                let reservation = context.reserve(ByteKind::Working, length)?;
                let mut owned = Vec::new();
                owned
                    .try_reserve_exact(bytes.len())
                    .map_err(|_| RuntimeError::Internal)?;
                owned.extend_from_slice(&bytes);
                Ok(Box::new(move |context| {
                    let mut hash = bumbledb::digest::Digest::new();
                    for chunk in owned.chunks(4096) {
                        context.step(chunk.len() as u64)?;
                        hash.update(chunk);
                    }
                    let result = context.reserve(ByteKind::Result, 32)?;
                    let digest = hash.finalize();
                    drop(owned);
                    drop(reservation);
                    Ok(Output::Hash(digest, result))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(External::new(OperationHandle {
        identity: identity(),
        runtime: Arc::clone(runtime),
        operation,
    }))
}

#[napi]
pub fn runtime_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Option<Buffer>> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    match handle
        .runtime
        .take(&handle.operation)
        .map_err(|error| thrown(env, error))?
    {
        Output::Ready => Ok(None),
        Output::Hash(value, _reservation) => Ok(Some(Buffer::from(value.to_vec()))),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

fn notification(callback: Function<(), ()>) -> napi::Result<Box<dyn FnOnce() + Send>> {
    let callback = callback.build_threadsafe_function().callee_handled::<false>().max_queue_size::<1>().build()?;
    Ok(Box::new(move || { let _ = callback.call((), ThreadsafeFunctionCallMode::NonBlocking); }))
}

fn reporter(callback: Function<CloseWire, ()>) -> napi::Result<Box<dyn FnOnce(CloseReport) + Send>> {
    let callback = callback.build_threadsafe_function().callee_handled::<false>().max_queue_size::<1>().build()?;
    Ok(Box::new(move |report| { let _ = callback.call(report.into(), ThreadsafeFunctionCallMode::NonBlocking); }))
}

fn directory(handle: &DirectoryHandle) -> Result<&crate::runtime::owners::DirectoryOwner, RuntimeError> {
    if handle.identity != identity() { return Err(RuntimeError::ForeignRuntime); }
    Ok(&handle.owner)
}

fn operation_handle(runtime: &Arc<Runtime>, operation: Arc<Operation>) -> External<OperationHandle> {
    External::new(OperationHandle { identity: identity(), runtime: Arc::clone(runtime), operation })
}

fn take_output(env: Env, handle: &OperationHandle) -> napi::Result<Output> {
    if handle.identity != identity() { return Err(thrown(env, RuntimeError::ForeignRuntime)); }
    handle.runtime.take(&handle.operation).map_err(|error| thrown(env, error))
}

#[napi]
pub fn runtime_directory_acquire(env: Env, handle: &External<RuntimeHandle>, policy: PolicyWire, path: String, callback: Function<(), ()>) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    if path.len() as u64 > runtime.options.chunk_bytes { return Err(thrown(env, RuntimeError::InvalidPath)); }
    let operation = runtime.acquire_directory(path, policy.parse().map_err(|error| thrown(env, error))?, notification(callback)?).map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_directory_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<External<DirectoryHandle>> {
    match take_output(env, handle)? {
        Output::Directory(owner) => Ok(External::new(DirectoryHandle { identity: identity(), owner })),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_directory_begin(env: Env, handle: &External<DirectoryHandle>, policy: PolicyWire) -> napi::Result<External<OperationHandle>> {
    let owner = directory(handle).map_err(|error| thrown(env, error))?;
    let operation = owner.begin_work(policy.parse().map_err(|error| thrown(env, error))?).map_err(|error| thrown(env, error))?;
    Ok(operation_handle(owner.runtime(), operation))
}

#[napi]
pub fn runtime_directory_check(env: Env, handle: &External<OperationHandle>) -> napi::Result<()> {
    if handle.identity != identity() { return Err(thrown(env, RuntimeError::ForeignRuntime)); }
    handle.runtime.checkpoint_external(&handle.operation).map_err(|error| thrown(env, error))
}

#[napi]
pub fn runtime_directory_end(env: Env, handle: &External<OperationHandle>) -> napi::Result<()> {
    if handle.identity != identity() { return Err(thrown(env, RuntimeError::ForeignRuntime)); }
    if !handle.operation.external { return Err(thrown(env, RuntimeError::InvalidArgument)); }
    handle.runtime.end_external(&handle.operation);
    Ok(())
}

#[napi]
pub fn runtime_directory_close(env: Env, handle: &External<DirectoryHandle>, remove: bool, callback: Function<CloseWire, ()>) -> napi::Result<()> {
    let owner = directory(handle).map_err(|error| thrown(env, error))?;
    let report = reporter(callback)?;
    owner.close_with(remove);
    owner.drain(report);
    Ok(())
}

#[napi]
pub fn runtime_directory_db_open(env: Env, handle: &External<DirectoryHandle>, policy: PolicyWire, child_name: String, spec: Object, create: bool, callback: Function<(), ()>) -> napi::Result<External<OperationHandle>> {
    use crate::runtime::owners::ManagedDbOutcome;
    let owner = directory(handle).map_err(|error| thrown(env, error))?;
    if child_name.len() as u64 > owner.runtime().options.chunk_bytes { return Err(thrown(env, RuntimeError::InvalidPath)); }
    let reference = owner.reference();
    // The legacy schema converter remains on the JS thread. The operation is
    // registered before conversion; only owned Rust descriptors reach workers.
    let mut marshal_error = None;
    let operation = owner.runtime().submit_owned(owner, policy.parse().map_err(|error| thrown(env, error))?, notification(callback)?, |context| {
        context.input(child_name.len() as u64)?;
        let parsed = match crate::descriptor_of(&spec) {
            Ok(parsed) => parsed,
            Err(error) => { marshal_error = Some(error); return Err(RuntimeError::InvalidArgument); }
        };
        Ok(Box::new(move |context| {
            let (descriptor, attrs) = match parsed {
                Ok(parsed) => parsed,
                Err(crate::OpenOutcome::SchemaError(message)) => return Ok(Output::Db(ManagedDbOutcome::Refused { kind: "schemaError", message })),
                Err(crate::OpenOutcome::NewtypeMismatch(message)) => return Ok(Output::Db(ManagedDbOutcome::Refused { kind: "newtypeMismatch", message })),
                Err(_) => return Err(RuntimeError::Internal),
            };
            let path = reference.child_path(&child_name)?;
            context.checkpoint()?;
            let opened = if create {
                match crate::Engine::create(&path, descriptor.clone()) {
                    Ok(bumbledb::Admission::Accepted(db)) => Ok(db),
                    Ok(bumbledb::Admission::Rejected(violations)) => return Ok(Output::Db(ManagedDbOutcome::Rejected(crate::violations_wire(&descriptor, &violations)))),
                    Err(error) => Err(error),
                }
            } else { crate::Engine::open(&path, descriptor.clone()) };
            match opened {
                Ok(db) => {
                    let managed = reference.attach_db(crate::assemble_inner(db, descriptor, attrs))?;
                    Ok(Output::Db(ManagedDbOutcome::Opened(managed)))
                }
                Err(bumbledb::Error::Schema(error)) => Ok(Output::Db(ManagedDbOutcome::Refused { kind: "schemaError", message: error.to_string() })),
                Err(error @ bumbledb::Error::SchemaMismatch { .. }) => Ok(Output::Db(ManagedDbOutcome::Refused { kind: "fingerprintMismatch", message: crate::marshal::engine_message(&error) })),
                Err(bumbledb::Error::EnvironmentLocked) => Err(RuntimeError::DirectoryBusy),
                Err(_) => Err(RuntimeError::Io { kind: std::io::ErrorKind::Other, code: None }),
            }
        }))
    });
    if let Some(error) = marshal_error { return Err(error); }
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(owner.runtime(), operation))
}

#[napi]
pub fn runtime_db_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Object<'_>> {
    use crate::runtime::owners::ManagedDbOutcome;
    let mut object = Object::new(&env)?;
    match take_output(env, handle)? {
        Output::Db(ManagedDbOutcome::Opened(db)) => { object.set("tag", "accepted")?; object.set("db", External::new(crate::DbHandle::managed(db)))?; }
        Output::Db(ManagedDbOutcome::Rejected(violations)) => { object.set("tag", "rejected")?; object.set("violations", violations)?; }
        Output::Db(ManagedDbOutcome::Refused { kind, message }) => { object.set("tag", "refused")?; object.set("kind", kind)?; object.set("message", message)?; }
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    }
    Ok(object)
}

#[napi]
pub fn runtime_managed_db_close(env: Env, db: &External<crate::DbHandle>, callback: Function<CloseWire, ()>) -> napi::Result<()> {
    match &db.inner {
        crate::DbOwner::Managed(owner) => { owner.drain(reporter(callback)?); Ok(()) }
        crate::DbOwner::Legacy(_) => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_fs(env: Env, handle: &External<RuntimeHandle>, policy: PolicyWire, request: Object, callback: Function<(), ()>) -> napi::Result<External<OperationHandle>> {
    use crate::runtime::fs::FsVerb;
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let root: String = request.get_named_property("root")?;
    let raw_key: String = request.get_named_property("key")?;
    let name: String = request.get_named_property("verb")?;
    let etag: Option<String> = request.get_named_property("etag")?;
    let token: Option<BigInt> = request.get_named_property("token")?;
    let token = token.as_ref().map(integer).transpose().map_err(|error| thrown(env, error))?.unwrap_or(0);
    let key = bumbledb_log::store::StoreKey::parse(&raw_key).map_err(|_| thrown(env, RuntimeError::InvalidPath))?;
    let length = root.len().checked_add(raw_key.len()).and_then(|size| size.checked_add(etag.as_ref().map_or(0, String::len))).ok_or_else(|| thrown(env, RuntimeError::InvalidArgument))?;
    if length as u64 > runtime.options.chunk_bytes { return Err(thrown(env, RuntimeError::InvalidPath)); }
    let input = if matches!(name.as_str(), "create" | "swap") {
        Some(unshared_input(env, request.get_named_property::<Unknown>("bytes")?, runtime.options.chunk_bytes)?)
    } else { None };
    let operation = runtime.submit(policy.parse().map_err(|error| thrown(env, error))?, notification(callback)?, |context| {
        let body_length = input.as_ref().map_or(0, |value| value.len()) as u64;
        let total = (length as u64).checked_add(body_length).ok_or(RuntimeError::InvalidArgument)?;
        context.input(total)?;
        let reservation = context.reserve(ByteKind::Working, total)?;
        let body = input.as_ref().map(|value| value.to_vec()).unwrap_or_default();
        let verb = match name.as_str() {
            "get" => FsVerb::Get,
            "poll" => FsVerb::Poll(bumbledb_log::store::Etag(etag.ok_or(RuntimeError::InvalidArgument)?)),
            "create" => FsVerb::Create { bytes: body, token },
            "swap" => FsVerb::Swap { bytes: body, token, etag: bumbledb_log::store::Etag(etag.ok_or(RuntimeError::InvalidArgument)?) },
            "delete" => FsVerb::Delete,
            _ => return Err(RuntimeError::InvalidArgument),
        };
        Ok(Box::new(move |context| {
            let result = crate::runtime::fs::execute(root, key, verb, context)?;
            drop(reservation);
            Ok(Output::Fs(result))
        }))
    }).map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_fs_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Object<'_>> {
    use crate::runtime::fs::FsOutput;
    use bumbledb_log::store::{Create, Poll, Swap};
    let mut object = Object::new(&env)?;
    let output = take_output(env, handle)?;
    let tag = match output {
        Output::Fs(FsOutput::Get(value)) => match value.value {
            None => "absent",
            Some(fetched) => { object.set("bytes", Buffer::from(fetched.bytes))?; object.set("etag", fetched.etag.0)?; "fetched" }
        },
        Output::Fs(FsOutput::Poll(value)) => match value.value {
            Poll::Unchanged => "unchanged",
            Poll::Changed(fetched) => { object.set("bytes", Buffer::from(fetched.bytes))?; object.set("etag", fetched.etag.0)?; "changed" }
        },
        Output::Fs(FsOutput::Create(value)) => match value.value {
            Create::Created(etag) => { object.set("etag", etag.0)?; "created" },
            Create::Exists => "exists", Create::Ambiguous => "ambiguous",
        },
        Output::Fs(FsOutput::Swap(value)) => match value.value {
            Swap::Swapped(etag) => { object.set("etag", etag.0)?; "swapped" },
            Swap::Moved => "moved", Swap::Ambiguous => "ambiguous",
        },
        Output::Fs(FsOutput::Delete(_)) => "deleted",
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    };
    object.set("tag", tag)?;
    Ok(object)
}
