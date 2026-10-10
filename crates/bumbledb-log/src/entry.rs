//! Log entries: the immutable objects at `log/{seq}`. A commands entry
//! carries its writer's judgment against the head it was decided at; every
//! other entry carries its whole effect.

use bumbledb::{Schema, SchemaFingerprint};

use crate::command::Command;
use crate::frame::{FrameError, Kind, Reader, Writer};
use crate::head::Rejection;
use crate::ids::{DatabaseId, ImageDigest, MigrationHash, Nonce, Seq};
use crate::receipt::{Outcome, put_evidence, put_outcome, take_evidence, take_outcome, take_seq};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub nonce: Nonce,
    pub body: Body,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Genesis(Genesis),
    Commands(Batch),
    Freeze(Freeze),
    Migration(Migration),
    Thaw(Thaw),
}

/// Commands judged in order against the head at `base`, whose schema is
/// `schema`. The recorded outcomes are a memo of that judgment: they hold
/// where the batch lands right after `base`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub base: Seq,
    pub schema: SchemaFingerprint,
    /// Never empty; encoded, because only `schema` reads them.
    proposals: Box<[u8]>,
}

/// One command and its writer's judgment at the batch's base.
#[derive(Debug, Clone)]
pub struct Proposal {
    pub command: Command,
    pub outcome: Outcome,
}

/// Creates an empty database at the schema of its initial migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Genesis {
    pub database: DatabaseId,
    pub initial: MigrationId,
    pub schema: SchemaFingerprint,
}

/// One bundled migration, named and content-hashed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MigrationId {
    pub name: Box<str>,
    pub hash: MigrationHash,
}

/// Holds every command until a `Migration` or `Thaw` lands, or until `lease`
/// past the object's store-assigned `Last-Modified`, when anyone may thaw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Freeze {
    pub migration: MigrationId,
    pub lease_millis: u64,
}

/// Replaces the state with the image at `mig/{image}`, built at `schema`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub migration: MigrationId,
    pub schema: SchemaFingerprint,
    pub image: ImageDigest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Thaw {
    /// Released without a verdict; the migration may run again.
    Lifted,
    /// The migrated state broke the new schema's laws. Sticky: code bundling
    /// this exact migration refuses to open.
    Rejected(Rejection),
}

const GENESIS: u8 = 1;
const COMMANDS: u8 = 2;
const FREEZE: u8 = 3;
const MIGRATION: u8 = 4;
const THAW: u8 = 5;

const LIFTED: u8 = 1;
const REJECTED: u8 = 2;

/// A lower bound on an encoded proposal: the command blob's length, the
/// command frame's tag, request, precondition tag and changes length, then
/// the outcome tag.
const MIN_PROPOSAL: usize = 4 + 15 + 16 + 1 + 4 + 1;

impl Batch {
    /// `None` when `proposals` is empty.
    #[must_use]
    pub fn new(base: Seq, schema: SchemaFingerprint, proposals: &[Proposal]) -> Option<Self> {
        if proposals.is_empty() {
            return None;
        }
        let mut writer = Writer::untagged();
        writer.len(proposals.len());
        for proposal in proposals {
            writer.blob(&proposal.command.encode());
            put_outcome(&mut writer, &proposal.outcome);
        }
        Some(Self {
            base,
            schema,
            proposals: writer.finish().into(),
        })
    }

    /// The proposals, read with the schema the batch was judged at.
    /// # Errors
    /// Malformed proposals, none at all, and change bytes `schema` refuses.
    pub fn proposals(&self, schema: &Schema) -> Result<Box<[Proposal]>, FrameError> {
        let mut reader = Reader::untagged(&self.proposals);
        let len = reader.len(MIN_PROPOSAL)?;
        if len == 0 {
            return Err(FrameError::Value);
        }
        let mut proposals = Vec::with_capacity(len);
        for _ in 0..len {
            let command = Command::parse(schema, reader.blob()?)?;
            let tag = reader.u8()?;
            let outcome = take_outcome(&mut reader, tag)?;
            proposals.push(Proposal { command, outcome });
        }
        reader.finish()?;
        Ok(proposals.into())
    }
}

impl Entry {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = Writer::new(Kind::Entry);
        writer.array(&self.nonce.0);
        match &self.body {
            Body::Genesis(genesis) => {
                writer.u8(GENESIS);
                writer.array(&genesis.database.0);
                put_migration_id(&mut writer, &genesis.initial);
                writer.array(&genesis.schema.0);
            }
            Body::Commands(batch) => {
                writer.u8(COMMANDS);
                writer.u64(batch.base.get());
                writer.array(&batch.schema.0);
                writer.array(&batch.proposals);
            }
            Body::Freeze(freeze) => {
                writer.u8(FREEZE);
                put_migration_id(&mut writer, &freeze.migration);
                writer.u64(freeze.lease_millis);
            }
            Body::Migration(migration) => {
                writer.u8(MIGRATION);
                put_migration_id(&mut writer, &migration.migration);
                writer.array(&migration.schema.0);
                writer.array(&migration.image.0);
            }
            Body::Thaw(Thaw::Lifted) => {
                writer.u8(THAW);
                writer.u8(LIFTED);
            }
            Body::Thaw(Thaw::Rejected(rejection)) => {
                writer.u8(THAW);
                writer.u8(REJECTED);
                put_migration_id(&mut writer, &rejection.migration);
                put_evidence(&mut writer, &rejection.evidence);
            }
        }
        writer.finish()
    }

    /// Parse an entry; a batch's proposals are read later, at its schema.
    /// # Errors
    /// Malformed frames.
    pub fn parse(bytes: &[u8]) -> Result<Self, FrameError> {
        let mut reader = Reader::new(bytes, Kind::Entry)?;
        let nonce = Nonce(reader.array()?);
        let body = match reader.u8()? {
            GENESIS => Body::Genesis(Genesis {
                database: DatabaseId(reader.array()?),
                initial: take_migration_id(&mut reader)?,
                schema: SchemaFingerprint(reader.array()?),
            }),
            COMMANDS => Body::Commands(Batch {
                base: take_seq(&mut reader)?,
                schema: SchemaFingerprint(reader.array()?),
                proposals: reader.rest().into(),
            }),
            FREEZE => Body::Freeze(Freeze {
                migration: take_migration_id(&mut reader)?,
                lease_millis: reader.u64()?,
            }),
            MIGRATION => Body::Migration(Migration {
                migration: take_migration_id(&mut reader)?,
                schema: SchemaFingerprint(reader.array()?),
                image: ImageDigest(reader.array()?),
            }),
            THAW => Body::Thaw(match reader.u8()? {
                LIFTED => Thaw::Lifted,
                REJECTED => Thaw::Rejected(Rejection {
                    migration: take_migration_id(&mut reader)?,
                    evidence: take_evidence(&mut reader)?,
                }),
                tag => return Err(FrameError::Tag(tag)),
            }),
            tag => return Err(FrameError::Tag(tag)),
        };
        reader.finish()?;
        Ok(Self { nonce, body })
    }
}

pub(crate) fn put_migration_id(writer: &mut Writer, id: &MigrationId) {
    writer.text(&id.name);
    writer.array(&id.hash.0);
}

pub(crate) fn take_migration_id(reader: &mut Reader<'_>) -> Result<MigrationId, FrameError> {
    Ok(MigrationId {
        name: reader.text()?.into(),
        hash: MigrationHash(reader.array()?),
    })
}
