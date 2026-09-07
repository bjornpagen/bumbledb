//! Database-free change drafts: one cumulative input/work/deadline across
//! chunks and finish. A failed draft is terminal (capability revoked).
//! Each napi operation still starts a fresh `WorkContext`; the draft's
//! ledger is independent and never reset.
//!
//! Cumulative budget: `used_input` / `used_rows` accumulate across chunks
//! and finish never resets them. A failed chunk poisons the persisted
//! [`crate::runtime::DraftLedger`] (`terminal`). Work/deadline live on
//! `DraftPayload.ledger` — they are not reconstructed at finish.

use std::time::Instant;

use bumbledb::work::{WorkContext, WorkError};
use bumbledb::{ChangeSet, RelationId, Value};

use crate::runtime::registry::Payload;
use crate::runtime::registry::registry_draft::{DraftPayload, PendingChange};
use crate::runtime::{Output, RuntimeError};

use super::change_error;

fn draft_spent(entry: &DraftPayload) -> bool {
    entry.ledger.terminal
        || entry.used_input > entry.allowance_input
        || entry.used_rows > entry.allowance_rows
        || entry.ledger.used_work > entry.ledger.allowance_work
}

fn mark_terminal(entry: &mut DraftPayload) {
    entry.ledger.terminal = true;
}

fn draft_deadline(entry: &mut DraftPayload) -> Result<(), RuntimeError> {
    if Instant::now() >= entry.ledger.deadline {
        mark_terminal(entry);
        return Err(RuntimeError::Work(WorkError::DeadlineExceeded));
    }
    Ok(())
}

/// One bounded ingestion chunk. Cumulative input/rows never reset. Budget
/// or shape failure spends the draft.
pub(crate) fn ingest_from_payload(
    payload: &mut Payload,
    context: &WorkContext,
    relation: u32,
    insert: bool,
    rows: Vec<Vec<Value>>,
    chunk_bytes: u64,
) -> Result<Output, RuntimeError> {
    let Payload::Draft(entry) = payload else {
        return Err(RuntimeError::Internal);
    };
    if draft_spent(entry) {
        return Err(RuntimeError::SpentHandle);
    }
    let result = ingest_chunk(entry, context, relation, insert, rows, chunk_bytes);
    if result.is_err() {
        mark_terminal(entry);
    }
    result
}

fn ingest_chunk(
    entry: &mut DraftPayload,
    context: &WorkContext,
    relation: u32,
    insert: bool,
    rows: Vec<Vec<Value>>,
    chunk_bytes: u64,
) -> Result<Output, RuntimeError> {
    context.checkpoint()?;
    draft_deadline(entry)?;
    let chunk_work = rows.len() as u64;
    let next_work = entry.ledger.used_work.saturating_add(chunk_work);
    if next_work > entry.ledger.allowance_work {
        mark_terminal(entry);
        return Err(RuntimeError::ResourceLimit {
            dimension: "workUnits",
            used: entry.ledger.used_work,
            requested: chunk_work,
            limit: entry.ledger.allowance_work,
        });
    }
    let next = entry.used_input.saturating_add(chunk_bytes);
    if next > entry.allowance_input {
        mark_terminal(entry);
        return Err(RuntimeError::ResourceLimit {
            dimension: "inputBytes",
            used: entry.used_input,
            requested: chunk_bytes,
            limit: entry.allowance_input,
        });
    }
    let submitted = rows.len() as u64;
    let next_rows = entry.used_rows.saturating_add(submitted);
    if next_rows > entry.allowance_rows {
        mark_terminal(entry);
        return Err(RuntimeError::ResourceLimit {
            dimension: "rows",
            used: entry.used_rows,
            requested: submitted,
            limit: entry.allowance_rows,
        });
    }
    // The JS-thread parser already charged this operation's input before
    // dispatch. Only the draft's cumulative ledger advances here.
    let rel = RelationId(relation);
    for values in rows {
        context.rows(1)?;
        context.step(1)?;
        entry.pending.push(PendingChange {
            relation: rel,
            insert,
            values,
        });
    }
    entry.used_input = next;
    entry.used_rows = next_rows;
    entry.ledger.used_work = next_work;
    Ok(Output::Mutation {
        submitted,
        changed: submitted,
    })
}

/// Consume the draft into one immutable schema-bound `ChangeSet`. Uses the
/// same cumulative ledger; failure is terminal.
pub(crate) fn finish_from_payload(
    payload: &mut Payload,
    context: &WorkContext,
) -> Result<Output, RuntimeError> {
    let Payload::Draft(entry) = payload else {
        return Err(RuntimeError::Internal);
    };
    if draft_spent(entry) {
        return Err(RuntimeError::SpentHandle);
    }
    context.checkpoint().inspect_err(|_| mark_terminal(entry))?;
    draft_deadline(entry)?;
    let finish_work = entry.pending.len() as u64;
    let next_work = entry.ledger.used_work.saturating_add(finish_work);
    if next_work > entry.ledger.allowance_work {
        mark_terminal(entry);
        return Err(RuntimeError::ResourceLimit {
            dimension: "workUnits",
            used: entry.ledger.used_work,
            requested: finish_work,
            limit: entry.ledger.allowance_work,
        });
    }
    let mut builder = ChangeSet::builder(&entry.schema, context.clone());
    for change in &entry.pending {
        if let Err(error) = context.step(1) {
            mark_terminal(entry);
            return Err(error.into());
        }
        entry.ledger.used_work = entry.ledger.used_work.saturating_add(1);
        let landed = if change.insert {
            builder.insert(change.relation, &change.values)
        } else {
            builder.delete(change.relation, &change.values)
        };
        if let Err(error) = landed {
            mark_terminal(entry);
            return Err(change_error(&error));
        }
    }
    match builder.finish() {
        Ok(changes) => {
            let fingerprint = crate::hex_fingerprint(&changes.schema().0);
            mark_terminal(entry);
            Ok(Output::Changes(super::ChangesOpened {
                changes,
                schema: std::sync::Arc::clone(&entry.schema),
                fingerprint,
            }))
        }
        Err(error) => {
            mark_terminal(entry);
            Err(change_error(&error))
        }
    }
}
