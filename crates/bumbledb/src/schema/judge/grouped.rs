//! Grouped judgment state.
//!
//! The streaming judge never materializes a relation; the state a statement
//! genuinely needs while streaming — determinant membership, capacity group
//! totals, pointwise span tables and coverage runs — lives here in exact
//! transient maps. Each determinant walk reuses one decoding workspace.
//! There are no byte allowances, pressure-triggered spills, or quota retries.

use crate::canonical::{DecodeScratch, RowError};
use crate::error::{Error, IoFailure, LmdbFailure};
use crate::exec::scratch::ScratchRelation;
use crate::ir::Value;
use crate::schema::{FieldDescriptor, FieldId};
use crate::storage::store::StoreError;
use crate::work::{WorkContext, WorkError};

use super::JudgeError;

/// A grouped-state scratch failure that is not a work refusal: allocation
/// in either tier, or the spilled tier's I/O/environment fault. Distinct physical
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

/// Conversion of internal map failures into a state's error channel.
/// This adapter does not choose a storage tier or enable pressure handling.
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

fn allocation_error<E>(channel: Option<fn(ScratchFault) -> E>) -> JudgeError<E> {
    channel.map_or(JudgeError::Allocation, |channel| {
        JudgeError::State(channel(ScratchFault::Allocation))
    })
}

const FLAG_OK: u8 = 0;
pub(super) const FLAG_RAY: u8 = 1;
pub(super) const FLAG_OVERFLOW: u8 = 2;

/// One scalar projection workspace, not a collection of visited groups.
/// Values and encoded-key capacity are reused between projections.
pub(super) struct ScalarKeyScratch<E> {
    values: Vec<Value>,
    encoded: Vec<u8>,
    channel: Option<fn(ScratchFault) -> E>,
}

impl<E> ScalarKeyScratch<E> {
    pub(super) fn new(
        work: &WorkContext,
        fields: usize,
        channel: Option<fn(ScratchFault) -> E>,
    ) -> Result<Self, JudgeError<E>> {
        work.checkpoint()?;
        let mut scratch = Self {
            values: Vec::new(),
            encoded: Vec::new(),
            channel,
        };
        scratch
            .values
            .try_reserve_exact(fields)
            .map_err(|_| allocation_error(channel))?;
        Ok(scratch)
    }

    pub(super) fn project(&mut self, row: &[Value], fields: &[usize]) {
        // Destroy old payloads before allocating their replacements.
        self.values.clear();
        self.values.extend(fields.iter().map(|&at| row[at].clone()));
    }

    pub(super) fn values(&self) -> &[Value] {
        &self.values
    }

    pub(super) fn key(&self) -> &[u8] {
        &self.encoded
    }

    pub(super) fn encode(&mut self) -> Result<&[u8], JudgeError<E>> {
        self.encoded.clear();
        // Every fixed Value image is at most 17 bytes. Variable payloads
        // add their length to a nine-byte tag/length prefix. This bound
        // reserves before the shared encoder writes, without another codec.
        let bound = self.values.iter().fold(0u64, |bytes, value| {
            bytes
                .saturating_add(17)
                .saturating_add(Self::payload_bytes(value))
        });
        self.reserve_encoding(bound)?;
        for value in &self.values {
            encode_value(value, &mut self.encoded);
        }
        Ok(&self.encoded)
    }

    /// Encode borrowed logical fields without cloning their payloads.
    /// Reuse the same mark-key buffer between projections.
    pub(super) fn encode_projection(
        &mut self,
        row: &[Value],
        fields: &[FieldId],
    ) -> Result<&[u8], JudgeError<E>> {
        self.encoded.clear();
        let bound = fields.iter().fold(0u64, |bytes, field| {
            bytes
                .saturating_add(17)
                .saturating_add(Self::payload_bytes(&row[usize::from(field.0)]))
        });
        self.reserve_encoding(bound)?;
        for field in fields {
            encode_value(&row[usize::from(field.0)], &mut self.encoded);
        }
        Ok(&self.encoded)
    }

    fn reserve_encoding(&mut self, bound: u64) -> Result<(), JudgeError<E>> {
        let needed = usize::try_from(bound).map_err(|_| allocation_error(self.channel))?;
        if needed > self.encoded.capacity() {
            self.encoded
                .try_reserve_exact(needed)
                .map_err(|_| allocation_error(self.channel))?;
        }
        Ok(())
    }

    fn payload_bytes(value: &Value) -> u64 {
        match value {
            Value::String(text) => text.len() as u64,
            Value::FixedBytes(bytes) => bytes.len() as u64,
            _ => 0,
        }
    }
}

/// One exact grouped map for a single statement's judgment, dropped with
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
        Self {
            inner: ScratchRelation::new(work),
            channel,
            scratch_value: Vec::new(),
        }
    }

    fn convert(&self, error: Error) -> JudgeError<E> {
        match split_fault(error) {
            Ok(work) => JudgeError::Work(work),
            Err(ScratchFault::Allocation) => allocation_error(self.channel),
            Err(fault) => match self.channel {
                Some(channel) => JudgeError::State(channel(fault)),
                None => {
                    unreachable!("grouped state without a scratch channel stays in the RAM tier")
                }
            },
        }
    }

    /// Run one operation without a quota-driven restart or representation switch.
    fn apply<T>(
        &mut self,
        op: impl FnOnce(&mut ScratchRelation) -> Result<T, Error>,
    ) -> Result<T, JudgeError<E>> {
        op(&mut self.inner).map_err(|error| self.convert(error))
    }

    pub(super) fn len(&self) -> u64 {
        self.inner.len()
    }

    #[cfg(test)]
    pub(super) fn force_spill(&mut self) -> Result<(), JudgeError<E>> {
        self.apply(ScratchRelation::force_spill)
    }

    #[cfg(test)]
    pub(super) fn scratch_path(&self) -> Option<std::path::PathBuf> {
        self.inner.scratch_path()
    }

    /// Exact insert-if-absent set membership; `Ok(true)` iff new.
    pub(super) fn insert_if_absent(&mut self, key: &[u8]) -> Result<bool, JudgeError<E>> {
        self.apply(|inner| inner.insert_if_absent(key, &[]))
    }

    pub(super) fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), JudgeError<E>> {
        self.apply(|inner| inner.put(key, value))
    }

    pub(super) fn contains(&mut self, key: &[u8]) -> Result<bool, JudgeError<E>> {
        let mut out = std::mem::take(&mut self.scratch_value);
        let found = self.apply(|inner| inner.get(key, &mut out));
        self.scratch_value = out;
        found
    }

    /// The dense token of `key`, minted at first sight (mint order is the
    /// deterministic first-encounter order of the walk that feeds it).
    pub(super) fn token_of(&mut self, key: &[u8]) -> Result<u64, JudgeError<E>> {
        let mut out = std::mem::take(&mut self.scratch_value);
        let result = self.apply(|inner| {
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
        let result = self.apply(|inner| {
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
        let result = self.apply(|inner| {
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

    /// Walk exact determinant keys with one reusable decode workspace.
    /// Logical coordinates come from the statement binding, not the
    /// interned index order. Spilled iteration lends reconstructed exact
    /// keys; the callback may operate on other independently owned maps.
    pub(super) fn for_each_determinant(
        &mut self,
        fields: &[FieldDescriptor],
        projection: &[FieldId],
        work: &WorkContext,
        mut visit: impl FnMut(&[u8], &[Value]) -> Result<bool, JudgeError<E>>,
    ) -> Result<(), JudgeError<E>> {
        let channel = self.channel;
        let mut decoded = DecodeScratch::new(work);
        self.for_each(|key, _| {
            decoded
                .with_decoded_payload(
                    projection.iter().map(|field| &fields[usize::from(field.0)]),
                    key,
                    |values| Ok::<_, RowError>(visit(key, values)),
                )
                .map_err(|error| match error {
                    RowError::Work(work) => JudgeError::Work(work),
                    RowError::Allocation | RowError::LengthOverflow => allocation_error(channel),
                    _ => match channel {
                        Some(channel) => JudgeError::State(channel(ScratchFault::Internal(
                            "scratch determinant payload",
                        ))),
                        None => unreachable!("in-memory determinant keys follow the typed encoder"),
                    },
                })?
        })
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
        Value::Uuid(v) => {
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
