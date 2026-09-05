//! The core db-bridge verb roster (C09/C05): the exact private surface
//! `ts/src/db-native.ts` declares and the chapter 35 core TypeScript
//! dispatches — schema compile, coherent snapshots and snapshot-bound
//! execution sessions, point reads, complete bounded execution into sealed
//! [`bumbledb::CompleteResult`]s with capped collect and one-shot cursor
//! transfer, database-free change drafts, one immutable final-state apply,
//! bounded inspection, the shared canonical row codec and the two read-only
//! migration-codec entrypoints.
//!
//! Every verb registers a bounded [`Operation`] in the ONE runtime registry
//! before any completion can run in JS; `runtime_cancel` cancels and joins
//! any of them. Retained resources (results, cursors, drafts, change sets)
//! are counted against `native_handle_capacity` and byte-charged against the
//! resultBytes aggregate while retained ([`crate::runtime::RetainedNative`]).
//! The engine's `!Send` read state stays inside the worker-affine reactor
//! (`runtime/session.rs`); only owned data crosses. `CompleteResult`/
//! `ResultCursor` are owned `Send` values by construction — the static
//! asserts below make that C05 assumption a compile lock, never a cast.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use bumbledb::work::{ByteKind, ExecutionPolicy, WorkContext};
use bumbledb::{
    ChangeError, ChangeSet, CompleteResult, RelationId, ResultCursor, Theory as _, Value,
};
use napi::bindgen_prelude::{Array, BigInt, Buffer, Env, External, Function, Object, Unknown};
use napi_derive::napi;

use crate::marshal::{self, ValueOut};
use crate::runtime::{Output, RetainedNative, Runtime, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, PolicyWire, RuntimeHandle, SessionHandle, notification, operation_handle,
    owner, reporter, session, take_output, thrown, unshared_input,
};

// C05 compile locks: the sealed result and its consuming cursor are OWNED
// `Send` values (RAM answers or a private temporary-LMDB scratch env plus
// charges). If a P03R change makes either `!Send`, this fails compile HERE
// and the result registry must move into the owning session thread — never
// an `unsafe impl Send`.
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<CompleteResult>();
    assert_send::<ResultCursor>();
};

fn identity() -> usize {
    crate::runtime_wire::addon_identity()
}

// ---------------------------------------------------------------------------
// Handles.
// ---------------------------------------------------------------------------

/// A coherent owned snapshot: one worker-affine read session pinning one
/// generation, plus the sealed roster datum for JS-thread marshalling. The
/// log's published snapshots mint this SAME type (`log_wire.rs`), so the
/// core reader capability is literally shared (chapter 30/35).
pub struct SnapshotHandle {
    identity: usize,
    pub(crate) session: Arc<crate::runtime::session::SnapshotSession>,
    pub(crate) sealed: Arc<crate::Sealed>,
}

impl SnapshotHandle {
    pub(crate) fn assemble(
        session: Arc<crate::runtime::session::SnapshotSession>,
        sealed: Arc<crate::Sealed>,
    ) -> Self {
        Self {
            identity: identity(),
            session,
            sealed,
        }
    }
}

pub(crate) fn snapshot(
    handle: &SnapshotHandle,
) -> Result<&Arc<crate::runtime::session::SnapshotSession>, RuntimeError> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    Ok(&handle.session)
}

/// The snapshot-bound execution session crossing back from
/// `runtime_snapshot_session` (taken by `runtime_session_take` as a bare
/// capability).
pub struct ExecSessionOpened {
    pub session: Arc<crate::runtime::session::SnapshotSession>,
    pub sealed: Arc<crate::Sealed>,
}

/// One sealed completed result. The slot distinguishes SPENT (transferred
/// to a cursor — later use refuses `SpentHandle`) from CLOSED (later use
/// refuses `ClosedHandle`), per chapter 35's result identity rows.
pub struct ResultHandle {
    identity: usize,
    shared: Arc<ResultShared>,
}

pub(crate) struct ResultShared {
    runtime: Arc<Runtime>,
    slot: Mutex<ResultSlot>,
}

struct ResultSlot {
    entry: Option<ResultEntry>,
    spent: bool,
}

struct ResultEntry {
    result: CompleteResult,
    _retained: RetainedNative,
}

/// The one consuming cursor over a spent result's backing.
pub struct CursorHandle {
    identity: usize,
    shared: Arc<CursorShared>,
}

pub(crate) struct CursorShared {
    runtime: Arc<Runtime>,
    slot: Mutex<Option<CursorEntry>>,
}

struct CursorEntry {
    cursor: ResultCursor,
    /// One row fetched past a byte-bounded page boundary, delivered first
    /// on the next pull — never dropped, never double-delivered.
    pending: Option<Vec<ValueOut>>,
    /// The terminal frame was observed AND everything (incl. pending) was
    /// delivered: the next pull is the `null` EOF.
    drained: bool,
    _retained: RetainedNative,
}

/// A database-free change draft (chapter 35 `ChangeSet.builder`).
pub struct DraftHandle {
    identity: usize,
    shared: Arc<DraftShared>,
}

pub(crate) struct DraftShared {
    runtime: Arc<Runtime>,
    slot: Mutex<DraftSlot>,
}

struct DraftSlot {
    entry: Option<DraftEntry>,
}

struct PendingChange {
    relation: RelationId,
    insert: bool,
    values: Vec<Value>,
}

struct DraftEntry {
    schema: Arc<bumbledb::schema::Schema>,
    sealed: Arc<crate::Sealed>,
    pending: Vec<PendingChange>,
    /// CUMULATIVE charged input bytes across every chunk of every call —
    /// chunks never reset it (chapter 35's aggregate draft budget).
    used_input: u64,
    /// The draft's whole-life input allowance, captured at open.
    allowance_input: u64,
    retained: RetainedNative,
}

/// The opened draft crossing back from the executor.
pub struct DraftOpened {
    schema: Arc<bumbledb::schema::Schema>,
    sealed: Arc<crate::Sealed>,
    allowance_input: u64,
}

/// A sealed immutable `ChangeSet` (one command, add-wins normalized).
pub struct ChangesHandle {
    identity: usize,
    shared: Arc<ChangesShared>,
}

pub(crate) struct ChangesShared {
    runtime: Arc<Runtime>,
    slot: Mutex<Option<ChangesEntry>>,
}

struct ChangesEntry {
    changes: ChangeSet,
    schema: Arc<bumbledb::schema::Schema>,
    fingerprint: String,
    _retained: RetainedNative,
}

/// The sealed `ChangeSet` crossing back from a draft finish.
pub struct ChangesOpened {
    changes: ChangeSet,
    schema: Arc<bumbledb::schema::Schema>,
    fingerprint: String,
}

/// One immutable final-state apply outcome, fully owned.
pub enum ApplyOutcomeOwned {
    Accepted {
        store: String,
        generation: u64,
    },
    NoChange {
        store: String,
        generation: u64,
    },
    Rejected(Vec<marshal::ViolationWire>),
    Moved {
        store: String,
        witnessed: u64,
        current: u64,
    },
}

/// Bounded database diagnostics.
pub struct DbInspectionOwned {
    pub generation: u64,
    pub map_bytes: u64,
    pub populated_bytes: u64,
    pub disk_bytes: u64,
    pub resident_estimate_bytes: u64,
    pub retained_operations: u64,
}

// ---------------------------------------------------------------------------
// Shared plumbing.
// ---------------------------------------------------------------------------

fn engine_error(error: &bumbledb::Error) -> RuntimeError {
    crate::runtime::session::engine_error(error)
}

fn change_error(error: &ChangeError) -> RuntimeError {
    RuntimeError::Engine {
        kind: crate::tags::error_family::VALIDATION,
        message: format!("bumbledb changes: {error:?}"),
    }
}

fn lock_poisoned<'a, T>(
    guard: Result<MutexGuard<'a, T>, std::sync::PoisonError<MutexGuard<'a, T>>>,
) -> MutexGuard<'a, T> {
    guard.unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The internal zero-budget policy for close/teardown jobs (mirrors
/// `ManagedDb::access`): teardown attempts use reserved cleanup capacity,
/// never the caller's byte budget.
fn teardown_policy(runtime: &Runtime) -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 0,
        working_bytes: 0,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 0,
        work_units: u64::MAX,
        timeout: runtime
            .options
            .cleanup_timeout
            .max(Duration::from_millis(1)),
    }
}

/// Runs one teardown closure on the executor and reports honestly: `Closed`
/// exactly when the teardown actually ran; `Failed` when it could not be
/// scheduled or was cancelled before running. The completed operation is
/// self-reclaiming (it cancels its own context after the work, so the
/// supervisor reclaims the slot — no take is owed).
pub(crate) fn spawn_teardown(
    runtime: &Arc<Runtime>,
    report: crate::runtime::Report,
    work: impl FnOnce() + Send + 'static,
) {
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = Arc::clone(&done);
    // The one report fires exactly once, whichever side reaches it first
    // (completion notify, or the refusal path when submit never admits).
    let report = Arc::new(Mutex::new(Some(report)));
    let notify_report = Arc::clone(&report);
    let submitted = runtime.submit(
        teardown_policy(runtime),
        Box::new(move || {
            if let Some(report) = lock_poisoned(notify_report.lock()).take() {
                report(if done.load(Ordering::Acquire) {
                    crate::runtime::CloseReport::Closed
                } else {
                    // The teardown never ran (cancelled/faulted before the
                    // work): the resource is NOT reclaimed — never claim
                    // quiescence.
                    crate::runtime::CloseReport::Failed
                });
            }
        }),
        move |_| {
            Ok(Box::new(move |context| {
                work();
                flag.store(true, Ordering::Release);
                // Self-reclaim: nobody takes a teardown outcome.
                context.cancel();
                Ok(Output::Ready)
            }))
        },
    );
    if submitted.is_err() {
        // No teardown capacity (queue full / runtime closed while a job
        // holds the slot): the resource is NOT reclaimed — report the
        // failure honestly instead of claiming quiescence.
        if let Some(report) = lock_poisoned(report.lock()).take() {
            report(crate::runtime::CloseReport::Failed);
        }
    }
}

/// Join-idempotent close of one Mutex-guarded native slot: an uncontended
/// slot drops on the JS thread and reports `Closed`; a contended slot (a
/// bounded job holds it on a worker) is drained by an internal teardown job
/// so the JS thread never blocks. Repeated close joins the empty slot.
fn close_slot<S, T>(
    runtime: &Arc<Runtime>,
    shared: &Arc<S>,
    slot_of: fn(&S) -> &Mutex<Option<T>>,
    report: crate::runtime::Report,
) where
    S: Send + Sync + 'static,
    T: Send + 'static,
{
    if let Ok(mut guard) = slot_of(shared).try_lock() {
        let taken = guard.take();
        drop(guard);
        drop(taken);
        report(crate::runtime::CloseReport::Closed);
        return;
    }
    let shared = Arc::clone(shared);
    spawn_teardown(runtime, report, move || {
        let taken = lock_poisoned(slot_of(&shared).lock()).take();
        drop(taken);
    });
}

/// Conservative owned byte estimate of one marshalled row (the same
/// convention as the engine's sealed-result charge: payload bytes plus a
/// fixed per-cell word).
fn row_out_bytes(row: &[ValueOut]) -> u64 {
    row.iter()
        .map(|cell| {
            8 + match cell {
                ValueOut::Text(text) | ValueOut::Id128(text) => text.len() as u64,
                ValueOut::Bytes(bytes) => bytes.len() as u64,
                ValueOut::IntervalU64 { .. }
                | ValueOut::IntervalI64 { .. }
                | ValueOut::IntervalF64 { .. } => 16,
                _ => 8,
            }
        })
        .sum()
}

fn answers_page_out(rows: &bumbledb::Answers) -> Vec<Vec<ValueOut>> {
    marshal::answers_out(rows)
}

// ---------------------------------------------------------------------------
// Schema compile (charged admission; detached descriptor data).
// ---------------------------------------------------------------------------

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_schema_compile(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    spec: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let mut marshal_error = None;
    let operation =
        runtime.submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            |_| {
                let parsed = match crate::descriptor_of(&spec) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        marshal_error = Some(error);
                        return Err(RuntimeError::InvalidArgument);
                    }
                };
                Ok(Box::new(move |context| {
                    use bumbledb::schema::ValidateDescriptor as _;
                    context.checkpoint()?;
                    let (descriptor, attrs) = match parsed {
                        Ok(parsed) => parsed,
                        Err(
                            crate::OpenOutcome::SchemaError(message)
                            | crate::OpenOutcome::NewtypeMismatch(message),
                        ) => {
                            return Err(RuntimeError::Engine {
                                kind: crate::tags::error_family::SCHEMA,
                                message,
                            });
                        }
                    };
                    context.step(1 + descriptor.relations.len() as u64)?;
                    let sealed = crate::seal(descriptor, attrs);
                    let schema = sealed.descriptor.clone().validate().map_err(|error| {
                        RuntimeError::Engine {
                            kind: crate::tags::error_family::SCHEMA,
                            message: error.to_string(),
                        }
                    })?;
                    let fingerprint = bumbledb::schema::fingerprint::fingerprint(&schema);
                    Ok(Output::Descriptor(marshal::DescriptorWire {
                        manifest: sealed.descriptor.manifest(),
                        statements: sealed.statements,
                        fingerprint: crate::hex_fingerprint(&fingerprint.0),
                        attrs: sealed.attrs,
                    }))
                }))
            },
        );
    if let Some(error) = marshal_error {
        return Err(error);
    }
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_schema_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<marshal::DescriptorWire> {
    match take_output(env, handle)? {
        Output::Descriptor(wire) => Ok(wire),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

// ---------------------------------------------------------------------------
// Snapshots and snapshot-bound sessions.
// ---------------------------------------------------------------------------

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_db_snapshot(
    env: Env,
    db: &External<crate::DbHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let owner = db.owner();
    let runtime = Arc::clone(owner.runtime());
    let operation = runtime
        .open_session(
            owner,
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_snapshot_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Object<'_>> {
    match take_output(env, handle)? {
        Output::Session(opened) => {
            let mut object = Object::new(&env)?;
            object.set(
                "snapshot",
                External::new(SnapshotHandle::assemble(
                    Arc::new(opened.session),
                    opened.sealed,
                )),
            )?;
            let mut witness = Object::new(&env)?;
            witness.set("store", opened.store)?;
            witness.set("generation", BigInt::from(opened.generation))?;
            object.set("witness", witness)?;
            Ok(object)
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_snapshot_close(
    env: Env,
    handle: &External<SnapshotHandle>,
    callback: Function<crate::runtime_wire::CloseWire, ()>,
) -> napi::Result<()> {
    let session = snapshot(handle).map_err(|error| thrown(env, error))?;
    session.drain(reporter(callback)?);
    Ok(())
}

/// A snapshot-bound execution session (chapter 35 `Snapshot.session`): a
/// spendable capability SHARING the snapshot's pinned session and coherent
/// generation. Registration runs one bounded liveness job on the pinned
/// thread so a closed/draining snapshot refuses before any capability mints.
#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_snapshot_session(
    env: Env,
    handle: &External<SnapshotHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let session = snapshot(handle).map_err(|error| thrown(env, error))?;
    let runtime = Arc::clone(session.runtime());
    let shared = Arc::clone(session);
    let sealed = Arc::clone(&handle.sealed);
    let operation = session
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context, _frame| {
                    context.checkpoint()?;
                    Ok(Output::ExecSession(ExecSessionOpened {
                        session: shared,
                        sealed,
                    }))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_snapshot_get(
    env: Env,
    handle: &External<SnapshotHandle>,
    policy: PolicyWire,
    relation: u32,
    key_statement: u32,
    key_values: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let session = snapshot(handle).map_err(|error| thrown(env, error))?;
    let runtime = Arc::clone(session.runtime());
    let sealed = Arc::clone(&handle.sealed);
    let operation = session
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                let (rel, key, row) = marshal::key_row(
                    &sealed.rosters,
                    &sealed.statements,
                    relation,
                    key_statement,
                    &key_values,
                )
                .map_err(|_| RuntimeError::InvalidArgument)?;
                Ok(Box::new(move |context, frame| {
                    context.checkpoint()?;
                    let found = frame
                        .instance
                        .get_dyn(rel, key, &row)
                        .map_err(|error| engine_error(&error))?;
                    Ok(Output::Row(found.map(|values| {
                        values.into_iter().map(ValueOut::from_value).collect()
                    })))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

/// One complete bounded execution job body: prepare, execute to a sealed
/// [`CompleteResult`] (atomic answers — failed work never becomes a logical
/// result), fully owned and independent of the pinned frame.
fn execute_complete_work(
    query: bumbledb::Query,
    params: Vec<marshal::OwnedParam>,
) -> crate::runtime::session::ReadWork {
    Box::new(move |context, frame| {
        context.checkpoint()?;
        let mut prepared = frame
            .instance
            .prepare(&query)
            .map_err(|error| engine_error(&error))?;
        let args = crate::param_args(&params);
        let result = prepared
            .execute_complete(frame.instance, args.as_slice())
            .map_err(|error| engine_error(&error))?;
        Ok(Output::CompleteResult(result))
    })
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_snapshot_execute(
    env: Env,
    handle: &External<SnapshotHandle>,
    policy: PolicyWire,
    query: Object,
    params: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let session = snapshot(handle).map_err(|error| thrown(env, error))?;
    let runtime = Arc::clone(session.runtime());
    let operation = session
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                let query = marshal::query_in(&query).map_err(|_| RuntimeError::InvalidArgument)?;
                let params =
                    marshal::params_in(&params).map_err(|_| RuntimeError::InvalidArgument)?;
                Ok(execute_complete_work(query, params))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

/// The `ParsedQuery` execution form on a snapshot-bound execution session
/// (the db-bridge `runtimeSessionExecute`; the retained-prepared worker
/// lane is `runtime_session_execute_prepared` in `runtime_wire.rs`).
#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_session_execute(
    env: Env,
    handle: &External<SessionHandle>,
    policy: PolicyWire,
    query: Object,
    params: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let shared = session(handle).map_err(|error| thrown(env, error))?;
    let runtime = Arc::clone(shared.runtime());
    let operation = shared
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                let query = marshal::query_in(&query).map_err(|_| RuntimeError::InvalidArgument)?;
                let params =
                    marshal::params_in(&params).map_err(|_| RuntimeError::InvalidArgument)?;
                Ok(execute_complete_work(query, params))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

// ---------------------------------------------------------------------------
// Results: take/collect/cursor/pages/close.
// ---------------------------------------------------------------------------

#[napi]
pub fn runtime_result_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<ResultHandle>> {
    let runtime = crate::runtime_wire::operation_runtime(handle);
    match take_output(env, handle)? {
        Output::CompleteResult(result) => {
            let retained = runtime
                .retain_native(result.byte_len())
                .map_err(|error| thrown(env, error))?;
            Ok(External::new(ResultHandle {
                identity: identity(),
                shared: Arc::new(ResultShared {
                    runtime,
                    slot: Mutex::new(ResultSlot {
                        entry: Some(ResultEntry {
                            result,
                            _retained: retained,
                        }),
                        spent: false,
                    }),
                }),
            }))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

fn result_shared(handle: &ResultHandle) -> Result<&Arc<ResultShared>, RuntimeError> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    Ok(&handle.shared)
}

/// Bounded total materialization over the sealed backing: refuses
/// (`ResourceLimit`) BEFORE materializing past the caller's resultBytes cap;
/// a cap refusal leaves the sealed backing available.
pub(crate) fn collect_result(
    shared: &ResultShared,
    context: &WorkContext,
    result_bytes: u64,
) -> Result<Output, RuntimeError> {
    let mut slot = lock_poisoned(shared.slot.lock());
    let Some(entry) = slot.entry.as_mut() else {
        return Err(if slot.spent {
            RuntimeError::SpentHandle
        } else {
            RuntimeError::ClosedHandle
        });
    };
    let bytes = entry.result.byte_len();
    if bytes > result_bytes {
        return Err(RuntimeError::ResourceLimit {
            dimension: "resultBytes",
            used: 0,
            requested: bytes,
            limit: result_bytes,
        });
    }
    let charge = context.reserve(ByteKind::Result, bytes)?;
    entry.result.rebind_work(context);
    let answers = entry
        .result
        .collect(u64::MAX)
        .map_err(|error| engine_error(&error))?;
    let rows = answers_page_out(&answers);
    drop(charge);
    Ok(Output::Rows(rows))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_result_collect(
    env: Env,
    handle: &External<ResultHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let shared = Arc::clone(result_shared(handle).map_err(|error| thrown(env, error))?);
    let runtime = Arc::clone(&shared.runtime);
    let parsed = policy.parse().map_err(|error| thrown(env, error))?;
    let result_bytes = parsed.result_bytes;
    let operation = runtime
        .submit(parsed, notification(callback)?, move |_| {
            Ok(Box::new(move |context| {
                context.checkpoint()?;
                collect_result(&shared, context, result_bytes)
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

/// The atomic spend: moves the completed result's backing into one private
/// cursor. A second transfer, or transfer racing collect, refuses
/// (`SpentHandle`) before touching the backing.
pub(crate) fn transfer_result(
    shared: &ResultShared,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    let mut slot = lock_poisoned(shared.slot.lock());
    let Some(entry) = slot.entry.take() else {
        return Err(if slot.spent {
            RuntimeError::SpentHandle
        } else {
            RuntimeError::ClosedHandle
        });
    };
    slot.spent = true;
    drop(slot);
    context.checkpoint()?;
    let ResultEntry {
        result,
        _retained: retained,
    } = entry;
    // Page granularity: one row per underlying pull; the byte-bounded page
    // assembly happens in `cursor_pull` under each pull's own policy.
    let cursor = result.into_cursor(1);
    drop(retained);
    Ok(Output::ResultCursor(cursor))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_result_cursor(
    env: Env,
    handle: &External<ResultHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let shared = Arc::clone(result_shared(handle).map_err(|error| thrown(env, error))?);
    let runtime = Arc::clone(&shared.runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    transfer_result(&shared, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_cursor_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<CursorHandle>> {
    let runtime = crate::runtime_wire::operation_runtime(handle);
    match take_output(env, handle)? {
        Output::ResultCursor(cursor) => {
            let retained = runtime
                .retain_native(cursor.byte_len())
                .map_err(|error| thrown(env, error))?;
            Ok(External::new(CursorHandle {
                identity: identity(),
                shared: Arc::new(CursorShared {
                    runtime,
                    slot: Mutex::new(Some(CursorEntry {
                        cursor,
                        pending: None,
                        drained: false,
                        _retained: retained,
                    })),
                }),
            }))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

fn cursor_shared(handle: &CursorHandle) -> Result<&Arc<CursorShared>, RuntimeError> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    Ok(&handle.shared)
}

/// One owned page bounded by the pull's resultBytes: at least one row per
/// page, rows accumulate until the next row would cross the cap (it is
/// buffered, never dropped). `None` is EOF — exactly once, only after the
/// terminal frame was observed AND everything was delivered. A read failure
/// stops delivery WITHOUT a terminal frame: the delivered prefix is
/// explicitly incomplete, never mistaken for the complete set.
pub(crate) fn cursor_pull(
    shared: &CursorShared,
    context: &WorkContext,
    page_bytes: u64,
) -> Result<Output, RuntimeError> {
    let mut slot = lock_poisoned(shared.slot.lock());
    let Some(entry) = slot.as_mut() else {
        return Err(RuntimeError::ClosedHandle);
    };
    if entry.drained {
        return Ok(Output::Page(None));
    }
    entry.cursor.rebind_work(context);
    let mut rows: Vec<Vec<ValueOut>> = Vec::new();
    let mut bytes: u64 = 0;
    let mut terminal = false;
    if let Some(pending) = entry.pending.take() {
        bytes += row_out_bytes(&pending);
        context
            .reserve(ByteKind::Result, row_out_bytes(&pending))
            .map(drop)?;
        rows.push(pending);
    }
    while !terminal && bytes < page_bytes.max(1) {
        context.checkpoint()?;
        let Some(page) = entry
            .cursor
            .next_page()
            .map_err(|error| engine_error(&error))?
        else {
            terminal = true;
            break;
        };
        if page.terminal {
            terminal = true;
        }
        if page.rows.is_empty() {
            continue;
        }
        let mut out = answers_page_out(&page.rows);
        for row in out.drain(..) {
            let size = row_out_bytes(&row);
            if !rows.is_empty() && bytes.saturating_add(size) > page_bytes.max(1) {
                entry.pending = Some(row);
                // The terminal frame is only honored once the buffered row
                // has been delivered.
                terminal = false;
                break;
            }
            context.reserve(ByteKind::Result, size).map(drop)?;
            bytes += size;
            rows.push(row);
        }
        if entry.pending.is_some() {
            break;
        }
    }
    if terminal && entry.pending.is_none() {
        entry.drained = true;
    }
    if rows.is_empty() {
        // Nothing left to deliver: this pull IS the EOF signal.
        entry.drained = true;
        return Ok(Output::Page(None));
    }
    Ok(Output::Page(Some(rows)))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_cursor_next(
    env: Env,
    handle: &External<CursorHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let shared = Arc::clone(cursor_shared(handle).map_err(|error| thrown(env, error))?);
    let runtime = Arc::clone(&shared.runtime);
    let parsed = policy.parse().map_err(|error| thrown(env, error))?;
    let page_bytes = parsed.result_bytes;
    let operation = runtime
        .submit(parsed, notification(callback)?, move |_| {
            Ok(Box::new(move |context| {
                context.checkpoint()?;
                cursor_pull(&shared, context, page_bytes)
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_page_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Option<Vec<Vec<ValueOut>>>> {
    match take_output(env, handle)? {
        Output::Page(page) => Ok(page),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_result_close(
    env: Env,
    handle: &External<ResultHandle>,
    callback: Function<crate::runtime_wire::CloseWire, ()>,
) -> napi::Result<()> {
    let shared = result_shared(handle).map_err(|error| thrown(env, error))?;
    let report = reporter(callback)?;
    // Results use the two-state slot (spent vs closed); adapt the generic
    // close over the entry Option.
    let runtime = Arc::clone(&shared.runtime);
    if let Ok(mut slot) = shared.slot.try_lock() {
        let taken = slot.entry.take();
        drop(slot);
        drop(taken);
        report(crate::runtime::CloseReport::Closed);
        return Ok(());
    }
    let inner = Arc::clone(shared);
    spawn_teardown(&runtime, report, move || {
        let taken = lock_poisoned(inner.slot.lock()).entry.take();
        drop(taken);
    });
    Ok(())
}

#[napi]
pub fn runtime_cursor_close(
    env: Env,
    handle: &External<CursorHandle>,
    callback: Function<crate::runtime_wire::CloseWire, ()>,
) -> napi::Result<()> {
    let shared = cursor_shared(handle).map_err(|error| thrown(env, error))?;
    close_slot(&shared.runtime, shared, |s| &s.slot, reporter(callback)?);
    Ok(())
}

// ---------------------------------------------------------------------------
// Drafts (database-free ChangeSet construction) and sealed change sets.
// ---------------------------------------------------------------------------

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_draft_open(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    spec: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let parsed_policy = policy.parse().map_err(|error| thrown(env, error))?;
    let allowance_input = parsed_policy.input_bytes;
    let mut marshal_error = None;
    let operation = runtime.submit(parsed_policy, notification(callback)?, |_| {
        let parsed = match crate::descriptor_of(&spec) {
            Ok(parsed) => parsed,
            Err(error) => {
                marshal_error = Some(error);
                return Err(RuntimeError::InvalidArgument);
            }
        };
        Ok(Box::new(move |context| {
            use bumbledb::schema::ValidateDescriptor as _;
            context.checkpoint()?;
            let (descriptor, attrs) = match parsed {
                Ok(parsed) => parsed,
                Err(
                    crate::OpenOutcome::SchemaError(message)
                    | crate::OpenOutcome::NewtypeMismatch(message),
                ) => {
                    return Err(RuntimeError::Engine {
                        kind: crate::tags::error_family::SCHEMA,
                        message,
                    });
                }
            };
            let sealed = Arc::new(crate::seal(descriptor, attrs));
            let schema =
                sealed
                    .descriptor
                    .clone()
                    .validate()
                    .map_err(|error| RuntimeError::Engine {
                        kind: crate::tags::error_family::SCHEMA,
                        message: error.to_string(),
                    })?;
            Ok(Output::Draft(DraftOpened {
                schema: Arc::new(schema),
                sealed,
                allowance_input,
            }))
        }))
    });
    if let Some(error) = marshal_error {
        return Err(error);
    }
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_draft_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<DraftHandle>> {
    let runtime = crate::runtime_wire::operation_runtime(handle);
    match take_output(env, handle)? {
        Output::Draft(opened) => {
            let retained = runtime
                .retain_native(0)
                .map_err(|error| thrown(env, error))?;
            Ok(External::new(DraftHandle {
                identity: identity(),
                shared: Arc::new(DraftShared {
                    runtime,
                    slot: Mutex::new(DraftSlot {
                        entry: Some(DraftEntry {
                            schema: opened.schema,
                            sealed: opened.sealed,
                            pending: Vec::new(),
                            used_input: 0,
                            allowance_input: opened.allowance_input,
                            retained,
                        }),
                    }),
                }),
            }))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

fn draft_shared(handle: &DraftHandle) -> Result<&Arc<DraftShared>, RuntimeError> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    Ok(&handle.shared)
}

/// One bounded ingestion chunk into the draft. Chunks share the draft's
/// CUMULATIVE input budget (never reset); a budget or shape failure SPENDS
/// the draft (the failed capability admits nothing further).
pub(crate) fn draft_ingest(
    shared: &DraftShared,
    context: &WorkContext,
    relation: u32,
    insert: bool,
    rows: Vec<Vec<Value>>,
    chunk_bytes: u64,
) -> Result<Output, RuntimeError> {
    let mut slot = lock_poisoned(shared.slot.lock());
    let Some(entry) = slot.entry.as_mut() else {
        return Err(RuntimeError::SpentHandle);
    };
    context.checkpoint()?;
    let next = entry.used_input.saturating_add(chunk_bytes);
    if next > entry.allowance_input {
        let refusal = RuntimeError::ResourceLimit {
            dimension: "inputBytes",
            used: entry.used_input,
            requested: chunk_bytes,
            limit: entry.allowance_input,
        };
        // Budget exhaustion spends the draft: drop everything staged.
        slot.entry = None;
        return Err(refusal);
    }
    if let Err(refusal) = entry.retained.grow(chunk_bytes) {
        slot.entry = None;
        return Err(refusal);
    }
    entry.used_input = next;
    let submitted = rows.len() as u64;
    let rel = RelationId(relation);
    for values in rows {
        context.rows(1)?;
        entry.pending.push(PendingChange {
            relation: rel,
            insert,
            values,
        });
    }
    Ok(Output::Mutation {
        submitted,
        changed: submitted,
    })
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn draft_mutation(
    env: Env,
    handle: &External<DraftHandle>,
    policy: PolicyWire,
    relation: u32,
    rows: &BigInt,
    cells: Array,
    callback: Function<(), ()>,
    insert: bool,
) -> napi::Result<External<OperationHandle>> {
    let shared = Arc::clone(draft_shared(handle).map_err(|error| thrown(env, error))?);
    let runtime = Arc::clone(&shared.runtime);
    let stated = marshal::u64_in(rows, "draft rows")?;
    // The sealed roster snapshot for JS-thread marshalling: a spent draft
    // refuses before any conversion.
    let sealed = {
        let slot = lock_poisoned(shared.slot.lock());
        match slot.entry.as_ref() {
            Some(entry) => Arc::clone(&entry.sealed),
            None => return Err(thrown(env, RuntimeError::SpentHandle)),
        }
    };
    let spend_on_error = Arc::clone(&shared);
    let operation = runtime.submit(
        policy.parse().map_err(|error| thrown(env, error))?,
        notification(callback)?,
        move |context| {
            // Parse-once, shape-proved rows built on the JS thread; a shape
            // failure spends the draft (typed input failure, tracked drain).
            let parsed = parse_draft_rows(&sealed, relation, stated, &cells, context);
            match parsed {
                Ok((rows, bytes)) => Ok(Box::new(move |context: &WorkContext| {
                    context.checkpoint()?;
                    draft_ingest(&shared, context, relation, insert, rows, bytes)
                }) as crate::runtime::Work),
                Err(error) => {
                    lock_poisoned(spend_on_error.slot.lock()).entry = None;
                    Err(error)
                }
            }
        },
    );
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

fn parse_draft_rows(
    sealed: &crate::Sealed,
    relation: u32,
    stated: u64,
    cells: &Array,
    context: &WorkContext,
) -> Result<(Vec<Vec<Value>>, u64), RuntimeError> {
    let roster = sealed
        .rosters
        .get(relation as usize)
        .ok_or(RuntimeError::InvalidArgument)?;
    let arity = roster.fields.len();
    let len = cells.len() as usize;
    let expected = u128::from(stated) * (arity as u128);
    if expected != len as u128 {
        return Err(RuntimeError::InvalidArgument);
    }
    if arity == 0 {
        // Arity-0 relations: N empty tuples collapse to at most one fact;
        // the stated count is data, never a loop bound.
        context.input(0)?;
        let rows = if stated == 0 {
            Vec::new()
        } else {
            vec![Vec::new()]
        };
        return Ok((rows, 0));
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(usize::try_from(stated).map_err(|_| RuntimeError::InvalidArgument)?)
        .map_err(|_| RuntimeError::Internal)?;
    let mut bytes: u64 = 0;
    let mut row = Vec::with_capacity(arity);
    for index in 0..cells.len() {
        let field = &roster.fields[(index as usize) % arity];
        let value = marshal::req_at::<Unknown>(cells, index, "draft cells")
            .map_err(|_| RuntimeError::InvalidArgument)?;
        let value = marshal::schema_value_in(&field.value_type, &value, &roster.name, &field.name)
            .map_err(|_| RuntimeError::InvalidArgument)?;
        bytes = bytes.saturating_add(value_bytes(&value));
        row.push(value);
        if row.len() == arity {
            rows.push(std::mem::replace(&mut row, Vec::with_capacity(arity)));
        }
    }
    context.input(bytes)?;
    Ok((rows, bytes))
}

fn value_bytes(value: &Value) -> u64 {
    8 + match value {
        Value::String(text) => text.len() as u64,
        Value::FixedBytes(bytes) => bytes.len() as u64,
        Value::Id128(_) | Value::IntervalU64(_) | Value::IntervalI64(_) | Value::IntervalF64(_) => {
            16
        }
        _ => 8,
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn runtime_draft_insert(
    env: Env,
    handle: &External<DraftHandle>,
    policy: PolicyWire,
    relation: u32,
    rows: BigInt,
    cells: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    draft_mutation(env, handle, policy, relation, &rows, cells, callback, true)
}

#[napi]
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn runtime_draft_delete(
    env: Env,
    handle: &External<DraftHandle>,
    policy: PolicyWire,
    relation: u32,
    rows: BigInt,
    cells: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    draft_mutation(env, handle, policy, relation, &rows, cells, callback, false)
}

#[napi]
pub fn runtime_report_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Object<'_>> {
    match take_output(env, handle)? {
        Output::Mutation { submitted, changed } => {
            let mut object = Object::new(&env)?;
            object.set("submitted", BigInt::from(submitted))?;
            object.set("changed", BigInt::from(changed))?;
            Ok(object)
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

/// Consumes the draft into one immutable schema-bound `ChangeSet` (one
/// command, add-wins normalization — the engine's own `ChangeSetBuilder`).
pub(crate) fn draft_finish(
    shared: &DraftShared,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    let entry = {
        let mut slot = lock_poisoned(shared.slot.lock());
        slot.entry.take().ok_or(RuntimeError::SpentHandle)?
    };
    context.checkpoint()?;
    let mut builder = ChangeSet::builder(&entry.schema, context.clone());
    for change in &entry.pending {
        context.step(1)?;
        let landed = if change.insert {
            builder.insert(change.relation, &change.values)
        } else {
            builder.delete(change.relation, &change.values)
        };
        landed.map_err(|error| change_error(&error))?;
    }
    let changes = builder.finish().map_err(|error| change_error(&error))?;
    let fingerprint = crate::hex_fingerprint(&changes.schema().0);
    Ok(Output::Changes(ChangesOpened {
        changes,
        schema: Arc::clone(&entry.schema),
        fingerprint,
    }))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_draft_finish(
    env: Env,
    handle: &External<DraftHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let shared = Arc::clone(draft_shared(handle).map_err(|error| thrown(env, error))?);
    let runtime = Arc::clone(&shared.runtime);
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    draft_finish(&shared, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_changes_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Object<'_>> {
    let runtime = crate::runtime_wire::operation_runtime(handle);
    match take_output(env, handle)? {
        Output::Changes(opened) => {
            let retained = runtime
                .retain_native(opened.changes.as_bytes().len() as u64)
                .map_err(|error| thrown(env, error))?;
            let fingerprint = opened.fingerprint.clone();
            let mut object = Object::new(&env)?;
            object.set(
                "changes",
                External::new(ChangesHandle {
                    identity: identity(),
                    shared: Arc::new(ChangesShared {
                        runtime,
                        slot: Mutex::new(Some(ChangesEntry {
                            changes: opened.changes,
                            schema: opened.schema,
                            fingerprint: opened.fingerprint,
                            _retained: retained,
                        })),
                    }),
                }),
            )?;
            object.set("fingerprint", fingerprint)?;
            Ok(object)
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_draft_close(
    env: Env,
    handle: &External<DraftHandle>,
    callback: Function<crate::runtime_wire::CloseWire, ()>,
) -> napi::Result<()> {
    let shared = draft_shared(handle).map_err(|error| thrown(env, error))?;
    let report = reporter(callback)?;
    let runtime = Arc::clone(&shared.runtime);
    if let Ok(mut slot) = shared.slot.try_lock() {
        let taken = slot.entry.take();
        drop(slot);
        drop(taken);
        report(crate::runtime::CloseReport::Closed);
        return Ok(());
    }
    let inner = Arc::clone(shared);
    spawn_teardown(&runtime, report, move || {
        let taken = lock_poisoned(inner.slot.lock()).entry.take();
        drop(taken);
    });
    Ok(())
}

/// The registered change capability for the log's command seal
/// (`log_wire.rs`): the retained sealed `ChangeSet`, its schema and its
/// owning runtime — the native side derives the runtime from the handle
/// (chapter 35: seal "retains the change's captured runtime").
pub(crate) fn changes_entry(
    handle: &ChangesHandle,
) -> Result<
    (
        ChangeSet,
        Arc<bumbledb::schema::Schema>,
        String,
        Arc<Runtime>,
    ),
    RuntimeError,
> {
    if handle.identity != identity() {
        return Err(RuntimeError::ForeignRuntime);
    }
    let slot = lock_poisoned(handle.shared.slot.lock());
    let entry = slot.as_ref().ok_or(RuntimeError::ClosedHandle)?;
    Ok((
        entry.changes.clone(),
        Arc::clone(&entry.schema),
        entry.fingerprint.clone(),
        Arc::clone(&handle.shared.runtime),
    ))
}

#[napi]
pub fn runtime_changes_close(
    env: Env,
    handle: &External<ChangesHandle>,
    callback: Function<crate::runtime_wire::CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    close_slot(
        &handle.shared.runtime,
        &handle.shared,
        |s| &s.slot,
        reporter(callback)?,
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Apply: one immutable final-state admission/commit.
// ---------------------------------------------------------------------------

pub(crate) enum ExpectedOwned {
    Any,
    Exact { store: String, generation: u64 },
}

/// Clears the database's single-writer admission flag however the apply
/// settles (the same fence the write-session reactor uses: refusal, never a
/// parked thread).
struct ApplyWriterFlag(Arc<crate::DbInner>);

impl Drop for ApplyWriterFlag {
    fn drop(&mut self) {
        self.0.writing.store(false, Ordering::Release);
    }
}

/// One immutable final-state apply over the managed owner: exclusive-writer
/// admission by refusal (`WriterBusy`), witness comparison as a DOMAIN
/// outcome (`moved`), the engine's complete final-state judgment, one
/// durable commit. The native side re-judges schema identity — a foreign
/// `ChangeSet` refuses typed regardless of what the host asserted.
pub(crate) fn apply_change_set(
    lease: &crate::runtime::owners::DbLease,
    changes: &ChangeSet,
    expected: &ExpectedOwned,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    context.checkpoint()?;
    let store_hex = lease.db().integration_store().identity().store.to_string();
    if lease.writing.swap(true, Ordering::AcqRel) {
        return Err(RuntimeError::WriterBusy);
    }
    let _flag = ApplyWriterFlag(lease.inner_arc());
    let mut session = lease
        .db()
        .integration_writer(context)
        .map_err(integration_error)?;
    if let ExpectedOwned::Exact { store, generation } = expected {
        if *store != store_hex {
            return Err(RuntimeError::Engine {
                kind: crate::tags::error_family::FOREIGN_WITNESS,
                message: "expected-state witness names a different store".into(),
            });
        }
        let current = session.generation().map_err(integration_error)?;
        if current.value() != *generation {
            return Ok(Output::Apply(ApplyOutcomeOwned::Moved {
                store: store_hex,
                witnessed: *generation,
                current: current.value(),
            }));
        }
    }
    match session.prepare(changes).map_err(integration_error)? {
        bumbledb::Admission::Rejected(violations) => {
            Ok(Output::Apply(ApplyOutcomeOwned::Rejected(
                crate::violations_wire(&lease.sealed.descriptor, &violations),
            )))
        }
        bumbledb::Admission::Accepted(prepared) => {
            let sealed = prepared
                .seal(bumbledb::integration::HostChanges {
                    records: &[],
                    attachment: bumbledb::integration::AttachmentChange::Keep,
                })
                .map_err(integration_error)?;
            let commit = sealed.commit().map_err(integration_error)?;
            let outcome = if commit.changed {
                ApplyOutcomeOwned::Accepted {
                    store: store_hex,
                    generation: commit.generation.value(),
                }
            } else {
                ApplyOutcomeOwned::NoChange {
                    store: store_hex,
                    generation: commit.generation.value(),
                }
            };
            Ok(Output::Apply(outcome))
        }
    }
}

pub(crate) fn integration_error(error: bumbledb::integration::IntegrationError) -> RuntimeError {
    use bumbledb::integration::IntegrationError;
    match error {
        IntegrationError::Core(error) => engine_error(&error),
        IntegrationError::Changes(error) => change_error(&error),
        IntegrationError::Host(error) => RuntimeError::Engine {
            kind: "hostSeal",
            message: format!("{error:?}"),
        },
        IntegrationError::Work(error) => RuntimeError::Work(error),
        IntegrationError::ForeignSchema => RuntimeError::Engine {
            kind: crate::tags::error_family::SCHEMA_MISMATCH,
            message: "the ChangeSet's schema is not this database's schema".into(),
        },
        IntegrationError::ReentrantWriter => RuntimeError::WriterBusy,
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_db_apply(
    env: Env,
    db: &External<crate::DbHandle>,
    policy: PolicyWire,
    changes: &External<ChangesHandle>,
    expected: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let owner = db.owner();
    let runtime = Arc::clone(owner.runtime());
    let (change_set, _schema, _fingerprint, changes_runtime) =
        changes_entry(changes).map_err(|error| thrown(env, error))?;
    if !Arc::ptr_eq(&runtime, &changes_runtime) {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let expected = {
        let kind: String = marshal::req(&expected, "kind", "apply expected")?;
        match kind.as_str() {
            "any" => ExpectedOwned::Any,
            "exact" => ExpectedOwned::Exact {
                store: marshal::req::<String>(&expected, "store", "apply expected")?,
                generation: marshal::u64_in(
                    &marshal::req::<BigInt>(&expected, "generation", "apply expected")?,
                    "apply expected",
                )?,
            },
            other => {
                return Err(marshal::err(format!(
                    "bumbledb marshal: unknown expected kind `{other}`"
                )));
            }
        }
    };
    let lease = db.owner().access().map_err(|error| thrown(env, error))?;
    let operation = runtime
        .submit_db(
            owner,
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |context| {
                context.input(change_set.as_bytes().len() as u64)?;
                Ok(Box::new(move |context| {
                    apply_change_set(&lease, &change_set, &expected, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_apply_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Object<'_>> {
    let mut object = Object::new(&env)?;
    let witness = |env: &Env, store: String, generation: u64| -> napi::Result<Object<'_>> {
        let mut wire = Object::new(env)?;
        wire.set("store", store)?;
        wire.set("generation", BigInt::from(generation))?;
        Ok(wire)
    };
    match take_output(env, handle)? {
        Output::Apply(ApplyOutcomeOwned::Accepted { store, generation }) => {
            object.set("tag", "accepted")?;
            object.set("witness", witness(&env, store, generation)?)?;
        }
        Output::Apply(ApplyOutcomeOwned::NoChange { store, generation }) => {
            object.set("tag", "no-change")?;
            object.set("witness", witness(&env, store, generation)?)?;
        }
        Output::Apply(ApplyOutcomeOwned::Rejected(violations)) => {
            object.set("tag", "invariant-rejected")?;
            object.set("violations", violations)?;
        }
        Output::Apply(ApplyOutcomeOwned::Moved {
            store,
            witnessed,
            current,
        }) => {
            object.set("tag", "moved")?;
            object.set("witnessed", witness(&env, store.clone(), witnessed)?)?;
            object.set("current", witness(&env, store, current)?)?;
        }
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    }
    Ok(object)
}

// ---------------------------------------------------------------------------
// Bounded inspection.
// ---------------------------------------------------------------------------

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_db_inspect(
    env: Env,
    db: &External<crate::DbHandle>,
    policy: PolicyWire,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let owner = db.owner();
    let runtime = Arc::clone(owner.runtime());
    let lease = owner.access().map_err(|error| thrown(env, error))?;
    let (owner_id, database_id) = owner.ids();
    let operation =
        runtime
            .submit_db(
                owner,
                policy.parse().map_err(|error| thrown(env, error))?,
                notification(callback)?,
                move |_| {
                    Ok(Box::new(move |context| {
                        context.checkpoint()?;
                        let generation = lease
                            .db()
                            .generation()
                            .map_err(|error| engine_error(&error))?;
                        let report = lease.db().integration_store().map_report(context).map_err(
                            |error| engine_error(&bumbledb::Error::Store(Box::new(error))),
                        )?;
                        let retained = lease.runtime().database_operations(owner_id, database_id);
                        Ok(Output::DbReport(DbInspectionOwned {
                            generation: generation.value(),
                            map_bytes: report.virtual_map_bytes,
                            populated_bytes: report.populated_file_bytes,
                            disk_bytes: report
                                .allocated_disk_bytes
                                .unwrap_or(report.populated_file_bytes),
                            resident_estimate_bytes: report.non_free_page_bytes,
                            retained_operations: retained,
                        }))
                    }))
                },
            )
            .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_db_inspect_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Object<'_>> {
    match take_output(env, handle)? {
        Output::DbReport(report) => {
            let mut object = Object::new(&env)?;
            object.set("generation", BigInt::from(report.generation))?;
            object.set("mapBytes", BigInt::from(report.map_bytes))?;
            object.set("populatedBytes", BigInt::from(report.populated_bytes))?;
            object.set("diskBytes", BigInt::from(report.disk_bytes))?;
            object.set(
                "residentEstimateBytes",
                BigInt::from(report.resident_estimate_bytes),
            )?;
            object.set(
                "retainedOperations",
                BigInt::from(report.retained_operations),
            )?;
            Ok(object)
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

// ---------------------------------------------------------------------------
// The shared canonical row codec (also the log/migration change encoding):
// the ChangeSet grammar — schema-fingerprint-bound, canonical order, set
// semantics. encodeRows emits one all-adds ChangeSet of the named relation;
// decodeRows strictly parses one and refuses foreign relations or removes.
// ---------------------------------------------------------------------------

pub(crate) fn encode_rows_bytes(
    schema: &bumbledb::schema::Schema,
    relation: RelationId,
    rows: &[Vec<Value>],
    context: &WorkContext,
) -> Result<Vec<u8>, RuntimeError> {
    let mut builder = ChangeSet::builder(schema, context.clone());
    for values in rows {
        context.step(1)?;
        builder
            .insert(relation, values)
            .map_err(|error| change_error(&error))?;
    }
    let changes = builder.finish().map_err(|error| change_error(&error))?;
    Ok(changes.as_bytes().to_vec())
}

pub(crate) fn decode_rows_values(
    schema: &bumbledb::schema::Schema,
    relation: RelationId,
    bytes: &[u8],
    context: &WorkContext,
) -> Result<Vec<Vec<Value>>, RuntimeError> {
    let changes = ChangeSet::parse(schema, bytes, context).map_err(|error| change_error(&error))?;
    let Some(relation_ref) = schema.relation_checked(relation) else {
        return Err(RuntimeError::InvalidArgument);
    };
    let fields = relation_ref.fields();
    let mut rows = Vec::new();
    for record in changes.records() {
        context.step(1)?;
        if record.relation != relation || record.kind != bumbledb::changes::ChangeKind::Add {
            return Err(RuntimeError::Engine {
                kind: crate::tags::error_family::VALIDATION,
                message: "decodeRows: the payload carries records outside the requested \
                          relation's adds"
                    .into(),
            });
        }
        let decoded =
            bumbledb::canonical::decode(fields, record.row, context).map_err(|error| {
                RuntimeError::Engine {
                    kind: crate::tags::error_family::CORRUPTION,
                    message: format!("decodeRows: {error}"),
                }
            })?;
        rows.push(decoded.values);
    }
    Ok(rows)
}

#[napi]
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn runtime_encode_rows(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    spec: Object,
    relation: u32,
    rows: BigInt,
    cells: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let stated = marshal::u64_in(&rows, "encode rows")?;
    let mut marshal_error = None;
    let operation = runtime.submit(
        policy.parse().map_err(|error| thrown(env, error))?,
        notification(callback)?,
        |context| {
            let prepared = (|| -> napi::Result<(bumbledb::schema::Schema, RelationId, Vec<Vec<Value>>, u64)> {
                use bumbledb::schema::ValidateDescriptor as _;
                let (descriptor, attrs) = match crate::descriptor_of(&spec)? {
                    Ok(parsed) => parsed,
                    Err(
                        crate::OpenOutcome::SchemaError(message)
                        | crate::OpenOutcome::NewtypeMismatch(message),
                    ) => {
                        return Err(marshal::err(message));
                    }
                };
                let sealed = crate::seal(descriptor, attrs);
                let schema = sealed
                    .descriptor
                    .clone()
                    .validate()
                    .map_err(|error| marshal::err(error.to_string()))?;
                let roster = sealed
                    .rosters
                    .get(relation as usize)
                    .ok_or_else(|| marshal::err("encodeRows: unknown relation id".into()))?;
                let arity = roster.fields.len();
                if u128::from(stated) * (arity as u128) != u128::from(cells.len()) {
                    return Err(marshal::err("encodeRows: stated rows disagree with cells".into()));
                }
                let mut rows = Vec::new();
                let mut bytes = 0u64;
                let mut row = Vec::with_capacity(arity.max(1));
                for index in 0..cells.len() {
                    if arity == 0 {
                        break;
                    }
                    let field = &roster.fields[(index as usize) % arity.max(1)];
                    let value = marshal::req_at::<Unknown>(&cells, index, "encodeRows cells")?;
                    let value = marshal::schema_value_in(
                        &field.value_type,
                        &value,
                        &roster.name,
                        &field.name,
                    )?;
                    bytes = bytes.saturating_add(value_bytes(&value));
                    row.push(value);
                    if row.len() == arity {
                        rows.push(std::mem::replace(&mut row, Vec::with_capacity(arity)));
                    }
                }
                Ok((schema, RelationId(relation), rows, bytes))
            })();
            match prepared {
                Ok((schema, relation, rows, bytes)) => {
                    context.input(bytes)?;
                    Ok(Box::new(move |context: &WorkContext| {
                        context.checkpoint()?;
                        let bytes = encode_rows_bytes(&schema, relation, &rows, context)?;
                        Ok(Output::Bytes(bytes))
                    }) as crate::runtime::Work)
                }
                Err(error) => {
                    marshal_error = Some(error);
                    Err(RuntimeError::InvalidArgument)
                }
            }
        },
    );
    if let Some(error) = marshal_error {
        return Err(error);
    }
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_bytes_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<Buffer> {
    match take_output(env, handle)? {
        Output::Bytes(bytes) => Ok(Buffer::from(bytes)),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_decode_rows(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    spec: Object,
    relation: u32,
    bytes: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let bytes = unshared_input(env, bytes, runtime.options.chunk_bytes)?;
    let mut marshal_error = None;
    let operation = runtime.submit(
        policy.parse().map_err(|error| thrown(env, error))?,
        notification(callback)?,
        |context| {
            let staged = (|| -> napi::Result<bumbledb::schema::Schema> {
                use bumbledb::schema::ValidateDescriptor as _;
                let (descriptor, _attrs) = match crate::descriptor_of(&spec)? {
                    Ok(parsed) => parsed,
                    Err(
                        crate::OpenOutcome::SchemaError(message)
                        | crate::OpenOutcome::NewtypeMismatch(message),
                    ) => {
                        return Err(marshal::err(message));
                    }
                };
                descriptor
                    .validate()
                    .map_err(|error| marshal::err(error.to_string()))
            })();
            match staged {
                Ok(schema) => {
                    context.input(bytes.len() as u64)?;
                    let owned = bytes.to_vec();
                    Ok(Box::new(move |context: &WorkContext| {
                        context.checkpoint()?;
                        let rows =
                            decode_rows_values(&schema, RelationId(relation), &owned, context)?;
                        Ok(Output::Rows(
                            rows.into_iter()
                                .map(|row| row.into_iter().map(ValueOut::from_value).collect())
                                .collect(),
                        ))
                    }) as crate::runtime::Work)
                }
                Err(error) => {
                    marshal_error = Some(error);
                    Err(RuntimeError::InvalidArgument)
                }
            }
        },
    );
    if let Some(error) = marshal_error {
        return Err(error);
    }
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

// ---------------------------------------------------------------------------
// Read-only migration-codec integration (C11): `hashChunk`-shaped bounded
// owned JSON request/response over P09's native schema_file/plan/manifest
// lanes. Neither verb opens, initializes, freezes or migrates a database.
// ---------------------------------------------------------------------------

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_migration_schema(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    spec: Object,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let mut marshal_error = None;
    let operation = runtime.submit(
        policy.parse().map_err(|error| thrown(env, error))?,
        notification(callback)?,
        |_| {
            let parsed = match crate::descriptor_of(&spec) {
                Ok(parsed) => parsed,
                Err(error) => {
                    marshal_error = Some(error);
                    return Err(RuntimeError::InvalidArgument);
                }
            };
            Ok(Box::new(move |context| {
                context.checkpoint()?;
                Ok(Output::Bytes(crate::migration_wire::schema_response(
                    parsed, context,
                )?))
            }))
        },
    );
    if let Some(error) = marshal_error {
        return Err(error);
    }
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_migration_read(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    request: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let request = unshared_input(env, request, runtime.options.chunk_bytes)?;
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            |context| {
                context.input(request.len() as u64)?;
                let owned = request.to_vec();
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    Ok(Output::Bytes(crate::migration_wire::chain_response(
                        &owned, context,
                    )?))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

// ---------------------------------------------------------------------------
// Engine-backed bridge tests (F1-authored; F3 executes).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
