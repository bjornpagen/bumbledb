//! Database-free change drafts. Failed ingestion or finish revokes the
//! capability and releases pending rows. Each operation has independent
//! cooperative cancellation; a draft has no hidden deadline or allowance.

use bumbledb::work::{WorkContext, WorkError};
use bumbledb::{ChangeSet, RelationId, Value};

use crate::runtime::registry::Payload;
use crate::runtime::registry::registry_draft::{DraftPayload, PendingChange};
use crate::runtime::{Output, RuntimeError};

use super::change_error;

fn mark_terminal(entry: &mut DraftPayload) {
    entry.terminal = true;
    entry.pending = Vec::new();
}

/// Transfer owned input rows into the draft. A failure spends the draft,
/// including any prefix staged by this or a previous chunk.
pub(crate) fn ingest_from_payload(
    payload: &mut Payload,
    context: &WorkContext,
    relation: u32,
    insert: bool,
    rows: Vec<Vec<Value>>,
) -> Result<Output, RuntimeError> {
    let Payload::Draft(entry) = payload else {
        return Err(RuntimeError::Internal);
    };
    if entry.terminal {
        return Err(RuntimeError::SpentHandle);
    }
    let result = (|| {
        context.checkpoint()?;
        entry
            .pending
            .try_reserve(rows.len())
            .map_err(|_| WorkError::Allocation)?;
        let submitted = rows.len() as u64;
        let relation = RelationId(relation);
        for values in rows {
            context.checkpoint()?;
            entry.pending.push(PendingChange {
                relation,
                insert,
                values,
            });
        }
        context.checkpoint()?;
        Ok(Output::Mutation {
            submitted,
            changed: submitted,
        })
    })();
    if result.is_err() {
        mark_terminal(entry);
    }
    result
}

/// Consume pending rows as they are encoded; no second collection of decoded
/// values survives alongside the finished canonical `ChangeSet`.
pub(crate) fn finish_from_payload(
    payload: &mut Payload,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    let Payload::Draft(entry) = payload else {
        return Err(RuntimeError::Internal);
    };
    if entry.terminal {
        return Err(RuntimeError::SpentHandle);
    }
    entry.terminal = true;
    let pending = std::mem::take(&mut entry.pending);
    context.checkpoint()?;
    let mut builder = ChangeSet::builder(&entry.schema, context.clone());
    for change in pending {
        context.checkpoint()?;
        let landed = if change.insert {
            builder.insert(change.relation, &change.values)
        } else {
            builder.delete(change.relation, &change.values)
        };
        landed.map_err(|error| change_error(&error))?;
    }
    let changes = builder.finish().map_err(|error| change_error(&error))?;
    let fingerprint = crate::hex_fingerprint(&changes.schema().0);
    Ok(Output::Changes(super::ChangesOpened {
        changes,
        schema: std::sync::Arc::clone(&entry.schema),
        fingerprint,
    }))
}
