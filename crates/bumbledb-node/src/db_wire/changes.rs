//! Immutable change values: native composition, checked bytes and bounded
//! delivery. Cursors own an `Arc` share, never a decoded copy or offset index.

use std::sync::Arc;

use bumbledb::changes::{ChangeCursor, ChangeKind};
use bumbledb::{ChangeSet, WorkContext};
use napi::bindgen_prelude::{Env, External, Function, Unknown};
use napi_derive::napi;

use crate::marshal::{self, ValueOut};
use crate::runtime::registry::{NativeKind, Payload, RegistryAdmission};
use crate::runtime::{Output, PublicationSink, QueuedBytes, RuntimeError};
use crate::runtime_wire::{
    CloseOut, OperationHandle, RuntimeHandle, notification, operation_handle, operation_runtime,
    owner, reporter, take_output, thrown, unshared_input, wrong_output,
};

use super::{ChangesHandle, ChangesOpened, change_error, changes_from_payload, close_admitted};

const PAGE_ROWS: usize = 256;
const PAGE_BYTES: usize = 64 * 1024;

pub struct ChangesCursorOpened {
    cursor: ChangeCursor,
    schema: Arc<bumbledb::Schema>,
}

/// An independent read position over a change set's shared bytes.
pub struct ChangesCursorHandle(RegistryAdmission);

#[napi(string_enum)]
pub enum ChangeKindOut {
    Add,
    Remove,
}

/// One change record: an addition or removal of one row of `relation`.
#[napi(object, object_from_js = false)]
pub struct ChangeRecordOut {
    pub relation: u32,
    pub kind: ChangeKindOut,
    #[napi(ts_type = "Array<CellValue>")]
    pub values: Vec<ValueOut>,
}

pub(crate) fn cursor_from_payload(payload: &Payload) -> Result<Output, RuntimeError> {
    let opened = changes_from_payload(payload)?;
    Ok(Output::ChangesCursor(ChangesCursorOpened {
        cursor: opened.changes.cursor(),
        schema: opened.schema,
    }))
}

pub(crate) fn publish_page(
    work: &WorkContext,
    payload: &mut Payload,
    publication: &mut PublicationSink<'_>,
) -> Result<Output, RuntimeError> {
    let Payload::ChangesCursor(opened) = payload else {
        return Err(RuntimeError::InvalidArgument);
    };
    let mut preview = opened.cursor.clone();
    let mut records = marshal::output_vec(PAGE_ROWS)?;
    let mut bytes = 0;
    while records.len() < PAGE_ROWS && bytes < PAGE_BYTES {
        work.checkpoint()?;
        let Some(record) = preview.next_record() else {
            break;
        };
        bytes += record.row.len();
        let fields = opened
            .schema
            .relation_checked(record.relation)
            .ok_or(RuntimeError::Internal)?
            .fields();
        let values = bumbledb::canonical::decode(fields, record.row, work)
            .map_err(|error| change_error(&error.into()))?;
        records.push(ChangeRecordOut {
            relation: record.relation.0,
            kind: match record.kind {
                ChangeKind::Add => ChangeKindOut::Add,
                ChangeKind::Remove => ChangeKindOut::Remove,
            },
            values: marshal::row_out(work, &values)?,
        });
    }
    let page = if records.is_empty() {
        None
    } else {
        Some(records)
    };
    publication.accept(Output::ChangePage(page), || {
        opened.cursor = preview;
    })?;
    Ok(Output::Ready)
}

/// Open a fresh read position over the change set's records.
#[napi]
pub fn runtime_changes_cursor(
    env: Env,
    handle: &External<ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = handle.0.runtime();
    let operation = runtime
        .submit_payload(
            handle.0.cap(),
            WorkContext::new(),
            notification(callback)?,
            |_| {
                Ok(Box::new(|work, payload, _| {
                    work.checkpoint()?;
                    cursor_from_payload(payload)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

#[napi]
pub fn runtime_changes_cursor_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<ChangesCursorHandle>> {
    let runtime = operation_runtime(handle);
    let Output::ChangesCursor(opened) = take_output(env, handle)? else {
        return Err(wrong_output(env));
    };
    let admission = RegistryAdmission::admit(
        runtime,
        NativeKind::ChangesCursor,
        Payload::ChangesCursor(opened),
    )
    .map_err(|error| thrown(env, error))?;
    Ok(External::new(ChangesCursorHandle(admission)))
}

#[napi]
pub fn runtime_changes_cursor_next(
    env: Env,
    handle: &External<ChangesCursorHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = handle.0.runtime();
    let operation = runtime
        .submit_payload(
            handle.0.cap(),
            WorkContext::new(),
            notification(callback)?,
            |_| Ok(Box::new(publish_page)),
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// One page of change records; `null` is the end of the cursor.
#[napi]
pub fn runtime_change_page_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Option<Vec<ChangeRecordOut>>> {
    match take_output(env, handle)? {
        Output::ChangePage(page) => Ok(page),
        _ => Err(wrong_output(env)),
    }
}

#[napi]
pub fn runtime_changes_cursor_close(
    handle: &External<ChangesCursorHandle>,
    callback: Function<CloseOut, ()>,
) -> napi::Result<()> {
    close_admitted(handle.0.runtime(), handle.0.cap(), reporter(callback)?);
    Ok(())
}

/// The change set's canonical bytes.
#[napi]
pub fn runtime_changes_bytes(
    env: Env,
    handle: &External<ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = handle.0.runtime();
    let operation = runtime
        .submit_payload(
            handle.0.cap(),
            WorkContext::new(),
            notification(callback)?,
            |_| {
                Ok(Box::new(|work, payload, _| {
                    let opened = changes_from_payload(payload)?;
                    Ok(Output::Bytes(QueuedBytes::copy_from(
                        work,
                        opened.changes.as_bytes(),
                    )?))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// Parse canonical change-set bytes under a compiled schema.
#[napi]
pub fn runtime_changes_parse(
    env: Env,
    handle: &External<RuntimeHandle>,
    schema: &External<Arc<crate::schema::SchemaHandle>>,
    #[napi(ts_arg_type = "Uint8Array")] bytes: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle);
    let bytes = unshared_input(env, bytes)?;
    let schema = Arc::clone(&schema.schema);
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |work| {
            let owned = QueuedBytes::copy_from(work, &bytes)?.bytes;
            Ok(Box::new(move |work| {
                work.checkpoint()?;
                let changes = ChangeSet::from_bytes(&schema, owned, work)
                    .map_err(|error| change_error(&error))?;
                let fingerprint = crate::schema::hex(&changes.schema().0);
                Ok(Output::Changes(ChangesOpened {
                    changes,
                    schema,
                    fingerprint,
                }))
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// Compose two change sets into one (`left` then `right`). The first stage
/// retains only an `Arc` share, then returns its worker immediately.
#[napi]
pub fn runtime_changes_compose(
    env: Env,
    left: &External<ChangesHandle>,
    right: &External<ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = left.0.runtime();
    if !Arc::ptr_eq(runtime, right.0.runtime()) {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let right = right.0.cap();
    let operation = runtime
        .submit_payload(
            left.0.cap(),
            WorkContext::new(),
            notification(callback)?,
            |_| {
                Ok(Box::new(move |work, payload, _| {
                    compose_from_payload(payload, right, work)
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

fn compose_from_payload(
    payload: &Payload,
    right: crate::runtime::Capability,
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    work.checkpoint()?;
    let left = changes_from_payload(payload)?;
    Ok(Output::PayloadContinuation {
        cap: right,
        work: Box::new(move |work, payload, _| {
            let right = changes_from_payload(payload)?;
            let changes = left
                .changes
                .compose(&right.changes, work)
                .map_err(|error| change_error(&error))?;
            Ok(Output::Changes(ChangesOpened {
                changes,
                schema: left.schema,
                fingerprint: left.fingerprint,
            }))
        }),
    })
}

#[cfg(test)]
mod tests;
