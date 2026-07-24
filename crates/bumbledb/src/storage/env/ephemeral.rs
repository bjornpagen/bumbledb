use std::path::Path;

use heed::types::Bytes;

use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;

use super::acquire_lock::acquire_lock;
use super::open_env::{OpenLane, open_env};
use super::read_meta::{
    MetaBlock, check_fingerprint, check_format_version, classify_meta_block, read_store_kind,
};
use super::{Environment, StoreKind};

impl Environment {
    /// Opens or initializes an EPHEMERAL environment at `path`
    /// (`docs/architecture/70-api.md` § environment lifecycle): a
    /// missing or empty directory is initialized fresh with the
    /// ephemeral kind marked in `_meta`; an existing ephemeral store is
    /// opened (version, kind, fingerprint — the same checks as
    /// [`Environment::open`]); a durable store refuses typed
    /// (`StoreKindMismatch`). The environment carries `MDB_NOSYNC` —
    /// the store's on-disk kind IS the no-machine-crash-durability
    /// claim, so the flag lies to no one. Everything else (NOTLS, the
    /// advisory lock, map size, reader table) is identical to a
    /// durable store.
    ///
    /// REFUSAL NEVER MUTATES — a law of the constructor, not of any
    /// flag set: an existing data file is probed first through a plain
    /// durable-flagged open, and the ephemeral flags are applied only
    /// after the probe runs EVERY check `verify_and_open` would, so
    /// every refusal fires before the flagged reopen ever holds the
    /// file. (The fixit that minted the law was WRITEMAP's open-time
    /// ftruncate, retired by cleanup-0.5.0 ruling 1; the probe-first
    /// shape stays because it keeps the reopen path itself — whatever
    /// flags the kind carries, now or later — structurally unable to
    /// touch a store it must refuse.) A refusal (`StoreKindMismatch`
    /// on a durable store, `AlreadyInitialized` on a foreign LMDB
    /// environment, `FormatMismatch`/`Corruption` on a stale or forged
    /// store, `SchemaMismatch` on a skewed fingerprint) leaves
    /// `data.mdb` byte-identical.
    ///
    /// # Errors
    ///
    /// `Io` on directory creation, `EnvironmentLocked` if another handle
    /// holds the environment, `AlreadyInitialized` on a directory
    /// holding a foreign LMDB environment, `FormatMismatch`/
    /// `StoreKindMismatch`/`SchemaMismatch` on an existing store that
    /// fails verification, `Corruption` on a missing or undecodable
    /// meta key, `Lmdb` otherwise.
    pub fn ephemeral(path: &Path, schema: &Schema) -> Result<Self> {
        std::fs::create_dir_all(path)?;
        let lock = acquire_lock(path)?;
        // The crash contract (ruled 2026-07-23, R18): a set dirty marker
        // means the last session never reached clean close — power loss,
        // or a process death — and `NOSYNC` makes the data pages
        // untrustworthy in exactly the way `_meta` cannot see (a meta
        // page flushed by incidental writeback over data pages that
        // never landed). The possibly-torn store is never opened at all:
        // wipe and re-initialize. The marker carries the claim — only
        // ephemeral opens mint one, and only a proven-synced close
        // clears it — so the wipe destroys nothing the kind promised to
        // keep.
        let marker = super::dirty_marker_path(path);
        let crashed = marker.try_exists()?;
        if crashed {
            for file in ["data.mdb", "lock.mdb"] {
                match std::fs::remove_file(path.join(file)) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(Error::Io(e)),
                }
            }
        }
        // A directory without a data file is fresh: nothing exists for
        // any open to damage, so create directly with the ephemeral
        // flags. Anything else is probed WITHOUT the flags first —
        // every refusal must fire before the flagged reopen. The
        // advisory lock (held above) keeps the probe→reopen window
        // race-free against other bumbledb handles.
        let has_meta = if !crashed && path.join("data.mdb").try_exists()? {
            Self::probe_ephemeral_kind(path, schema)?
        } else {
            false
        };
        // Set the marker, SYNCED, before the NOSYNC environment writes
        // anything — the open-side half of the kind's only fsyncs (the
        // clean close's forced data sync and marker clear is the other,
        // `Environment`'s drop). Set after the probe: a refusal must
        // leave the store byte-identical, marker included.
        std::fs::File::create(&marker)?.sync_all()?;
        super::sync_dirent_chain(path)?;
        let env = open_env(path, OpenLane::Write(StoreKind::Ephemeral))?;
        let mut opened = if has_meta {
            Self::verify_and_open(env, lock, schema, StoreKind::Ephemeral)?
        } else {
            Self::initialize(env, lock, schema, StoreKind::Ephemeral)?
        };
        opened.dirty_marker = Some(marker);
        Ok(opened)
    }

    /// The non-mutating probe over an EXISTING data file: a plain
    /// durable-flagged open (which leaves the data file's byte length
    /// and contents identical; the byte-identity is pinned by
    /// `ephemeral_refusal_on_a_durable_store_
    /// leaves_the_data_file_byte_identical` and its foreign-env,
    /// fingerprint-mismatch, and fingerprint-missing twins), one read
    /// transaction, and EVERY check [`Environment::verify_and_open`]
    /// runs — version, kind, database presence, fingerprint — so no
    /// refusal is left to fire after the mutating reopen. Returns
    /// `Ok(true)` on a verified ephemeral store (the caller reopens
    /// with the flags and re-verifies through the shared body),
    /// `Ok(false)` on a half-created store (empty root, no `_meta` —
    /// the crash window between directory creation and the meta
    /// commit), and every refusal typed:
    ///
    /// - `AlreadyInitialized` — no `_meta` but a non-empty root: a
    ///   foreign LMDB environment (never ftruncate someone else's env);
    /// - `FormatMismatch` — a pre-v5 store (version before kind, as
    ///   everywhere);
    /// - `Corruption(MetaMissing)`/`Corruption(StoreKindInvalid)` — a
    ///   v5 store whose kind marker is absent / undecodable, or whose
    ///   `_data`/`_dict`/fingerprint a torn or forged store lacks;
    /// - `StoreKindMismatch` — a durable store;
    /// - `SchemaMismatch` — an ephemeral store fingerprinted by a
    ///   different schema.
    ///
    /// The probe environment is fully dropped before this returns
    /// (heed closes the LMDB env when the last handle drops), so the
    /// caller's flagged reopen of the same path is legal.
    fn probe_ephemeral_kind(path: &Path, schema: &Schema) -> Result<bool> {
        let env = open_env(path, OpenLane::Write(StoreKind::Durable))?;
        let rtxn = env.read_txn()?;
        let MetaBlock::Present(meta) = classify_meta_block(&env, &rtxn)? else {
            return Ok(false);
        };
        check_format_version(&meta, &rtxn)?;
        let found_kind = read_store_kind(&meta, &rtxn)?;
        if found_kind != StoreKind::Ephemeral {
            return Err(Error::StoreKindMismatch {
                found: found_kind,
                expected: StoreKind::Ephemeral,
            });
        }
        // The refusals `verify_and_open` would raise past the kind
        // check, raised here instead — no refusal may wait until the
        // flagged reopen holds the file: the three databases'
        // presence, then the fingerprint.
        if env
            .open_database::<Bytes, Bytes>(&rtxn, Some("_data"))?
            .is_none()
            || env
                .open_database::<Bytes, Bytes>(&rtxn, Some("_dict"))?
                .is_none()
        {
            return Err(Error::Corruption(CorruptionError::MetaMissing));
        }
        check_fingerprint(&meta, &rtxn, schema)?;
        Ok(true)
    }
}
