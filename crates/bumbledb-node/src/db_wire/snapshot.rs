//! Snapshot jobs over a worker-held owned read. Each operation gets a fresh
//! `WorkContext`; one-shot plans drop with their operation, and an explicit
//! preparation owns one reusable worker-table entry sharing the same pin.

use std::sync::Arc;

use bumbledb::work::WorkContext;
use bumbledb::{Query, RelationId, StatementId, Value};

use crate::query::QueryHandle;

use crate::runtime::session::{SnapshotAccess, SnapshotWork};
use crate::runtime::{Output, RuntimeError};

use super::engine_error;

/// Point read against the worker-owned pinned read. Fresh work is the
/// frame's, not the snapshot lifetime.
pub(crate) fn snapshot_get_work(
    relation: RelationId,
    key: StatementId,
    row: Vec<Value>,
) -> SnapshotWork {
    Box::new(move |context, access| {
        context.checkpoint()?;
        let hit = access
            .owned
            .get_dyn(relation, key, &row, context)
            .map_err(|error| engine_error(&error))?;
        Ok(Output::Row(
            hit.map(|row| crate::marshal::queued_row(context, &row))
                .transpose()?,
        ))
    })
}

/// Prepare, execute and drop the one-shot plan before returning its
/// independent completed result. Nothing is installed for later operations.
pub(crate) fn execute_complete_work(
    query: Arc<QueryHandle>,
    params: Vec<crate::marshal::OwnedParam>,
) -> SnapshotWork {
    Box::new(move |context, access| {
        context.checkpoint()?;
        let result = owned_execute_complete(access, context, &query.query, &params)?;
        Ok(Output::CompleteResult(result))
    })
}

fn owned_execute_complete(
    access: &mut SnapshotAccess<'_>,
    context: &WorkContext,
    query: &Query,
    params: &[crate::marshal::OwnedParam],
) -> Result<bumbledb::CompleteResult, RuntimeError> {
    let frame = access.frame(context);
    let mut prepared = frame.prepare(query).map_err(|error| engine_error(&error))?;
    let args = crate::param_args(params);
    let result = prepared
        .execute_complete_with_work(&frame, context, args.as_slice())
        .map_err(|error| engine_error(&error))?;
    Ok(result)
}

pub(crate) fn prepare_work(
    runtime: Arc<crate::runtime::Runtime>,
    query: Arc<QueryHandle>,
) -> SnapshotWork {
    Box::new(move |context, access| {
        context.checkpoint()?;
        access
            .prepare(&runtime, &query.query, context)
            .map(Output::Prepared)
    })
}

pub(crate) fn execute_prepared_work(params: Vec<crate::marshal::OwnedParam>) -> SnapshotWork {
    Box::new(move |context, access| {
        context.checkpoint()?;
        access
            .execute(context, &crate::param_args(&params))
            .map(Output::CompleteResult)
    })
}

pub(crate) fn release_prepared_memory_work() -> SnapshotWork {
    Box::new(|context, access| {
        context.checkpoint()?;
        access
            .prepared
            .as_deref_mut()
            .ok_or(RuntimeError::ClosedHandle)?
            .release_memory();
        Ok(Output::Ready)
    })
}
