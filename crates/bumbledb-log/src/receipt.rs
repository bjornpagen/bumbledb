//! Decided outcomes and the receipts that record them.

use crate::command::CommandRef;
use crate::frame::{FrameError, Kind, Reader, Writer};
use crate::ids::{CommandDigest, RequestId, Revision, Seq};

/// Net facts a committed command changed; never both zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delta {
    added: u64,
    removed: u64,
}

impl Delta {
    /// `None` when nothing changed: that decision is [`Outcome::NoChange`].
    #[must_use]
    pub const fn new(added: u64, removed: u64) -> Option<Self> {
        if added == 0 && removed == 0 {
            None
        } else {
            Some(Self { added, removed })
        }
    }

    #[must_use]
    pub const fn added(self) -> u64 {
        self.added
    }

    #[must_use]
    pub const fn removed(self) -> u64 {
        self.removed
    }
}

/// The engine's canonical violation evidence, opaque to the log; never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence(Box<[u8]>);

impl Evidence {
    #[must_use]
    pub fn new(bytes: Box<[u8]>) -> Option<Self> {
        (!bytes.is_empty()).then_some(Self(bytes))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Committed(Delta),
    NoChange,
    PreconditionFailed {
        expected: Revision,
        observed: Revision,
    },
    InvariantRejected(Evidence),
}

/// One request's durable decision: the entry that decided it and the
/// revision right after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub command: CommandRef,
    pub seq: Seq,
    pub revision: Revision,
    pub outcome: Outcome,
}

impl Receipt {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = Writer::new(Kind::Receipt);
        put_command(&mut writer, self.command);
        writer.u64(self.seq.get());
        writer.u64(self.revision.0);
        put_outcome(&mut writer, &self.outcome);
        writer.finish()
    }

    /// # Errors
    /// Malformed frames.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let mut reader = Reader::new(bytes, Kind::Receipt)?;
        let command = take_command(&mut reader)?;
        let seq = take_seq(&mut reader)?;
        let revision = Revision(reader.u64()?);
        let tag = reader.u8()?;
        let outcome = take_outcome(&mut reader, tag)?;
        reader.finish()?;
        Ok(Self {
            command,
            seq,
            revision,
            outcome,
        })
    }
}

const COMMITTED: u8 = 1;
const NO_CHANGE: u8 = 2;
const PRECONDITION_FAILED: u8 = 3;
const INVARIANT_REJECTED: u8 = 4;

pub(crate) fn put_command(writer: &mut Writer, command: CommandRef) {
    writer.array(&command.request.0);
    writer.array(&command.digest.0);
}

pub(crate) fn take_command(reader: &mut Reader<'_>) -> Result<CommandRef, FrameError> {
    Ok(CommandRef {
        request: RequestId(reader.array()?),
        digest: CommandDigest(reader.array()?),
    })
}

pub(crate) fn take_seq(reader: &mut Reader<'_>) -> Result<Seq, FrameError> {
    Seq::new(reader.u64()?).ok_or(FrameError::Value)
}

pub(crate) fn put_delta(writer: &mut Writer, delta: Delta) {
    writer.u64(delta.added);
    writer.u64(delta.removed);
}

pub(crate) fn take_delta(reader: &mut Reader<'_>) -> Result<Delta, FrameError> {
    let added = reader.u64()?;
    Delta::new(added, reader.u64()?).ok_or(FrameError::Value)
}

pub(crate) fn put_evidence(writer: &mut Writer, evidence: &Evidence) {
    writer.blob(evidence.as_bytes());
}

pub(crate) fn take_evidence(reader: &mut Reader<'_>) -> Result<Evidence, FrameError> {
    Evidence::new(reader.blob()?.into()).ok_or(FrameError::Value)
}

pub(crate) fn put_outcome(writer: &mut Writer, outcome: &Outcome) {
    match outcome {
        Outcome::Committed(delta) => {
            writer.u8(COMMITTED);
            put_delta(writer, *delta);
        }
        Outcome::NoChange => writer.u8(NO_CHANGE),
        Outcome::PreconditionFailed { expected, observed } => {
            writer.u8(PRECONDITION_FAILED);
            writer.u64(expected.0);
            writer.u64(observed.0);
        }
        Outcome::InvariantRejected(evidence) => {
            writer.u8(INVARIANT_REJECTED);
            put_evidence(writer, evidence);
        }
    }
}

pub(crate) fn take_outcome(reader: &mut Reader<'_>, tag: u8) -> Result<Outcome, FrameError> {
    Ok(match tag {
        COMMITTED => Outcome::Committed(take_delta(reader)?),
        NO_CHANGE => Outcome::NoChange,
        PRECONDITION_FAILED => Outcome::PreconditionFailed {
            expected: Revision(reader.u64()?),
            observed: Revision(reader.u64()?),
        },
        INVARIANT_REJECTED => Outcome::InvariantRejected(take_evidence(reader)?),
        tag => return Err(FrameError::Tag(tag)),
    })
}
