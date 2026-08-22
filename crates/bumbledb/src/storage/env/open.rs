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
    /// # Errors

    /// LMDB environment; `FormatMismatch`, then `SchemaMismatch`;
    pub fn open(path: &Path, schema: &Schema) -> Result<Self> {
        Self::open_lane(path, schema, OpenLane::Write)
    }

    #[doc(hidden)]
    pub(crate) fn open_nosync(path: &Path, schema: &Schema) -> Result<Self> {
        Self::open_lane(path, schema, OpenLane::Nosync)
    }

    fn open_lane(path: &Path, schema: &Schema, lane: OpenLane) -> Result<Self> {
        let lock = acquire_lock(path)?;
        let env = open_env(path, lane)?;
        Self::verify_and_open(env, lock, schema)
    }

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
