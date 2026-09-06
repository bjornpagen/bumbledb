//! Completed results and paged delivery (C05, chapter 12 §8).
//!
//! Execution builds a private result; only after **all** relational work,
//! aggregate finalization, value checks and result storage succeed does a
//! [`CompleteResult`] exist, bound to the snapshot generation and source
//! identity it was computed at. No caller-owned buffer is gradually
//! populated — failed work never becomes a caller's new logical result.
//!
//! The backing is RAM ([`super::Answers`]) or the one temporary-LMDB
//! scratch map when the sealed rows exceed the result RAM allowance.
//! `collect(limit)` is an additional conversion: it returns a fully owned
//! collection or an error, leaving the sealed backing available.
//! `into_cursor(page_rows)` **consumes** the owner and transfers the
//! sealed backing to one explicitly chunked cursor with completion
//! identity and terminal framing — paged delivery after completion, never
//! early-result streaming. A disk failure while paging reports the
//! delivered prefix as incomplete (no terminal page), never as the
//! complete set. Dropping the cursor closes its own storage.

use super::source::PinnedSource;
use super::{AnswerValue, Answers, ResolveMemo};
use crate::error::{Error, Result};
use crate::exec::scratch::{ScratchAppend, ScratchMapId, ScratchRelation};
use crate::storage::GenerationId;
use crate::work::{ByteKind, ByteReservation, ChargedBuffer, WorkContext, WorkError};

/// The result RAM allowance before rows move to the scratch tier. A
/// tuning default (measured at F3), never a row-count or size cap: the
/// scratch tier is complete.
pub(crate) const RESULT_RAM_BYTES: usize = 8 << 20;

/// Which source and version this result is the complete answer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultIdentity {
    pub(crate) source: PinnedSource,
    /// The snapshot generation for store sources; `None` for heap
    /// instances (which carry no durable identity).
    pub(crate) generation: Option<GenerationId>,
}

impl ResultIdentity {
    #[must_use]
    pub fn generation(&self) -> Option<GenerationId> {
        self.generation
    }
}

enum Backing {
    Ram(Answers),
    Scratch {
        rows: ScratchRelation,
        arity: usize,
        count: u64,
    },
}

/// One sealed, completely evaluated answer set.
pub struct CompleteResult {
    identity: ResultIdentity,
    backing: Backing,
    /// One growing reservation follows the sealed backing until disposal.
    charge: Option<ByteReservation>,
}

/// A row borrowed from the sealed backing. Scratch decoding consumes the
/// mapped bytes in the same read transaction that admitted their size.
enum SealedRow<'a> {
    Ram(super::Answer<'a>),
    Encoded { bytes: &'a [u8], arity: usize },
}

impl SealedRow<'_> {
    fn encoded_len(&self) -> u64 {
        match self {
            Self::Ram(row) => (0..row.buffer.arity())
                .map(|column| encoded_value_len(&row.get(column)))
                .sum(),
            Self::Encoded { bytes, .. } => bytes.len() as u64,
        }
    }

    fn copy_to(self, out: &mut Answers) -> Result<()> {
        match self {
            Self::Ram(row) => {
                for column in 0..row.buffer.arity() {
                    out.push_value(&row.get(column));
                }
                Ok(())
            }
            Self::Encoded { bytes, arity } => decode_row(bytes, arity, out),
        }
    }
}

impl Backing {
    /// Visit a bounded sequence in one read transaction, stopping before
    /// the next row when the consumer has enough. Every expected ordinal
    /// must exist; exhaustion of corrupt storage is never successful EOF.
    fn visit_rows(
        &mut self,
        start: u64,
        end: u64,
        mut visit: impl FnMut(SealedRow<'_>) -> Result<bool>,
    ) -> Result<()> {
        if start >= end {
            return Ok(());
        }
        match self {
            Self::Ram(answers) => {
                if end > answers.len() as u64 {
                    return Err(missing_row());
                }
                for row in answers
                    .answers()
                    .skip(usize::try_from(start).map_err(|_| missing_row())?)
                    .take(usize::try_from(end - start).map_err(|_| missing_row())?)
                {
                    if !visit(SealedRow::Ram(row))? {
                        break;
                    }
                }
            }
            Self::Scratch { rows, arity, count } => {
                if end > *count {
                    return Err(missing_row());
                }
                let mut index = start;
                let mut stopped = false;
                rows.visit_from(&start.to_be_bytes(), &mut |key: &[u8], bytes: &[u8]| {
                    if key != index.to_be_bytes() {
                        return Err(missing_row());
                    }
                    if !visit(SealedRow::Encoded {
                        bytes,
                        arity: *arity,
                    })? {
                        stopped = true;
                        return Ok(false);
                    }
                    index += 1;
                    Ok(index < end)
                })?;
                if !stopped && index != end {
                    return Err(missing_row());
                }
            }
        }
        Ok(())
    }
}

fn reserve_result(
    charge: &mut Option<ByteReservation>,
    work: &WorkContext,
    bytes: u64,
) -> Result<()> {
    if let Some(charge) = charge {
        charge.resize(bytes).map_err(super::source::work_error)
    } else {
        *charge = Some(
            work.reserve(ByteKind::Result, bytes)
                .map_err(super::source::work_error)?,
        );
        Ok(())
    }
}

fn delivery_overflow(work: &WorkContext, requested: u64, limit: u64) -> Error {
    super::source::work_error(crate::work::WorkError::Exhausted {
        resource: crate::work::Resource::ResultBytes,
        used: work.used(crate::work::Resource::ResultBytes),
        requested,
        limit,
    })
}

/// Reuse the scratch substrate's bounded, charged staging. The input
/// carrier survives until all batches commit; a failure publishes nothing.
fn append_answers(
    rows: &mut ScratchRelation,
    answers: &Answers,
    start: u64,
    encoded: &mut ChargedBuffer,
) -> Result<u64> {
    let mut append = ScratchAppend::new(rows);
    let mut bytes = 0;
    for (index, answer) in answers.answers().enumerate() {
        encoded.clear();
        for column in 0..answers.arity() {
            encode_value(&answer.get(column), encoded).map_err(super::source::work_error)?;
        }
        append.append(
            ScratchMapId::Default,
            &(start + index as u64).to_be_bytes(),
            encoded.as_slice(),
        )?;
        bytes += encoded.len() as u64;
    }
    append.finish()?;
    Ok(bytes)
}

/// The logical bytes one RAM answer set retains: the cell array plus its
/// text/byte heaps — the result-byte charge basis on both backings.
#[cfg(test)]
pub(crate) fn logical_bytes_for_test(answers: &Answers) -> u64 {
    logical_bytes(answers)
}

fn logical_bytes(answers: &Answers) -> u64 {
    (answers.cells.len() * std::mem::size_of::<super::Cell>()
        + answers.text.len()
        + answers.blob.len()) as u64
}

impl CompleteResult {
    /// Seal a finalized answer set: charge its bytes as result capacity
    /// and move it beyond the RAM allowance into scratch. Only complete,
    /// finalized rows arrive here (the execution failed before this point
    /// otherwise). Post-hoc compatibility constructor over an
    /// already-materialized set — the production execute path charges
    /// DURING construction through [`ResultCharge`].
    pub(crate) fn seal(
        answers: Answers,
        identity: ResultIdentity,
        work: &WorkContext,
        ram_allowance: usize,
    ) -> Result<Self> {
        let bytes = logical_bytes(&answers);
        let charge = work
            .reserve(ByteKind::Result, bytes)
            .map_err(super::source::work_error)?;
        if bytes <= ram_allowance as u64 {
            return Ok(Self {
                identity,
                backing: Backing::Ram(answers),
                charge: Some(charge),
            });
        }
        // Beyond the allowance: the sealed backing is the scratch map
        // (already-complete rows copy over in bounded batches; the RAM
        // owner drops after the copy finishes — reserve-before-growth is
        // the scratch relation's own contract).
        let mut rows = ScratchRelation::new(work, 0);
        rows.force_spill()?;
        let arity = answers.arity();
        let mut encoded = ChargedBuffer::with_capacity(work, ByteKind::Working, 0)
            .map_err(super::source::work_error)?;
        append_answers(&mut rows, &answers, 0, &mut encoded)?;
        let count = answers.len() as u64;
        drop(answers);
        Ok(Self {
            identity,
            backing: Backing::Scratch { rows, arity, count },
            charge: Some(charge),
        })
    }

    #[must_use]
    pub fn identity(&self) -> ResultIdentity {
        self.identity
    }

    #[must_use]
    pub fn len(&self) -> u64 {
        match &self.backing {
            Backing::Ram(answers) => answers.len() as u64,
            Backing::Scratch { count, .. } => *count,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn arity(&self) -> usize {
        match &self.backing {
            Backing::Ram(answers) => answers.arity(),
            Backing::Scratch { arity, .. } => *arity,
        }
    }

    /// The sealed rows' byte charge — what this retained result holds
    /// against its `result_bytes` reservation (the bridge's retained-byte
    /// accounting reads this).
    #[must_use]
    pub fn byte_len(&self) -> u64 {
        self.charge.as_ref().map_or(0, ByteReservation::bytes)
    }

    /// Re-home a RETAINED result's scratch reads onto `work` — the
    /// chapter-35 seam: this result outlives the execute operation that
    /// sealed it, and without rebinding, later `collect`/page reads would
    /// keep charging that operation's ledger, whose deadline eventually
    /// expires — a spurious `DeadlineExceeded` on perfectly good sealed
    /// rows. Subsequent reads step/checkpoint against `work` instead.
    ///
    /// Terminal paths stay exactly-once: the sealed-byte reservation and
    /// every scratch reservation already taken keep their originating
    /// ledgers (each refunds once on drop), and the scratch directory
    /// unlinks once by drop order — rebinding swaps only the handle used
    /// for FUTURE charges. RAM-backed results read without consulting any
    /// ledger; rebinding one is a harmless no-op.
    pub fn rebind_work(&mut self, work: &WorkContext) {
        if let Backing::Scratch { rows, .. } = &mut self.backing {
            rows.rebind_work(work);
        }
    }

    /// Convert into one fully owned collection under a fresh delivery
    /// operation policy, or fail — a cap refusal leaves this sealed backing
    /// untouched and available.
    /// # Errors
    /// `ResultBytesOverflow` when the row count exceeds `limit`; delivery
    /// byte/work refusal; scratch read failure.
    pub fn collect_with_work(
        &mut self,
        limit: u64,
        work: &WorkContext,
        byte_allowance: u64,
    ) -> Result<Answers> {
        if self.len() > limit {
            return Err(Error::ResultBytesOverflow);
        }
        work.step(1).map_err(super::source::work_error)?;
        self.rebind_work(work);
        let estimated = if self.is_empty() {
            0
        } else {
            self.byte_len()
                .saturating_mul(limit.min(self.len()))
                .div_ceil(self.len())
        };
        if estimated > byte_allowance {
            return Err(super::source::work_error(
                crate::work::WorkError::Exhausted {
                    resource: crate::work::Resource::ResultBytes,
                    used: work.used(crate::work::Resource::ResultBytes),
                    requested: estimated,
                    limit: byte_allowance,
                },
            ));
        }
        let mut out = Answers::new();
        out.begin(self.arity());
        let mut used = 0u64;
        let mut charge = None;
        self.backing.visit_rows(0, self.len(), |row| {
            let row_bytes = row.encoded_len();
            if used.saturating_add(row_bytes) > byte_allowance {
                return Err(delivery_overflow(
                    work,
                    used.saturating_add(row_bytes),
                    byte_allowance,
                ));
            }
            used = used.saturating_add(row_bytes);
            reserve_result(&mut charge, work, used)?;
            row.copy_to(&mut out)?;
            Ok(true)
        })?;
        Ok(out)
    }

    /// Consume this result: the sealed backing transfers to one explicitly
    /// chunked cursor whose `page_rows` cap [`DeliveryTicket::preview_page`]
    /// honors (never a hardcoded 1). The result handle is spent; abandoning
    /// the cursor closes its own storage after active access drains.
    #[must_use]
    pub fn into_cursor(self, page_rows: usize) -> ResultCursor {
        ResultCursor {
            identity: self.identity,
            backing: self.backing,
            charge: self.charge,
            page_rows: page_rows.max(1),
            next_row: 0,
            done: false,
            failed: None,
        }
    }
}

/// Streamed result-construction accounting (chapter 12 §7 for the result
/// phase): finalize notes every appended row, result bytes are reserved in
/// bounded quanta as the set grows — never one post-hoc seal charge — and
/// past the RAM allowance rows route into the scratch backing DURING
/// construction. A tiny `result_bytes` budget therefore refuses before the
/// whole set materializes, a beyond-RAM result streams, and a
/// cancellation/refusal mid-construction surfaces before any
/// [`CompleteResult`] exists (Q-ATOMIC — no partial published answer).
pub(super) struct ResultCharge<'w> {
    work: &'w WorkContext,
    ram_allowance: usize,
    /// Logical result bytes already reserved.
    charged: u64,
    charge: Option<ByteReservation>,
    /// Rows appended since the last charge (bounded by the quantum).
    pending_rows: u32,
    /// Encoded bytes moved into the scratch tier so far.
    spilled_bytes: u64,
    spill: Option<ResultSpill>,
}

struct ResultSpill {
    rows: ScratchRelation,
    count: u64,
    encoded: ChargedBuffer,
}

impl<'w> ResultCharge<'w> {
    pub(super) fn new(work: &'w WorkContext, ram_allowance: usize) -> Self {
        Self {
            work,
            ram_allowance,
            charged: 0,
            charge: None,
            pending_rows: 0,
            spilled_bytes: 0,
            spill: None,
        }
    }

    fn total_bytes(&self, out: &Answers) -> u64 {
        self.spilled_bytes + logical_bytes(out)
    }

    /// Whether construction already routed rows into the scratch backing
    /// (the beyond-allowance streaming regime), before any seal.
    #[cfg(test)]
    pub(super) fn spilled(&self) -> bool {
        self.spill.is_some()
    }

    /// Reserve up to `target` logical bytes (monotone; bounded-quantum
    /// callers batch the deltas).
    fn charge_to(&mut self, target: u64) -> Result<()> {
        if target > self.charged {
            reserve_result(&mut self.charge, self.work, target)?;
            self.charged = target;
        }
        Ok(())
    }

    /// Move a bounded carrier into scratch. Its caller invalidates the
    /// resolve memo whenever this clears the referenced text heap.
    fn drain_to_scratch(&mut self, out: &mut Answers) -> Result<()> {
        self.charge_to(self.total_bytes(out))?;
        let spill = self.spill.as_mut().expect("drain under an open spill");
        self.spilled_bytes +=
            append_answers(&mut spill.rows, out, spill.count, &mut spill.encoded)?;
        spill.count += out.len() as u64;
        out.clear();
        Ok(())
    }

    /// Note one complete appended row: charge at the bounded quantum and
    /// route past-allowance growth into the scratch backing now, not at
    /// seal.
    pub(super) fn note_row(&mut self, out: &mut Answers, memo: &mut ResolveMemo) -> Result<()> {
        if out.arity() == 0 {
            return Ok(());
        }
        if self.spill.is_none() && logical_bytes(out) > self.ram_allowance as u64 {
            // Crossing the allowance: true-up the charge BEFORE the copy,
            // then continue in the scratch backing during construction.
            self.charge_to(self.total_bytes(out))?;
            let mut rows = ScratchRelation::new(self.work, 0);
            rows.force_spill()?;
            self.spill = Some(ResultSpill {
                rows,
                count: 0,
                encoded: ChargedBuffer::with_capacity(self.work, ByteKind::Working, 0)
                    .map_err(super::source::work_error)?,
            });
            self.drain_to_scratch(out)?;
            memo.clear();
        } else if self.spill.is_some()
            && (out.len() >= crate::exec::sink::STEP_QUANTUM as usize
                || logical_bytes(out) >= RESULT_RAM_BYTES as u64)
        {
            // Retain at most one polling quantum (or one RAM allowance,
            // plus the crossing row). Reuse both the carrier and text memo
            // across rows, and commit once per batch, never once per row.
            self.drain_to_scratch(out)?;
            memo.clear();
        }
        self.pending_rows += 1;
        if self.pending_rows >= crate::exec::sink::STEP_QUANTUM {
            self.pending_rows = 0;
            self.charge_to(self.total_bytes(out))?;
        }
        Ok(())
    }

    /// True up a resident collection before transferring the caller-owned
    /// answer buffer. Like collected copies from a `CompleteResult`, that
    /// buffer's lifetime is controlled by the Rust caller, not a result lease.
    pub(super) fn finish_resident(mut self, answers: &Answers) -> Result<()> {
        debug_assert!(self.spill.is_none(), "resident collection never spills");
        self.charge_to(self.total_bytes(answers))
    }

    /// Seal the constructed set: the final true-up charge plus the backing
    /// the construction already chose. Flush the final partial carrier
    /// before exposing any scratch-backed result.
    pub(super) fn seal(
        mut self,
        mut answers: Answers,
        identity: ResultIdentity,
    ) -> Result<CompleteResult> {
        if self.spill.is_some() && !answers.is_empty() {
            self.drain_to_scratch(&mut answers)?;
        }
        self.charge_to(self.total_bytes(&answers))?;
        let Self {
            charge,
            spill,
            ram_allowance,
            work,
            ..
        } = self;
        match spill {
            Some(spill) => {
                debug_assert!(answers.is_empty(), "sealed spill drains its final batch");
                Ok(CompleteResult {
                    identity,
                    backing: Backing::Scratch {
                        rows: spill.rows,
                        arity: answers.arity(),
                        count: spill.count,
                    },
                    charge,
                })
            }
            // Rows that bypassed the noted appends (the key-probe direct
            // fill) can still exceed the allowance: reuse the post-hoc
            // copy. `seal` re-charges, so hand it no reservations twice —
            // drop ours after it owns the set.
            None if logical_bytes(&answers) > ram_allowance as u64 => {
                drop(charge);
                CompleteResult::seal(answers, identity, work, ram_allowance)
            }
            None => Ok(CompleteResult {
                identity,
                backing: Backing::Ram(answers),
                charge,
            }),
        }
    }
}

/// One public pull's atomicity boundary (C8): `preview_page` / `adopt`
/// under admitted overlap, then `commit` cursor position only after the
/// native output owner is registered. Predelivery refusal aborts with no
/// advancement. `commit` applies **this ticket's** admitted advance only
/// — no second allocation, read, checkpoint, or fallible preview.
///
/// L13 commit/abort rules:
/// 1. `open` a ticket over the cursor.
/// 2. `preview_page` / `adopt` copy bounded rows; position does not advance.
/// 3. Register the complete native output owner (the delivery reservation).
/// 4. `commit` the **same live ticket** then advances `next_row` and may
///    set terminal. A fresh unpreviewed ticket has nothing to commit.
/// 5. `abort` always drops the preview, refunds charges, and discards
///    this ticket's pending advance — including after `adopt`. Retry
///    starts at the same row; no data is delivered. Pending advancement
///    is ticket-local: it does not survive on the cursor.
/// 6. Resource refusal / cancellation aborts the pull without advancing
///    or permanently poisoning the cursor. Later pulls retry the same
///    row once capacity is available.
/// 7. A following row that does not fit a nonempty page ends the page
///    successfully (not an error).
/// 8. An oversized first row refuses; the cursor is unchanged.
/// 9. Terminal scratch/storage/corruption stays **failed** via
///    [`Self::fail_backing`]; later pulls return that error, never EOF.
/// 10. No second public raw cursor.
pub struct DeliveryTicket<'cursor> {
    cursor: &'cursor mut ResultCursor,
    preview: Option<Answers>,
    preview_charge: Option<ByteReservation>,
    pending: Option<PendingAdvance>,
    committed: bool,
    closed: bool,
}

impl<'cursor> DeliveryTicket<'cursor> {
    /// Open a ticket over a completed result cursor.
    pub fn open(cursor: &'cursor mut ResultCursor) -> Self {
        Self {
            cursor,
            preview: None,
            preview_charge: None,
            pending: None,
            committed: false,
            closed: false,
        }
    }

    #[must_use]
    pub fn previewed_rows(&self) -> u64 {
        self.preview.as_ref().map_or(0, |rows| rows.len() as u64)
    }

    #[must_use]
    pub fn will_be_terminal(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| pending.terminal)
    }

    /// The result-byte reservation held with this preview. Aborts refund
    /// it; [`Self::take_preview_charge`] moves it onto the published page.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn preview_charged_bytes(&self) -> u64 {
        self.preview_charge
            .as_ref()
            .map_or(0, ByteReservation::bytes)
    }

    /// Copy the next admitted page without advancing the cursor.
    ///
    /// Fit is decided from the sealed representation before any preview
    /// growth: RAM cell/heap lengths, or mapped scratch values visited
    /// in one page-scoped read transaction. There is no per-row length
    /// directory or intermediate encoded copy. Admitted rows reserve
    /// result bytes, then decode from that same borrow. A later row that
    /// does not fit a nonempty page ends the page successfully. An oversized
    /// first row refuses with the cursor unchanged and no preview
    /// allocation. Resource refusal / cancellation aborts this pull
    /// without poisoning the cursor. Terminal backing corruption stays
    /// failed and yields no page. The row cap is [`ResultCursor`]'s
    /// `page_rows` from [`CompleteResult::into_cursor`], never a
    /// hardcoded 1.
    /// # Errors
    /// Delivery byte/work refusal or terminal backing failure.
    pub fn preview_page(
        &mut self,
        work: &WorkContext,
        byte_allowance: u64,
    ) -> Result<Option<&Answers>> {
        self.preview_page_with_cost(work, byte_allowance, |bytes| bytes)
    }

    /// Preview with a caller-supplied bound on total delivery bytes per row.
    /// The input is the encoded row length; adapters include their conversion
    /// and overlap costs here, before copying any row. The engine retains no
    /// knowledge of the host's object layout. Cost must be at least the input.
    /// # Errors
    /// As [`Self::preview_page`], or an invalid (underestimating) cost function.
    pub fn preview_page_with_cost(
        &mut self,
        work: &WorkContext,
        byte_allowance: u64,
        mut delivery_cost: impl FnMut(u64) -> u64,
    ) -> Result<Option<&Answers>> {
        if let Some(error) = &self.cursor.failed {
            return Err(error.clone());
        }
        if self.cursor.done {
            return Ok(None);
        }
        // A new attempt invalidates a previous preview even if admission
        // itself refuses. It must not leave an old advance to commit.
        self.discard_preview();
        self.pending = None;
        work.step(1).map_err(super::source::work_error)?;
        work.checkpoint().map_err(super::source::work_error)?;
        self.cursor.rebind_work(work);
        let mut rows = Answers::new();
        rows.begin(self.cursor.arity());
        let mut used = 0u64;
        let mut retained = 0u64;
        let start = self.cursor.next_row;
        let mut index = start;
        let total = self.cursor.len();
        let row_cap = self.cursor.page_rows as u64;
        let end = start.saturating_add(row_cap).min(total);
        let charge = &mut self.preview_charge;
        let copied = self.cursor.backing.visit_rows(start, end, |row| {
            let row_bytes = row.encoded_len();
            let delivery_bytes = delivery_cost(row_bytes);
            if delivery_bytes < row_bytes {
                return Err(Error::ResultBytesOverflow);
            }
            if used.saturating_add(delivery_bytes) > byte_allowance {
                if rows.is_empty() {
                    return Err(delivery_overflow(work, delivery_bytes, byte_allowance));
                }
                return Ok(false);
            }
            retained = retained
                .checked_add(row_bytes)
                .ok_or(Error::ResultBytesOverflow)?;
            reserve_result(charge, work, retained)?;
            row.copy_to(&mut rows)?;
            used = used.saturating_add(delivery_bytes);
            index += 1;
            Ok(true)
        });
        if let Err(error) = copied {
            return Err(self.surface_preview_error(error));
        }
        self.pending = Some(PendingAdvance {
            next_row: index,
            terminal: index >= total,
        });
        if rows.is_empty() && index >= total {
            self.preview = Some(rows);
            return Ok(self.preview.as_ref());
        }
        if rows.is_empty() {
            return Ok(None);
        }
        self.preview = Some(rows);
        Ok(self.preview.as_ref())
    }

    /// Take the previewed page. Position is still uncommitted. The
    /// preview reservation stays on the ticket until
    /// [`Self::take_preview_charge`] or abort.
    pub fn adopt(&mut self) -> Option<Answers> {
        self.preview.take()
    }

    /// Move the preview reservation onto the published page owner.
    pub(crate) fn take_preview_charge(&mut self) -> Option<ByteReservation> {
        self.preview_charge.take()
    }

    /// Commit after the complete native output owner is registered.
    /// Applies **this ticket's** admitted advance only — no allocation,
    /// read, or checkpoint. A ticket that never previewed is a no-op.
    pub fn commit(mut self) {
        if self.closed {
            return;
        }
        if let Some(pending) = self.pending.take() {
            self.cursor.next_row = pending.next_row;
            self.cursor.done = pending.terminal;
        }
        self.committed = true;
        self.preview = None;
    }

    /// Abort without advancing `next_row`. Drops any uncommitted preview,
    /// refunds its charge, and discards this ticket's pending advance
    /// even after `adopt`. A later fresh ticket cannot commit that page.
    pub fn abort(mut self) {
        self.discard_preview();
        self.pending = None;
    }

    fn discard_preview(&mut self) {
        self.preview = None;
        self.preview_charge = None;
    }

    /// Resource refusal / cancellation: drop this pull's preview and
    /// pending advance. The cursor stays live for a later retry.
    fn abort_pull(&mut self, error: Error) -> Error {
        self.pending = None;
        self.discard_preview();
        error
    }

    fn surface_preview_error(&mut self, error: Error) -> Error {
        if is_resource_refusal(&error) || matches!(error, Error::ResultBytesOverflow) {
            self.abort_pull(error)
        } else {
            self.fail_backing(error)
        }
    }

    /// Terminal backing / storage / corruption. Later pulls return this
    /// error, never EOF. Resource refusals must not take this path.
    fn fail_backing(&mut self, error: Error) -> Error {
        self.cursor.failed = Some(error.clone());
        self.closed = true;
        self.pending = None;
        self.discard_preview();
        error
    }
}

impl Drop for DeliveryTicket<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.discard_preview();
            self.pending = None;
        }
    }
}

/// One page of delivered rows plus the terminal frame: `terminal` is true
/// exactly on the page that completes the set (possibly empty). A cursor
/// that failed mid-delivery never returns a terminal page — the delivered
/// prefix is explicitly incomplete, not the complete set. `charge` is
/// the preview reservation when the page was built under a delivery
/// work context.
pub struct ResultPage {
    pub rows: Answers,
    pub terminal: bool,
    charge: Option<ByteReservation>,
}

impl ResultPage {
    /// Preview reservation held with this page (zero when the page was
    /// not built under a delivery work context).
    #[must_use]
    pub fn charged_bytes(&self) -> u64 {
        self.charge.as_ref().map_or(0, ByteReservation::bytes)
    }
}

struct PendingAdvance {
    next_row: u64,
    terminal: bool,
}

/// The one consuming cursor over a spent [`CompleteResult`].
pub struct ResultCursor {
    identity: ResultIdentity,
    backing: Backing,
    charge: Option<ByteReservation>,
    page_rows: usize,
    next_row: u64,
    done: bool,
    failed: Option<Error>,
}

impl ResultCursor {
    #[must_use]
    pub fn identity(&self) -> ResultIdentity {
        self.identity
    }

    #[must_use]
    pub fn len(&self) -> u64 {
        match &self.backing {
            Backing::Ram(answers) => answers.len() as u64,
            Backing::Scratch { count, .. } => *count,
        }
    }

    #[must_use]
    pub fn arity(&self) -> usize {
        match &self.backing {
            Backing::Ram(answers) => answers.arity(),
            Backing::Scratch { arity, .. } => *arity,
        }
    }

    #[cfg(test)]
    pub(crate) fn debug_next_row(&self) -> u64 {
        self.next_row
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The transferred rows' byte charge, held until the cursor drops (the
    /// retained-byte accounting twin of [`CompleteResult::byte_len`]).
    #[must_use]
    pub fn byte_len(&self) -> u64 {
        self.charge.as_ref().map_or(0, ByteReservation::bytes)
    }

    /// As [`CompleteResult::rebind_work`]: re-home a retained cursor's
    /// scratch page reads onto `work`, so paging a result kept past its
    /// execute operation never charges that operation's expired ledger.
    /// Existing reservations keep their originating ledgers (released
    /// exactly once, on drop); RAM backings are a no-op.
    pub fn rebind_work(&mut self, work: &WorkContext) {
        if let Backing::Scratch { rows, .. } = &mut self.backing {
            rows.rebind_work(work);
        }
    }

    /// The next chunk under a fresh delivery operation policy.
    /// # Errors
    /// Delivery byte/work refusal; scratch read failure.
    pub fn next_page_with_work(
        &mut self,
        work: &WorkContext,
        byte_allowance: u64,
    ) -> Result<Option<ResultPage>> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        let mut ticket = DeliveryTicket::open(self);
        let Some(_) = ticket.preview_page(work, byte_allowance)? else {
            ticket.commit();
            return Ok(None);
        };
        let rows = ticket.adopt().ok_or_else(missing_row)?;
        let charge = ticket.take_preview_charge();
        let terminal = ticket.will_be_terminal();
        ticket.commit();
        Ok(Some(ResultPage {
            rows,
            terminal,
            charge,
        }))
    }

    #[cfg(test)]
    pub(crate) fn inject_backing_failure(&mut self, error: Error) {
        self.failed = Some(error);
    }
}

// ---- the sealed row codec (scratch backing only) ------------------------
//
// Tags mirror the canonical row codec's shapes but store answer cells
// (text inline; interval endpoints as their host values). Internal to the
// backing — never a wire format.

fn missing_row() -> Error {
    Error::Corruption(crate::error::CorruptionError::MalformedValue(
        "result row sequence",
    ))
}

/// Work-ledger refusal or cancellation — abort the pull, do not poison
/// the cursor. Store I/O and corruption stay sticky via `fail_backing`.
fn is_resource_refusal(error: &Error) -> bool {
    matches!(
        error,
        Error::Store(store)
            if matches!(
                **store,
                crate::storage::store::StoreError::Work(_)
            )
    )
}

/// Encoded length of one cell — the delivery-fit basis — without writing
/// an uncharged buffer.
fn encoded_value_len(value: &AnswerValue<'_>) -> u64 {
    match value {
        AnswerValue::Bool(_) => 2,
        AnswerValue::U64(_) | AnswerValue::I64(_) | AnswerValue::F64(_) => 9,
        AnswerValue::String(text) => 9 + text.len() as u64,
        AnswerValue::FixedBytes(bytes) => 9 + bytes.len() as u64,
        AnswerValue::IntervalU64(_)
        | AnswerValue::IntervalI64(_)
        | AnswerValue::IntervalF64(_)
        | AnswerValue::Uuid(_) => 17,
    }
}

fn encode_value(
    value: &AnswerValue<'_>,
    out: &mut ChargedBuffer,
) -> std::result::Result<(), WorkError> {
    match value {
        AnswerValue::Bool(v) => {
            out.try_extend_from_slice(&[0, u8::from(*v)])?;
        }
        AnswerValue::U64(v) => {
            out.try_extend_from_slice(&[1])?;
            out.try_extend_from_slice(&v.to_be_bytes())?;
        }
        AnswerValue::I64(v) => {
            out.try_extend_from_slice(&[2])?;
            out.try_extend_from_slice(&v.to_be_bytes())?;
        }
        AnswerValue::F64(v) => {
            out.try_extend_from_slice(&[3])?;
            out.try_extend_from_slice(&v.to_be_bytes())?;
        }
        AnswerValue::String(text) => {
            out.try_extend_from_slice(&[4])?;
            out.try_extend_from_slice(&(text.len() as u64).to_be_bytes())?;
            out.try_extend_from_slice(text.as_bytes())?;
        }
        AnswerValue::FixedBytes(bytes) => {
            out.try_extend_from_slice(&[5])?;
            out.try_extend_from_slice(&(bytes.len() as u64).to_be_bytes())?;
            out.try_extend_from_slice(bytes)?;
        }
        AnswerValue::IntervalU64(interval) => {
            out.try_extend_from_slice(&[6])?;
            out.try_extend_from_slice(&interval.start().to_be_bytes())?;
            out.try_extend_from_slice(&interval.end().to_be_bytes())?;
        }
        AnswerValue::IntervalI64(interval) => {
            out.try_extend_from_slice(&[7])?;
            out.try_extend_from_slice(&interval.start().to_be_bytes())?;
            out.try_extend_from_slice(&interval.end().to_be_bytes())?;
        }
        AnswerValue::Uuid(id) => {
            out.try_extend_from_slice(&[8])?;
            out.try_extend_from_slice(id.as_bytes())?;
        }
        AnswerValue::IntervalF64(interval) => {
            out.try_extend_from_slice(&[9])?;
            out.try_extend_from_slice(&interval.start().to_be_bytes())?;
            out.try_extend_from_slice(&interval.end().to_be_bytes())?;
        }
    }
    Ok(())
}

fn decode_row(mut bytes: &[u8], arity: usize, out: &mut Answers) -> Result<()> {
    let malformed = || {
        Error::Corruption(crate::error::CorruptionError::MalformedValue(
            "sealed result row",
        ))
    };
    let mut take = |len: usize| -> Result<&[u8]> {
        let (head, rest) = bytes.split_at_checked(len).ok_or_else(malformed)?;
        bytes = rest;
        Ok(head)
    };
    for _ in 0..arity {
        let tag = take(1)?[0];
        match tag {
            0 => {
                let v = take(1)?[0];
                out.push_value(&AnswerValue::Bool(v != 0));
            }
            1 => {
                let v = u64::from_be_bytes(take(8)?.try_into().expect("eight"));
                out.push_value(&AnswerValue::U64(v));
            }
            2 => {
                let v = i64::from_be_bytes(take(8)?.try_into().expect("eight"));
                out.push_value(&AnswerValue::I64(v));
            }
            3 => {
                let word: [u8; 8] = take(8)?.try_into().expect("eight");
                let v =
                    bumbledb_theory::F64::from_canonical_be_bytes(word).map_err(|_| malformed())?;
                out.push_value(&AnswerValue::F64(v));
            }
            4 => {
                let len = usize::try_from(u64::from_be_bytes(take(8)?.try_into().expect("eight")))
                    .map_err(|_| malformed())?;
                let text = std::str::from_utf8(take(len)?).map_err(|_| malformed())?;
                out.push_value(&AnswerValue::String(text));
            }
            5 => {
                let len = usize::try_from(u64::from_be_bytes(take(8)?.try_into().expect("eight")))
                    .map_err(|_| malformed())?;
                let raw = take(len)?;
                out.push_value(&AnswerValue::FixedBytes(raw));
            }
            6 => {
                let start = u64::from_be_bytes(take(8)?.try_into().expect("eight"));
                let end = u64::from_be_bytes(take(8)?.try_into().expect("eight"));
                let interval = bumbledb_theory::Interval::new(start, end).ok_or_else(malformed)?;
                out.push_value(&AnswerValue::IntervalU64(interval));
            }
            7 => {
                let start = i64::from_be_bytes(take(8)?.try_into().expect("eight"));
                let end = i64::from_be_bytes(take(8)?.try_into().expect("eight"));
                let interval = bumbledb_theory::Interval::new(start, end).ok_or_else(malformed)?;
                out.push_value(&AnswerValue::IntervalI64(interval));
            }
            8 => {
                let raw: [u8; 16] = take(16)?.try_into().expect("sixteen");
                out.push_value(&AnswerValue::Uuid(bumbledb_theory::Uuid::from_bytes(raw)));
            }
            9 => {
                let start: [u8; 8] = take(8)?.try_into().expect("eight");
                let end: [u8; 8] = take(8)?.try_into().expect("eight");
                let start = bumbledb_theory::F64::from_canonical_be_bytes(start)
                    .map_err(|_| malformed())?;
                let end =
                    bumbledb_theory::F64::from_canonical_be_bytes(end).map_err(|_| malformed())?;
                let interval = bumbledb_theory::Interval::new(start, end).ok_or_else(malformed)?;
                out.push_value(&AnswerValue::IntervalF64(interval));
            }
            _ => return Err(malformed()),
        }
    }
    if !bytes.is_empty() {
        return Err(malformed());
    }
    Ok(())
}

impl<S> super::PreparedQuery<S> {
    /// Execute to one sealed [`CompleteResult`]: the atomic-answer entry
    /// (QRY-001) — either the complete evaluated set seals, or the typed
    /// error is the only outcome. The result owns RAM or scratch backing
    /// and is charged to the caller's `result_bytes`.
    /// # Errors
    /// As `execute`, plus result-capacity refusal.
    ///
    /// `doc(hidden)` bridge seam: the native runtime (P06's db bridge) is
    /// the intended caller; embedders use `execute`/`execute_collect`.
    #[doc(hidden)]
    pub fn execute_complete<'p, P: super::BindArgs<'p>>(
        &mut self,
        instance: &crate::api::db::ReadInstance<'_, S>,
        params: P,
    ) -> Result<CompleteResult> {
        self.execute_complete_with_work(instance, instance.work(), params)
    }

    /// As [`Self::execute_complete`], under the CALLER's work context
    /// instead of the lease's embedded one — the native runtime threads
    /// each wire operation's bounded `WorkContext` (deadline, cancellation
    /// and byte/step budgets) through here so the executor's in-join polls
    /// and the streamed result-construction charge observe the operation's
    /// own policy, not the long-lived session lease's unbounded ledger.
    /// # Errors
    /// As `execute_complete`.
    ///
    /// `doc(hidden)` bridge seam (P06/W3-SESSION); embedders use
    /// `execute`/`execute_collect`.
    #[doc(hidden)]
    pub fn execute_complete_with_work<'p, P: super::BindArgs<'p>>(
        &mut self,
        instance: &crate::api::db::ReadInstance<'_, S>,
        work: &WorkContext,
        params: P,
    ) -> Result<CompleteResult> {
        let mut answers = Answers::new();
        // Result bytes charge DURING construction (bounded quanta) and
        // past-allowance rows stream into scratch in bounded batches —
        // a tiny result budget refuses before the whole set materializes,
        // and a failure here seals nothing (Q-ATOMIC).
        let mut charge = ResultCharge::new(work, RESULT_RAM_BYTES);
        let source = super::source::QuerySource::store(instance.snapshot(), work);
        self.execute_source_charged(&source, params, &mut answers, Some(&mut charge))?;
        let identity = ResultIdentity {
            source: PinnedSource::Store(instance.snapshot().identity()),
            generation: Some(instance.snapshot().generation()),
        };
        charge.seal(answers, identity)
    }
}

#[cfg(test)]
mod tests;
