use std::path::Path;

use heed::types::Bytes;
use heed::{Database, WithoutTls};

use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;

use super::Environment;
use super::acquire_lock::acquire_lock;
use super::open_env::{OpenLane, open_env};
use super::read_meta::{MetaBlock, classify_meta_block, parse_meta, parse_meta_head};

impl Environment {
    /// Opens an existing environment, verifying the storage format
    /// version then the schema fingerprint — each mismatch is a
    /// distinct hard failure. Format 8 only; every earlier version is
    /// [`Error::FormatMismatch`].
    ///
    /// # Errors
    ///
    /// `EnvironmentLocked` if another handle holds the environment;
    /// `AlreadyInitialized` on a half-created empty root or a foreign
    /// LMDB environment; `FormatMismatch`, then `SchemaMismatch`;
    /// `Corruption(MetaMissing)` if an initialized store lacks
    /// bumbledb's databases or meta keys; `Lmdb` otherwise.
    pub fn open(path: &Path, schema: &Schema) -> Result<Self> {
        Self::open_lane(path, schema, OpenLane::Write)
    }

    /// Bench-only NOSYNC open of a published store. Not a store kind.
    #[doc(hidden)]
    pub(crate) fn open_nosync(path: &Path, schema: &Schema) -> Result<Self> {
        Self::open_lane(path, schema, OpenLane::Nosync)
    }

    fn open_lane(path: &Path, schema: &Schema, lane: OpenLane) -> Result<Self> {
        let lock = acquire_lock(path)?;
        let env = open_env(path, lane)?;
        Self::verify_and_open(env, lock, schema)
    }

    /// Shared open body: version, then fingerprint. Named databases
    /// still have to exist (`MetaMissing`); they are not `_meta` keys.
    pub(super) fn verify_and_open(
        env: heed::Env<WithoutTls>,
        lock: std::fs::File,
        schema: &Schema,
    ) -> Result<Self> {
        let wtxn = env.write_txn()?;
        let meta = match classify_meta_block(&env, &wtxn)? {
            MetaBlock::Present(meta) => meta,
            MetaBlock::HalfCreated => return Err(Error::AlreadyInitialized),
        };
        parse_meta_head(&meta, &wtxn)?;
        let data: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_data"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let dict: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_dict"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let store = parse_meta(&meta, &wtxn)?;
        store.matches_schema(schema)?;
        wtxn.commit()?;
        Ok(Self::assemble(env, meta, data, dict, lock))
    }
}
