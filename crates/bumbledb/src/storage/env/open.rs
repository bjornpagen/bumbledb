use std::path::Path;

use heed::types::Bytes;
use heed::{Database, WithoutTls};

use crate::error::{CorruptionError, Error, Mismatch, Result};
use crate::schema::Schema;

use super::acquire_lock::acquire_lock;
use super::open_env::{OpenLane, open_env};
use super::read_meta::{MetaBlock, classify_meta_block, parse_meta, parse_meta_head};
use super::{Environment, StoreKind};

impl Environment {
    /// Opens an existing DURABLE environment, verifying the storage
    /// format version first, the store kind second, the database roster
    /// third, the schema fingerprint fourth, and the persisted
    /// descriptor last — each mismatch is a distinct hard failure.
    /// Format 8 only; every earlier version is [`Error::FormatMismatch`].
    ///
    /// # Errors
    ///
    /// `EnvironmentLocked` if another handle holds the environment;
    /// `AlreadyInitialized` on a half-created empty root or a foreign
    /// LMDB environment; `FormatMismatch`, then `StoreKindMismatch`,
    /// then `SchemaMismatch`; `Corruption(MetaMissing)` if an
    /// initialized store lacks bumbledb's databases or meta keys
    /// (descriptor included); `Corruption(DescriptorFingerprintDesync)`
    /// when the stored descriptor is not the fingerprint's preimage;
    /// `Corruption(StoreKindInvalid)` on a present-but-undecodable kind
    /// marker; `Lmdb` otherwise.
    pub fn open(path: &Path, schema: &Schema) -> Result<Self> {
        let lock = acquire_lock(path)?;
        let env = open_env(path, OpenLane::Write(StoreKind::Durable))?;
        let opened = Self::verify_and_open(env, lock, schema, StoreKind::Durable, None)?;
        super::clear_orphan_marker(path)?;
        Ok(opened)
    }

    /// Shared open body: [`parse_meta`] then kind, roster, schema.
    /// Version and kind are the first two fields of the parse; roster
    /// stays a database check, not a `_meta` key.
    pub(super) fn verify_and_open(
        env: heed::Env<WithoutTls>,
        lock: std::fs::File,
        schema: &Schema,
        expected_kind: StoreKind,
        dirty_marker: Option<std::path::PathBuf>,
    ) -> Result<Self> {
        let wtxn = env.write_txn()?;
        let meta = match classify_meta_block(&env, &wtxn)? {
            MetaBlock::Present(meta) => meta,
            MetaBlock::HalfCreated => return Err(Error::AlreadyInitialized),
        };
        let (_, kind) = parse_meta_head(&meta, &wtxn)?;
        if kind != expected_kind {
            return Err(Error::StoreKindMismatch {
                mismatch: Mismatch {
                    witnessed: kind,
                    required: expected_kind,
                },
            });
        }
        let data: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_data"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let dict: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_dict"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let store = parse_meta(&meta, &wtxn)?;
        store.matches_schema(schema)?;
        wtxn.commit()?;
        Ok(Self::assemble(
            env,
            meta,
            data,
            dict,
            Self::mode_for(expected_kind, lock, dirty_marker),
        ))
    }
}
