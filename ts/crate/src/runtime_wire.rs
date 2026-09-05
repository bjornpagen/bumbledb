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

fn thrown(env: Env, error: RuntimeError) -> napi::Error {
    let make = || -> napi::Result<()> {
        let mut object = Object::new(&env)?;
        object.set("_tag", error_code(&error))?;
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
    }
}
