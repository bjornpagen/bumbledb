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

/// The one process-local addon identity, shared by every sibling wire
/// module's handle checks (`db_wire`, `log_wire`).
pub(crate) fn addon_identity() -> usize {
    identity()
}

/// The runtime owning one registered operation (sibling wire modules wrap
/// taken outputs into retained resources under the same runtime).
pub(crate) fn operation_runtime(handle: &OperationHandle) -> Arc<Runtime> {
    Arc::clone(&handle.runtime)
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

pub(crate) fn owner(handle: &RuntimeHandle) -> Result<&Arc<Runtime>, RuntimeError> {
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
    pub(crate) fn parse(&self) -> Result<ExecutionPolicy, RuntimeError> {
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
    "WriterBusy",
    "InvalidPath",
    "Io",
    "ResourceLimit",
    "Engine",
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
        RuntimeError::WriterBusy => "WriterBusy",
        RuntimeError::InvalidPath => "InvalidPath",
        RuntimeError::Io { .. } => "Io",
        RuntimeError::ResourceLimit { .. } | RuntimeError::Work(WorkError::Exhausted { .. }) => {
            "ResourceLimit"
        }
        RuntimeError::Engine { .. } => "Engine",
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

/// The typed reason object a core failure crosses as (`{_tag, ...}` — the
/// `DbReason` roster in ts/src/runtime-errors.ts). Shared with the log wire,
/// which nests the same object inside its `{source, reason}` frame.
pub(crate) fn reason_object(env: &Env, error: RuntimeError) -> napi::Result<Object<'_>> {
    let mut object = Object::new(env)?;
    object.set("_tag", error_code(&error))?;
    match error {
        RuntimeError::Io { kind, code } => {
            object.set("kind", format!("{kind:?}"))?;
            object.set("osCode", code)?;
        }
        RuntimeError::Engine { kind, message } => {
            object.set("kind", kind)?;
            object.set("message", message)?;
        }
        RuntimeError::ResourceLimit {
            dimension,
            used,
            requested,
            limit,
        } => {
            object.set("dimension", dimension)?;
            object.set("used", BigInt::from(used))?;
            object.set("requested", BigInt::from(requested))?;
            object.set("limit", BigInt::from(limit))?;
        }
        RuntimeError::Work(WorkError::Exhausted {
            resource,
            used,
            requested,
            limit,
        }) => {
            object.set("dimension", resource_name(resource))?;
            object.set("used", BigInt::from(used))?;
            object.set("requested", BigInt::from(requested))?;
            object.set("limit", BigInt::from(limit))?;
        }
        _ => {}
    }
    Ok(object)
}

pub(crate) fn thrown(env: Env, error: RuntimeError) -> napi::Error {
    let make = |error: RuntimeError| -> napi::Result<()> {
        let object = reason_object(&env, error)?;
        env.throw(object)
    };
    make(error)
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
    pub natives: BigInt,
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
            natives: BigInt::from(value.natives as u64),
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
pub(crate) fn unshared_input(env: Env, value: Unknown, maximum: u64) -> napi::Result<Uint8Array> {
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

/// Take one completed operation's payload. PINNED double-take contract
/// (P12's F3 note, decided wave-E): the FIRST take spends the operation;
/// every later take of the same handle THROWS the typed `SpentHandle`
/// refusal — it never returns `null`. `null` is reserved for a payload-less
/// completion (`Output::Ready`), so silence can never be mistaken for a
/// spent handle. The same contract holds for every `*Take` verb riding
/// `Runtime::take` (db and log bridges included).
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

pub(crate) fn notification(callback: Function<(), ()>) -> napi::Result<Box<dyn FnOnce() + Send>> {
    let callback = callback
        .build_threadsafe_function()
        .callee_handled::<false>()
        .max_queue_size::<1>()
        .build()?;
    Ok(Box::new(move || {
        let _ = callback.call((), ThreadsafeFunctionCallMode::NonBlocking);
    }))
}

pub(crate) fn reporter(
    callback: Function<CloseWire, ()>,
) -> napi::Result<Box<dyn FnOnce(CloseReport) + Send>> {
    let callback = callback
        .build_threadsafe_function()
        .callee_handled::<false>()
        .max_queue_size::<1>()
        .build()?;
    Ok(Box::new(move |report| {
        let _ = callback.call(report.into(), ThreadsafeFunctionCallMode::NonBlocking);
    }))
}

fn directory(
    handle: &DirectoryHandle,
) -> Result<&crate::runtime::owners::DirectoryOwner, RuntimeError> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    Ok(&handle.owner)
}

pub(crate) fn operation_handle(
    runtime: &Arc<Runtime>,
    operation: Arc<Operation>,
) -> External<OperationHandle> {
    External::new(OperationHandle {
        identity: identity(),
        runtime: Arc::clone(runtime),
        operation,
    })
}

pub(crate) fn take_output(env: Env, handle: &OperationHandle) -> napi::Result<Output> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    handle
        .runtime
        .take(&handle.operation)
        .map_err(|error| thrown(env, error))
}

#[napi]
pub fn runtime_directory_acquire(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    path: String,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    if path.len() as u64 > runtime.options.chunk_bytes {
        return Err(thrown(env, RuntimeError::InvalidPath));
    }
    let operation = runtime
        .acquire_directory(
            path,
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_directory_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<DirectoryHandle>> {
    match take_output(env, handle)? {
        Output::Directory(owner) => Ok(External::new(DirectoryHandle {
            identity: identity(),
            owner,
        })),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_directory_begin(
    env: Env,
    handle: &External<DirectoryHandle>,
    policy: PolicyWire,
) -> napi::Result<External<OperationHandle>> {
    let owner = directory(handle).map_err(|error| thrown(env, error))?;
    let operation = owner
        .begin_work(policy.parse().map_err(|error| thrown(env, error))?)
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(owner.runtime(), operation))
}

#[napi]
pub fn runtime_directory_check(env: Env, handle: &External<OperationHandle>) -> napi::Result<()> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    handle
        .runtime
        .checkpoint_external(&handle.operation)
        .map_err(|error| thrown(env, error))
}

#[napi]
pub fn runtime_directory_end(env: Env, handle: &External<OperationHandle>) -> napi::Result<()> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    if !handle.operation.is_external() {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    handle.runtime.end_external(&handle.operation);
    Ok(())
}

#[napi]
pub fn runtime_directory_close(
    env: Env,
    handle: &External<DirectoryHandle>,
    remove: bool,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    let owner = directory(handle).map_err(|error| thrown(env, error))?;
    let report = reporter(callback)?;
    owner.close_with(remove);
    owner.drain(report);
    Ok(())
}

#[napi]
pub fn runtime_directory_db_open(
    env: Env,
    handle: &External<DirectoryHandle>,
    policy: PolicyWire,
    child_name: String,
    spec: Object,
    create: bool,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    use crate::runtime::owners::ManagedDbOutcome;
    let owner = directory(handle).map_err(|error| thrown(env, error))?;
    if child_name.len() as u64 > owner.runtime().options.chunk_bytes {
        return Err(thrown(env, RuntimeError::InvalidPath));
    }
    let reference = owner.reference();
    // The legacy schema converter remains on the JS thread. The operation is
    // registered before conversion; only owned Rust descriptors reach workers.
    let mut marshal_error = None;
    let operation = owner.runtime().submit_owned(
        owner,
        policy.parse().map_err(|error| thrown(env, error))?,
        notification(callback)?,
        |context| {
            context.input(child_name.len() as u64)?;
            let parsed = match crate::descriptor_of(&spec) {
                Ok(parsed) => parsed,
                Err(error) => {
                    marshal_error = Some(error);
                    return Err(RuntimeError::InvalidArgument);
                }
            };
            Ok(Box::new(move |context| {
                let (descriptor, attrs) = match parsed {
                    Ok(parsed) => parsed,
                    Err(crate::OpenOutcome::SchemaError(message)) => {
                        return Ok(Output::Db(ManagedDbOutcome::Refused {
                            kind: crate::tags::open_kind::SCHEMA_ERROR,
                            message,
                        }));
                    }
                    Err(crate::OpenOutcome::NewtypeMismatch(message)) => {
                        return Ok(Output::Db(ManagedDbOutcome::Refused {
                            kind: crate::tags::open_kind::NEWTYPE_MISMATCH,
                            message,
                        }));
                    }
                };
                let path = reference.child_path(&child_name)?;
                context.checkpoint()?;
                let opened = if create {
                    match crate::Engine::create(&path, descriptor.clone(), context.clone()) {
                        Ok(bumbledb::Admission::Accepted(db)) => Ok(db),
                        Ok(bumbledb::Admission::Rejected(violations)) => {
                            return Ok(Output::Db(ManagedDbOutcome::Rejected(
                                crate::violations_wire(&descriptor, &violations),
                            )));
                        }
                        Err(error) => Err(error),
                    }
                } else {
                    crate::Engine::open(&path, descriptor.clone(), context.clone())
                };
                match opened {
                    Ok(db) => {
                        let managed =
                            reference.attach_db(crate::assemble_inner(db, descriptor, attrs))?;
                        Ok(Output::Db(ManagedDbOutcome::Opened(managed)))
                    }
                    Err(bumbledb::Error::Schema(error)) => {
                        Ok(Output::Db(ManagedDbOutcome::Refused {
                            kind: crate::tags::open_kind::SCHEMA_ERROR,
                            message: error.to_string(),
                        }))
                    }
                    Err(error @ bumbledb::Error::SchemaMismatch { .. }) => {
                        Ok(Output::Db(ManagedDbOutcome::Refused {
                            kind: crate::tags::open_kind::FINGERPRINT_MISMATCH,
                            message: crate::marshal::engine_message(&error),
                        }))
                    }
                    // Db.create refuses existing authority as a DOMAIN refusal
                    // (chapter 30), never a generic Io failure.
                    Err(error @ bumbledb::Error::DestinationExists { .. }) => {
                        Ok(Output::Db(ManagedDbOutcome::Refused {
                            kind: crate::tags::open_kind::DESTINATION_EXISTS,
                            message: crate::marshal::engine_message(&error),
                        }))
                    }
                    Err(bumbledb::Error::EnvironmentLocked) => Err(RuntimeError::DirectoryBusy),
                    Err(error) => Err(crate::runtime::session::engine_error(&error)),
                }
            }))
        },
    );
    if let Some(error) = marshal_error {
        return Err(error);
    }
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(owner.runtime(), operation))
}

#[napi]
pub fn runtime_db_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Object<'_>> {
    use crate::runtime::owners::ManagedDbOutcome;
    let mut object = Object::new(&env)?;
    match take_output(env, handle)? {
        Output::Db(ManagedDbOutcome::Opened(db)) => {
            object.set("tag", "accepted")?;
            object.set("db", External::new(crate::DbHandle::managed(db)))?;
        }
        Output::Db(ManagedDbOutcome::Rejected(violations)) => {
            object.set("tag", "rejected")?;
            object.set("violations", violations)?;
        }
        Output::Db(ManagedDbOutcome::Refused { kind, message }) => {
            object.set("tag", "refused")?;
            object.set("kind", kind)?;
            object.set("message", message)?;
        }
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    }
    Ok(object)
}

#[napi]
pub fn runtime_managed_db_close(
    _env: Env,
    db: &External<crate::DbHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    // One close authority: begin the owner's drain and report its real
    // outcome. There is no second synchronous JS-owned close verb.
    db.owner().drain(reporter(callback)?);
    Ok(())
}

// The 0.x five-verb JS filesystem transport (`runtime_fs`/`runtime_fs_take`)
// is DELETED with the TS CAS authority: object-store work is native (C07,
// P05's store rewrite), driven by the log machine (`log_wire.rs`) — no JS
// layer holds a conditional-store verb anymore.

// ---------------------------------------------------------------------------
// Worker-table snapshots (C7): capability tokens into a fixed worker's
// resource table. JS-driven WriterSession/HostWrite ABI is deleted; sealed
// apply/submit keeps the whole writer operation inside one native job.
// ---------------------------------------------------------------------------

pub struct SessionHandle {
    identity: usize,
    session: Arc<crate::runtime::session::SnapshotSession>,
    /// `None`: the owning worker-session capability — close drains the
    /// pinned thread. `Some(closed)`: a snapshot-bound EXECUTION session
    /// sharing the snapshot's pinned session (chapter 35 `Snapshot.session`)
    /// — close spends only this capability, never the snapshot's thread.
    exec: Option<Arc<std::sync::atomic::AtomicBool>>,
}

impl SessionHandle {
    pub(crate) fn exec_over(session: Arc<crate::runtime::session::SnapshotSession>) -> Self {
        Self {
            identity: identity(),
            session,
            exec: Some(Arc::new(std::sync::atomic::AtomicBool::new(false))),
        }
    }
}

pub(crate) fn session(
    handle: &SessionHandle,
) -> Result<&crate::runtime::session::SnapshotSession, RuntimeError> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    if let Some(closed) = &handle.exec
        && closed.load(std::sync::atomic::Ordering::Acquire)
    {
        return Err(RuntimeError::ClosedHandle);
    }
    Ok(&handle.session)
}

/// Adopt one snapshot-bound execution session. Snapshot adoption has its
/// own typed output; no caller must inspect a union of unrelated handles.
#[napi]
pub fn runtime_session_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<SessionHandle>> {
    match take_output(env, handle)? {
        Output::ExecSession(opened) => Ok(External::new(SessionHandle::exec_over(opened.session))),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_session_close(
    env: Env,
    handle: &External<SessionHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let report = reporter(callback)?;
    if let Some(closed) = &handle.exec {
        // An execution session is a spendable capability SHARING the
        // snapshot's pinned session: close spends only this capability
        // (idempotent — a second close joins the spent state); the pinned
        // thread and its in-flight jobs stay owned by the snapshot, whose
        // own close drains them. Nothing native is held here, so the
        // report is Closed by construction.
        closed.store(true, std::sync::atomic::Ordering::Release);
        report(crate::runtime::CloseReport::Closed);
        return Ok(());
    }
    handle.session.drain(report);
    Ok(())
}

#[napi]
pub fn runtime_rows_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<crate::runtime::QueuedOutput> {
    match take_output(env, handle)? {
        Output::Rows(queued) => Ok(queued),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_row_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Option<crate::runtime::QueuedRow>> {
    match take_output(env, handle)? {
        Output::Row(row) => Ok(row),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}
