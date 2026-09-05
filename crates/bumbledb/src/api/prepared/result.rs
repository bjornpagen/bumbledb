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
use super::{AnswerValue, Answers};
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
    /// The sealed rows' byte charge, held until disposal.
    charge: Option<ByteReservation>,
}

impl CompleteResult {
    /// Seal a finalized answer set: charge its bytes as result capacity
    /// and move it beyond the RAM allowance into scratch. Only complete,
    /// finalized rows arrive here (the execution failed before this point
    /// otherwise).
    pub(crate) fn seal(
        answers: Answers,
        identity: ResultIdentity,
        work: &WorkContext,
        ram_allowance: usize,
    ) -> Result<Self> {
        let bytes = answers.byte_len() as u64;
        let charge = work
            .reserve(ByteKind::Result, bytes)
            .map_err(super::source::work_error)?;
        if answers.byte_len() <= ram_allowance {
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

    /// Convert into one fully owned collection, or fail — a cap refusal
    /// leaves this sealed backing untouched and available.
    /// # Errors
    /// `ResultBytesOverflow` when the row count exceeds `limit`; scratch
    /// read failure.
    pub fn collect(&mut self, limit: u64) -> Result<Answers> {
        if self.len() > limit {
            return Err(Error::ResultBytesOverflow);
        }
        match &mut self.backing {
            Backing::Ram(answers) => {
                // The sealed backing stays: copy the rows out.
                let mut out = Answers::new();
                out.begin(answers.arity());
                for answer in answers.answers() {
                    for column in 0..answers.arity() {
                        out.push_value(&answer.get(column));
                    }
                }
                Ok(out)
            }
            Backing::Scratch { rows, arity, .. } => {
                let arity = *arity;
                let mut out = Answers::new();
                out.begin(arity);
                let mut failure = None;
                rows.for_each(
                    &mut |_, encoded| match decode_row(encoded, arity, &mut out) {
                        Ok(()) => Ok(true),
                        Err(error) => {
                            failure = Some(error);
                            Ok(false)
                        }
                    },
                )?;
                match failure {
                    Some(error) => Err(error),
                    None => Ok(out),
                }
            }
        }
    }

    /// Consume this result: the sealed backing transfers to one explicitly
    /// chunked cursor. The result handle is spent; abandoning the cursor
    /// closes its own storage after active access drains.
    #[must_use]
    pub fn into_cursor(self, page_rows: usize) -> ResultCursor {
        ResultCursor {
            identity: self.identity,
            backing: self.backing,
            charge: self.charge,
            page_rows: page_rows.max(1),
            next_row: 0,
            done: false,
        }
    }
}

/// One page of delivered rows plus the terminal frame: `terminal` is true
/// exactly on the page that completes the set (possibly empty). A cursor
/// that failed mid-delivery never returns a terminal page — the delivered
/// prefix is explicitly incomplete, not the complete set.
pub struct ResultPage {
    pub rows: Answers,
    pub terminal: bool,
}

/// The one consuming cursor over a spent [`CompleteResult`].
pub struct ResultCursor {
    identity: ResultIdentity,
    backing: Backing,
    charge: Option<ByteReservation>,
    page_rows: usize,
    next_row: u64,
    done: bool,
}

impl ResultCursor {
    #[must_use]
    pub fn identity(&self) -> ResultIdentity {
        self.identity
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

    /// The next chunk. `Ok(None)` after the terminal page was delivered.
    /// # Panics
    /// Only on programmer-invariant violations (a corrupt in-memory page
    /// shape); never on caller input.
    /// # Errors
    /// Scratch read failure — delivery stops without a terminal frame; the
    /// already-delivered prefix must not be mistaken for the complete set.
    pub fn next_page(&mut self) -> Result<Option<ResultPage>> {
        if self.done {
            return Ok(None);
        }
        let start = self.next_row;
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
        self.next_row = (start + take).min(total);
        let terminal = self.next_row >= total;
        self.done = terminal;
        Ok(Some(ResultPage { rows, terminal }))
    }
}

// ---- the sealed row codec (scratch backing only) ------------------------
//
// Tags mirror the canonical row codec's shapes but store answer cells
// (text inline; interval endpoints as their host values). Internal to the
// backing — never a wire format.

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
        let mut answers = Answers::new();
        self.execute(instance, params, &mut answers)?;
        let identity = ResultIdentity {
            source: PinnedSource::Store(instance.snapshot().identity()),
            generation: Some(instance.snapshot().generation()),
        };
        CompleteResult::seal(answers, identity, instance.work(), RESULT_RAM_BYTES)
    }
}

#[cfg(test)]
mod tests;
