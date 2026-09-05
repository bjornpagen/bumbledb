//! Shared bounded framing primitives for the internal history machine.
//!
//! Every frame is `family || layout(u16) || kind(u8) || fields`, big-endian,
//! fixed widths, explicit spans, and no trailing bytes. Framing establishes
//! grammar only: it never upgrades bytes into a verified core value, a durable
//! decision, or authority. Allocation is capped before it happens.

use bumbledb::Id128;

use super::{
    CommandId, DatabaseId, DatabaseIdentity, DecisionDigest, DecisionStamp, IncarnationId,
    ReceiptEpoch, RequestId, SchemaId, StateStamp,
};

/// One shared framing refusal roster across command/receipt/decision/control
/// frames. Every arm is a bounded grammar refusal, never partial data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    LimitExceeded,
    LengthOverflow,
    Allocation,
    Truncated {
        at: usize,
    },
    Family,
    Layout {
        got: u16,
    },
    Kind {
        got: u8,
    },
    Tag {
        at: usize,
        got: u8,
    },
    InvalidEpoch,
    StateIdentityMismatch,
    EmptyChangeSummary,
    EmptyEvidence,
    InvalidTerminalStamp,
    InvalidPreconditionEvidence,
    /// Receipt-policy invariants (rotation/retirement) violated in a frame.
    InvalidPolicy,
    /// Decision/genesis sequence or parent linkage grammar violated.
    InvalidSequence,
    /// A bounded list length field exceeds its declared cap.
    InvalidCount,
    TrailingBytes {
        at: usize,
    },
}

pub(crate) fn check_limit(length: usize, cap: usize) -> Result<(), FrameError> {
    if length > cap {
        Err(FrameError::LimitExceeded)
    } else {
        Ok(())
    }
}

/// Sums part lengths over the `family || layout || kind` header without
/// overflow. `family_len` is the module's family constant length.
pub(crate) fn frame_len(family_len: usize, parts: &[usize]) -> Result<usize, FrameError> {
    parts.iter().try_fold(family_len + 3, |sum, part| {
        sum.checked_add(*part).ok_or(FrameError::LengthOverflow)
    })
}

pub(crate) fn allocate_frame(len: usize, cap: usize) -> Result<Vec<u8>, FrameError> {
    check_limit(len, cap)?;
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| FrameError::Allocation)?;
    Ok(out)
}

pub(crate) fn begin_frame(
    family: &[u8],
    layout: u16,
    kind: u8,
    len: usize,
    cap: usize,
) -> Result<Vec<u8>, FrameError> {
    let mut out = allocate_frame(len, cap)?;
    out.extend_from_slice(family);
    out.extend_from_slice(&layout.to_be_bytes());
    out.push(kind);
    Ok(out)
}

pub(crate) fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(crate) fn put_identity(out: &mut Vec<u8>, identity: DatabaseIdentity) {
    out.extend_from_slice(identity.database_id.as_core().as_bytes());
    out.extend_from_slice(identity.incarnation_id.as_core().as_bytes());
    out.extend_from_slice(&identity.schema_id.0);
}

pub(crate) fn put_id(out: &mut Vec<u8>, id: CommandId) {
    put_u64(out, id.receipt_epoch.get());
    out.extend_from_slice(id.request_id.as_core().as_bytes());
}

pub(crate) fn put_state(out: &mut Vec<u8>, state: StateStamp) {
    out.extend_from_slice(state.incarnation.as_core().as_bytes());
    put_u64(out, state.data_revision);
}

pub(crate) fn put_stamp(out: &mut Vec<u8>, stamp: DecisionStamp) {
    put_u64(out, stamp.seq);
    out.extend_from_slice(stamp.hash.as_bytes());
}

pub(crate) fn put_span(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), FrameError> {
    put_u64(
        out,
        u64::try_from(bytes.len()).map_err(|_| FrameError::LengthOverflow)?,
    );
    out.extend_from_slice(bytes);
    Ok(())
}

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn begin(
        bytes: &'a [u8],
        family: &[u8],
        layout: u16,
        kind: u8,
        cap: usize,
    ) -> Result<Self, FrameError> {
        check_limit(bytes.len(), cap)?;
        let mut input = Self { bytes, at: 0 };
        if input.take(family.len())? != family {
            return Err(FrameError::Family);
        }
        let version = u16::from_be_bytes(input.array()?);
        if version != layout {
            return Err(FrameError::Layout { got: version });
        }
        let (_, got) = input.tag()?;
        if got != kind {
            return Err(FrameError::Kind { got });
        }
        Ok(input)
    }

    pub(crate) fn take(&mut self, len: usize) -> Result<&'a [u8], FrameError> {
        let end = self.at.checked_add(len).ok_or(FrameError::LengthOverflow)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(FrameError::Truncated { at: self.at })?;
        self.at = end;
        Ok(bytes)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], FrameError> {
        let mut array = [0; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    pub(crate) fn u64(&mut self) -> Result<u64, FrameError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub(crate) fn tag(&mut self) -> Result<(usize, u8), FrameError> {
        let at = self.at;
        Ok((at, self.array::<1>()?[0]))
    }

    pub(crate) fn identity(&mut self) -> Result<DatabaseIdentity, FrameError> {
        Ok(DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes(self.array()?)),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes(self.array()?)),
            schema_id: SchemaId(self.array()?),
        })
    }

    pub(crate) fn id(&mut self) -> Result<CommandId, FrameError> {
        Ok(CommandId {
            receipt_epoch: ReceiptEpoch::new(self.u64()?).ok_or(FrameError::InvalidEpoch)?,
            request_id: RequestId::from_core(Id128::from_bytes(self.array()?)),
        })
    }

    pub(crate) fn state(&mut self) -> Result<StateStamp, FrameError> {
        Ok(StateStamp {
            incarnation: IncarnationId::from_core(Id128::from_bytes(self.array()?)),
            data_revision: self.u64()?,
        })
    }

    pub(crate) fn stamp(&mut self) -> Result<DecisionStamp, FrameError> {
        Ok(DecisionStamp {
            seq: self.u64()?,
            hash: DecisionDigest::from_bytes(self.array()?),
        })
    }

    pub(crate) fn span(&mut self, cap: usize) -> Result<&'a [u8], FrameError> {
        let len = usize::try_from(self.u64()?).map_err(|_| FrameError::LengthOverflow)?;
        check_limit(len, cap)?;
        self.take(len)
    }

    pub(crate) fn end(self) -> Result<(), FrameError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(FrameError::TrailingBytes { at: self.at })
        }
    }
}
