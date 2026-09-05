//! Charged grouped judgment state (finding C of the F3 review).
//!
//! The streaming judge never materializes a relation; the state a statement
//! genuinely needs while streaming — determinant membership, capacity group
//! totals, pointwise span tables and coverage runs — lives here, in the ONE
//! charged transient map (`exec::scratch::ScratchRelation`, chapter 12 §4):
//! a fallibly grown RAM table charged to the operation's working allowance,
//! spilling to the execution-owned temporary LMDB environment under
//! pressure so admission of a change to a beyond-RAM relation completes
//! under a bounded working budget instead of demanding database-sized
//! memory.
//!
//! A spilled tier can fail in ways a pure candidate state never does
//! (scratch I/O, allocation, its own environment). Those failures travel
//! through the state's OWN error channel — [`JudgeScratch`] names the
//! conversion. A judgment without a channel keeps grouped state in the
//! charged RAM tier only: it never spills, so the only failures it can
//! produce are the typed work refusals every caller already handles.
//! Budget refusals are never silently converted into disk usage the caller
//! did not permit — the scratch tier charges the scratch allowance.

use crate::error::{Error, IoFailure, LmdbFailure};
use crate::exec::scratch::ScratchRelation;
use crate::ir::Value;
use crate::storage::store::StoreError;
use crate::work::{Resource, WorkContext, WorkError};

use super::JudgeError;

/// A grouped-state scratch failure that is not a work refusal: the spilled
/// tier's own I/O, allocation, or environment fault. Distinct physical
/// conditions stay distinct; a work refusal is never represented here (it
/// surfaces as [`JudgeError::Work`] for every state).
#[derive(Debug)]
pub enum ScratchFault {
    /// Filesystem failure creating or unlinking the scratch directory.
    Io(IoFailure),
    /// The temporary scratch environment's LMDB failure.
    Lmdb(LmdbFailure),
    /// A fallible in-memory allocation was refused by the host.
    Allocation,
    /// The judge's own scratch framing failed to parse back — a defect in
    /// this transient environment, never a claim about the store.
    Internal(&'static str),
}

/// How spilled grouped-state failures enter a state's error channel.
///
/// [`JudgeScratch::disabled`] (what states without a channel get) keeps
/// grouped state in charged RAM only — beyond-budget judgments refuse with
/// the typed working-byte exhaustion instead of unaccounted growth.
/// [`JudgeScratch::channel`] opts the judgment into the disk tier.
#[derive(Debug)]
pub struct JudgeScratch<E> {
    pub(super) channel: Option<fn(ScratchFault) -> E>,
}

// Manual, bound-free copies: the struct holds only a function pointer, so
// it is copyable for EVERY error type (a derive would demand `E: Copy`).
impl<E> Clone for JudgeScratch<E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E> Copy for JudgeScratch<E> {}

impl<E> JudgeScratch<E> {
    #[must_use]
    pub fn disabled() -> Self {
        Self { channel: None }
    }

    #[must_use]
    pub fn channel(channel: fn(ScratchFault) -> E) -> Self {
        Self {
            channel: Some(channel),
        }
    }
}

/// The [`StoreError`] image of a scratch fault. `Internal` rides the LMDB
/// decoding channel deliberately: the fault is in the judge's TRANSIENT
/// environment, never store corruption, so it must not trip corruption
/// handling upstream.
#[must_use]
pub fn store_fault(fault: ScratchFault) -> StoreError {
    match fault {
        ScratchFault::Io(failure) => StoreError::Io(failure),
        ScratchFault::Lmdb(failure) => StoreError::Lmdb(failure),
        ScratchFault::Allocation => StoreError::Allocation,
        ScratchFault::Internal(_) => StoreError::Lmdb(LmdbFailure::Decoding),
    }
}

/// Split a scratch-layer failure into the work channel or a typed fault.
fn split_fault(error: Error) -> Result<WorkError, ScratchFault> {
    match error {
        Error::Store(boxed) => match *boxed {
            StoreError::Work(work) => Ok(work),
            StoreError::Allocation => Err(ScratchFault::Allocation),
            StoreError::ReaderSlotsExhausted => Err(ScratchFault::Lmdb(LmdbFailure::Decoding)),
            _ => Err(ScratchFault::Internal("unexpected scratch store error")),
        },
        Error::Lmdb(failure) => Err(ScratchFault::Lmdb(failure)),
        Error::Io(failure) => Err(ScratchFault::Io(failure)),
        Error::Corruption(_) => Err(ScratchFault::Internal("scratch framing")),
        _ => Err(ScratchFault::Internal("unexpected scratch failure")),
    }
}

fn is_working_refusal(error: &Error) -> bool {
    matches!(
        error,
        Error::Store(boxed) if matches!(
            boxed.as_ref(),
            StoreError::Work(WorkError::Exhausted {
                resource: Resource::WorkingBytes,
                ..
            })
        )
    )
}

/// The RAM allowance one grouped map takes before spilling: a fraction of
/// the operation's REMAINING working bytes at construction, so several
/// concurrent maps plus streamed rows and citations fit the budget. Purely
/// budget-driven — an effectively unbounded embedded ledger keeps grouped
/// state in charged RAM (the old memory profile, now accounted), while a
/// small budget forces the disk tier immediately.
fn ram_allowance(work: &WorkContext) -> usize {
    let limit = work.limit(Resource::WorkingBytes);
    let used = work.used(Resource::WorkingBytes);
    usize::try_from(limit.saturating_sub(used) / 8).unwrap_or(usize::MAX)
}

const FLAG_OK: u8 = 0;
pub(super) const FLAG_RAY: u8 = 1;
pub(super) const FLAG_OVERFLOW: u8 = 2;

/// One charged grouped map for a single statement's judgment, dropped with
/// the statement. Keys are exact encoded value tuples or fixed-width words;
/// no hash verdict participates (forced fingerprint collisions can slow the
/// spilled tier's oversized-key buckets, never merge distinct tuples).
pub(super) struct GroupedMap<E> {
    inner: ScratchRelation,
    channel: Option<fn(ScratchFault) -> E>,
    scratch_value: Vec<u8>,
}

impl<E> GroupedMap<E> {
    pub(super) fn new(work: &WorkContext, channel: Option<fn(ScratchFault) -> E>) -> Self {
        let ram_limit = match channel {
            None => usize::MAX,
            Some(_) => ram_allowance(work),
        };
        Self {
            inner: ScratchRelation::new(work, ram_limit),
            channel,
            scratch_value: Vec::new(),
        }
    }

    fn convert(&self, error: Error) -> JudgeError<E> {
        match split_fault(error) {
            Ok(work) => JudgeError::Work(work),
            Err(fault) => match self.channel {
                Some(channel) => JudgeError::State(channel(fault)),
                None => unreachable!(
                    "grouped state without a scratch channel stays in the charged RAM tier"
                ),
            },
        }
    }

    /// Run one mutating scratch operation; a WORKING-byte refusal with a
    /// spill channel forces the disk tier and retries once, so a bounded
    /// working budget degrades to charged disk instead of failing.
    fn with_spill<T>(
        &mut self,
        mut op: impl FnMut(&mut ScratchRelation) -> Result<T, Error>,
    ) -> Result<T, JudgeError<E>> {
        match op(&mut self.inner) {
            Ok(value) => Ok(value),
            Err(error) => {
                if self.channel.is_some() && !self.inner.spilled() && is_working_refusal(&error) {
                    self.inner
                        .force_spill()
                        .map_err(|spill| self.convert(spill))?;
                    op(&mut self.inner).map_err(|retry| self.convert(retry))
                } else {
                    Err(self.convert(error))
                }
            }
        }
    }

    pub(super) fn len(&self) -> u64 {
        self.inner.len()
    }

    /// Exact insert-if-absent set membership; `Ok(true)` iff new.
    pub(super) fn insert_if_absent(&mut self, key: &[u8]) -> Result<bool, JudgeError<E>> {
        self.with_spill(|inner| inner.insert_if_absent(key, &[]))
    }

    pub(super) fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), JudgeError<E>> {
        self.with_spill(|inner| inner.put(key, value))
    }

    pub(super) fn contains(&mut self, key: &[u8]) -> Result<bool, JudgeError<E>> {
        let mut out = std::mem::take(&mut self.scratch_value);
        let found = self.with_spill(|inner| inner.get(key, &mut out));
        self.scratch_value = out;
        found
    }

    /// The dense token of `key`, minted at first sight (mint order is the
    /// deterministic first-encounter order of the walk that feeds it).
    pub(super) fn token_of(&mut self, key: &[u8]) -> Result<u64, JudgeError<E>> {
        let mut out = std::mem::take(&mut self.scratch_value);
        let result = self.with_spill(|inner| {
            if inner.get(key, &mut out)? {
                let word: [u8; 8] = out
                    .as_slice()
                    .try_into()
                    .map_err(|_| corrupt("grouped token width"))?;
                Ok(u64::from_be_bytes(word))
            } else {
                let token = inner.len();
                inner.put(key, &token.to_be_bytes())?;
                Ok(token)
            }
        });
        self.scratch_value = out;
        result
    }

    /// The token of `key` if one was minted.
    pub(super) fn lookup_token(&mut self, key: &[u8]) -> Result<Option<u64>, JudgeError<E>> {
        let mut out = std::mem::take(&mut self.scratch_value);
        let result = self.with_spill(|inner| {
            if inner.get(key, &mut out)? {
                let word: [u8; 8] = out
                    .as_slice()
                    .try_into()
                    .map_err(|_| corrupt("grouped token width"))?;
                Ok(Some(u64::from_be_bytes(word)))
            } else {
                Ok(None)
            }
        });
        self.scratch_value = out;
        result
    }

    /// One capacity group's widened running total plus its sticky
    /// first-failure flag; `(0, FLAG_OK)` for an unseen group.
    pub(super) fn group_total(&mut self, key: &[u8]) -> Result<(u128, u8), JudgeError<E>> {
        let mut out = std::mem::take(&mut self.scratch_value);
        let result = self.with_spill(|inner| {
            if inner.get(key, &mut out)? {
                if out.len() != 17 {
                    return Err(corrupt("grouped total width"));
                }
                let total = u128::from_be_bytes(out[..16].try_into().expect("checked width"));
                Ok((total, out[16]))
            } else {
                Ok((0, FLAG_OK))
            }
        });
        self.scratch_value = out;
        result
    }

    pub(super) fn put_group_total(
        &mut self,
        key: &[u8],
        total: u128,
        flag: u8,
    ) -> Result<(), JudgeError<E>> {
        let mut value = [0u8; 17];
        value[..16].copy_from_slice(&total.to_be_bytes());
        value[16] = flag;
        self.put(key, &value)
    }

    /// Ordered walk over every (key, value); the callback returns `false`
    /// to stop early. Exact byte order for inline-sized keys — the judge's
    /// fixed-width span/run keys always are.
    pub(super) fn for_each(
        &mut self,
        mut visit: impl FnMut(&[u8], &[u8]) -> Result<bool, JudgeError<E>>,
    ) -> Result<(), JudgeError<E>> {
        let mut smuggled: Option<JudgeError<E>> = None;
        let walked = self
            .inner
            .for_each(&mut |key, value| match visit(key, value) {
                Ok(keep) => Ok(keep),
                Err(error) => {
                    smuggled = Some(error);
                    Ok(false)
                }
            });
        if let Some(error) = smuggled {
            return Err(error);
        }
        walked.map_err(|error| self.convert(error))
    }

    /// Predecessor query over fixed-width keys (coverage-run probes): the
    /// last entry ≤ `bound`, copied into `key_out`/`value_out`.
    pub(super) fn last_at_or_before(
        &mut self,
        bound: &[u8],
        key_out: &mut Vec<u8>,
        value_out: &mut Vec<u8>,
    ) -> Result<bool, JudgeError<E>> {
        let result = self.inner.last_at_or_before(bound, key_out, value_out);
        result.map_err(|error| self.convert(error))
    }
}

fn corrupt(what: &'static str) -> Error {
    Error::Corruption(crate::error::CorruptionError::MalformedValue(what))
}

/// Append one value's exact, injective, prefix-free byte image: the
/// canonical wire tags and payloads (the SAME payload rules as
/// `canonical::CanonicalRow`), so byte equality of two encoded tuples is
/// exactly canonical value equality — floats by canonical payload bits,
/// never a hash and never a lossy fold.
pub(super) fn encode_value(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Bool(v) => out.extend_from_slice(&[0, u8::from(*v)]),
        Value::U64(v) => {
            out.push(1);
            out.extend_from_slice(&v.to_be_bytes());
        }
        Value::I64(v) => {
            out.push(2);
            out.extend_from_slice(&v.to_be_bytes());
        }
        Value::F64(v) => {
            out.push(3);
            out.extend_from_slice(&v.to_be_bytes());
        }
        Value::String(v) => {
            out.push(4);
            out.extend_from_slice(&(v.len() as u64).to_be_bytes());
            out.extend_from_slice(v.as_bytes());
        }
        Value::FixedBytes(v) => {
            out.push(5);
            out.extend_from_slice(&(v.len() as u64).to_be_bytes());
            out.extend_from_slice(v);
        }
        Value::IntervalU64(v) => {
            out.push(6);
            out.extend_from_slice(&v.start().to_be_bytes());
            out.extend_from_slice(&v.end().to_be_bytes());
        }
        Value::IntervalI64(v) => {
            out.push(7);
            out.extend_from_slice(&v.start().to_be_bytes());
            out.extend_from_slice(&v.end().to_be_bytes());
        }
        Value::Id128(v) => {
            out.push(8);
            out.extend_from_slice(v.as_bytes());
        }
        Value::IntervalF64(v) => {
            out.push(9);
            out.extend_from_slice(&v.start().to_be_bytes());
            out.extend_from_slice(&v.end().to_be_bytes());
        }
    }
}

/// The retained cost of citing `values` as one example fact, charged to the
/// working allowance while the judgment holds it.
pub(super) fn values_charge(values: &[Value]) -> u64 {
    let mut bytes = 32u64;
    for value in values {
        bytes = bytes.saturating_add(32).saturating_add(match value {
            Value::String(text) => text.len() as u64,
            Value::FixedBytes(payload) => payload.len() as u64,
            _ => 0,
        });
    }
    bytes
}
