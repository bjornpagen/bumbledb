//! Completed results and transactional paged delivery.
//!
//! Execution builds one private answer set. Only a successful, fully
//! evaluated result can be sealed. Rows borrow that owner; there is no
//! quota-triggered copy into temporary storage or second result codec.
//! Paging bounds delivery, not query execution or the size of the result.
//! A failed/cancelled delivery never advances its cursor.

use super::source::{PinnedSource, work_error};
use super::{Answer, AnswerValue, Answers};
use crate::error::Result;
use crate::storage::GenerationId;
use crate::work::WorkContext;

/// Which source and version this result is the complete answer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultIdentity {
    pub(crate) source: PinnedSource,
    /// Store snapshot generation; heap instances have no durable identity.
    pub(crate) generation: Option<GenerationId>,
}

impl ResultIdentity {
    #[must_use]
    pub fn generation(&self) -> Option<GenerationId> {
        self.generation
    }
}

/// One sealed, completely evaluated answer set.
pub struct CompleteResult {
    identity: ResultIdentity,
    answers: Answers,
}

/// A row borrowed from a completed result, not an early execution result.
pub struct ResultRow<'a> {
    answer: Answer<'a>,
}

impl ResultRow<'_> {
    #[must_use]
    pub fn arity(&self) -> usize {
        self.answer.buffer.arity()
    }

    /// Typed values in column order. Text and bytes borrow the result's
    /// existing heaps; visiting values does not decode or allocate.
    pub fn values(&self) -> impl ExactSizeIterator<Item = AnswerValue<'_>> {
        (0..self.arity()).map(|column| self.answer.get(column))
    }
}

impl CompleteResult {
    /// Take an already finalized private answer set without copying it.
    pub(crate) fn seal(
        answers: Answers,
        identity: ResultIdentity,
        work: &WorkContext,
    ) -> Result<Self> {
        work.checkpoint().map_err(work_error)?;
        Ok(Self { identity, answers })
    }

    #[must_use]
    pub fn identity(&self) -> ResultIdentity {
        self.identity
    }

    #[must_use]
    pub fn len(&self) -> u64 {
        self.answers.len() as u64
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.answers.is_empty()
    }

    #[must_use]
    pub fn arity(&self) -> usize {
        self.answers.arity()
    }

    /// Borrow completed rows without another result-sized collection.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = ResultRow<'_>> {
        self.answers.answers().map(|answer| ResultRow { answer })
    }

    /// Visit all completed rows under this delivery's cancellation context.
    /// Keep any new output private until success: a failed visitor can have
    /// observed a prefix, but the sealed result remains unchanged.
    /// # Errors
    /// Cancellation or a visitor error.
    pub fn visit_rows(
        &self,
        work: &WorkContext,
        mut visit: impl FnMut(ResultRow<'_>) -> Result<()>,
    ) -> Result<()> {
        work.checkpoint().map_err(work_error)?;
        for row in self.rows() {
            work.checkpoint().map_err(work_error)?;
            visit(row)?;
        }
        work.checkpoint().map_err(work_error)
    }

    /// Consume the result and move its answer storage, without copying.
    #[must_use]
    pub fn into_answers(self) -> Answers {
        self.answers
    }

    /// Consume the result into one cursor with bounded delivery batches.
    /// Zero selects a single row per page. This does not stream execution.
    #[must_use]
    pub fn into_cursor(self, page_rows: usize) -> ResultCursor {
        ResultCursor {
            result: self,
            page_rows: page_rows.max(1),
            next_row: 0,
            done: false,
        }
    }
}

/// One pull's atomicity boundary. A visit or owned preview records only a
/// pending advance. Publish the complete output before committing this
/// same ticket; abort/drop leaves the cursor at its previous position.
pub struct DeliveryTicket<'cursor> {
    cursor: &'cursor mut ResultCursor,
    preview: Option<Answers>,
    pending: Option<PendingAdvance>,
}

impl<'cursor> DeliveryTicket<'cursor> {
    pub fn open(cursor: &'cursor mut ResultCursor) -> Self {
        Self {
            cursor,
            preview: None,
            pending: None,
        }
    }

    #[must_use]
    pub fn previewed_rows(&self) -> u64 {
        self.pending.as_ref().map_or(0, |pending| {
            (pending.next_row - self.cursor.next_row) as u64
        })
    }

    #[must_use]
    pub fn will_be_terminal(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| pending.terminal)
    }

    /// Copy only this page into an ordinary Rust answer buffer. Position
    /// stays unchanged until commit. Adapters can use the borrowed page
    /// visitor to write directly into their own output instead of copying twice.
    /// # Errors
    /// Cancellation; an incomplete preview is discarded.
    pub fn preview_page(&mut self, work: &WorkContext) -> Result<Option<&Answers>> {
        let mut rows = Answers::new();
        rows.begin(self.cursor.arity());
        let visited = self.visit_page(work, |row| {
            for (column, value) in row.values().enumerate() {
                if column % 64 == 0 {
                    work.checkpoint().map_err(work_error)?;
                }
                rows.push_value(&value);
            }
            Ok(())
        })?;
        if visited.is_none() {
            return Ok(None);
        }
        self.preview = Some(rows);
        Ok(self.preview.as_ref())
    }

    /// Visit the next page directly into a private destination. Success
    /// and failure leave the cursor unchanged. A failed destination must
    /// discard its prefix. Every new attempt discards the prior pending
    /// advance, including after adopting its owned preview.
    ///
    /// Some(0) is the empty terminal frame; None means delivery is
    /// already complete. A visitor failure does not corrupt stored rows
    /// and may be retried with a new ticket.
    /// # Errors
    /// Cancellation or a visitor error.
    pub fn visit_page(
        &mut self,
        work: &WorkContext,
        mut visit: impl FnMut(ResultRow<'_>) -> Result<()>,
    ) -> Result<Option<u64>> {
        self.preview = None;
        self.pending = None;
        work.checkpoint().map_err(work_error)?;
        if self.cursor.done {
            return Ok(None);
        }
        let start = self.cursor.next_row;
        let total = self.cursor.result.answers.len();
        let end = start.saturating_add(self.cursor.page_rows).min(total);
        for answer in self
            .cursor
            .result
            .answers
            .answers()
            .skip(start)
            .take(end - start)
        {
            work.checkpoint().map_err(work_error)?;
            visit(ResultRow { answer })?;
        }
        work.checkpoint().map_err(work_error)?;
        self.pending = Some(PendingAdvance {
            next_row: end,
            terminal: end == total,
        });
        Ok(Some((end - start) as u64))
    }

    /// Move the owned preview out of the ticket. Its advance stays pending.
    pub fn adopt(&mut self) -> Option<Answers> {
        self.preview.take()
    }

    /// Commit only this ticket's successfully completed advance. No
    /// allocation, storage access or cancellation check occurs here.
    pub fn commit(self) {
        if let Some(pending) = self.pending {
            self.cursor.next_row = pending.next_row;
            self.cursor.done = pending.terminal;
        }
    }

    /// Drop the preview and pending advance, without modifying the cursor.
    pub fn abort(self) {}
}

/// An owned delivery page. Terminal is true exactly on the page that
/// completes the result, including an empty result's sole empty page.
pub struct ResultPage {
    pub rows: Answers,
    pub terminal: bool,
}

struct PendingAdvance {
    next_row: usize,
    terminal: bool,
}

/// One consuming cursor over a completed result.
pub struct ResultCursor {
    result: CompleteResult,
    page_rows: usize,
    next_row: usize,
    done: bool,
}

impl ResultCursor {
    #[must_use]
    pub fn identity(&self) -> ResultIdentity {
        self.result.identity()
    }

    #[must_use]
    pub fn len(&self) -> u64 {
        self.result.len()
    }

    #[must_use]
    pub fn arity(&self) -> usize {
        self.result.arity()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.result.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn debug_next_row(&self) -> u64 {
        self.next_row as u64
    }

    /// Copy the next delivery batch and commit its advance together.
    /// # Errors
    /// Cancellation leaves the cursor unchanged.
    /// # Panics
    /// Only if an internal invariant is broken: a successful preview must own its page.
    pub fn next_page(&mut self, work: &WorkContext) -> Result<Option<ResultPage>> {
        let mut ticket = DeliveryTicket::open(self);
        let Some(_) = ticket.preview_page(work)? else {
            return Ok(None);
        };
        let rows = ticket.adopt().expect("successful preview owns its page");
        let terminal = ticket.will_be_terminal();
        ticket.commit();
        Ok(Some(ResultPage { rows, terminal }))
    }
}

impl<S> super::PreparedQuery<S> {
    /// Execute into one private, complete answer owner.
    /// # Errors
    /// Bind, storage, cancellation or query evaluation failure. No partial
    /// result is returned.
    #[doc(hidden)]
    pub fn execute_complete<'p, P: super::BindArgs<'p>>(
        &mut self,
        instance: &crate::api::db::ReadInstance<'_, S>,
        params: P,
    ) -> Result<CompleteResult> {
        self.execute_complete_with_work(instance, instance.work(), params)
    }

    /// Execute with the current caller's cancellation context; a retained
    /// result does not consult that context during later delivery.
    /// # Errors
    /// As execute_complete.
    #[doc(hidden)]
    pub fn execute_complete_with_work<'p, P: super::BindArgs<'p>>(
        &mut self,
        instance: &crate::api::db::ReadInstance<'_, S>,
        work: &WorkContext,
        params: P,
    ) -> Result<CompleteResult> {
        let mut answers = Answers::new();
        let source = super::source::QuerySource::store(instance.snapshot(), work);
        self.execute_source(&source, params, &mut answers)?;
        let identity = ResultIdentity {
            source: PinnedSource::Store(instance.snapshot().identity()),
            generation: Some(instance.snapshot().generation()),
        };
        CompleteResult::seal(answers, identity, work)
    }
}

#[cfg(test)]
mod tests;
