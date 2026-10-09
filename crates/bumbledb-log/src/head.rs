//! The applied head: everything the protocol knows about the state at one
//! log position, stored with the facts it describes.

use bumbledb::SchemaFingerprint;

use crate::entry::{Freeze, MigrationId, put_migration_id, take_migration_id};
use crate::frame::{FrameError, Kind, Reader, Writer};
use crate::ids::{DatabaseId, Millis, Revision, Seq};
use crate::receipt::{Evidence, put_evidence, take_evidence, take_seq};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub database: DatabaseId,
    pub seq: Seq,
    pub revision: Revision,
    pub schema: SchemaFingerprint,
    pub ledger: Ledger,
    pub mode: Mode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Open,
    /// `since` is the Freeze object's store-assigned time.
    Frozen {
        freeze: Freeze,
        since: Millis,
    },
}

impl Mode {
    /// When anyone may thaw a frozen database.
    #[must_use]
    pub fn deadline(&self) -> Option<Millis> {
        match self {
            Self::Open => None,
            Self::Frozen { freeze, since } => {
                Some(Millis(since.0.saturating_add(freeze.lease_millis)))
            }
        }
    }
}

/// Applied migrations in order, and the sticky rejections.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ledger {
    pub applied: Vec<MigrationId>,
    pub rejected: Vec<Rejection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    pub migration: MigrationId,
    pub evidence: Evidence,
}

/// How the bundled migrations relate to the applied ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Equal,
    /// The ledger is a proper prefix of the bundle; `next` is the bundle
    /// index of the first pending migration.
    Behind {
        next: usize,
    },
    /// The bundle is a proper prefix of the ledger: this code is older.
    Ahead,
    /// The two disagree at `index` (name or hash).
    Diverged {
        index: usize,
    },
}

impl Ledger {
    #[must_use]
    pub fn compare(&self, bundled: &[MigrationId]) -> Comparison {
        let applied = &self.applied;
        match applied.iter().zip(bundled).position(|(a, b)| a != b) {
            Some(index) => Comparison::Diverged { index },
            None if applied.len() == bundled.len() => Comparison::Equal,
            None if applied.len() < bundled.len() => Comparison::Behind {
                next: applied.len(),
            },
            None => Comparison::Ahead,
        }
    }

    /// The recorded rejection of any bundled migration.
    #[must_use]
    pub fn rejection_of<'a>(&'a self, bundled: &[MigrationId]) -> Option<&'a Rejection> {
        self.rejected
            .iter()
            .find(|rejection| bundled.contains(&rejection.migration))
    }
}

const OPEN: u8 = 0;
const FROZEN: u8 = 1;

impl Head {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = Writer::new(Kind::Head);
        writer.array(&self.database.0);
        writer.u64(self.seq.get());
        writer.u64(self.revision.0);
        writer.array(&self.schema.0);
        writer.len(self.ledger.applied.len());
        for id in &self.ledger.applied {
            put_migration_id(&mut writer, id);
        }
        writer.len(self.ledger.rejected.len());
        for rejection in &self.ledger.rejected {
            put_migration_id(&mut writer, &rejection.migration);
            put_evidence(&mut writer, &rejection.evidence);
        }
        match &self.mode {
            Mode::Open => writer.u8(OPEN),
            Mode::Frozen { freeze, since } => {
                writer.u8(FROZEN);
                put_migration_id(&mut writer, &freeze.migration);
                writer.u64(freeze.lease_millis);
                writer.u64(since.0);
            }
        }
        writer.finish()
    }

    /// # Errors
    /// Malformed frames.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let mut reader = Reader::new(bytes, Kind::Head)?;
        let database = DatabaseId(reader.array()?);
        let seq = take_seq(&mut reader)?;
        let revision = Revision(reader.u64()?);
        let schema = SchemaFingerprint(reader.array()?);
        let applied = (0..reader.len(36)?)
            .map(|_| take_migration_id(&mut reader))
            .collect::<Result<_, _>>()?;
        let rejected = (0..reader.len(40)?)
            .map(|_| {
                Ok(Rejection {
                    migration: take_migration_id(&mut reader)?,
                    evidence: take_evidence(&mut reader)?,
                })
            })
            .collect::<Result<_, FrameError>>()?;
        let mode = match reader.u8()? {
            OPEN => Mode::Open,
            FROZEN => Mode::Frozen {
                freeze: Freeze {
                    migration: take_migration_id(&mut reader)?,
                    lease_millis: reader.u64()?,
                },
                since: Millis(reader.u64()?),
            },
            tag => return Err(FrameError::Tag(tag)),
        };
        reader.finish()?;
        Ok(Self {
            database,
            seq,
            revision,
            schema,
            ledger: Ledger { applied, rejected },
            mode,
        })
    }
}
