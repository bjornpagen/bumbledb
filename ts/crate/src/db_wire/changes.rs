//! Immutable change values: native composition, checked bytes and bounded
//! delivery. Cursors own an Arc share, never a decoded copy or offset index.

use std::sync::Arc;

use bumbledb::changes::{ChangeCursor, ChangeKind};
use bumbledb::{ChangeSet, WorkContext};
use napi::bindgen_prelude::{Env, External, Function, Object, Unknown};
use napi_derive::napi;

use crate::marshal::{self, ValueOut};
use crate::runtime::registry::{NativeKind, Payload, RegistryAdmission};
use crate::runtime::{Output, PublicationSink, QueuedBytes, Runtime, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, reporter, take_output,
    thrown, unshared_input,
};

use super::{
    ChangesHandle, ChangesOpened, change_error, changes_from_payload, changes_route,
    close_admitted, identity,
};

const PAGE_ROWS: usize = 256;
const PAGE_BYTES: usize = 64 * 1024;

pub struct ChangesCursorOpened {
    cursor: ChangeCursor,
    schema: Arc<bumbledb::Schema>,
}

pub struct ChangesCursorHandle {
    identity: usize,
    runtime: Arc<Runtime>,
    admission: RegistryAdmission,
}

#[napi(object, object_from_js = false)]
pub struct ChangeRecordWire {
    pub relation: u32,
    pub kind: String,
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
        records.push(ChangeRecordWire {
            relation: record.relation.0,
            kind: match record.kind {
                ChangeKind::Add => "add",
                ChangeKind::Remove => "remove",
            }
            .into(),
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

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_changes_cursor(
    env: Env,
    handle: &External<ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let (runtime, cap) = changes_route(handle).map_err(|error| thrown(env, error))?;
    let operation = runtime
        .submit_payload(cap, WorkContext::new(), notification(callback)?, |_| {
            Ok(Box::new(|work, payload, _| {
                work.checkpoint()?;
                cursor_from_payload(payload)
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
pub fn runtime_changes_cursor_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<ChangesCursorHandle>> {
    let runtime = crate::runtime_wire::operation_runtime(handle);
    let Output::ChangesCursor(opened) = take_output(env, handle)? else {
        return Err(thrown(env, RuntimeError::InvalidArgument));
    };
    let admission = RegistryAdmission::admit(
        Arc::clone(&runtime),
        NativeKind::ChangesCursor,
        Payload::ChangesCursor(opened),
    )
    .map_err(|error| thrown(env, error))?;
    Ok(External::new(ChangesCursorHandle {
        identity: identity(),
        runtime,
        admission,
    }))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_changes_cursor_next(
    env: Env,
    handle: &External<ChangesCursorHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let operation = handle
        .runtime
        .submit_payload(
            handle.admission.cap(),
            WorkContext::new(),
            notification(callback)?,
            |_| Ok(Box::new(publish_page)),
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&handle.runtime, operation))
}

#[napi]
pub fn runtime_change_page_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<Option<Vec<ChangeRecordWire>>> {
    match take_output(env, handle)? {
        Output::ChangePage(page) => Ok(page),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

#[napi]
pub fn runtime_changes_cursor_close(
    env: Env,
    handle: &External<ChangesCursorHandle>,
    callback: Function<crate::runtime_wire::CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    close_admitted(&handle.runtime, handle.admission.cap(), reporter(callback)?);
    Ok(())
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_changes_bytes(
    env: Env,
    handle: &External<ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let (runtime, cap) = changes_route(handle).map_err(|error| thrown(env, error))?;
    let operation = runtime
        .submit_payload(cap, WorkContext::new(), notification(callback)?, |_| {
            Ok(Box::new(|work, payload, _| {
                let opened = changes_from_payload(payload)?;
                Ok(Output::Bytes(QueuedBytes::copy_from(
                    work,
                    opened.changes.as_bytes(),
                )?))
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_changes_parse(
    env: Env,
    handle: &External<RuntimeHandle>,
    spec: Object,
    bytes: Unknown,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    let bytes = unshared_input(env, bytes)?;
    let mut marshal_error = None;
    let operation = runtime.submit(WorkContext::new(), notification(callback)?, |work| {
        let parsed = match crate::descriptor_of(&spec) {
            Ok(parsed) => parsed,
            Err(error) => {
                marshal_error = Some(error);
                return Err(RuntimeError::InvalidArgument);
            }
        };
        let owned = QueuedBytes::copy_from(work, &bytes)?.bytes;
        Ok(Box::new(move |work| {
            use bumbledb::schema::ValidateDescriptor as _;
            work.checkpoint()?;
            let (descriptor, _) = parsed.map_err(|error| RuntimeError::Engine {
                kind: crate::tags::error_family::SCHEMA,
                message: match error {
                    crate::OpenOutcome::SchemaError(message)
                    | crate::OpenOutcome::NewtypeMismatch(message) => message,
                },
            })?;
            let schema = Arc::new(
                descriptor
                    .validate()
                    .map_err(|error| RuntimeError::Engine {
                        kind: crate::tags::error_family::SCHEMA,
                        message: error.to_string(),
                    })?,
            );
            let changes = ChangeSet::from_bytes(&schema, owned, work)
                .map_err(|error| change_error(&error))?;
            let fingerprint = crate::hex_fingerprint(&changes.schema().0);
            Ok(Output::Changes(ChangesOpened {
                changes,
                schema,
                fingerprint,
            }))
        }))
    });
    if let Some(error) = marshal_error {
        return Err(error);
    }
    let operation = operation.map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// Two routed borrows under one operation and cancellation context. The
/// first stage retains only an Arc, then returns its worker immediately.
#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_changes_compose(
    env: Env,
    left: &External<ChangesHandle>,
    right: &External<ChangesHandle>,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let (runtime, left) = changes_route(left).map_err(|error| thrown(env, error))?;
    let (other_runtime, right) = changes_route(right).map_err(|error| thrown(env, error))?;
    if !Arc::ptr_eq(&runtime, &other_runtime) {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let operation = runtime
        .submit_payload(left, WorkContext::new(), notification(callback)?, |_| {
            Ok(Box::new(move |work, payload, _| {
                compose_from_payload(payload, right, work)
            }))
        })
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(&runtime, operation))
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
