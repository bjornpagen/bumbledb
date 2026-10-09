//! The persisted format: three named databases and the `meta` keys.
//!
//! A directory is a bumbledb store exactly when its `meta` `format` entry
//! equals [`FORMAT`]; anything else is refused before any write. Every
//! other meta entry is read only after that check.

use heed::RoTxn;
use heed::types::Bytes;

use crate::error::CorruptionError;
use crate::error::{Error, Result};
use crate::schema::fingerprint::SchemaFingerprint;
use bumbledb_theory::schema::RelationId;

/// The on-disk layout this build reads and writes.
pub(crate) const LAYOUT: u32 = 1;

/// Magic plus layout, stored under [`K_FORMAT`].
pub(crate) const FORMAT: [u8; 12] = {
    let mut format = *b"bumbledb\0\0\0\0";
    let layout = LAYOUT.to_be_bytes();
    format[8] = layout[0];
    format[9] = layout[1];
    format[10] = layout[2];
    format[11] = layout[3];
    format
};

pub(crate) const META_DB: &str = "meta";
pub(crate) const ROWS_DB: &str = "rows";
pub(crate) const DET_DB: &str = "det";

pub(crate) const K_FORMAT: &[u8] = &[0];
pub(crate) const K_SCHEMA: &[u8] = &[1];
pub(crate) const K_DATABASE: &[u8] = &[2];
pub(crate) const K_GENERATION: &[u8] = &[3];
pub(crate) const K_NEXT_ROW_ID: &[u8] = &[4];
pub(crate) const K_HEAD: &[u8] = &[5];
/// Per-relation `(count u64, version u64)`: `[6, relation u16]`.
const K_RELATION: u8 = 6;
/// Host records: `[7, host key]`.
pub(crate) const K_HOST: u8 = 7;

/// The database identity: the log's Genesis id, or minted at local create.
/// Copies and images of one database share it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DatabaseId(pub [u8; 16]);

impl DatabaseId {
    /// A fresh identity for a database no log names.
    #[must_use]
    pub fn mint() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NONCE: AtomicU64 = AtomicU64::new(0);
        let mut digest = crate::digest::Digest::new();
        digest.update(b"bumbledb/database-id");
        digest.update(&std::process::id().to_be_bytes());
        digest.update(&NONCE.fetch_add(1, Ordering::Relaxed).to_be_bytes());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |age| age.as_nanos());
        digest.update(&now.to_be_bytes());
        let mut id = [0; 16];
        id.copy_from_slice(&digest.finalize()[..16]);
        Self(id)
    }
}

impl std::fmt::Display for DatabaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Per-open environment identity, never persisted: plans and caches bound to
/// one open environment refuse another, even a copy of the same database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EnvironmentId(std::num::NonZeroU64);

impl EnvironmentId {
    pub(crate) fn mint() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let raw = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(std::num::NonZeroU64::new(raw).expect("environment ids start at 1"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StoreIdentity {
    pub(crate) database: DatabaseId,
    pub(crate) environment: EnvironmentId,
}

/// A local row ordinal: a storage surrogate, never an application value.
/// It orders rows of one home bucket and ranks rows for judgment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(crate) struct RowId(pub(crate) u64);

/// One relation's change version: advanced exactly when a committed
/// transaction changed that relation's rows. Equal versions of one
/// environment prove equal rows, so images keyed by version are reusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub(crate) struct RelationVersion(u64);

impl RelationVersion {
    /// The version of a relation no committed transaction ever changed.
    pub(crate) const fn initial() -> Self {
        Self(0)
    }

    #[cfg(test)]
    pub(crate) const fn from_storage(word: u64) -> Self {
        Self(word)
    }

    fn next(self) -> Result<Self> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(Error::Exhausted(crate::error::Counter::Generations))
    }
}

impl std::fmt::Display for RelationVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// One relation's live row count and change version, stored together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RelationMeta {
    pub(crate) count: u64,
    pub(crate) version: RelationVersion,
}

impl RelationMeta {
    pub(crate) fn changed(self, count: u64) -> Result<Self> {
        Ok(Self {
            count,
            version: self.version.next()?,
        })
    }

    pub(crate) fn encode(self) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&self.count.to_be_bytes());
        bytes[8..].copy_from_slice(&self.version.0.to_be_bytes());
        bytes
    }
}

/// The relation ids a store can address: one `u16` per relation.
pub(crate) fn relation_word(relation: RelationId) -> Result<[u8; 2]> {
    u16::try_from(relation.0)
        .map(u16::to_be_bytes)
        .map_err(|_| Error::ForeignSchema)
}

pub(crate) fn relation_key(relation: RelationId) -> Result<[u8; 3]> {
    let [high, low] = relation_word(relation)?;
    Ok([K_RELATION, high, low])
}

pub(crate) fn read_relation_meta(
    meta: &heed::Database<Bytes, Bytes>,
    txn: &RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
) -> Result<RelationMeta> {
    let Some(bytes) = meta
        .get(txn, &relation_key(relation)?)
        .map_err(Error::from)?
    else {
        return Ok(RelationMeta::default());
    };
    let bytes: &[u8; 16] = bytes
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MetaMissing("relation meta")))?;
    let (count, version) = bytes.split_at(8);
    Ok(RelationMeta {
        count: u64::from_be_bytes(count.try_into().expect("8 bytes")),
        version: RelationVersion(u64::from_be_bytes(version.try_into().expect("8 bytes"))),
    })
}

pub(crate) fn read_u64(
    meta: &heed::Database<Bytes, Bytes>,
    txn: &RoTxn<'_, heed::AnyTls>,
    key: &[u8],
    what: &'static str,
) -> Result<u64> {
    let bytes = meta
        .get(txn, key)
        .map_err(Error::from)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing(what)))?;
    Ok(u64::from_be_bytes(bytes.try_into().map_err(|_| {
        Error::Corruption(CorruptionError::MetaMissing(what))
    })?))
}

/// The one format check, then the schema check, against one read view.
pub(crate) fn verify_meta(
    meta: &heed::Database<Bytes, Bytes>,
    txn: &RoTxn<'_, heed::AnyTls>,
    path: &std::path::Path,
    schema_fp: &SchemaFingerprint,
) -> Result<DatabaseId> {
    if meta.get(txn, K_FORMAT).map_err(Error::from)? != Some(FORMAT.as_slice()) {
        return Err(Error::NotABumbleDb {
            path: path.to_path_buf(),
        });
    }
    let stored = meta
        .get(txn, K_SCHEMA)
        .map_err(Error::from)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing("schema")))?;
    if stored != schema_fp.0 {
        return Err(Error::SchemaMismatch);
    }
    let database = meta
        .get(txn, K_DATABASE)
        .map_err(Error::from)?
        .and_then(|bytes| <[u8; 16]>::try_from(bytes).ok())
        .ok_or(Error::Corruption(CorruptionError::MetaMissing(
            "database id",
        )))?;
    Ok(DatabaseId(database))
}
