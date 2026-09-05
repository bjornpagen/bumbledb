//! Transactional collection and paging (C8 / CORE-008 / TS-001/005).
//!
//! One public pull is the atomicity boundary. The L05 ticket previews and
//! adopts under admitted overlap; L13 registers the native `QueuedOutput`,
//! then `commit` advances `next_row` once. Predelivery refusal aborts with
//! no rows and no advancement. A next row that does not fit ends a nonempty
//! page successfully. An oversized first row refuses unchanged.
//! Collect and `cursor_next` honor `min(requested, work.resultBytes)` —
//! a caller cap cannot enlarge the operation ledger.
//!
//! L05 ticket on the one native pull:
//! 1. [`DeliveryTicket::open`]`(&mut ResultCursor)`
//! 2. [`DeliveryTicket::preview_page`] / [`DeliveryTicket::adopt`] — bounded
//!    rows; `next_row` does not advance
//! 3. Register the complete native output owner ([`QueuedOutput`])
//! 4. [`PublicationSink::accept`] writes output and `commit`s the **same**
//!    ticket. On accept `Err`, `abort` that ticket. No second preview.
//! 5. Budget refusal / cancel after adopt is abort: not a committed page,
//!    not a sticky failed cursor. `abort` discards `pending_advance`.
//!    Do not `publication.accept` an abandoned preview. A fresh
//!    unpreviewed ticket must have nothing to `commit`.
//! 7. A next row that does not fit a nonempty page ends that page
//! 8. Oversized first row refuses; cursor unchanged
//! 9. Terminal backing corruption stays sticky; no complete page or EOF
//! 10. No second public raw cursor (`next_page` / `next_page_with_work`
//!     are not addon verbs)
//!
//! A checkpoint after [`register_page`] must not drop the owner: the page
//! is already consumed into [`QueuedOutput`]. Resource refusal before that
//! owner exists leaves `next_row` unchanged. Backing failure never sets
//! the lawful-EOF `drained` bit.
//!
//! L12 handoff: [`publish_from_payload`] calls
//! `publication.accept(committed_output, || ticket.commit())`.
//! This file does not write `operation.output`.
//!
//! Delete: `inspect`/`copy` twins, eager `next_page_with_work` as the
//! pull, `Payload::Cursor.pending`, unbounded collect, `into_cursor(1)`.

use bumbledb::work::{ByteKind, Resource, WorkContext, WorkError};
use bumbledb::{Answers, DeliveryTicket};

use crate::marshal;
use crate::runtime::registry::{Payload, ResultState};
use crate::runtime::{Output, PublicationSink, QueuedOutput, RuntimeError};

use super::engine_error;

/// L16: the public collect/page cap cannot enlarge `work.resultBytes`.
#[must_use]
pub fn intersected_result_bytes(requested: u64, work: &WorkContext) -> u64 {
    requested.min(work.limit(Resource::ResultBytes))
}

/// Page-row cap for [`bumbledb::CompleteResult::into_cursor`]: the work
/// row limit intersected with remaining sealed rows. Not the literal `1`
/// that forced one-row native pages.
#[must_use]
pub fn page_row_cap(work: &WorkContext, remaining: u64) -> usize {
    let capped = work.limit(Resource::Rows).min(remaining);
    usize::try_from(capped).unwrap_or(usize::MAX)
}

pub(crate) fn resource_limit(error: WorkError) -> RuntimeError {
    match error {
        WorkError::Exhausted {
            resource,
            used,
            requested,
            limit,
        } => RuntimeError::ResourceLimit {
            dimension: match resource {
                bumbledb::work::Resource::InputBytes => "inputBytes",
                bumbledb::work::Resource::WorkingBytes => "workingBytes",
                bumbledb::work::Resource::ScratchBytes => "scratchBytes",
                bumbledb::work::Resource::ResultBytes => "resultBytes",
                bumbledb::work::Resource::Rows => "rows",
                bumbledb::work::Resource::WorkUnits => "workUnits",
            },
            used,
            requested,
            limit,
        },
        other => RuntimeError::Work(other),
    }
}

fn ticket_error(error: &bumbledb::Error) -> RuntimeError {
    match engine_error(error) {
        RuntimeError::Work(work) => resource_limit(work),
        other => other,
    }
}

/// Convert admitted answers into queued output. Reserve overlapping
/// conversion charge before any cell copy (D01). This file does not
/// encode a candidate row into an uncharged `Vec`.
pub(crate) fn register_page(
    work: &WorkContext,
    answers: &Answers,
) -> Result<QueuedOutput, RuntimeError> {
    work.checkpoint()?;
    let (rows, charge) = marshal::answers_out_charged(work, answers)?;
    Ok(QueuedOutput { rows, charge })
}

/// Bounded collect: preflight the sealed size, reserve, convert, never
/// hide a full-result conversion behind the result handle.
pub(crate) fn collect_from_payload(
    payload: &mut Payload,
    work: &WorkContext,
    result_bytes: u64,
    row_limit: u64,
) -> Result<Output, RuntimeError> {
    let Payload::Result { result, state } = payload else {
        return Err(RuntimeError::Internal);
    };
    if *state == ResultState::Spent {
        return Err(RuntimeError::SpentHandle);
    }
    let Some(result) = result.as_mut() else {
        return Err(RuntimeError::SpentHandle);
    };
    work.checkpoint()?;
    result.rebind_work(work);
    if result.len() > row_limit {
        return Err(RuntimeError::ResourceLimit {
            dimension: "rows",
            used: result.len(),
            requested: result.len(),
            limit: row_limit,
        });
    }
    let cap = intersected_result_bytes(result_bytes, work);
    let estimated = result.byte_len();
    if estimated > cap {
        return Err(RuntimeError::ResourceLimit {
            dimension: "resultBytes",
            used: work.used(Resource::ResultBytes),
            requested: estimated,
            limit: cap,
        });
    }
    let overlap = work
        .reserve(ByteKind::Result, estimated)
        .map_err(resource_limit)?;
    let answers = result
        .collect_with_work(row_limit, work, cap)
        .map_err(|error| engine_error(&error))?;
    let queued = match register_page(work, &answers) {
        Ok(queued) => queued,
        Err(error) => {
            drop(overlap);
            return Err(error);
        }
    };
    drop(overlap);
    Ok(Output::Rows(queued))
}

/// Atomic spend: the sealed backing moves into one cursor. Second spend
/// refuses before touching the backing.
pub(crate) fn transfer_from_payload(
    payload: &mut Payload,
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    work.checkpoint()?;
    let Payload::Result { result, state } = payload else {
        return Err(RuntimeError::Internal);
    };
    if *state == ResultState::Spent {
        return Err(RuntimeError::SpentHandle);
    }
    let Some(result) = result.take() else {
        return Err(RuntimeError::SpentHandle);
    };
    *state = ResultState::Spent;
    let page_rows = page_row_cap(work, result.len());
    Ok(Output::ResultCursor(result.into_cursor(page_rows)))
}

fn open_preview<'a>(
    cursor: &'a mut bumbledb::ResultCursor,
    work: &WorkContext,
    page_bytes: u64,
) -> Result<(DeliveryTicket<'a>, Option<Answers>, bool), RuntimeError> {
    work.checkpoint()?;
    cursor.rebind_work(work);
    let cap = intersected_result_bytes(page_bytes, work);
    // Bound the queued row's fixed cell/vector allocations, plus UUID text
    // (the only scalar expanding into a string), without decoding the row.
    // Two encoded copies cover the preview and variable-sized output heaps;
    // a third covers the one-row scratch decode buffer on disk-backed pages.
    let fixed = (std::mem::size_of::<Vec<marshal::ValueOut>>() as u64).saturating_add(
        (cursor.arity() as u64)
            .saturating_mul(std::mem::size_of::<marshal::ValueOut>() as u64 + 36),
    );
    let mut ticket = DeliveryTicket::open(cursor);
    match ticket.preview_page_with_cost(work, cap, |bytes| {
        fixed.saturating_add(bytes.saturating_mul(3))
    }) {
        Ok(None) => {
            ticket.abort();
            Err(RuntimeError::ClosedHandle)
        }
        Ok(Some(answers)) => {
            let empty = answers.is_empty();
            let terminal = ticket.will_be_terminal();
            if empty {
                Ok((ticket, None, true))
            } else {
                let Some(answers) = ticket.adopt() else {
                    ticket.abort();
                    return Err(RuntimeError::Internal);
                };
                Ok((ticket, Some(answers), terminal))
            }
        }
        Err(error) => {
            ticket.abort();
            Err(ticket_error(&error))
        }
    }
}

/// Live pull: same ticket through `publication.accept` + `commit`.
pub(crate) fn publish_from_payload(
    payload: &mut Payload,
    work: &WorkContext,
    page_bytes: u64,
    publication: &mut PublicationSink<'_>,
) -> Result<Output, RuntimeError> {
    let Payload::Cursor { cursor, drained } = payload else {
        return Err(RuntimeError::Internal);
    };
    if *drained {
        return Ok(Output::Page(None));
    }
    let (ticket, answers, terminal) = open_preview(cursor, work, page_bytes)?;
    let queued = match answers {
        None => None,
        Some(answers) => match register_page(work, &answers) {
            Ok(queued) => Some(queued),
            Err(error) => {
                ticket.abort();
                return Err(error);
            }
        },
    };
    let output = Output::Page(queued);
    let mut ticket = Some(ticket);
    match publication.accept(output, || {
        ticket.take().expect("live ticket").commit();
        *drained = terminal;
    }) {
        Ok(()) => {
            if let Some(leftover) = ticket.take() {
                leftover.abort();
            }
            Ok(Output::Ready)
        }
        Err(error) => {
            if let Some(ticket) = ticket.take() {
                ticket.abort();
            }
            Err(error)
        }
    }
}

/// Test/discriminator pull: preview, register, abort. Cursor unchanged.
#[cfg(test)]
pub(crate) fn pull_from_payload(
    payload: &mut Payload,
    work: &WorkContext,
    page_bytes: u64,
) -> Result<PullOutcome, RuntimeError> {
    let Payload::Cursor { cursor, drained } = payload else {
        return Err(RuntimeError::Internal);
    };
    if *drained {
        return Ok(PullOutcome::Eof);
    }
    let (ticket, answers, terminal) = match open_preview(cursor, work, page_bytes) {
        Ok(opened) => opened,
        Err(error) if is_terminal_backing(&error) => {
            return Ok(PullOutcome::Terminal(error));
        }
        Err(error) => return Err(error),
    };
    let Some(answers) = answers else {
        ticket.abort();
        return Ok(PullOutcome::Eof);
    };
    let queued = match register_page(work, &answers) {
        Ok(queued) => queued,
        Err(error) => {
            ticket.abort();
            return Err(error);
        }
    };
    ticket.abort();
    Ok(PullOutcome::Page { queued, terminal })
}

#[cfg(test)]
pub(crate) enum PullOutcome {
    Page {
        queued: QueuedOutput,
        terminal: bool,
    },
    Eof,
    Terminal(RuntimeError),
}

#[cfg(test)]
impl PullOutcome {
    /// Output L12's sink registers. Live pull commits the same ticket.
    pub fn committed_output(self) -> Result<Output, RuntimeError> {
        match self {
            Self::Page { queued, .. } => Ok(Output::Page(Some(queued))),
            Self::Eof => Ok(Output::Page(None)),
            Self::Terminal(error) => Err(error),
        }
    }
}

pub(crate) fn is_terminal_backing(error: &RuntimeError) -> bool {
    matches!(
        error,
        RuntimeError::Engine {
            kind: crate::tags::error_family::CORRUPTION | crate::tags::error_family::STORE,
            ..
        }
    )
}
