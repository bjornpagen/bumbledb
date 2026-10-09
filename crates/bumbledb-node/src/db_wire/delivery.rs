//! Delivery of completed results. Rows convert directly into one
//! [`QueuedOutput`]; a page holds one [`DeliveryTicket`] until
//! [`PublicationSink::accept`] commits its position, so cancellation or
//! rejection retries the same row. Terminal backing corruption stays failed,
//! never end of results.

use bumbledb::DeliveryTicket;
use bumbledb::work::WorkContext;

use crate::marshal;
use crate::runtime::registry::{Payload, ResultState};
use crate::runtime::{Output, PublicationSink, QueuedOutput, RuntimeError};

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
    let mut queued = marshal::result_rows(work, capacity)?;
    marshal::visit_with(|stop| {
        result.visit_rows(work, |row| {
            stop(marshal::push_result_row(work, &mut queued, &row))
        })
    })?;
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
    let mut queued = marshal::result_rows(work, 0)?;
    let visited = marshal::visit_with(|stop| {
        ticket.visit_page(work, |row| {
            stop(marshal::push_result_row(work, &mut queued, &row))
        })
    });
    match visited {
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
            Err(error)
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
    /// The output the publication sink registers; a live pull commits the
    /// same ticket.
    pub fn committed_output(self) -> Result<Output, RuntimeError> {
        match self {
            Self::Page { queued, .. } => Ok(Output::Page(Some(queued))),
            Self::Eof => Ok(Output::Page(None)),
            Self::Terminal(error) => Err(error),
        }
    }
}

/// A refusal of the result's backing that a retry cannot clear.
pub(crate) fn is_terminal_backing(error: &RuntimeError) -> bool {
    use bumbledb::ErrorKind;
    [ErrorKind::Corruption, ErrorKind::Io, ErrorKind::Lmdb]
        .into_iter()
        .any(|kind| error.is_engine(kind))
}
