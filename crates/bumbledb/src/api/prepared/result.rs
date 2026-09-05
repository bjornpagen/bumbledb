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
use crate::exec::scratch::ScratchRelation;
use crate::storage::GenerationId;
use crate::work::{ByteKind, ByteReservation, WorkContext};

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
    /// The sealed rows' byte charges, held until disposal (one post-hoc
    /// reservation on the compatibility [`Self::seal`] path; the streamed
    /// construction path accumulates bounded-quantum reservations).
    charges: Vec<ByteReservation>,
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
                charges: vec![charge],
            });
        }
        // Beyond the allowance: the sealed backing is the scratch map
        // (already-complete rows copy over in bounded batches; the RAM
        // owner drops after the copy finishes — reserve-before-growth is
        // the scratch relation's own contract).
        let mut rows = ScratchRelation::new(work, 0);
        rows.force_spill()?;
        let arity = answers.arity();
        let mut encoded = Vec::new();
        for (index, answer) in answers.answers().enumerate() {
            encoded.clear();
            for column in 0..arity {
                encode_value(&answer.get(column), &mut encoded);
            }
            rows.put(&(index as u64).to_be_bytes(), &encoded)?;
        }
        let count = answers.len() as u64;
        drop(answers);
        Ok(Self {
            identity,
            backing: Backing::Scratch { rows, arity, count },
            charges: vec![charge],
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
        self.charges.iter().map(ByteReservation::bytes).sum()
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
        let estimated = if self.len() > 0 {
            self.byte_len()
                .saturating_mul(limit.min(self.len()))
                .div_ceil(self.len())
        } else {
            0
        };
        if estimated > byte_allowance {
            return Err(super::source::work_error(crate::work::WorkError::Exhausted {
                resource: crate::work::Resource::ResultBytes,
                used: work.used(crate::work::Resource::ResultBytes),
                requested: estimated,
                limit: byte_allowance,
            }));
        }
        let mut out = Answers::new();
        out.begin(self.arity());
        let mut used = 0u64;
        let mut encoded = Vec::new();
        for index in 0..self.len() {
            encoded.clear();
            self.encode_row(index, &mut encoded)?;
            let row_bytes = encoded.len() as u64;
            if used.saturating_add(row_bytes) > byte_allowance {
                return Err(super::source::work_error(crate::work::WorkError::Exhausted {
                    resource: crate::work::Resource::ResultBytes,
                    used: work.used(crate::work::Resource::ResultBytes),
                    requested: used.saturating_add(row_bytes),
                    limit: byte_allowance,
                }));
            }
            self.push_row(index, &mut out)?;
            used = used.saturating_add(row_bytes);
        }
        let _delivery = work
            .reserve(ByteKind::Result, used)
            .map_err(super::source::work_error)?;
        Ok(out)
    }

    /// Convert into one fully owned collection, or fail — a cap refusal
    /// leaves this sealed backing untouched and available.
    /// # Errors
    /// `ResultBytesOverflow` when the row count exceeds `limit`; scratch
    /// read failure.
    pub(crate) fn collect(&mut self, limit: u64) -> Result<Answers> {
        if self.len() > limit {
            return Err(Error::ResultBytesOverflow);
        }
        let mut out = Answers::new();
        out.begin(self.arity());
        for index in 0..self.len() {
            self.push_row(index, &mut out)?;
        }
        Ok(out)
    }

    fn encode_row(&mut self, index: u64, encoded: &mut Vec<u8>) -> Result<()> {
        encoded.clear();
        match &mut self.backing {
            Backing::Ram(answers) => {
                let row = usize::try_from(index).expect("64-bit usize");
                if row >= answers.len() {
                    return Err(missing_row());
                }
                for column in 0..answers.arity() {
                    encode_value(&answers.get(row, column), encoded);
                }
                Ok(())
            }
            Backing::Scratch { rows, .. } => {
                if !rows.get(&index.to_be_bytes(), encoded)? {
                    return Err(missing_row());
                }
                Ok(())
            }
        }
    }

    fn push_row(&mut self, index: u64, out: &mut Answers) -> Result<()> {
        match &mut self.backing {
            Backing::Ram(answers) => ram_push_row(answers, index, out),
            Backing::Scratch { rows, arity, .. } => {
                let arity = *arity;
                let mut value = Vec::new();
                if !rows.get(&index.to_be_bytes(), &mut value)? {
                    return Err(missing_row());
                }
                decode_row(&value, arity, out)
            }
        }
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
            charges: self.charges,
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
    reservations: Vec<ByteReservation>,
    /// Rows appended since the last charge (bounded by the quantum).
    pending_rows: u32,
    /// Encoded bytes moved into the scratch tier so far.
    spilled_bytes: u64,
    spill: Option<ResultSpill>,
}

struct ResultSpill {
    rows: ScratchRelation,
    count: u64,
    encoded: Vec<u8>,
}

impl<'w> ResultCharge<'w> {
    pub(super) fn new(work: &'w WorkContext, ram_allowance: usize) -> Self {
        Self {
            work,
            ram_allowance,
            charged: 0,
            reservations: Vec::new(),
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
            let reservation = self
                .work
                .reserve(ByteKind::Result, target - self.charged)
                .map_err(super::source::work_error)?;
            self.reservations.push(reservation);
            self.charged = target;
        }
        Ok(())
    }

    /// Move every row currently in `out` into the scratch tier and clear
    /// the carrier (the memo's text ranges point into the cleared heap, so
    /// it resets with it).
    fn drain_to_scratch(&mut self, out: &mut Answers, memo: &mut ResolveMemo) -> Result<()> {
        let spill = self.spill.as_mut().expect("drain under an open spill");
        let arity = out.arity();
        for answer in out.answers() {
            spill.encoded.clear();
            for column in 0..arity {
                encode_value(&answer.get(column), &mut spill.encoded);
            }
            spill.rows.put(&spill.count.to_be_bytes(), &spill.encoded)?;
            spill.count += 1;
            self.spilled_bytes += spill.encoded.len() as u64;
        }
        out.clear();
        memo.clear();
        Ok(())
    }

    /// Note one complete appended row: charge at the bounded quantum and
    /// route past-allowance growth into the scratch backing now, not at
    /// seal.
    pub(super) fn note_row(&mut self, out: &mut Answers, memo: &mut ResolveMemo) -> Result<()> {
        if out.arity() == 0 {
            return Ok(());
        }
        if self.spill.is_some() {
            // Streaming regime: the carrier holds exactly the one row just
            // appended; move it over (its encoded bytes are the charge
            // basis from here on).
            self.drain_to_scratch(out, memo)?;
        } else if logical_bytes(out) > self.ram_allowance as u64 {
            // Crossing the allowance: true-up the charge BEFORE the copy,
            // then continue in the scratch backing during construction.
            self.charge_to(self.total_bytes(out))?;
            let mut rows = ScratchRelation::new(self.work, 0);
            rows.force_spill()?;
            self.spill = Some(ResultSpill {
                rows,
                count: 0,
                encoded: Vec::new(),
            });
            self.drain_to_scratch(out, memo)?;
        }
        self.pending_rows += 1;
        if self.pending_rows >= crate::exec::sink::STEP_QUANTUM {
            self.pending_rows = 0;
            self.charge_to(self.total_bytes(out))?;
        }
        Ok(())
    }

    /// Seal the constructed set: the final true-up charge plus the backing
    /// the construction already chose. `answers` holds the complete RAM
    /// set when nothing spilled, and is empty otherwise.
    pub(super) fn seal(mut self, answers: Answers, identity: ResultIdentity) -> Result<CompleteResult> {
        self.charge_to(self.total_bytes(&answers))?;
        let Self {
            reservations,
            spill,
            ram_allowance,
            work,
            ..
        } = self;
        match spill {
            Some(spill) => {
                debug_assert!(
                    answers.is_empty(),
                    "a spilled construction drains every row as it lands"
                );
                Ok(CompleteResult {
                    identity,
                    backing: Backing::Scratch {
                        rows: spill.rows,
                        arity: answers.arity(),
                        count: spill.count,
                    },
                    charges: reservations,
                })
            }
            // Rows that bypassed the noted appends (the key-probe direct
            // fill) can still exceed the allowance: reuse the post-hoc
            // copy. `seal` re-charges, so hand it no reservations twice —
            // drop ours after it owns the set.
            None if logical_bytes(&answers) > ram_allowance as u64 => {
                drop(reservations);
                CompleteResult::seal(answers, identity, work, ram_allowance)
            }
            None => Ok(CompleteResult {
                identity,
                backing: Backing::Ram(answers),
                charges: reservations,
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
    scratch: Vec<u8>,
    scratch_charge: Option<ByteReservation>,
    scratch_charged: usize,
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
            scratch: Vec::new(),
            scratch_charge: None,
            scratch_charged: 0,
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
    /// growth: RAM cell/heap lengths, or a scoped borrowed scratch
    /// lookup (`ScratchRelation::visit_from`) for one row's encoded
    /// size. There is no resident per-row length directory. Admitted
    /// rows reserve result bytes, then copy. A later row that does not
    /// fit a nonempty page ends the page successfully. An oversized
    /// first row refuses with the cursor unchanged and no preview
    /// allocation. Resource refusal / cancellation aborts this pull
    /// without poisoning the cursor. Terminal backing corruption stays
    /// failed and yields no page. The row cap is [`ResultCursor`]'s
    /// `page_rows` from [`CompleteResult::into_cursor`], never a
    /// hardcoded 1.
    /// # Errors
    /// Delivery byte/work refusal or terminal backing failure.
    pub fn preview_page(&mut self, work: &WorkContext, byte_allowance: u64) -> Result<Option<&Answers>> {
        if let Some(error) = &self.cursor.failed {
            return Err(error.clone());
        }
        if self.cursor.done {
            return Ok(None);
        }
        work.step(1).map_err(super::source::work_error)?;
        work.checkpoint().map_err(super::source::work_error)?;
        self.preview = None;
        self.preview_charge = None;
        self.pending = None;
        let mut rows = Answers::new();
        rows.begin(self.cursor.arity());
        let mut used = 0u64;
        let start = self.cursor.next_row;
        let mut index = start;
        let total = self.cursor.len();
        let row_cap = self.cursor.page_rows as u64;
        let scratch_backing = self.cursor.backing_is_scratch();
        while index < total && (index - start) < row_cap {
            let row_bytes = match self.cursor.row_fit_bytes(index) {
                Ok(bytes) => bytes,
                Err(error) => return Err(self.surface_preview_error(error)),
            };
            if used.saturating_add(row_bytes) > byte_allowance {
                if rows.is_empty() {
                    return Err(self.abort_pull(super::source::work_error(
                        crate::work::WorkError::Exhausted {
                            resource: crate::work::Resource::ResultBytes,
                            used: work.used(crate::work::Resource::ResultBytes),
                            requested: row_bytes,
                            limit: byte_allowance,
                        },
                    )));
                }
                break;
            }
            if let Err(error) = self.charge_row(work, row_bytes) {
                return Err(self.abort_pull(error));
            }
            if scratch_backing {
                if let Err(error) = self.ensure_scratch(work, row_bytes) {
                    return Err(self.abort_pull(error));
                }
                self.scratch.clear();
                if let Err(error) = self.cursor.load_scratch_row(index, &mut self.scratch) {
                    return Err(self.surface_preview_error(error));
                }
            }
            if let Err(error) = self.cursor.copy_fit_row(index, &self.scratch, &mut rows) {
                return Err(self.surface_preview_error(error));
            }
            used = used.saturating_add(row_bytes);
            index += 1;
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

    fn charge_row(&mut self, work: &WorkContext, row_bytes: u64) -> Result<()> {
        if row_bytes == 0 {
            return Ok(());
        }
        let reservation = work
            .reserve(ByteKind::Result, row_bytes)
            .map_err(super::source::work_error)?;
        match &mut self.preview_charge {
            Some(owned) => owned.join(reservation),
            None => self.preview_charge = Some(reservation),
        }
        Ok(())
    }

    fn ensure_scratch(&mut self, work: &WorkContext, row_bytes: u64) -> Result<()> {
        let need = usize::try_from(row_bytes).unwrap_or(usize::MAX);
        if need <= self.scratch_charged {
            return Ok(());
        }
        let extra = (need - self.scratch_charged) as u64;
        let reservation = work
            .reserve(ByteKind::Result, extra)
            .map_err(super::source::work_error)?;
        let additional = need.saturating_sub(self.scratch.len());
        if additional > 0 && self.scratch.try_reserve(additional).is_err() {
            drop(reservation);
            return Err(super::source::work_error(crate::work::WorkError::Exhausted {
                resource: crate::work::Resource::ResultBytes,
                used: work.used(crate::work::Resource::ResultBytes),
                requested: extra,
                limit: work.limit(crate::work::Resource::ResultBytes),
            }));
        }
        match &mut self.scratch_charge {
            Some(owned) => owned.join(reservation),
            None => self.scratch_charge = Some(reservation),
        }
        self.scratch_charged = need;
        Ok(())
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
        if is_resource_refusal(&error) {
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
    charges: Vec<ByteReservation>,
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

    fn backing_is_scratch(&self) -> bool {
        matches!(self.backing, Backing::Scratch { .. })
    }

    /// Encoded delivery size of `index`. RAM reads cell/heap lengths;
    /// scratch measures one borrowed value via
    /// [`ScratchRelation::visit_from`] — neither path serializes an
    /// oversized row into an uncharged buffer, and scratch keeps no
    /// resident per-row length directory.
    fn row_fit_bytes(&mut self, index: u64) -> Result<u64> {
        match &mut self.backing {
            Backing::Ram(answers) => ram_encoded_row_len(answers, index),
            Backing::Scratch { rows, count, .. } => {
                if index >= *count {
                    return Err(missing_row());
                }
                scratch_row_encoded_len(rows, index)
            }
        }
    }

    fn load_scratch_row(&mut self, index: u64, out: &mut Vec<u8>) -> Result<()> {
        match &mut self.backing {
            Backing::Scratch { rows, .. } => {
                if !rows.get(&index.to_be_bytes(), out)? {
                    return Err(missing_row());
                }
                Ok(())
            }
            Backing::Ram(_) => Ok(()),
        }
    }

    /// Copy a row already admitted by [`Self::row_fit_bytes`]. Scratch
    /// reuses that load; RAM copies from the sealed cells.
    fn copy_fit_row(&mut self, index: u64, scratch: &[u8], out: &mut Answers) -> Result<()> {
        match &mut self.backing {
            Backing::Ram(answers) => ram_push_row(answers, index, out),
            Backing::Scratch { arity, .. } => decode_row(scratch, *arity, out),
        }
    }

    /// The transferred rows' byte charge, held until the cursor drops (the
    /// retained-byte accounting twin of [`CompleteResult::byte_len`]).
    #[must_use]
    pub fn byte_len(&self) -> u64 {
        self.charges.iter().map(ByteReservation::bytes).sum()
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
        let rows = ticket.adopt().expect("previewed page");
        let charge = ticket.take_preview_charge();
        let terminal = ticket.will_be_terminal();
        ticket.commit();
        Ok(Some(ResultPage {
            rows,
            terminal,
            charge,
        }))
    }

    /// The next chunk. `Ok(None)` after the terminal page was delivered.
    /// # Panics
    /// Only on programmer-invariant violations (a corrupt in-memory page
    /// shape); never on caller input.
    /// # Errors
    /// Scratch read failure — delivery stops without a terminal frame; the
    /// already-delivered prefix must not be mistaken for the complete set.
    pub(crate) fn next_page(&mut self) -> Result<Option<ResultPage>> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        if self.done {
            return Ok(None);
        }
        match self.build_page(self.next_row) {
            Ok((page, next_row, done)) => {
                self.next_row = next_row;
                self.done = done;
                Ok(page)
            }
            Err(error) => {
                self.failed = Some(error.clone());
                Err(error)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn inject_backing_failure(&mut self, error: Error) {
        self.failed = Some(error);
    }

    fn build_page(&mut self, start: u64) -> Result<(Option<ResultPage>, u64, bool)> {
        let take = self.page_rows as u64;
        let mut rows = Answers::new();
        rows.begin(match &self.backing {
            Backing::Ram(answers) => answers.arity(),
            Backing::Scratch { arity, .. } => *arity,
        });
        let total = match &mut self.backing {
            Backing::Ram(answers) => {
                let arity = answers.arity();
                let end = (start + take).min(answers.len() as u64);
                let skip = usize::try_from(start).expect("64-bit usize");
                let take_rows = usize::try_from(end - start).expect("64-bit usize");
                for answer in answers.answers().skip(skip).take(take_rows) {
                    for column in 0..arity {
                        rows.push_value(&answer.get(column));
                    }
                }
                answers.len() as u64
            }
            Backing::Scratch {
                rows: stored,
                arity,
                count,
                ..
            } => {
                let arity = *arity;
                let end = (start + take).min(*count);
                let mut value = Vec::new();
                for row in start..end {
                    if !stored.get(&row.to_be_bytes(), &mut value)? {
                        return Err(Error::Corruption(
                            crate::error::CorruptionError::MalformedValue("result row sequence"),
                        ));
                    }
                    decode_row(&value, arity, &mut rows)?;
                }
                *count
            }
        };
        let next_row = (start + take).min(total);
        let terminal = next_row >= total;
        Ok((
            Some(ResultPage {
                rows,
                terminal,
                charge: None,
            }),
            next_row,
            terminal,
        ))
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

/// Borrowed scratch lookup of one sealed row's encoded length. Stops
/// after the first key ≥ `index`; does not retain a length table.
fn scratch_row_encoded_len(rows: &mut ScratchRelation, index: u64) -> Result<u64> {
    let key = index.to_be_bytes();
    let mut found = None;
    rows.visit_from(&key, &mut |k, value| {
        if k == key.as_slice() {
            found = Some(value.len() as u64);
        }
        Ok(false)
    })?;
    found.ok_or_else(missing_row)
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
        | AnswerValue::Id128(_) => 17,
    }
}

fn ram_encoded_row_len(answers: &Answers, index: u64) -> Result<u64> {
    let row = usize::try_from(index).expect("64-bit usize");
    if row >= answers.len() {
        return Err(missing_row());
    }
    let mut bytes = 0u64;
    for column in 0..answers.arity() {
        bytes = bytes.saturating_add(encoded_value_len(&answers.get(row, column)));
    }
    Ok(bytes)
}

fn ram_push_row(answers: &Answers, index: u64, out: &mut Answers) -> Result<()> {
    let row = usize::try_from(index).expect("64-bit usize");
    if row >= answers.len() {
        return Err(missing_row());
    }
    for column in 0..answers.arity() {
        out.push_value(&answers.get(row, column));
    }
    Ok(())
}

fn encode_value(value: &AnswerValue<'_>, out: &mut Vec<u8>) {
    match value {
        AnswerValue::Bool(v) => {
            out.push(0);
            out.push(u8::from(*v));
        }
        AnswerValue::U64(v) => {
            out.push(1);
            out.extend_from_slice(&v.to_be_bytes());
        }
        AnswerValue::I64(v) => {
            out.push(2);
            out.extend_from_slice(&v.to_be_bytes());
        }
        AnswerValue::F64(v) => {
            out.push(3);
            out.extend_from_slice(&v.to_be_bytes());
        }
        AnswerValue::String(text) => {
            out.push(4);
            out.extend_from_slice(&(text.len() as u64).to_be_bytes());
            out.extend_from_slice(text.as_bytes());
        }
        AnswerValue::FixedBytes(bytes) => {
            out.push(5);
            out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            out.extend_from_slice(bytes);
        }
        AnswerValue::IntervalU64(interval) => {
            out.push(6);
            out.extend_from_slice(&interval.start().to_be_bytes());
            out.extend_from_slice(&interval.end().to_be_bytes());
        }
        AnswerValue::IntervalI64(interval) => {
            out.push(7);
            out.extend_from_slice(&interval.start().to_be_bytes());
            out.extend_from_slice(&interval.end().to_be_bytes());
        }
        AnswerValue::Id128(id) => {
            out.push(8);
            out.extend_from_slice(id.as_bytes());
        }
        AnswerValue::IntervalF64(interval) => {
            out.push(9);
            out.extend_from_slice(&interval.start().to_be_bytes());
            out.extend_from_slice(&interval.end().to_be_bytes());
        }
    }
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
                out.push_value(&AnswerValue::Id128(bumbledb_theory::Id128::from_bytes(raw)));
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
        // past-allowance rows stream into the scratch backing as they
        // land — a tiny result budget refuses before the whole set
        // materializes, and a failure here seals nothing (Q-ATOMIC).
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

impl<S> crate::api::db::ReadFrame<'_, S> {
    /// Seal a complete result on this frame's fresh work (C4/C8).
    /// # Errors
    /// As [`super::PreparedQuery::execute_complete_with_work`].
    #[doc(hidden)]
    pub fn execute_complete<'p, P: super::BindArgs<'p>>(
        &self,
        prepared: &mut super::PreparedQuery<S>,
        params: P,
    ) -> Result<CompleteResult> {
        prepared.execute_complete_with_work(self, self.work(), params)
    }
}

#[cfg(test)]
mod tests;
