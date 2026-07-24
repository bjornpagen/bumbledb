use std::path::Path;
use std::sync::atomic::Ordering;

use heed::Database;
use heed::types::Bytes;

use crate::error::{CorruptionError, Error, Result};

use super::open_env::{OpenLane, open_env};
use super::read_meta::{check_format_version, read_fingerprint, read_store_kind};
use super::{Environment, META_SCHEMA_DESCRIPTOR, NEXT_INSTANCE, StoreKind};

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
    /// exhume takes no durability decision and reads BOTH kinds. There
    /// is no fingerprint CHECK here: with no theory in hand there is
    /// nothing to compare, and the caller's descriptor-hash verification
    /// is the integrity gate.
    ///
    /// Genuinely read-only down to the storage layer (the lock law is a
    /// writer law — ruled 2026-07-23, R17): the environment opens
    /// `MDB_RDONLY` through the read-only lane of the one raw-open
    /// chokepoint, dbis register through a read transaction, and no
    /// advisory lock is taken — a read-only environment can corrupt
    /// nothing, so there is nothing for a lock to protect. The archival
    /// lane thereby works on read-only media, restored snapshots, and
    /// mounted backups, with no carve-outs; from a read-only environment
    /// the write path is unrepresentable (LMDB refuses write
    /// transactions outright).
    ///
    /// # Errors
    ///
    /// `Io` on a nonexistent path, `FormatMismatch` on any other
    /// version, `Corruption(MetaMissing)`/`Corruption(StoreKindInvalid)`
    /// on a store missing its databases or meta keys,
    /// `DescriptorMissing` on a store not yet adopted (the remedy: one
    /// `Db::open` under the creating schema back-fills it), `Lmdb`
    /// otherwise.
    pub(crate) fn exhume(path: &Path) -> Result<ExhumedEnvironment> {
        let env = open_env(path, OpenLane::ReadOnly)?;
        // Dbi registration through a read transaction — LMDB opens
        // existing named databases read-only, so the old
        // write-txn-that-writes-nothing oddity is gone with the lock.
        let rtxn = env.read_txn()?;
        let meta: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_meta"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        check_format_version(&meta, &rtxn)?;
        let kind = read_store_kind(&meta, &rtxn)?;
        let data: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_data"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let dict: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some("_dict"))?
            .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
        let fingerprint = read_fingerprint(&meta, &rtxn)?;
        let descriptor = meta
            .get(&rtxn, META_SCHEMA_DESCRIPTOR)?
            .map(<[u8]>::to_vec)
            .ok_or(Error::DescriptorMissing)?;
        // Committed, not dropped: LMDB keeps txn-opened dbi handles alive
        // past a COMMIT only (an abort closes them) — a read commit
        // writes nothing, so the read-only lane stays read-only.
        rtxn.commit()?;
        Ok(ExhumedEnvironment {
            env: Self {
                env,
                meta,
                data,
                dict,
                instance: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
                // The lock law is a writer law (R17): the read-only lane
                // holds none.
                _lock: None,
                dirty_marker: None,
            },
            kind,
            fingerprint,
            descriptor,
        })
    }
}
