//! Database verbs: coherent snapshots and their prepared plans, point
//! reads, sealed completed results and their one-shot cursors, database-free
//! change drafts, apply and judge of sealed change sets, inspection, and the
//! canonical row codec.
//!
//! Every verb registers a bounded operation before any completion can run in
//! JS, and each operation has its own `WorkContext`. Retained resources live
//! in a worker table behind a capability; a JS wrapper is never the
//! retention authority. Paging commits each `DeliveryTicket` exactly once
//! through `PublicationSink::accept`; cancellation aborts it.

use std::sync::Arc;

use bumbledb::work::WorkContext;
use bumbledb::{ChangeError, ChangeSet, CompleteResult, RelationId, ResultCursor};
use napi::bindgen_prelude::{Array, BigInt, Env, External, Function, Unknown};
use napi_derive::napi;

use crate::marshal::{self, ViolationOut};
use crate::query::QueryHandle;
use crate::runtime::registry::{
    NativeKind, Payload, RegistryAdmission, ResultState, registry_draft::DraftPayload,
};
use crate::runtime::{Output, QueuedBytes, QueuedOutput, RuntimeError};
use crate::runtime_wire::{
    CloseOut, OperationHandle, PreparedHandle, RuntimeHandle, notification, operation_handle,
    operation_runtime, owner, prepared, reporter, take_output, thrown, unshared_input,
    wrong_output,
};
use crate::schema::SchemaHandle;

mod apply;
pub mod changes;
mod close;
mod codec;
mod delivery;
mod draft;
mod snapshot;

pub(crate) use apply::{WriteMode, changes_from_payload, decide_change_set, inspect_db};
pub use changes::ChangesCursorOpened;
pub(crate) use close::close_admitted;
pub(crate) use codec::{decode_rows_values, encode_rows_bytes, parse_input_rows};
pub(crate) use delivery::{
    collect_from_payload, is_terminal_backing, publish_from_payload, transfer_from_payload,
};
pub(crate) use draft::{finish_from_payload, ingest_from_payload};
pub(crate) use snapshot::{
    execute_complete_work, execute_prepared_work, prepare_work, release_prepared_memory_work,
    snapshot_get_work,
};

const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<CompleteResult>();
    assert_send::<ResultCursor>();
};

/// A coherent owned snapshot plus the schema its keys and rows marshal by.
/// Closing drains its session.
pub struct SnapshotHandle {
    session: Arc<crate::runtime::session::SnapshotSession>,
    schema: Arc<SchemaHandle>,
}

/// One sealed completed result.
pub struct ResultHandle(RegistryAdmission);

/// The one consuming cursor over a spent result's backing.
pub struct CursorHandle(RegistryAdmission);

/// A database-free change draft under one compiled schema.
pub struct DraftHandle {
    admission: RegistryAdmission,
    schema: Arc<SchemaHandle>,
}

/// A sealed immutable `ChangeSet`.
pub struct ChangesHandle(RegistryAdmission);

/// The sealed `ChangeSet` crossing back from a draft finish or parse.
pub struct ChangesOpened {
    pub changes: ChangeSet,
    pub schema: Arc<bumbledb::schema::Schema>,
    pub fingerprint: String,
}

/// A database state: the store identity and its committed generation.
#[napi(object, object_from_js = false)]
#[derive(Clone)]
pub struct WitnessOut {
    pub store: String,
    pub generation: u64,
}

/// The state a write expects to find.
#[napi(object, object_to_js = false)]
pub struct WitnessIn {
    pub store: String,
    pub generation: BigInt,
}

pub(crate) struct Expected {
    pub(crate) store: String,
    pub(crate) generation: u64,
}

#[napi(object, object_from_js = false)]
pub struct ChangeCounts {
    pub added: u64,
    pub removed: u64,
}

/// One apply outcome. `NoChange` committed nothing new; `Moved` means the
/// expected state was not the current one.
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum ApplyOutcome {
    Committed {
        witness: WitnessOut,
    },
    NoChange {
        witness: WitnessOut,
    },
    Rejected {
        violations: Vec<ViolationOut>,
    },
    Moved {
        witnessed: WitnessOut,
        current: WitnessOut,
    },
}

/// One judgment of a private candidate; the database never changes.
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum JudgeOutcome {
    Admitted {
        base: WitnessOut,
        changes: ChangeCounts,
    },
    Rejected {
        base: WitnessOut,
        changes: ChangeCounts,
        violations: Vec<ViolationOut>,
    },
    Moved {
        witnessed: WitnessOut,
        current: WitnessOut,
    },
}

/// Bounded database diagnostics, never rows.
#[napi(object, object_from_js = false)]
pub struct DbInspection {
    pub generation: u64,
    pub disk_bytes: u64,
    pub retained_operations: u64,
}

pub(crate) fn engine_error(error: &bumbledb::Error) -> RuntimeError {
    crate::runtime::session::engine_error(error)
}

pub(crate) fn change_error(error: &ChangeError) -> RuntimeError {
    if let ChangeError::Work(error) | ChangeError::Row(bumbledb::canonical::RowError::Work(error)) =
        error
    {
        return (*error).into();
    }
    RuntimeError::Engine {
        kind: crate::tags::error_family::VALIDATION.into(),
        message: format!("bumbledb changes: {error:?}"),
    }
}

fn admit(
    runtime: &Arc<crate::runtime::Runtime>,
    kind: NativeKind,
    payload: Payload,
    env: Env,
) -> napi::Result<RegistryAdmission> {
    RegistryAdmission::admit(Arc::clone(runtime), kind, payload).map_err(|error| thrown(env, error))
}

#[napi]
pub fn runtime_db_snapshot(
    env: Env,
    db: &External<crate::DbHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let owner = db.owner();
    let runtime = Arc::clone(owner.runtime());
    let operation = runtime
        .open_session(owner, WorkContext::new(), notification(callback)?)
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi(object, object_from_js = false)]
pub struct SnapshotOpened {
    pub snapshot: External<SnapshotHandle>,
    pub witness: WitnessOut,
}

#[napi]
pub fn runtime_snapshot_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<SnapshotOpened> {
    match take_output(env, handle)? {
        Output::Session(opened) => Ok(SnapshotOpened {
            snapshot: External::new(SnapshotHandle {
                session: Arc::new(opened.session),
                schema: opened.schema,
            }),
            witness: WitnessOut {
                store: opened.store,
                generation: opened.generation,
            },
        }),
        _ => Err(wrong_output(env)),
    }
}

#[napi]
pub fn runtime_snapshot_close(
    handle: &External<SnapshotHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    handle.session.drain(reporter(callback)?);
    Ok(())
}

#[napi]
pub fn runtime_snapshot_prepare(
    env: Env,
    handle: &External<SnapshotHandle>,
    query: &External<Arc<QueryHandle>>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = Arc::clone(handle.session.runtime());
    let target = Arc::clone(&runtime);
    let query = Arc::clone(query);
    let operation = handle
        .session
        .submit(WorkContext::new(), notification(callback)?, move |_| {
            Ok(prepare_work(target, query))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

/// Read one row by a key statement of `relation`.
#[napi]
pub fn runtime_snapshot_get(
    env: Env,
    handle: &External<SnapshotHandle>,
    relation: u32,
    key_statement: u32,
    #[napi(ts_arg_type = "Array<CellValue>")] key_values: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = Arc::clone(handle.session.runtime());
    let schema = Arc::clone(&handle.schema);
    let operation = handle
        .session
        .submit(WorkContext::new(), notification(callback)?, move |_| {
            let (rel, key, row) = marshal::key_row(&schema, relation, key_statement, &key_values)?;
            Ok(snapshot_get_work(rel, key, row))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_snapshot_execute(
    env: Env,
    handle: &External<SnapshotHandle>,
    query: &External<Arc<QueryHandle>>,
    #[napi(ts_arg_type = "Array<ParamIn>")] params: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = Arc::clone(handle.session.runtime());
    let query = Arc::clone(query);
    let operation = handle
        .session
        .submit(WorkContext::new(), notification(callback)?, move |_| {
            let params = marshal::params_in(&params)?;
            Ok(execute_complete_work(query, params))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_prepared_execute(
    env: Env,
    handle: &External<PreparedHandle>,
    #[napi(ts_arg_type = "Array<ParamIn>")] params: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let session = prepared(handle);
    let runtime = Arc::clone(session.runtime());
    let operation = session
        .submit(WorkContext::new(), notification(callback)?, move |_| {
            let params = marshal::params_in(&params)?;
            Ok(execute_prepared_work(params))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_prepared_release_memory(
    env: Env,
    handle: &External<PreparedHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let session = prepared(handle);
    let runtime = Arc::clone(session.runtime());
    let operation = session
        .submit(WorkContext::new(), notification(callback)?, |_| {
            Ok(release_prepared_memory_work())
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_result_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<ResultHandle>> {
    let runtime = operation_runtime(handle);
    match take_output(env, handle)? {
        Output::CompleteResult(result) => {
            let payload = Payload::Result {
                result: Some(result),
                state: ResultState::Live,
            };
            Ok(External::new(ResultHandle(admit(
                &runtime,
                NativeKind::Result,
                payload,
                env,
            )?)))
        }
        _ => Err(wrong_output(env)),
    }
}

/// Collect every row of a sealed result; the result stays available.
#[napi]
pub fn runtime_result_collect(
    env: Env,
    handle: &External<ResultHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = handle.0.runtime();
    let operation = runtime
        .submit_payload(
            handle.0.cap(),
            WorkContext::new(),
            notification(callback)?,
            |_| {
                Ok(Box::new(|context, payload, _| {
                    context.checkpoint()?;
                    collect_from_payload(payload, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// Move a sealed result's backing into its one cursor.
#[napi]
pub fn runtime_result_cursor(
    env: Env,
    handle: &External<ResultHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = handle.0.runtime();
    let operation = runtime
        .submit_payload(
            handle.0.cap(),
            WorkContext::new(),
            notification(callback)?,
            |_| {
                Ok(Box::new(|context, payload, _| {
                    context.checkpoint()?;
                    transfer_from_payload(payload, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_cursor_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<CursorHandle>> {
    let runtime = operation_runtime(handle);
    match take_output(env, handle)? {
        Output::ResultCursor(cursor) => {
            let payload = Payload::Cursor {
                cursor,
                drained: false,
            };
            Ok(External::new(CursorHandle(admit(
                &runtime,
                NativeKind::Cursor,
                payload,
                env,
            )?)))
        }
        _ => Err(wrong_output(env)),
    }
}

/// Publish the cursor's next page; a terminal store failure closes it.
#[napi]
pub fn runtime_cursor_next(
    env: Env,
    handle: &External<CursorHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = handle.0.runtime();
    let cap = handle.0.cap();
    let closer = Arc::clone(runtime);
    let operation = runtime
        .submit_payload(
            cap,
            WorkContext::new(),
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context, payload, publication| {
                    context.checkpoint()?;
                    match publish_from_payload(payload, context, publication) {
                        Err(error) if is_terminal_backing(&error) => {
                            let _ = closer.request_resource_close(cap);
                            Err(error)
                        }
                        other => other,
                    }
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// One page of rows; `null` is the end of the cursor.
#[napi(ts_return_type = "Array<Array<CellValue>> | null")]
pub fn runtime_page_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Option<QueuedOutput>> {
    match take_output(env, handle)? {
        Output::Page(page) => Ok(page),
        Output::Rows(queued) => Ok(Some(queued)),
        _ => Err(wrong_output(env)),
    }
}

#[napi]
pub fn runtime_result_close(
    handle: &External<ResultHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    close_admitted(handle.0.runtime(), handle.0.cap(), reporter(callback)?);
    Ok(())
}

#[napi]
pub fn runtime_cursor_close(
    handle: &External<CursorHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    close_admitted(handle.0.runtime(), handle.0.cap(), reporter(callback)?);
    Ok(())
}

/// Open a database-free change draft under a compiled schema.
#[napi]
pub fn runtime_draft_open(
    env: Env,
    handle: &External<RuntimeHandle>,
    schema: &External<Arc<SchemaHandle>>,
) -> napi::Result<External<DraftHandle>> {
    let payload = Payload::Draft(DraftPayload {
        schema: Arc::clone(&schema.schema),
        pending: Vec::new(),
        terminal: false,
    });
    Ok(External::new(DraftHandle {
        admission: admit(owner(handle), NativeKind::Draft, payload, env)?,
        schema: Arc::clone(schema),
    }))
}

fn draft_mutation(
    env: Env,
    handle: &DraftHandle,
    relation: u32,
    rows: &BigInt,
    cells: Array,
    callback: Function<(), ()>,
    insert: bool,
) -> napi::Result<External<OperationHandle>> {
    let runtime = handle.admission.runtime();
    let cap = handle.admission.cap();
    let stated = marshal::u64_in(rows, "draft rows").map_err(|error| thrown(env, error))?;
    let schema = Arc::clone(&handle.schema);
    let operation = runtime.submit_payload(
        cap,
        WorkContext::new(),
        notification(callback)?,
        move |context| {
            let rows = parse_input_rows(&schema, relation, stated, &cells, context)?;
            Ok(Box::new(
                move |context: &WorkContext, payload, _publication| {
                    context.checkpoint()?;
                    ingest_from_payload(payload, context, relation, insert, rows)
                },
            ))
        },
    );
    match operation {
        Ok(operation) => Ok(operation_handle(runtime, operation)),
        Err(error) => {
            let _ = runtime.request_resource_close(cap);
            Err(thrown(env, error))
        }
    }
}

/// Stage `rows` additions of `relation`; `cells` is row-major in sealed
/// field order. A refused chunk spends the draft.
#[napi]
pub fn runtime_draft_insert(
    env: Env,
    handle: &External<DraftHandle>,
    relation: u32,
    rows: BigInt,
    #[napi(ts_arg_type = "Array<CellValue>")] cells: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    draft_mutation(env, handle, relation, &rows, cells, callback, true)
}

/// Stage `rows` removals of `relation`, shaped like `runtimeDraftInsert`.
#[napi]
pub fn runtime_draft_delete(
    env: Env,
    handle: &External<DraftHandle>,
    relation: u32,
    rows: BigInt,
    #[napi(ts_arg_type = "Array<CellValue>")] cells: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    draft_mutation(env, handle, relation, &rows, cells, callback, false)
}

/// The number of rows one draft chunk staged.
#[napi]
pub fn runtime_staged_take(env: Env, handle: &External<OperationHandle>) -> napi::Result<u64> {
    match take_output(env, handle)? {
        Output::Staged(rows) => Ok(rows),
        _ => Err(wrong_output(env)),
    }
}

/// Seal the draft into an immutable change set; the draft is spent.
#[napi]
pub fn runtime_draft_finish(
    env: Env,
    handle: &External<DraftHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = handle.admission.runtime();
    let operation = runtime
        .submit_payload(
            handle.admission.cap(),
            WorkContext::new(),
            notification(callback)?,
            |_| {
                Ok(Box::new(|context, payload, _| {
                    context.checkpoint()?;
                    finish_from_payload(payload, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// A sealed change set and its summary.
#[napi(object, object_from_js = false)]
pub struct ChangesOut {
    pub changes: External<ChangesHandle>,
    pub fingerprint: String,
    pub counts: ChangeCounts,
    pub byte_length: u64,
}

#[napi]
pub fn runtime_changes_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<ChangesOut> {
    let runtime = operation_runtime(handle);
    match take_output(env, handle)? {
        Output::Changes(opened) => {
            let fingerprint = opened.fingerprint.clone();
            let counts = opened.changes.counts();
            let byte_length = opened.changes.as_bytes().len() as u64;
            let payload = Payload::Changes {
                changes: opened.changes,
                schema: opened.schema,
                fingerprint: opened.fingerprint,
            };
            Ok(ChangesOut {
                changes: External::new(ChangesHandle(admit(
                    &runtime,
                    NativeKind::Changes,
                    payload,
                    env,
                )?)),
                fingerprint,
                counts: ChangeCounts {
                    added: counts.added,
                    removed: counts.removed,
                },
                byte_length,
            })
        }
        _ => Err(wrong_output(env)),
    }
}

#[napi]
pub fn runtime_draft_close(
    handle: &External<DraftHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    close_admitted(
        handle.admission.runtime(),
        handle.admission.cap(),
        reporter(callback)?,
    );
    Ok(())
}

#[napi]
pub fn runtime_changes_close(
    handle: &External<ChangesHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    close_admitted(handle.0.runtime(), handle.0.cap(), reporter(callback)?);
    Ok(())
}

/// Apply a sealed change set as one judged commit. `expected` absent
/// applies to whatever state is current.
#[napi]
pub fn runtime_db_apply(
    env: Env,
    db: &External<crate::DbHandle>,
    changes: &External<ChangesHandle>,
    expected: Option<WitnessIn>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    submit_change_set(env, db, changes, expected, callback, WriteMode::Apply)
}

/// Judge a sealed change set against the current state without committing.
#[napi]
pub fn runtime_db_judge(
    env: Env,
    db: &External<crate::DbHandle>,
    changes: &External<ChangesHandle>,
    expected: Option<WitnessIn>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    submit_change_set(env, db, changes, expected, callback, WriteMode::Judge)
}

fn submit_change_set(
    env: Env,
    db: &crate::DbHandle,
    changes: &ChangesHandle,
    expected: Option<WitnessIn>,
    callback: Function<(), ()>,
    mode: WriteMode,
) -> napi::Result<External<OperationHandle>> {
    let owner = db.owner();
    let runtime = owner.runtime();
    if !Arc::ptr_eq(runtime, changes.0.runtime()) {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let expected = expected
        .map(|witness| {
            Ok::<_, RuntimeError>(Expected {
                store: witness.store,
                generation: marshal::u64_in(&witness.generation, "expected generation")?,
            })
        })
        .transpose()
        .map_err(|error| thrown(env, error))?;
    let lease = owner.access().map_err(|error| thrown(env, error))?;
    let operation = runtime
        .submit_payload(
            changes.0.cap(),
            WorkContext::new(),
            notification(callback)?,
            move |context| {
                context.checkpoint()?;
                Ok(Box::new(move |context, payload, _| {
                    let opened = changes_from_payload(payload)?;
                    context.checkpoint()?;
                    decide_change_set(&lease, &opened.changes, expected.as_ref(), context, mode)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_judge_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<JudgeOutcome> {
    match take_output(env, handle)? {
        Output::Judge(outcome) => Ok(outcome),
        _ => Err(wrong_output(env)),
    }
}

#[napi]
pub fn runtime_apply_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<ApplyOutcome> {
    match take_output(env, handle)? {
        Output::Apply(outcome) => Ok(outcome),
        _ => Err(wrong_output(env)),
    }
}

/// Drop the database's derived query caches.
#[napi]
pub fn runtime_db_clear_cache(
    env: Env,
    db: &External<crate::DbHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let owner = db.owner();
    let runtime = owner.runtime();
    let lease = owner.access().map_err(|error| thrown(env, error))?;
    let operation = runtime
        .submit_db(
            owner,
            WorkContext::new(),
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    context.checkpoint()?;
                    lease.db().clear_cache();
                    Ok(Output::Ready)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_db_inspect(
    env: Env,
    db: &External<crate::DbHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let owner = db.owner();
    let runtime = owner.runtime();
    let lease = owner.access().map_err(|error| thrown(env, error))?;
    let (owner_id, database_id) = owner.ids();
    let operation = runtime
        .submit_db(
            owner,
            WorkContext::new(),
            notification(callback)?,
            move |_| {
                Ok(Box::new(move |context| {
                    inspect_db(&lease, owner_id, database_id, context)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_db_inspect_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<DbInspection> {
    match take_output(env, handle)? {
        Output::DbReport(report) => Ok(report),
        _ => Err(wrong_output(env)),
    }
}

/// Encode `rows` rows of `relation` into the canonical change-set row codec.
#[napi]
pub fn runtime_encode_rows(
    env: Env,
    handle: &External<RuntimeHandle>,
    schema: &External<Arc<SchemaHandle>>,
    relation: u32,
    rows: BigInt,
    #[napi(ts_arg_type = "Array<CellValue>")] cells: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle);
    let stated = marshal::u64_in(&rows, "encode rows").map_err(|error| thrown(env, error))?;
    let schema = Arc::clone(schema);
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |context| {
            let rows = parse_input_rows(&schema, relation, stated, &cells, context)?;
            Ok(Box::new(move |context: &WorkContext| {
                context.checkpoint()?;
                let bytes =
                    encode_rows_bytes(&schema.schema, RelationId(relation), &rows, context)?;
                Ok(Output::Bytes(bytes))
            }) as crate::runtime::Work)
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi(ts_return_type = "Uint8Array")]
pub fn runtime_bytes_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<QueuedBytes> {
    match take_output(env, handle)? {
        Output::Bytes(bytes) => Ok(bytes),
        _ => Err(wrong_output(env)),
    }
}

/// Decode canonical-codec bytes holding only additions of `relation`.
#[napi]
pub fn runtime_decode_rows(
    env: Env,
    handle: &External<RuntimeHandle>,
    schema: &External<Arc<SchemaHandle>>,
    relation: u32,
    #[napi(ts_arg_type = "Uint8Array")] bytes: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle);
    let bytes = unshared_input(env, bytes)?;
    let schema = Arc::clone(&schema.schema);
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |context| {
            context.checkpoint()?;
            let owned = bytes.to_vec();
            Ok(Box::new(move |context: &WorkContext| {
                context.checkpoint()?;
                Ok(Output::Rows(decode_rows_values(
                    &schema,
                    RelationId(relation),
                    &owned,
                    context,
                )?))
            }) as crate::runtime::Work)
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[cfg(test)]
mod tests;
