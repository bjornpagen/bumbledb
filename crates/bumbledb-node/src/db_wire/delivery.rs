//! Transactional delivery of completed results, not streaming execution.
//!
//! Borrowed rows convert directly into one private [`QueuedOutput`]. Collect
//! leaves the sealed backing available. A page holds one [`DeliveryTicket`]
//! from visitation through [`PublicationSink::accept`]; only acceptance
//! commits its position. Cancellation or rejection drops the output and
//! retries the same row. No second preview or full `Answers` copy is needed.
//!
//! Delivery batches bound the number of rows copied per pull, not the total
//! query result. Terminal backing corruption stays failed, never EOF.

use bumbledb::DeliveryTicket;
use bumbledb::work::WorkContext;

use crate::marshal;
use crate::runtime::registry::{Payload, ResultState};
use crate::runtime::{Output, PublicationSink, QueuedOutput, RuntimeError};

use super::engine_error;

/// A scheduling/delivery quantum, not a row or memory allowance.
const PAGE_ROWS: usize = 256;

/// Collection is explicit full materialization into the final JS-facing
/// representation. The sealed result stays available for another delivery.
pub(crate) fn collect_from_payload(
    payload: &mut Payload,
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    let Payload::Result { result, state } = payload else {
        return Err(RuntimeError::Internal);
    };
    if *state == ResultState::Spent {
        return Err(RuntimeError::SpentHandle);
    }
    let Some(result) = result.as_ref() else {
        return Err(RuntimeError::SpentHandle);
    };
    work.checkpoint()?;
    let capacity = usize::try_from(result.len()).map_err(|_| RuntimeError::InvalidArgument)?;
    let mut queued = marshal::result_rows(work, capacity).map_err(|error| engine_error(&error))?;
    result
        .visit_rows(work, |row| {
            marshal::push_result_row(work, &mut queued, &row)
        })
        .map_err(|error| engine_error(&error))?;
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
    Ok(Output::ResultCursor(result.into_cursor(PAGE_ROWS)))
}

fn open_preview<'a>(
    cursor: &'a mut bumbledb::ResultCursor,
    work: &WorkContext,
) -> Result<(DeliveryTicket<'a>, QueuedOutput, bool), RuntimeError> {
    work.checkpoint()?;
    let mut ticket = DeliveryTicket::open(cursor);
    let mut queued = marshal::result_rows(work, 0).map_err(|error| engine_error(&error))?;
    match ticket.visit_page(work, |row| {
        marshal::push_result_row(work, &mut queued, &row)
    }) {
        Ok(None) => {
            ticket.abort();
            Err(RuntimeError::ClosedHandle)
        }
        Ok(Some(_)) => {
            let terminal = ticket.will_be_terminal();
            // Even an empty result publishes one empty terminal page.
            Ok((ticket, queued, terminal))
        }
        Err(error) => {
            ticket.abort();
            Err(engine_error(&error))
        }
    }
}

/// Live pull: same ticket through `publication.accept` + `commit`.
pub(crate) fn publish_from_payload(
    payload: &mut Payload,
    work: &WorkContext,
    publication: &mut PublicationSink<'_>,
) -> Result<Output, RuntimeError> {
    let Payload::Cursor { cursor, drained } = payload else {
        return Err(RuntimeError::Internal);
    };
    if *drained {
        return Ok(Output::Page(None));
    }
    let (ticket, queued, terminal) = open_preview(cursor, work)?;
    let output = Output::Page(Some(queued));
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
) -> Result<PullOutcome, RuntimeError> {
    let Payload::Cursor { cursor, drained } = payload else {
        return Err(RuntimeError::Internal);
    };
    if *drained {
        return Ok(PullOutcome::Eof);
    }
    let (ticket, queued, terminal) = match open_preview(cursor, work) {
        Ok(opened) => opened,
        Err(error) if is_terminal_backing(&error) => {
            return Ok(PullOutcome::Terminal(error));
        }
        Err(error) => return Err(error),
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
