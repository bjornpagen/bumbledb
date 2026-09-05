//! Contextual bounded object reception (C6): work/deadline checkpoints,
//! maximum bytes before allocation, and incremental digest verification while
//! bytes stream in. Adapters report what they observed; the composition layer
//! decides verification refusal before interpretation.
//!
//! A length header or filesystem stat is not a receiving bound. Production
//! reads push bounded chunks through [`ReceiveAccumulator`]; `receive_whole`
//! and `receive_head_whole` are deleted. [`ReceiveAccumulator::finish`]
//! returns a [`ReceivedBody`] so a live work reservation travels with the
//! bytes until the caller decodes or drops them.

use std::io;

use bumbledb::work::{ByteKind, ChargedBuffer, ChargedBytes};
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

/// Received object bytes. When a [`WorkContext`] was present, the receive
/// reservation travels with the payload until the caller drops or decodes it.
#[derive(Debug)]
pub enum ReceivedBody {
    Charged(ChargedBytes),
    Plain(Box<[u8]>),
}

impl ReceivedBody {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Charged(body) => body.as_bytes(),
            Self::Plain(body) => body,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    /// Take the charged owner. `None` when the receive had no work context.
    #[must_use]
    pub fn into_charged(self) -> Option<ChargedBytes> {
        match self {
            Self::Charged(body) => Some(body),
            Self::Plain(_) => None,
        }
    }
}

impl PartialEq<[u8]> for ReceivedBody {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_bytes() == other
    }
}

/// Head receive that keeps the same output owner as [`ReceivedBody`].
#[derive(Debug)]
pub enum ReceivedHead {
    Present {
        version: HeadVersion,
        body: ReceivedBody,
    },
    Absent,
}

enum AccBuf {
    Empty,
    Charged(ChargedBuffer),
    Plain(Vec<u8>),
}

impl AccBuf {
    fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Charged(buf) => buf.len(),
            Self::Plain(buf) => buf.len(),
        }
    }
}

/// Incremental receive session: cap, deadline, overlap-admitted copy, and
/// optional domain-separated digest. Never allocates past the envelope.
/// Reservations stay on the buffer until [`Self::finish`].
pub struct ReceiveAccumulator<'a> {
    ctx: TransportContext<'a>,
    buf: AccBuf,
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
            Self::Work(error) => {
                io::Error::new(io::ErrorKind::TimedOut, format!("{error:?}"))
            }
            Self::Capped { cap, got } => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("object {key} length {got}, expected at most {cap}"),
            ),
            Self::Overflow => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("object {key} length overflow"),
            ),
            Self::Alloc => {
                io::Error::new(io::ErrorKind::OutOfMemory, "receive allocation failed")
            }
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
            buf: AccBuf::Empty,
            hasher: None,
            expected_length: None,
            expected_digest: None,
        }
    }

    /// Cap at the reference length (intersected with the caller's envelope)
    /// and hash chunks under the kind's digest domain as they arrive.
    #[must_use]
    pub fn verified(
        ctx: TransportContext<'a>,
        kind: ObjectKind,
        reference: &ObjectRef,
    ) -> Self {
        let max_bytes = ctx.receive.max_bytes.min(reference.length);
        Self {
            ctx: TransportContext {
                work: ctx.work,
                receive: ReceiveLimits { max_bytes },
            },
            buf: AccBuf::Empty,
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
    pub fn remaining(&self) -> u64 {
        self.ctx.receive.max_bytes.saturating_sub(self.len())
    }

    pub fn checkpoint(&self) -> Result<(), ReceiveFault> {
        self.ctx.checkpoint().map_err(ReceiveFault::Work)
    }

    /// Copy `chunk` only after the envelope and a work reservation admit it.
    ///
    /// # Errors
    /// Deadline, cancellation, envelope overrun, or allocation refusal.
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
        match &mut self.buf {
            AccBuf::Empty => {
                if let Some(work) = self.ctx.work {
                    let mut charged = ChargedBuffer::with_capacity(
                        work,
                        ByteKind::Working,
                        chunk.len(),
                    )
                    .map_err(ReceiveFault::Work)?;
                    charged
                        .try_extend_from_slice(chunk)
                        .map_err(ReceiveFault::Work)?;
                    self.buf = AccBuf::Charged(charged);
                } else {
                    let mut plain = Vec::new();
                    plain
                        .try_reserve(chunk.len())
                        .map_err(|_| ReceiveFault::Alloc)?;
                    plain.extend_from_slice(chunk);
                    self.buf = AccBuf::Plain(plain);
                }
            }
            AccBuf::Charged(buf) => buf
                .try_extend_from_slice(chunk)
                .map_err(ReceiveFault::Work)?,
            AccBuf::Plain(buf) => {
                buf.try_reserve(chunk.len())
                    .map_err(|_| ReceiveFault::Alloc)?;
                buf.extend_from_slice(chunk);
            }
        }
        if let Some(hasher) = &mut self.hasher {
            hasher.update(chunk);
        }
        Ok(())
    }

    /// Finish the stream. Length and digest are checked when this session
    /// was constructed with [`Self::verified`].
    ///
    /// # Errors
    /// Length or digest disagreement with the reference.
    pub fn finish(self) -> Result<ReceivedBody, ReceiveFault> {
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
        match self.buf {
            AccBuf::Charged(buf) => Ok(ReceivedBody::Charged(buf.into_bytes())),
            AccBuf::Plain(buf) => Ok(ReceivedBody::Plain(buf.into_boxed_slice())),
            AccBuf::Empty => match self.ctx.work {
                Some(work) => ChargedBytes::adopt(work, ByteKind::Working, Box::from([]))
                    .map(ReceivedBody::Charged)
                    .map_err(ReceiveFault::Work),
                None => Ok(ReceivedBody::Plain(Box::from([]))),
            },
        }
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
) -> Result<(), super::ObjectError> {
    if body.len() as u64 != reference.length {
        return Err(super::ObjectError::WrongLength {
            key: key.to_string(),
            expected: reference.length,
            got: body.len() as u64,
        });
    }
    if super::object_digest(kind, body) != reference.digest {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::object_digest;
    use bumbledb::ExecutionPolicy;
    use std::time::Duration;

    fn work(working: u64, timeout: Duration) -> WorkContext {
        ExecutionPolicy {
            input_bytes: 0,
            working_bytes: working,
            scratch_bytes: 0,
            result_bytes: 0,
            rows: 0,
            work_units: 1_024,
            timeout,
        }
        .start()
        .expect("start")
    }

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
    fn accumulator_reserves_before_copy_and_refunds_on_drop() {
        let ctx = work(64, Duration::from_secs(5));
        let baseline = ctx.used(bumbledb::work::Resource::WorkingBytes);
        {
            let mut acc = ReceiveAccumulator::new(TransportContext::new(
                &ctx,
                ReceiveLimits::capped(32),
            ));
            acc.push(b"payload").expect("push");
            assert!(ctx.used(bumbledb::work::Resource::WorkingBytes) > baseline);
            drop(acc);
        }
        assert_eq!(ctx.used(bumbledb::work::Resource::WorkingBytes), baseline);
    }

    #[test]
    fn accumulator_checkpoints_deadline_between_chunks() {
        let ctx = work(1_024, Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(3));
        let mut acc = ReceiveAccumulator::new(TransportContext::new(
            &ctx,
            ReceiveLimits::capped(32),
        ));
        assert!(matches!(
            acc.push(b"late"),
            Err(ReceiveFault::Work(WorkError::DeadlineExceeded))
        ));
        assert_eq!(acc.len(), 0);
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
        assert_eq!(got.as_bytes(), bytes);
        assert_eq!(object_digest(kind, got.as_bytes()), reference.digest);

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
    fn finish_keeps_the_receive_charge_until_the_owner_drops() {
        let ctx = work(64, Duration::from_secs(5));
        let baseline = ctx.used(bumbledb::work::Resource::WorkingBytes);
        let body = {
            let mut acc = ReceiveAccumulator::new(TransportContext::new(
                &ctx,
                ReceiveLimits::capped(32),
            ));
            acc.push(b"payload").expect("push");
            acc.finish().expect("finish")
        };
        assert!(
            ctx.used(bumbledb::work::Resource::WorkingBytes) > baseline,
            "finish must not refund the receive reservation"
        );
        assert_eq!(body.as_bytes(), b"payload");
        drop(body);
        assert_eq!(ctx.used(bumbledb::work::Resource::WorkingBytes), baseline);
    }
}
