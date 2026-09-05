//! Snapshot and execution-session jobs over L07/L12 owned read/frame.
//!
//! Each operation receives a fresh `WorkContext` from its policy. The
//! snapshot's acquisition deadline is never reused. Prepared queries stay
//! in the worker table (`SnapshotAccess::install`) so L12's execute-prepared
//! path reuses the real object.
//!
//! Exact L07 adaptations: `OwnedRead::get_dyn` / `ReadFrame::get_dyn_into`
//! and `ReadFrame::prepare` plus `PreparedQuery::execute_complete_with_work`
//! on the frame (`ReadInstance` is now that frame). Hold the snapshot
//! generation for the job; L07 request: `OwnedRead::generation_handle()`.
//!
//! No borrowed `ReadInstance` lease, no unsafe Send.
//! Published snapshots are [`super::SnapshotHandle`] only — mint with
//! `assemble`, close with `runtime_snapshot_close`. No writable `Db`.

use bumbledb::work::WorkContext;
use bumbledb::{Query, RelationId, StatementId, Value};

use crate::marshal::ValueOut;
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
        // C3: pin generation identity for the token-bearing read.
        // L07 request: return/hold `GenerationHandle` from the owned cache.
        let _generation = access.owned.generation();
        let hit = access
            .owned
            .get_dyn(relation, key, &row, context)
            .map_err(engine_error)?;
        Ok(Output::Row(hit.map(|values| {
            values.into_iter().map(ValueOut::from_value).collect()
        })))
    })
}

/// One complete bounded execution: prepare on the owned frame, seal a
/// `CompleteResult` (failed work never becomes a logical result), then
/// install the prepared query so the worker table retains reuse.
pub(crate) fn execute_complete_work(
    query: Query,
    params: Vec<crate::marshal::OwnedParam>,
) -> SnapshotWork {
    Box::new(move |context, access| {
        context.checkpoint()?;
        let _generation = access.owned.generation();
        let result = owned_execute_complete(access, context, query, &params)?;
        Ok(Output::CompleteResult(result))
    })
}

fn owned_execute_complete(
    access: &mut SnapshotAccess<'_>,
    context: &WorkContext,
    query: Query,
    params: &[crate::marshal::OwnedParam],
) -> Result<bumbledb::CompleteResult, RuntimeError> {
    let frame = access.frame(context);
    let mut prepared = frame.prepare(&query).map_err(engine_error)?;
    let args = crate::param_args(params);
    let result = prepared
        .execute_complete_with_work(&frame, context, args.as_slice())
        .map_err(engine_error)?;
    access.install(prepared);
    Ok(result)
}

