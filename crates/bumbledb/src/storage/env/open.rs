use std::path::Path;
use std::sync::atomic::Ordering;

use heed::types::Bytes;
use heed::{Database, WithoutTls};

use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;

use super::acquire_lock::acquire_lock;
use super::open_env::open_env;
use super::read_meta::{check_fingerprint, read_store_kind, read_u32};
use super::{Environment, FORMAT_VERSION, META_FORMAT_VERSION, NEXT_INSTANCE, StoreKind};

impl Environment {
    /// Opens an existing DURABLE environment, verifying the storage
    /// format version first, the store kind second, and the schema
    /// fingerprint third — each mismatch is a distinct hard failure.
    ///
    /// # Errors
    ///
    /// `EnvironmentLocked` if another handle holds the environment;
    /// `FormatMismatch`, then `StoreKindMismatch`, then `SchemaMismatch`;
    /// `Corruption(MetaMissing)` if the environment lacks bumbledb's
    /// databases or meta keys; `Corruption(StoreKindInvalid)` on a
    /// present-but-undecodable kind marker; `Lmdb` otherwise.
    pub fn open(path: &Path, schema: &Schema) -> Result<Self> {
        let lock = acquire_lock(path)?;
        let env = open_env(path, StoreKind::Durable)?;
        Self::verify_and_open(env, lock, schema, StoreKind::Durable)
    }

    /// The shared open body ([`Environment::open`] durable,
    /// [`Environment::ephemeral`]'s existing-store arm): version, then
    /// kind, then fingerprint. The kind check is what makes the store
    /// kind parse-don't-validate: a durable constructor can never hold
    /// an ephemeral store's handle, nor the reverse.
    pub(super) fn verify_and_open(
        env: heed::Env<WithoutTls>,
        lock: std::fs::File,
        schema: &Schema,
        expected_kind: StoreKind,
    ) -> Result<Self> {
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
        let found_kind = read_store_kind(&meta, &rtxn)?;
        if found_kind != expected_kind {
            return Err(Error::StoreKindMismatch {
                found: found_kind,
                expected: expected_kind,
            });
        }
        check_fingerprint(&meta, &rtxn, schema)?;
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
