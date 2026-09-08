//! Snapshot and execution-session jobs over L07/L12 owned read/frame.
//!
//! Each operation receives a fresh cancellation `WorkContext`. The
//! snapshot's age does not expire later operations. One-shot plans drop
//! with their operation. Explicit preparation owns one reusable worker-table
//! object and an independent share of the same snapshot pin.
//!
//! Point reads decode canonical rows from `OwnedRead::get_dyn`. Prepared
//! executions acquire their own cache resolver through the core query path;
//! the worker-held LMDB snapshot does not retain obsolete text resolvers.
//!
//! No borrowed `ReadInstance` lease, no unsafe Send.
//! Published snapshots are [`super::SnapshotHandle`] only — mint with
//! `assemble`, close with `runtime_snapshot_close`. No writable `Db`.

use bumbledb::work::WorkContext;
use bumbledb::{Query, RelationId, StatementId, Value};

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
    query: Query,
    params: Vec<crate::marshal::OwnedParam>,
) -> SnapshotWork {
    Box::new(move |context, access| {
        context.checkpoint()?;
        let result = owned_execute_complete(access, context, &query, &params)?;
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
    runtime: std::sync::Arc<crate::runtime::Runtime>,
    query: Query,
) -> SnapshotWork {
    Box::new(move |context, access| {
        context.checkpoint()?;
        access
            .prepare(&runtime, &query, context)
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
