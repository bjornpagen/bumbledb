//! The executor's JavaScript surface: the one live runtime, operation
//! handles, completion and close callbacks, directory owners and databases.
//! napi's `External` provenance proves every handle is a live one of its
//! own type from this addon.
use std::sync::{Arc, Mutex};

use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Env, External, FromNapiValue, Function, JsValue, Uint8Array, Unknown};
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_derive::napi;

use crate::runtime::owners::ManagedDbOutcome;
use crate::runtime::{CloseReport, Inspection, Operation, Output, Phase, Runtime, RuntimeError};

static LIVE: Mutex<Option<Arc<Runtime>>> = Mutex::new(None);

pub struct RuntimeHandle {
    runtime: Arc<Runtime>,
}

pub struct OperationHandle {
    runtime: Arc<Runtime>,
    operation: Arc<Operation>,
}

pub struct DirectoryHandle {
    owner: crate::runtime::owners::DirectoryOwner,
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

pub(crate) fn owner(handle: &RuntimeHandle) -> &Arc<Runtime> {
    &handle.runtime
}

/// The runtime owning one registered operation.
pub(crate) fn operation_runtime(handle: &OperationHandle) -> Arc<Runtime> {
    Arc::clone(&handle.runtime)
}

/// Throw `error` as its `_tag` object and return the pending-exception error.
pub(crate) fn thrown(env: Env, error: RuntimeError) -> napi::Error {
    env.throw(error)
        .err()
        .unwrap_or_else(|| napi::Error::from_status(napi::Status::PendingException))
}

/// Start the one live runtime. `optionsJson` is a `RuntimeOptionsIn`.
#[napi]
pub fn runtime_open(env: Env, options_json: String) -> napi::Result<External<RuntimeHandle>> {
    let options: crate::input::options::RuntimeOptionsIn =
        crate::input::decode(&options_json).map_err(|malformed| thrown(env, malformed.into()))?;
    let mut live = LIVE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if live
        .as_ref()
        .is_some_and(|runtime| runtime.inspect().phase != Phase::Closed)
    {
        return Err(thrown(env, RuntimeError::RuntimeAlreadyLive));
    }
    let runtime = Runtime::start(options.into()).map_err(|error| thrown(env, error))?;
    *live = Some(Arc::clone(&runtime));
    Ok(External::new(RuntimeHandle { runtime }))
}

/// Bounded runtime bookkeeping counts.
#[napi(object, object_from_js = false)]
pub struct InspectionOut {
    pub phase: Phase,
    pub queued: u64,
    pub active: u64,
    pub retained: u64,
    pub owners: u64,
    pub databases: u64,
    pub natives: u64,
}

impl From<Inspection> for InspectionOut {
    fn from(value: Inspection) -> Self {
        Self {
            phase: value.phase,
            queued: value.queued as u64,
            active: value.active as u64,
            retained: value.retained as u64,
            owners: value.owners as u64,
            databases: value.databases as u64,
            natives: value.natives as u64,
        }
    }
}

/// A close or cancel outcome: joined, timed out with work still
/// outstanding, or failed (cleanup capacity exhausted or teardown failed).
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum CloseOut {
    Closed,
    Incomplete { outstanding: InspectionOut },
    Failed,
}

impl From<CloseReport> for CloseOut {
    fn from(value: CloseReport) -> Self {
        match value {
            CloseReport::Closed => Self::Closed,
            CloseReport::Incomplete(inspection) => Self::Incomplete {
                outstanding: inspection.into(),
            },
            CloseReport::Failed => Self::Failed,
        }
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
    callback: Function<CloseOut, ()>,
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

pub(crate) fn operation_handle(
    runtime: &Arc<Runtime>,
    operation: Arc<Operation>,
) -> External<OperationHandle> {
    External::new(OperationHandle {
        runtime: Arc::clone(runtime),
        operation,
    })
}

/// Spend one completed operation's output. A second take of the same handle
/// throws `SpentHandle`; it never returns an empty value.
pub(crate) fn take_output(env: Env, handle: &OperationHandle) -> napi::Result<Output> {
    handle
        .runtime
        .take(&handle.operation)
        .map_err(|error| thrown(env, error))
}

pub(crate) fn wrong_output(env: Env) -> napi::Error {
    thrown(env, RuntimeError::InvalidArgument)
}

#[napi]
pub fn runtime_close(
    handle: &External<RuntimeHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    handle.runtime.drain(None, reporter(callback)?);
    Ok(())
}

#[napi]
pub fn runtime_cancel(
    handle: &External<OperationHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    handle
        .runtime
        .drain(Some(&handle.operation), reporter(callback)?);
    Ok(())
}

#[napi]
#[must_use]
pub fn runtime_inspect(handle: &External<RuntimeHandle>) -> InspectionOut {
    handle.runtime.inspect().into()
}

/// One executor round trip with no payload.
#[napi]
pub fn runtime_ready(
    env: Env,
    handle: &External<RuntimeHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let operation = handle
        .runtime
        .submit(WorkContext::new(), notification(callback)?, |_| {
            Ok(Box::new(|context| {
                context.checkpoint()?;
                Ok(Output::Ready)
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&handle.runtime, operation))
}

/// Take a payload-less completion.
#[napi]
pub fn runtime_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<()> {
    match take_output(env, handle)? {
        Output::Ready => Ok(()),
        _ => Err(wrong_output(env)),
    }
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
    // SAFETY: exact Uint8 kind, attached unshared backing. No application
    // callback or JS runs between this check and the owned copy.
    unsafe { Uint8Array::from_napi_value(env.raw(), value.raw()) }
}

#[napi]
pub fn runtime_directory_acquire(
    env: Env,
    handle: &External<RuntimeHandle>,
    path: String,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let operation = handle
        .runtime
        .acquire_directory(path, WorkContext::new(), notification(callback)?)
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&handle.runtime, operation))
}

#[napi]
pub fn runtime_directory_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<DirectoryHandle>> {
    match take_output(env, handle)? {
        Output::Directory(owner) => Ok(External::new(DirectoryHandle { owner })),
        _ => Err(wrong_output(env)),
    }
}

/// Hold the directory open across awaited JavaScript work; end it with
/// `runtimeDirectoryEnd`.
#[napi]
pub fn runtime_directory_begin(
    env: Env,
    handle: &External<DirectoryHandle>,
) -> napi::Result<External<OperationHandle>> {
    let operation = handle
        .owner
        .begin_work(WorkContext::new())
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(handle.owner.runtime(), operation))
}

#[napi]
pub fn runtime_directory_check(env: Env, handle: &External<OperationHandle>) -> napi::Result<()> {
    handle
        .runtime
        .checkpoint_external(&handle.operation)
        .map_err(|error| thrown(env, error))
}

#[napi]
pub fn runtime_directory_end(env: Env, handle: &External<OperationHandle>) -> napi::Result<()> {
    if !handle.operation.is_external() {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    }
    handle.runtime.end_external(&handle.operation);
    Ok(())
}

#[napi]
pub fn runtime_directory_close(
    handle: &External<DirectoryHandle>,
    remove: bool,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    let report = reporter(callback)?;
    handle.owner.close_with(remove);
    handle.owner.drain(report);
    Ok(())
}

/// Open (or create) one database in a child directory of an owned
/// directory, under a compiled schema.
#[napi]
pub fn runtime_directory_db_open(
    env: Env,
    handle: &External<DirectoryHandle>,
    child_name: String,
    schema: &External<Arc<crate::schema::SchemaHandle>>,
    create: bool,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let owner = &handle.owner;
    let reference = owner.reference();
    let schema = Arc::clone(schema);
    let operation = owner
        .runtime()
        .submit_owned(owner, WorkContext::new(), notification(callback)?, |_| {
            Ok(Box::new(move |context| {
                let path = reference.child_path(&child_name)?;
                context.checkpoint()?;
                let descriptor = schema.descriptor.clone();
                let opened = if create {
                    match crate::Engine::create(&path, descriptor, context.clone()) {
                        Ok(bumbledb::Admission::Accepted(db)) => Ok(db),
                        Ok(bumbledb::Admission::Rejected(violations)) => {
                            return Ok(Output::Db(ManagedDbOutcome::Rejected(
                                crate::violations_out(&schema.descriptor, &violations),
                            )));
                        }
                        Err(error) => Err(error),
                    }
                } else {
                    crate::Engine::open(&path, descriptor, context.clone())
                };
                match opened {
                    Ok(db) => {
                        let managed = reference.attach_db(crate::assemble_inner(db, schema))?;
                        Ok(Output::Db(ManagedDbOutcome::Opened(managed)))
                    }
                    Err(error @ bumbledb::Error::SchemaMismatch { .. }) => {
                        Ok(Output::Db(ManagedDbOutcome::FingerprintMismatch {
                            message: error.to_string(),
                        }))
                    }
                    Err(error @ bumbledb::Error::DestinationExists { .. }) => {
                        Ok(Output::Db(ManagedDbOutcome::DestinationExists {
                            message: error.to_string(),
                        }))
                    }
                    Err(bumbledb::Error::EnvironmentLocked) => Err(RuntimeError::DirectoryBusy),
                    Err(error) => Err(crate::runtime::session::engine_error(&error)),
                }
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(owner.runtime(), operation))
}

/// A database open outcome. Refusals are domain outcomes, not failures.
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum DbOpened {
    Opened {
        db: External<crate::DbHandle>,
    },
    /// A created database's closed relations violate the schema's laws.
    Rejected {
        violations: Vec<crate::marshal::ViolationOut>,
    },
    /// The directory holds a database of a different schema.
    FingerprintMismatch {
        message: String,
    },
    /// `create` found an existing database.
    DestinationExists {
        message: String,
    },
}

#[napi]
pub fn runtime_db_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<DbOpened> {
    match take_output(env, handle)? {
        Output::Db(ManagedDbOutcome::Opened(db)) => Ok(DbOpened::Opened {
            db: External::new(crate::DbHandle::managed(db)),
        }),
        Output::Db(ManagedDbOutcome::Rejected(violations)) => Ok(DbOpened::Rejected { violations }),
        Output::Db(ManagedDbOutcome::FingerprintMismatch { message }) => {
            Ok(DbOpened::FingerprintMismatch { message })
        }
        Output::Db(ManagedDbOutcome::DestinationExists { message }) => {
            Ok(DbOpened::DestinationExists { message })
        }
        _ => Err(wrong_output(env)),
    }
}

/// The one close authority: begin the database's drain and report its
/// real outcome.
#[napi]
pub fn runtime_managed_db_close(
    db: &External<crate::DbHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    db.owner().drain(reporter(callback)?);
    Ok(())
}

/// One compiled plan pinned to its snapshot on a fixed worker.
pub struct PreparedHandle {
    session: Arc<crate::runtime::session::SnapshotSession>,
}

pub(crate) fn prepared(handle: &PreparedHandle) -> &crate::runtime::session::SnapshotSession {
    &handle.session
}

/// Adopt one compiled worker-owned plan. Dropping an untaken output closes
/// the same resource; JavaScript is never its only cleanup authority.
#[napi]
pub fn runtime_prepared_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<PreparedHandle>> {
    match take_output(env, handle)? {
        Output::Prepared(session) => Ok(External::new(PreparedHandle {
            session: Arc::new(session),
        })),
        _ => Err(wrong_output(env)),
    }
}

#[napi]
pub fn runtime_prepared_close(
    handle: &External<PreparedHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    handle.session.drain(reporter(callback)?);
    Ok(())
}

#[napi(ts_return_type = "Array<Array<CellValue>>")]
pub fn runtime_rows_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<crate::runtime::QueuedOutput> {
    match take_output(env, handle)? {
        Output::Rows(queued) => Ok(queued),
        _ => Err(wrong_output(env)),
    }
}

#[napi(ts_return_type = "Array<CellValue> | null")]
pub fn runtime_row_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Option<crate::runtime::QueuedRow>> {
    match take_output(env, handle)? {
        Output::Row(row) => Ok(row),
        _ => Err(wrong_output(env)),
    }
}
