//! Log entries: the immutable objects at `log/{seq}`. An entry carries what
//! was decided, so applying it never re-judges.

use bumbledb::{ChangeSet, Schema, SchemaFingerprint, WorkContext};

use crate::command::CommandRef;
use crate::frame::{FrameError, Kind, Reader, Writer};
use crate::head::Rejection;
use crate::ids::{DatabaseId, ImageDigest, MigrationHash, Nonce, Revision};
use crate::receipt::{
    Delta, Evidence, Outcome, put_command, put_evidence, put_outcome, take_command, take_evidence,
    take_outcome,
};

#[derive(Debug, Clone)]
pub struct Entry {
    pub nonce: Nonce,
    pub body: Body,
}

#[derive(Debug, Clone)]
pub enum Body {
    Genesis(Genesis),
    /// Commands decided in order, each against the state the earlier ones
    /// produced. Never empty.
    Commands(Box<[Decided]>),
    Freeze(Freeze),
    Migration(Migration),
    Thaw(Thaw),
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

#[derive(Debug, Clone)]
pub struct Decided {
    pub command: CommandRef,
    pub verdict: Verdict,
}

/// A decision as the log records it: a commit carries its changes.
#[derive(Debug, Clone)]
pub enum Verdict {
    Committed {
        changes: ChangeSet,
        delta: Delta,
    },
    NoChange,
    PreconditionFailed {
        expected: Revision,
        observed: Revision,
    },
    InvariantRejected(Evidence),
}

impl Verdict {
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        match self {
            Self::Committed { delta, .. } => Outcome::Committed(*delta),
            Self::NoChange => Outcome::NoChange,
            Self::PreconditionFailed { expected, observed } => Outcome::PreconditionFailed {
                expected: *expected,
                observed: *observed,
            },
            Self::InvariantRejected(evidence) => Outcome::InvariantRejected(evidence.clone()),
        }
    }
}

const GENESIS: u8 = 1;
const COMMANDS: u8 = 2;
const FREEZE: u8 = 3;
const MIGRATION: u8 = 4;
const THAW: u8 = 5;

const LIFTED: u8 = 1;
const REJECTED: u8 = 2;

/// The smallest encoded decided command: ref, outcome tag.
const MIN_DECIDED: usize = 16 + 32 + 1;

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
            Body::Commands(decided) => {
                writer.u8(COMMANDS);
                writer.len(decided.len());
                for decided in decided {
                    put_command(&mut writer, decided.command);
                    put_outcome(&mut writer, &decided.verdict.outcome());
                    if let Verdict::Committed { changes, .. } = &decided.verdict {
                        writer.blob(changes.as_bytes());
                    }
                }
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

    /// Parse an entry; committed changes must belong to `schema`.
    /// # Errors
    /// Malformed frames and change bytes the schema refuses.
    pub fn parse(schema: &Schema, bytes: &[u8]) -> Result<Self, FrameError> {
        let mut reader = Reader::new(bytes, Kind::Entry)?;
        let nonce = Nonce(reader.array()?);
        let body = match reader.u8()? {
            GENESIS => Body::Genesis(Genesis {
                database: DatabaseId(reader.array()?),
                initial: take_migration_id(&mut reader)?,
                schema: SchemaFingerprint(reader.array()?),
            }),
            COMMANDS => {
                let len = reader.len(MIN_DECIDED)?;
                if len == 0 {
                    return Err(FrameError::Value);
                }
                let work = WorkContext::new();
                let mut decided = Vec::with_capacity(len);
                for _ in 0..len {
                    let command = take_command(&mut reader)?;
                    let tag = reader.u8()?;
                    let verdict = match take_outcome(&mut reader, tag)? {
                        Outcome::Committed(delta) => Verdict::Committed {
                            changes: ChangeSet::parse(schema, reader.blob()?, &work)?,
                            delta,
                        },
                        Outcome::NoChange => Verdict::NoChange,
                        Outcome::PreconditionFailed { expected, observed } => {
                            Verdict::PreconditionFailed { expected, observed }
                        }
                        Outcome::InvariantRejected(evidence) => {
                            Verdict::InvariantRejected(evidence)
                        }
                    };
                    decided.push(Decided { command, verdict });
                }
                Body::Commands(decided.into())
            }
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
