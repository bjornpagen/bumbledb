use std::path::Path;
use std::sync::atomic::Ordering;

use heed::Database;
use heed::types::Bytes;

use crate::error::{CorruptionError, Error, Result};

use super::acquire_lock::acquire_lock;
use super::open_env::open_env;
use super::read_meta::{read_fingerprint, read_store_kind, read_u32};
use super::{
    Environment, FORMAT_VERSION, META_FORMAT_VERSION, META_SCHEMA_DESCRIPTOR, NEXT_INSTANCE,
    StoreKind,
};

/// What [`Environment::exhume`] hands the API layer: the opened
/// environment plus the raw self-description the store carries — the
/// caller (the one exhume entry, `crate::exhume`) verifies the descriptor
/// hash against the fingerprint and decodes; this layer only reads.
pub(crate) struct ExhumedEnvironment {
    pub(crate) env: Environment,
    pub(crate) kind: StoreKind,
    /// The stored `_meta` fingerprint, raw.
    pub(crate) fingerprint: [u8; 32],
    /// The persisted canonical schema-descriptor bytes.
    pub(crate) descriptor: Vec<u8>,
}

impl Environment {
    /// Opens an existing environment FROM ITS OWN DESCRIPTION — no
    /// caller-supplied theory anywhere (`docs/architecture/70-api.md`
    /// § exhume). The open-time precedence holds where it applies:
    /// format version first, then the store-kind marker — read and
    /// validated but never compared against an expectation, because
    /// exhume takes no durability decision and reads BOTH kinds (the
    /// environment opens with plain durable flags either way: `NOSYNC`
    /// is a write-path affordance and exhume never writes — the
    /// ephemeral probe's precedent). There is no fingerprint CHECK here:
    /// with no theory in hand there is nothing to compare, and the
    /// caller's descriptor-hash verification is the integrity gate.
    ///
    /// Holds the same exclusive advisory lock as every other constructor
    /// (one handle per path — the record being read stays still); what
    /// an exhumed handle never takes is the writer path, which the API
    /// layer does not expose.
    ///
    /// # Errors
    ///
    /// `Io` on a nonexistent path, `EnvironmentLocked` if another handle
    /// holds the environment, `FormatMismatch` on any other version,
    /// `Corruption(MetaMissing)`/`Corruption(StoreKindInvalid)` on a
    /// store missing its databases or meta keys, `DescriptorMissing` on
    /// a store not yet adopted (the remedy: one `Db::open` under the
    /// creating schema back-fills it), `Lmdb` otherwise.
    pub(crate) fn exhume(path: &Path) -> Result<ExhumedEnvironment> {
        let lock = acquire_lock(path)?;
        let env = open_env(path, StoreKind::Durable)?;
        // Dbi registration goes through a write transaction exactly as
        // `verify_and_open`'s does; it commits without writing anything
        // — exhume never mutates a store, not even to adopt it.
        let wtxn = env.write_txn()?;
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
        let kind = read_store_kind(&meta, &wtxn)?;
        let data: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_data"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let dict: Database<Bytes, Bytes> = env
            .open_database(&wtxn, Some("_dict"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let fingerprint = read_fingerprint(&meta, &wtxn)?;
        let descriptor = meta
            .get(&wtxn, META_SCHEMA_DESCRIPTOR)?
            .map(<[u8]>::to_vec)
            .ok_or(Error::DescriptorMissing)?;
        wtxn.commit()?;
        Ok(ExhumedEnvironment {
            env: Self {
                env,
                meta,
                data,
                dict,
                instance: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
                _lock: lock,
            },
            kind,
            fingerprint,
            descriptor,
        })
    }
}
