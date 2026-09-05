//! The core db-bridge verb roster (C09/C05): schema compile, coherent
//! snapshots and snapshot-bound execution sessions, point reads, complete
//! bounded execution into sealed [`bumbledb::CompleteResult`]s, one-shot
//! cursor transfer, database-free change drafts, one immutable final-state
//! apply, bounded inspection, the shared canonical row codec and the two
//! read-only migration-codec entrypoints.
//!
//! Every verb registers a bounded operation before any completion can run
//! in JS. Retained resources live in the worker table behind
//! [`crate::runtime::registry::Capability`]; JS wrappers are not retention
//! authority. Each operation starts a fresh [`WorkContext`]. Collection and
//! paging open a core [`bumbledb::DeliveryTicket`], preview/adopt under
//! admitted overlap, register the native [`crate::runtime::QueuedOutput`],
//! then `commit` the same ticket once through `publication.accept`.
//! Budget refusal and cancel abort that ticket (no accepted page, no
//! leftover `pending_advance`). Terminal backing stays sticky.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bumbledb::work::{ExecutionPolicy, WorkContext};
use bumbledb::{ChangeError, ChangeSet, CompleteResult, RelationId, ResultCursor, Value};
use napi::bindgen_prelude::{Array, BigInt, Buffer, Env, External, Function, Object, Unknown};
use napi_derive::napi;

use crate::marshal::{self, ValueOut};
use crate::runtime::registry::{
    Capability, NativeKind, Payload, RegistryAdmission, ResultState,
    registry_draft::DraftPayload,
};
use crate::runtime::{DraftLedger, Output, QueuedOutput, Runtime, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, PolicyWire, RuntimeHandle, SessionHandle, notification, operation_handle,
    owner, reporter, session, take_output, thrown, unshared_input,
};

mod apply;
mod close;
mod codec;
mod delivery;
mod draft;
mod snapshot;

pub(crate) use apply::{apply_change_set, changes_from_payload, inspect_db, integration_error};
pub(crate) use close::{close_admitted, spawn_teardown};
pub(crate) use codec::{decode_rows_values, encode_rows_bytes};
pub(crate) use delivery::{
    PagePlan, PullOutcome, accept_publication, collect_from_payload, intersected_result_bytes,
    is_terminal_backing, plan_page, preview_error_outcome, preview_none_outcome,
    publish_from_payload, pull_from_payload, register_page, reject_publication,
    transfer_from_payload,
};
pub(crate) use draft::{finish_from_payload, ingest_from_payload, parse_draft_rows};
pub(crate) use snapshot::{execute_complete_work, snapshot_get_work};

const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<CompleteResult>();
    assert_send::<ResultCursor>();
};

fn identity() -> usize {
    crate::runtime_wire::addon_identity()
}

// ---------------------------------------------------------------------------
// Handles — capability tokens only. No JS-held payload Arc / RetainedGuard.
// ---------------------------------------------------------------------------

/// A coherent owned snapshot: capability-bearing session plus the sealed
/// roster for JS-thread marshalling. This is the only published-snapshot
/// type — L14 mints it with [`SnapshotHandle::assemble`]. It is not a
/// writable core `Db` and does not expose one. Close is
/// [`runtime_snapshot_close`] (session drain).
pub struct SnapshotHandle {
    identity: usize,
    pub(crate) session: Arc<crate::runtime::session::SnapshotSession>,
    pub(crate) sealed: Arc<crate::Sealed>,
}

impl SnapshotHandle {
    /// The one published-snapshot constructor (core take and L14 log pin).
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
/// `runtime_snapshot_session`.
pub struct ExecSessionOpened {
    pub session: Arc<crate::runtime::session::SnapshotSession>,
    pub sealed: Arc<crate::Sealed>,
}

/// One sealed completed result. Capability routes to the worker table.
pub struct ResultHandle {
    identity: usize,
    shared: Arc<ResultShared>,
}

pub(crate) struct ResultShared {
    runtime: Arc<Runtime>,
    cap: Capability,
    _admission: RegistryAdmission,
}

/// The one consuming cursor over a spent result's backing.
pub struct CursorHandle {
    identity: usize,
    shared: Arc<CursorShared>,
}

pub(crate) struct CursorShared {
    runtime: Arc<Runtime>,
    cap: Capability,
    _admission: RegistryAdmission,
}

/// A database-free change draft. `sealed` is the marshalling roster, not
/// draft-payload authority.
pub struct DraftHandle {
    identity: usize,
    shared: Arc<DraftShared>,
}

pub(crate) struct DraftShared {
    runtime: Arc<Runtime>,
    cap: Capability,
    _admission: RegistryAdmission,
    sealed: Arc<crate::Sealed>,
}

/// The opened draft crossing back from the executor. `ledger` is the
/// cumulative work/deadline L12 persists on `DraftPayload` at take.
pub struct DraftOpened {
    schema: Arc<bumbledb::schema::Schema>,
    sealed: Arc<crate::Sealed>,
    allowance_input: u64,
    allowance_rows: u64,
    ledger: DraftLedger,
}

/// A sealed immutable `ChangeSet`.
pub struct ChangesHandle {
    identity: usize,
    shared: Arc<ChangesShared>,
}

pub(crate) struct ChangesShared {
    runtime: Arc<Runtime>,
    cap: Capability,
    _admission: RegistryAdmission,
}

/// The sealed `ChangeSet` crossing back from a draft finish.
pub struct ChangesOpened {
    pub changes: ChangeSet,
    pub schema: Arc<bumbledb::schema::Schema>,
    pub fingerprint: String,
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

pub(crate) enum ExpectedOwned {
    Any,
    Exact { store: String, generation: u64 },
}

// ---------------------------------------------------------------------------
// Shared plumbing.
// ---------------------------------------------------------------------------

pub(crate) fn engine_error(error: &bumbledb::Error) -> RuntimeError {
    crate::runtime::session::engine_error(error)
}

pub(crate) fn change_error(error: &ChangeError) -> RuntimeError {
    RuntimeError::Engine {
        kind: crate::tags::error_family::VALIDATION,
        message: format!("bumbledb changes: {error:?}"),
    }
}

pub(crate) fn value_bytes(value: &Value) -> u64 {
    8 + match value {
        Value::String(text) => text.len() as u64,
        Value::FixedBytes(bytes) => bytes.len() as u64,
        Value::Id128(_) | Value::IntervalU64(_) | Value::IntervalI64(_) | Value::IntervalF64(_) => {
            16
        }
        _ => 8,
    }
}

fn hop_policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: 1 << 16,
        result_bytes: 1 << 20,
        rows: 1 << 16,
        work_units: 1 << 16,
        timeout: Duration::from_secs(10),
    }
}

// ---------------------------------------------------------------------------
// Schema compile.
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
                Ok(Box::new(move |context, _access| {
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
                Ok(snapshot_get_work(rel, key, row))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
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
            let admission = RegistryAdmission::admit(
                runtime,
                NativeKind::Result,
                result.byte_len(),
                Payload::Result {
                    result: Some(result),
                    state: ResultState::Live,
                },
            )
            .map_err(|error| thrown(env, error))?;
            let cap = admission.cap();
            Ok(External::new(ResultHandle {
                identity: identity(),
                shared: Arc::new(ResultShared {
                    runtime,
                    cap,
                    _admission: admission,
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
    let requested = parsed.result_bytes;
    let row_limit = parsed.rows;
    let operation = runtime
        .submit_payload(shared.cap, parsed, notification(callback)?, move |_| {
            Ok(Box::new(move |context, payload, _publication| {
                context.checkpoint()?;
                // L16: requested maxBytes cannot enlarge work.resultBytes.
                let cap = intersected_result_bytes(requested, context);
                collect_from_payload(payload, context, cap, row_limit)
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
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
        .submit_payload(
            shared.cap,
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context, payload, _publication| {
                    context.checkpoint()?;
                    transfer_from_payload(payload, context)
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
            let admission = RegistryAdmission::admit(
                runtime,
                NativeKind::Cursor,
                cursor.byte_len(),
                Payload::Cursor {
                    cursor,
                    drained: false,
                },
            )
            .map_err(|error| thrown(env, error))?;
            let cap = admission.cap();
            Ok(External::new(CursorHandle {
                identity: identity(),
                shared: Arc::new(CursorShared {
                    runtime,
                    cap,
                    _admission: admission,
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
    let requested = parsed.result_bytes;
    let cap = shared.cap;
    let charge_runtime = Arc::clone(&shared.runtime);
    let operation = runtime
        .submit_payload(shared.cap, parsed, notification(callback)?, move |_| {
            Ok(Box::new(move |context, payload, publication| {
                context.checkpoint()?;
                // L16: requested pageBytes cannot enlarge work.resultBytes.
                let page_bytes = intersected_result_bytes(requested, context);
                match publish_from_payload(payload, context, page_bytes, publication) {
                    Err(error) if is_terminal_backing(&error) => {
                        let _ = charge_runtime.request_resource_close(cap);
                        Err(error)
                    }
                    other => other,
                }
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
        Output::Page(page) => Ok(page.map(|queued| queued.rows)),
        Output::Rows(queued) => Ok(Some(queued.rows)),
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
    close_admitted(
        &shared.runtime,
        shared.cap,
        &shared._admission,
        reporter(callback)?,
    );
    Ok(())
}

#[napi]
pub fn runtime_cursor_close(
    env: Env,
    handle: &External<CursorHandle>,
    callback: Function<crate::runtime_wire::CloseWire, ()>,
) -> napi::Result<()> {
    let shared = cursor_shared(handle).map_err(|error| thrown(env, error))?;
    close_admitted(
        &shared.runtime,
        shared.cap,
        &shared._admission,
        reporter(callback)?,
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Drafts and sealed change sets.
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
    let allowance_rows = parsed_policy.rows;
    let allowance_work = parsed_policy.work_units;
    let deadline = Instant::now() + parsed_policy.timeout;
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
                allowance_rows,
                ledger: DraftLedger {
                    used_work: 0,
                    allowance_work,
                    deadline,
                    terminal: false,
                },
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
            let sealed = Arc::clone(&opened.sealed);
            let admission = RegistryAdmission::admit(
                runtime,
                NativeKind::Draft,
                0,
                Payload::Draft(DraftPayload {
                    schema: opened.schema,
                    sealed: opened.sealed,
                    pending: Vec::new(),
                    used_input: 0,
                    used_rows: 0,
                    allowance_input: opened.allowance_input,
                    allowance_rows: opened.allowance_rows,
                    ledger: opened.ledger,
                }),
            )
            .map_err(|error| thrown(env, error))?;
            let cap = admission.cap();
            Ok(External::new(DraftHandle {
                identity: identity(),
                shared: Arc::new(DraftShared {
                    runtime,
                    cap,
                    _admission: admission,
                    sealed,
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
    let sealed = Arc::clone(&shared.sealed);
    let cap = shared.cap;
    let operation = runtime.submit_payload(
        cap,
        policy.parse().map_err(|error| thrown(env, error))?,
        notification(callback)?,
        move |context| {
            let parsed = parse_draft_rows(&sealed, relation, stated, &cells, context);
            match parsed {
                Ok((rows, bytes)) => Ok(Box::new(move |context: &WorkContext, payload, _publication| {
                    context.checkpoint()?;
                    ingest_from_payload(payload, context, relation, insert, rows, bytes)
                })),
                Err(error) => Err(error),
            }
        },
    );
    match operation {
        Ok(operation) => Ok(operation_handle(&runtime, operation)),
        Err(error) => {
            let _ = runtime.request_resource_close(cap);
            Err(thrown(env, error))
        }
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
    let cap = shared.cap;
    let operation = runtime
        .submit_payload(
            cap,
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context, payload, _publication| {
                    context.checkpoint()?;
                    finish_from_payload(payload, context)
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
            let fingerprint = opened.fingerprint.clone();
            let admission = RegistryAdmission::admit(
                runtime,
                NativeKind::Changes,
                opened.changes.as_bytes().len() as u64,
                Payload::Changes {
                    changes: opened.changes,
                    schema: opened.schema,
                    fingerprint: opened.fingerprint,
                },
            )
            .map_err(|error| thrown(env, error))?;
            let cap = admission.cap();
            let mut object = Object::new(&env)?;
            object.set(
                "changes",
                External::new(ChangesHandle {
                    identity: identity(),
                    shared: Arc::new(ChangesShared {
                        runtime,
                        cap,
                        _admission: admission,
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
    close_admitted(
        &shared.runtime,
        shared.cap,
        &shared._admission,
        reporter(callback)?,
    );
    Ok(())
}

/// L14 request: submit a payload job and call [`changes_from_payload`].
/// This helper hops to the owning worker (not a JS-thread payload lock).
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
    let runtime = Arc::clone(&handle.shared.runtime);
    let (tx, rx) = std::sync::mpsc::channel();
    let operation = runtime.submit_payload(
        handle.shared.cap,
        hop_policy(),
        Box::new({
            let tx = tx.clone();
            move || {
                let _ = tx.send(());
            }
        }),
        move |_| {
            Ok(Box::new(move |_context, payload, _publication| {
                Ok(Output::Changes(changes_from_payload(payload)?))
            }))
        },
    )?;
    rx.recv_timeout(Duration::from_secs(10))
        .map_err(|_| RuntimeError::Internal)?;
    match runtime.take(&operation)? {
        Output::Changes(opened) => Ok((
            opened.changes,
            opened.schema,
            opened.fingerprint,
            runtime,
        )),
        _ => Err(RuntimeError::InvalidArgument),
    }
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
    let shared = Arc::clone(&handle.shared);
    close_admitted(
        &shared.runtime,
        shared.cap,
        &shared._admission,
        reporter(callback)?,
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Apply.
// ---------------------------------------------------------------------------

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
    if changes.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    if !Arc::ptr_eq(&runtime, &changes.shared.runtime) {
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
    let cap = changes.shared.cap;
    let operation = runtime
        .submit_payload(
            cap,
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |context| {
                context.checkpoint()?;
                Ok(Box::new(move |context, payload, _publication| {
                    let opened = changes_from_payload(payload)?;
                    context.input(opened.changes.as_bytes().len() as u64)?;
                    apply_change_set(&lease, &opened.changes, &expected, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    let _ = owner;
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
    let operation = runtime
        .submit_db(
            owner,
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    inspect_db(&lease, owner_id, database_id, context)
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
// Shared canonical row codec.
// ---------------------------------------------------------------------------

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
                        let out: Vec<Vec<ValueOut>> = rows
                            .into_iter()
                            .map(|row| row.into_iter().map(ValueOut::from_value).collect())
                            .collect();
                        Ok(Output::Rows(QueuedOutput::admit(context, out, 0)?))
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
// Read-only migration-codec integration.
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
// Engine-backed bridge tests (authored now; verification NotRun).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
