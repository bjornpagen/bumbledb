use std::path::Path;
use std::sync::atomic::Ordering;

use heed::Database;
use heed::types::Bytes;

use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;
use crate::schema::fingerprint::{SchemaFingerprint, fingerprint};

use super::acquire_lock::acquire_lock;
use super::open_env::open_env;
use super::read_meta::read_u32;
use super::{Environment, FORMAT_VERSION, META_FINGERPRINT, META_FORMAT_VERSION, NEXT_INSTANCE};

impl Environment {
    /// Opens an existing environment, verifying the storage format version
    /// first and the schema fingerprint second — each mismatch is a distinct
    /// hard failure.
    ///
    /// # Errors
    ///
    /// `EnvironmentLocked` if another handle holds the environment;
    /// `FormatMismatch`, then `SchemaMismatch`; `Corruption(MetaMissing)` if
    /// the environment lacks bumbledb's databases or meta keys; `Lmdb`
    /// otherwise.
    pub fn open(path: &Path, schema: &Schema) -> Result<Self> {
        let lock = acquire_lock(path)?;
        let env = open_env(path)?;
        // Database handles opened inside a transaction are private to it
        // until that transaction commits (LMDB dbi semantics): a read txn
        // would invalidate them on drop, so registration goes through a
        // write transaction that commits without writing anything.
        let rtxn = env.write_txn()?;
        let meta: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_meta"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let data: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_data"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let dict: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_dict"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;

        let found_version = read_u32(&meta, &rtxn, META_FORMAT_VERSION)?;
        if found_version != FORMAT_VERSION {
            return Err(Error::FormatMismatch {
                found: found_version,
                expected: FORMAT_VERSION,
            });
        }
        let stored: [u8; 32] = meta
            .get(&rtxn, META_FINGERPRINT)?
            .and_then(|b| b.try_into().ok())
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let expected = fingerprint(schema);
        if stored != expected.0 {
            return Err(Error::SchemaMismatch {
                found: SchemaFingerprint(stored),
                expected,
            });
        }
        rtxn.commit()?;
        Ok(Self {
            env,
            meta,
            data,
            dict,
            instance: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
            _lock: lock,
        })
    }
}
