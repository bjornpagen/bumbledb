//! Contextual bounded object reception (C6): cancellation checkpoints,
//! maximum bytes before allocation, and incremental digest verification while
//! bytes stream in. Adapters report what they observed; the composition layer
//! decides verification refusal before interpretation.
//!
//! A length header or filesystem stat is not a receiving bound. Production
//! reads push bounded chunks through [`ReceiveAccumulator`]; `receive_whole`
//! and `receive_head_whole` are deleted. [`ReceiveAccumulator::finish`]
//! transfers its buffer to the caller without copying or shrinking it.

use std::io;

use bumbledb::{WorkContext, WorkError};

use super::{ObjectKind, ObjectRef};
use crate::writer::verbs::{ConditionalStore, HeadVersion};

/// One receive quantum. Adapters may read less; they must not copy more
/// than the remaining envelope plus one overflow byte of detection.
pub const RECEIVE_CHUNK_BYTES: usize = 65_536;

/// Maximum bytes one receive may retain. The reference's declared length is
/// the authoritative cap for verified immutable objects; HEAD and other
/// callers supply an explicit envelope cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiveLimits {
    pub max_bytes: u64,
}

impl ReceiveLimits {
    #[must_use]
    pub const fn exact(length: u64) -> Self {
        Self { max_bytes: length }
    }

    #[must_use]
    pub const fn capped(max_bytes: u64) -> Self {
        Self { max_bytes }
    }
}

/// Work and receive caps carried together on lifecycle/transport paths (C2/C6).
#[derive(Debug, Clone, Copy)]
pub struct TransportContext<'a> {
    pub work: Option<&'a WorkContext>,
    pub receive: ReceiveLimits,
}

impl<'a> TransportContext<'a> {
    #[must_use]
    pub const fn new(work: &'a WorkContext, receive: ReceiveLimits) -> Self {
        Self {
            work: Some(work),
            receive,
        }
    }

    /// Receive with an explicit envelope and no work owner.
    #[must_use]
    pub const fn limited(max_bytes: u64) -> TransportContext<'static> {
        TransportContext {
            work: None,
            receive: ReceiveLimits::capped(max_bytes),
        }
    }

    /// Checkpoint when a work owner is present; otherwise a no-op.
    pub fn checkpoint(&self) -> Result<(), WorkError> {
        match self.work {
            Some(work) => work.checkpoint(),
            None => Ok(()),
        }
    }
}

/// What the backend actually observed. These are transport facts, never a
/// publication verdict — L08 interprets certainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportObservation {
    /// Definite absence (ENOENT / typed 404).
    Missing,
    /// Access refused (EACCES / typed 403 / unauthenticated).
    Denied,
    /// The bucket is not addressable. Not a missing object.
    Bucket,
    /// Region or endpoint mismatch. Not a missing object.
    Region,
    /// Typed conditional 412 / not-modified.
    Precondition,
    /// Typed 409 / already-exists that cannot prove win or loss.
    Conflict,
    /// Receive exceeded the admitted envelope during the stream.
    Capped,
    /// Dispatched; outcome unknown (timeout, reset, 5xx, lost ack).
    Indeterminate,
}

/// Adapter errors carry a transport observation so callers do not guess.
pub trait ObservedError {
    fn observation(&self) -> TransportObservation;
}

impl<T: ObservedError + ?Sized> ObservedError for &T {
    fn observation(&self) -> TransportObservation {
        (**self).observation()
    }
}

/// One received object. Ordinary owned bytes, independent of the operation
/// that fetched them. Verification is performed before interpretation.
pub type ReceivedBody = Vec<u8>;

/// Head receive that keeps the same output owner as [`ReceivedBody`].
#[derive(Debug)]
pub enum ReceivedHead {
    Present {
        version: HeadVersion,
        body: ReceivedBody,
    },
    Absent,
}

/// Incremental receive session: checked input length, cancellation, and an
/// optional domain-separated digest. Payload never exceeds the envelope;
/// Vec capacity grows geometrically, clamped to that envelope.
pub struct ReceiveAccumulator<'a> {
    ctx: TransportContext<'a>,
    buf: Vec<u8>,
    hasher: Option<blake3::Hasher>,
    expected_length: Option<u64>,
    expected_digest: Option<[u8; 32]>,
}

/// A push/finish refusal from [`ReceiveAccumulator`].
#[derive(Debug)]
pub enum ReceiveFault {
    Work(WorkError),
    Capped { cap: u64, got: u64 },
    Overflow,
    Alloc,
    Io(io::Error),
    WrongLength { expected: u64, got: u64 },
    WrongDigest,
}

impl ReceiveFault {
    #[must_use]
    pub fn observation(&self) -> TransportObservation {
        match self {
            Self::Capped { .. } => TransportObservation::Capped,
            Self::Io(error) => match error.kind() {
                io::ErrorKind::NotFound => TransportObservation::Missing,
                io::ErrorKind::PermissionDenied => TransportObservation::Denied,
                _ => TransportObservation::Indeterminate,
            },
            Self::Work(_)
            | Self::Overflow
            | Self::Alloc
            | Self::WrongLength { .. }
            | Self::WrongDigest => TransportObservation::Indeterminate,
        }
    }

    #[must_use]
    pub fn into_io(self, key: &str) -> io::Error {
        match self {
            Self::Work(error) => io::Error::new(io::ErrorKind::TimedOut, format!("{error:?}")),
            Self::Capped { cap, got } => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("object {key} length {got}, expected at most {cap}"),
            ),
            Self::Overflow => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("object {key} length overflow"),
            ),
            Self::Alloc => io::Error::new(io::ErrorKind::OutOfMemory, "receive allocation failed"),
            Self::Io(error) => error,
            Self::WrongLength { expected, got } => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("object {key} length {got}, expected {expected}"),
            ),
            Self::WrongDigest => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("object digest mismatch: {key}"),
            ),
        }
    }
}

impl<'a> ReceiveAccumulator<'a> {
    #[must_use]
    pub fn new(ctx: TransportContext<'a>) -> Self {
        Self {
            ctx,
            buf: Vec::new(),
            hasher: None,
            expected_length: None,
            expected_digest: None,
        }
    }

    /// Cap at the reference length (intersected with the caller's envelope)
    /// and hash chunks under the kind's digest domain as they arrive.
    #[must_use]
    pub fn verified(ctx: TransportContext<'a>, kind: ObjectKind, reference: &ObjectRef) -> Self {
        let max_bytes = ctx.receive.max_bytes.min(reference.length);
        Self {
            ctx: TransportContext {
                work: ctx.work,
                receive: ReceiveLimits { max_bytes },
            },
            buf: Vec::new(),
            hasher: Some(blake3::Hasher::new_derive_key(kind.digest_domain())),
            expected_length: Some(reference.length),
            expected_digest: Some(reference.digest),
        }
    }

    #[must_use]
    pub fn len(&self) -> u64 {
        self.buf.len() as u64
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.ctx.receive.max_bytes.saturating_sub(self.len())
    }

    pub fn checkpoint(&self) -> Result<(), ReceiveFault> {
        self.ctx.checkpoint().map_err(ReceiveFault::Work)
    }

    /// Copy `chunk` only after checking its complete input length. Even a
    /// direct caller's large chunk is copied and hashed in bounded quanta.
    ///
    /// # Errors
    /// Cancellation, envelope overrun, length overflow, or allocation refusal.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), ReceiveFault> {
        self.ctx.checkpoint().map_err(ReceiveFault::Work)?;
        if chunk.is_empty() {
            return Ok(());
        }
        let next = self
            .len()
            .checked_add(chunk.len() as u64)
            .ok_or(ReceiveFault::Overflow)?;
        if next > self.ctx.receive.max_bytes {
            return Err(ReceiveFault::Capped {
                cap: self.ctx.receive.max_bytes,
                got: next,
            });
        }
        if let Some(expected) = self.expected_length
            && next > expected
        {
            return Err(ReceiveFault::Capped {
                cap: expected,
                got: next,
            });
        }
        let next = usize::try_from(next).map_err(|_| ReceiveFault::Overflow)?;
        if next > self.buf.capacity() {
            let capacity = (self.buf.capacity() as u64)
                .saturating_mul(2)
                .max(next as u64)
                .min(self.ctx.receive.max_bytes);
            let capacity = usize::try_from(capacity).map_err(|_| ReceiveFault::Overflow)?;
            self.buf
                .try_reserve_exact(capacity - self.buf.len())
                .map_err(|_| ReceiveFault::Alloc)?;
        }
        for chunk in chunk.chunks(RECEIVE_CHUNK_BYTES) {
            self.ctx.checkpoint().map_err(ReceiveFault::Work)?;
            self.buf.extend_from_slice(chunk);
            if let Some(hasher) = &mut self.hasher {
                hasher.update(chunk);
            }
        }
        Ok(())
    }

    /// Finish the stream. Length and digest are checked when this session
    /// was constructed with [`Self::verified`].
    ///
    /// # Errors
    /// Cancellation, length or digest disagreement with the reference.
    pub fn finish(self) -> Result<ReceivedBody, ReceiveFault> {
        self.ctx.checkpoint().map_err(ReceiveFault::Work)?;
        if let Some(expected) = self.expected_length
            && self.len() != expected
        {
            return Err(ReceiveFault::WrongLength {
                expected,
                got: self.len(),
            });
        }
        if let (Some(hasher), Some(digest)) = (self.hasher.as_ref(), self.expected_digest)
            && *hasher.finalize().as_bytes() != digest
        {
            return Err(ReceiveFault::WrongDigest);
        }
        Ok(self.buf)
    }
}

/// Bounded reception beyond the raw conditional-store verbs. Production
/// adapters stream chunks into a [`ReceiveAccumulator`]; there is no
/// whole-body default.
pub trait ReceivingStore: ConditionalStore {
    /// Fetch one object under a hard byte cap, checkpointing work between
    /// chunks where the adapter streams.
    fn receive_object(
        &self,
        key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedBody, <Self as ConditionalStore>::Error>;

    /// Read the head object under an explicit cap.
    fn receive_head(
        &self,
        head_key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedHead, <Self as ConditionalStore>::Error>;
}

/// Verify an already-admitted body against a reference. Prefer
/// [`ReceiveAccumulator::verified`] so length and digest are checked while
/// chunks arrive; this is the leftover check for a capped `receive_object`.
pub(crate) fn verify_body(
    key: &str,
    kind: ObjectKind,
    reference: &ObjectRef,
    body: &[u8],
    ctx: TransportContext<'_>,
) -> Result<(), super::ObjectError> {
    ctx.checkpoint().map_err(super::backend)?;
    if body.len() as u64 != reference.length {
        return Err(super::ObjectError::WrongLength {
            key: key.to_string(),
            expected: reference.length,
            got: body.len() as u64,
        });
    }
    let mut hasher = blake3::Hasher::new_derive_key(kind.digest_domain());
    for chunk in body.chunks(RECEIVE_CHUNK_BYTES) {
        ctx.checkpoint().map_err(super::backend)?;
        hasher.update(chunk);
    }
    if *hasher.finalize().as_bytes() != reference.digest {
        return Err(super::ObjectError::WrongDigest {
            key: key.to_string(),
        });
    }
    Ok(())
}

impl<T: ReceivingStore + ?Sized> ReceivingStore for &T {
    fn receive_object(
        &self,
        key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedBody, T::Error> {
        (*self).receive_object(key, ctx)
    }

    fn receive_head(
        &self,
        head_key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedHead, T::Error> {
        (*self).receive_head(head_key, ctx)
    }
}

impl<T: ReceivingStore + ?Sized> ReceivingStore for std::sync::Arc<T> {
    fn receive_object(
        &self,
        key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedBody, T::Error> {
        (**self).receive_object(key, ctx)
    }

    fn receive_head(
        &self,
        head_key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedHead, T::Error> {
        (**self).receive_head(head_key, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::object_digest;
    use bumbledb::WorkContext;

    #[test]
    fn accumulator_refuses_past_the_envelope_without_retaining_the_overflow() {
        let mut acc = ReceiveAccumulator::new(TransportContext {
            work: None,
            receive: ReceiveLimits::capped(4),
        });
        acc.push(b"abcd").expect("exact envelope");
        assert_eq!(acc.len(), 4);
        let overflow = acc.push(b"x");
        assert!(matches!(
            overflow,
            Err(ReceiveFault::Capped { cap: 4, got: 5 })
        ));
        assert_eq!(acc.len(), 4, "overflow is not copied");
    }

    #[test]
    fn accumulator_grows_geometrically_without_preallocating_the_envelope() {
        let mut acc = ReceiveAccumulator::new(TransportContext::limited(65_536));
        assert_eq!(acc.buf.capacity(), 0);
        let mut growths = 0;
        for len in 1..=8193 {
            let before = acc.buf.capacity();
            acc.push(b"x").unwrap();
            growths += usize::from(acc.buf.capacity() != before);
            assert_eq!(acc.buf.len(), len);
            assert!(acc.buf.capacity() <= 2 * len);
        }
        assert!(
            growths <= 15,
            "tiny chunks must not cause linear reallocations"
        );
        assert!(acc.finish().unwrap().iter().all(|&byte| byte == b'x'));
    }

    #[test]
    fn accumulator_checkpoints_cancellation_between_chunks_and_at_finish() {
        let ctx = WorkContext::new();
        let mut acc =
            ReceiveAccumulator::new(TransportContext::new(&ctx, ReceiveLimits::capped(32)));
        acc.push(b"first").unwrap();
        ctx.cancel();
        assert!(matches!(
            acc.push(b"late"),
            Err(ReceiveFault::Work(WorkError::Cancelled))
        ));
        assert_eq!(acc.len(), 5, "a refused chunk is not copied");
        assert!(matches!(
            acc.push(&[]),
            Err(ReceiveFault::Work(WorkError::Cancelled))
        ));
        assert!(matches!(
            acc.finish(),
            Err(ReceiveFault::Work(WorkError::Cancelled))
        ));
    }

    #[test]
    fn verified_session_hashes_incrementally_and_refuses_wrong_length() {
        let kind = ObjectKind::Chunk;
        let bytes = b"manifest-bytes";
        let reference = ObjectRef::of(1, kind, bytes);
        let mut acc = ReceiveAccumulator::verified(
            TransportContext {
                work: None,
                receive: ReceiveLimits::exact(reference.length),
            },
            kind,
            &reference,
        );
        for piece in bytes.chunks(3) {
            acc.push(piece).expect("chunk");
        }
        let got = acc.finish().expect("verified");
        assert_eq!(got.as_slice(), bytes);
        assert_eq!(object_digest(kind, &got), reference.digest);

        let mut short = ReceiveAccumulator::verified(
            TransportContext {
                work: None,
                receive: ReceiveLimits::exact(reference.length),
            },
            kind,
            &reference,
        );
        short.push(b"short").expect("under length");
        assert!(matches!(
            short.finish(),
            Err(ReceiveFault::WrongLength { .. })
        ));
    }

    #[test]
    fn finish_moves_the_receive_buffer_and_drop_frees_its_actual_capacity() {
        let ctx = WorkContext::new();
        let mut acc =
            ReceiveAccumulator::new(TransportContext::new(&ctx, ReceiveLimits::capped(32)));
        acc.push(b"payload").unwrap();
        acc.push(b"!").unwrap();
        let pointer = acc.buf.as_ptr();
        let capacity = acc.buf.capacity();
        assert!(
            capacity > acc.buf.len(),
            "exercise spare capacity at finish"
        );
        #[cfg(feature = "alloc-counter")]
        let before = bumbledb::alloc_counter::snapshot();
        let body = acc.finish().unwrap();
        assert_eq!(body.as_ptr(), pointer);
        assert_eq!(
            body.capacity(),
            capacity,
            "no shrink/reallocation on transfer"
        );
        #[cfg(feature = "alloc-counter")]
        assert_eq!(bumbledb::alloc_counter::snapshot().window, before.window);
        ctx.cancel();
        assert_eq!(
            body.as_slice(),
            b"payload!",
            "completed bytes own their lifetime"
        );
        drop(body);
        #[cfg(feature = "alloc-counter")]
        assert_eq!(
            bumbledb::alloc_counter::snapshot().absolute.live_bytes + capacity as u64,
            before.absolute.live_bytes,
        );
    }

    #[test]
    fn verified_session_rejects_same_length_wrong_bytes_and_accepts_empty_objects() {
        let reference = ObjectRef::of(1, ObjectKind::Chunk, b"abc");
        let mut acc = ReceiveAccumulator::verified(
            TransportContext::limited(3),
            ObjectKind::Chunk,
            &reference,
        );
        acc.push(b"abd").unwrap();
        assert!(matches!(acc.finish(), Err(ReceiveFault::WrongDigest)));
        let empty = ObjectRef::of(1, ObjectKind::Chunk, b"");
        assert!(
            ReceiveAccumulator::verified(TransportContext::limited(0), ObjectKind::Chunk, &empty,)
                .finish()
                .unwrap()
                .is_empty()
        );
    }
}
