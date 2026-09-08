//! Exact-version Node ownership boundary for the shared native executor.
#![allow(
    clippy::needless_pass_by_value,
    reason = "N-API owns value argument conversion"
)]
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bumbledb::work::{WorkContext, WorkError};
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
    pub workers: Option<f64>,
    pub queue_capacity: Option<f64>,
    pub cleanup_capacity: Option<f64>,
    pub owner_capacity: Option<f64>,
    pub native_handle_capacity: Option<f64>,
    pub cleanup_timeout_ms: Option<f64>,
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
];

fn error_code(error: &RuntimeError) -> &'static str {
    match error {
        RuntimeError::RuntimeAlreadyLive => "RuntimeAlreadyLive",
        RuntimeError::ForeignRuntime => "ForeignRuntime",
        RuntimeError::ClosedHandle => "ClosedHandle",
        RuntimeError::SpentHandle => "SpentHandle",
        RuntimeError::QueueFull => "QueueFull",
        RuntimeError::InvalidArgument => "InvalidArgument",
        RuntimeError::Internal => "Internal",
        RuntimeError::DirectoryBusy => "DirectoryBusy",
        RuntimeError::WriterBusy => "WriterBusy",
        RuntimeError::InvalidPath => "InvalidPath",
        RuntimeError::Io { .. } | RuntimeError::Work(WorkError::Allocation) => "Io",
        RuntimeError::ResourceLimit { .. } => "ResourceLimit",
        RuntimeError::Engine { .. } => "Engine",
        RuntimeError::Work(WorkError::Cancelled) => "Cancelled",
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
        RuntimeError::Work(WorkError::Allocation) => {
            object.set("kind", "OutOfMemory")?;
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
        let defaults = Options::default();
        Ok(Options {
            workers: options
                .workers
                .map(unsigned)
                .transpose()?
                .map_or(defaults.workers, |value| value as usize),
            queue_capacity: options
                .queue_capacity
                .map(unsigned)
                .transpose()?
                .map_or(defaults.queue_capacity, |value| value as usize),
            cleanup_capacity: options
                .cleanup_capacity
                .map(unsigned)
                .transpose()?
                .map_or(defaults.cleanup_capacity, |value| value as usize),
            owner_capacity: options
                .owner_capacity
                .map(unsigned)
                .transpose()?
                .map_or(defaults.owner_capacity, |value| value as usize),
            native_handle_capacity: options
                .native_handle_capacity
                .map(unsigned)
                .transpose()?
                .map_or(defaults.native_handle_capacity, |value| value as usize),
            cleanup_timeout: options
                .cleanup_timeout_ms
                .map(unsigned)
                .transpose()?
                .map_or(defaults.cleanup_timeout, |value| {
                    Duration::from_millis(u64::from(value))
                }),
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
            WorkContext::new(),
            Box::new(move || {
                let _ = callback.call((), ThreadsafeFunctionCallMode::NonBlocking);
            }),
            |_| {
                Ok(Box::new(|context| {
                    context.checkpoint()?;
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
pub(crate) fn unshared_input(env: Env, value: Unknown) -> napi::Result<Uint8Array> {
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
    // SAFETY: exact Uint8 kind, attached unshared backing. No
    // application callback/JS runs between this check and the owned copy.
    unsafe { Uint8Array::from_napi_value(env.raw(), value.raw()) }
}

#[napi]
pub fn runtime_hash(
    env: Env,
    handle: &External<RuntimeHandle>,
    bytes: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let bytes = unshared_input(env, bytes)?;
    let callback = callback
        .build_threadsafe_function()
        .callee_handled::<false>()
        .max_queue_size::<1>()
        .build()?;
    let operation = runtime
        .submit(
            WorkContext::new(),
            Box::new(move || {
                let _ = callback.call((), ThreadsafeFunctionCallMode::NonBlocking);
            }),
            |context| {
                context.checkpoint()?;
                let mut owned = Vec::new();
                owned
                    .try_reserve_exact(bytes.len())
                    .map_err(|_| WorkError::Allocation)?;
                owned.extend_from_slice(&bytes);
                Ok(Box::new(move |context| {
                    let mut hash = bumbledb::digest::Digest::new();
                    for chunk in owned.chunks(4096) {
                        context.checkpoint()?;
                        hash.update(chunk);
                    }
                    let digest = hash.finalize();
                    drop(owned);
                    Ok(Output::Hash(digest))
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
        Output::Hash(value) => Ok(Some(Buffer::from(value.to_vec()))),
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
    path: String,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let operation = runtime
        .acquire_directory(path, WorkContext::new(), notification(callback)?)
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
) -> napi::Result<External<OperationHandle>> {
    let owner = directory(handle).map_err(|error| thrown(env, error))?;
    let operation = owner
        .begin_work(WorkContext::new())
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
    child_name: String,
    spec: Object,
    create: bool,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    use crate::runtime::owners::ManagedDbOutcome;
    let owner = directory(handle).map_err(|error| thrown(env, error))?;
    let reference = owner.reference();
    // The legacy schema converter remains on the JS thread. The operation is
    // registered before conversion; only owned Rust descriptors reach workers.
    let mut marshal_error = None;
    let operation = owner.runtime().submit_owned(
        owner,
        WorkContext::new(),
        notification(callback)?,
        |context| {
            context.checkpoint()?;
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

pub struct PreparedHandle {
    identity: usize,
    session: Arc<crate::runtime::session::SnapshotSession>,
}

impl PreparedHandle {
    pub(crate) fn new(session: crate::runtime::session::SnapshotSession) -> Self {
        Self {
            identity: identity(),
            session: Arc::new(session),
        }
    }
}

pub(crate) fn prepared(
    handle: &PreparedHandle,
) -> Result<&crate::runtime::session::SnapshotSession, RuntimeError> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    if handle.session.capability().kind != crate::runtime::NativeKind::Prepared {
        return Err(RuntimeError::InvalidArgument);
    }
    Ok(&handle.session)
}

/// Adopt one compiled worker-owned plan. Dropping an untaken output closes
/// the same resource; JavaScript is never its only cleanup authority.
#[napi]
pub fn runtime_prepared_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<PreparedHandle>> {
    match take_output(env, handle)? {
        Output::Prepared(session) => Ok(External::new(PreparedHandle::new(session))),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_prepared_close(
    env: Env,
    handle: &External<PreparedHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    prepared(handle)
        .map_err(|error| thrown(env, error))?
        .drain(reporter(callback)?);
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
