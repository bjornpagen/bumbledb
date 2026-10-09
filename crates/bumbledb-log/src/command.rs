//! A command: one named, conditional change request, bound by its digest.

use bumbledb::{ChangeSet, Schema, WorkContext};

use crate::frame::{FrameError, Kind, Reader, Writer};
use crate::ids::{CommandDigest, RequestId, Revision};

const DIGEST_CONTEXT: &str = "bdb.command.v1 digest";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precondition {
    None,
    /// Decide only if the database is exactly at this revision.
    ExactRevision(Revision),
}

/// The identity a receipt answers for: the request and the exact command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandRef {
    pub request: RequestId,
    pub digest: CommandDigest,
}

#[derive(Debug, Clone)]
pub struct Command {
    request: RequestId,
    precondition: Precondition,
    changes: ChangeSet,
    digest: CommandDigest,
}

impl Command {
    #[must_use]
    pub fn seal(request: RequestId, precondition: Precondition, changes: ChangeSet) -> Self {
        let digest = digest(&encode(request, precondition, &changes));
        Self {
            request,
            precondition,
            changes,
            digest,
        }
    }

    /// # Errors
    /// Malformed frames and change bytes the schema refuses.
    pub fn parse(schema: &Schema, bytes: &[u8]) -> Result<Self, FrameError> {
        let mut reader = Reader::new(bytes, Kind::Command)?;
        let request = RequestId(reader.array()?);
        let precondition = match reader.u8()? {
            0 => Precondition::None,
            1 => Precondition::ExactRevision(Revision(reader.u64()?)),
            tag => return Err(FrameError::Tag(tag)),
        };
        let changes = ChangeSet::parse(schema, reader.blob()?, &WorkContext::new())?;
        reader.finish()?;
        Ok(Self {
            request,
            precondition,
            changes,
            digest: digest(bytes),
        })
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        encode(self.request, self.precondition, &self.changes)
    }

    #[must_use]
    pub const fn reference(&self) -> CommandRef {
        CommandRef {
            request: self.request,
            digest: self.digest,
        }
    }

    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn precondition(&self) -> Precondition {
        self.precondition
    }

    #[must_use]
    pub const fn changes(&self) -> &ChangeSet {
        &self.changes
    }
}

fn encode(request: RequestId, precondition: Precondition, changes: &ChangeSet) -> Vec<u8> {
    let mut writer = Writer::new(Kind::Command);
    writer.array(&request.0);
    match precondition {
        Precondition::None => writer.u8(0),
        Precondition::ExactRevision(revision) => {
            writer.u8(1);
            writer.u64(revision.0);
        }
    }
    writer.blob(changes.as_bytes());
    writer.finish()
}

fn digest(encoded: &[u8]) -> CommandDigest {
    let mut hasher = blake3::Hasher::new_derive_key(DIGEST_CONTEXT);
    hasher.update(encoded);
    CommandDigest(*hasher.finalize().as_bytes())
}
