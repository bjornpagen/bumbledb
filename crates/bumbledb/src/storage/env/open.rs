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
    /// kind, then the database roster, then fingerprint — ONE check
    /// precedence, shared verbatim with the ephemeral probe
    /// (`Environment::probe_ephemeral_kind`), so a store carrying two
    /// faults gets the same diagnosis from every constructor. Version
    /// precedes the roster because a pre-v5 store's database layout is
    /// not this version's to judge: convicting it of corruption for
    /// lacking `_data` would misname a merely-old store. The kind check
    /// is what makes the store kind parse-don't-validate: a durable
    /// constructor can never hold an ephemeral store's handle, nor the
    /// reverse.
    pub(super) fn verify_and_open(
        env: heed::Env<WithoutTls>,
        lock: std::fs::File,
        schema: &Schema,
        expected_kind: StoreKind,
    ) -> Result<Self> {
        // Database handles opened inside a transaction are private to it
        // until that transaction commits (LMDB dbi semantics): a read txn
        // would invalidate them on drop, so registration goes through a
        // write transaction — which normally commits without writing
        // anything, except for the one descriptor back-fill below.
        let mut wtxn = env.write_txn()?;
        let meta: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_meta"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let found_version = read_u32(&meta, &wtxn, META_FORMAT_VERSION)?;
        if found_version != FORMAT_VERSION {
            return Err(Error::FormatMismatch {
                found: found_version,
                expected: FORMAT_VERSION,
            });
        }
        let found_kind = read_store_kind(&meta, &wtxn)?;
        if found_kind != expected_kind {
            return Err(Error::StoreKindMismatch {
                found: found_kind,
                expected: expected_kind,
            });
        }
        let data: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_data"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let dict: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_dict"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        check_fingerprint(&meta, &wtxn, schema)?;
        // The descriptor back-fill — the adoption path for every store
        // created before descriptors were persisted
        // (`docs/architecture/50-storage.md` § the `_meta` block): the
        // fingerprint just verified proves the caller's theory IS the
        // creating theory, so its canonical bytes are the store's own
        // descriptor, written here in this open's committed transaction.
        // Absence is the only trigger — a present descriptor is never
        // rewritten (`Db::verify_store` convicts a desynced one; open
        // stays lean). The storage tx id is untouched: the descriptor is
        // not query-visible state, so images and witnesses stay valid.
        if meta.get(&wtxn, super::META_SCHEMA_DESCRIPTOR)?.is_none() {
            let descriptor = crate::schema::fingerprint::canonical_descriptor(schema);
            meta.put(
                &mut wtxn,
                super::META_SCHEMA_DESCRIPTOR,
                descriptor.as_slice(),
            )?;
        }
        wtxn.commit()?;
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
