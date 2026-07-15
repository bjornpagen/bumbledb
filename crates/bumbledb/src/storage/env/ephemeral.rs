use std::path::Path;

use heed::types::Bytes;

use crate::error::{Error, Result};
use crate::schema::Schema;

use super::acquire_lock::acquire_lock;
use super::open_env::open_env;
use super::read_meta::{read_store_kind, read_u32};
use super::{Environment, FORMAT_VERSION, META_FORMAT_VERSION, StoreKind};

impl Environment {
    /// Opens or initializes an EPHEMERAL environment at `path`
    /// (`docs/architecture/70-api.md` § environment lifecycle): a
    /// missing or empty directory is initialized fresh with the
    /// ephemeral kind marked in `_meta`; an existing ephemeral store is
    /// opened (version, kind, fingerprint — the same checks as
    /// [`Environment::open`]); a durable store refuses typed
    /// (`StoreKindMismatch`). The environment carries
    /// `MDB_WRITEMAP|MDB_NOSYNC` — the store's on-disk kind IS the
    /// no-machine-crash-durability claim, so the flags lie to no one.
    /// Everything else (NOTLS, the advisory lock, map size, reader
    /// table) is identical to a durable store.
    ///
    /// REFUSAL NEVER MUTATES: `MDB_WRITEMAP` ftruncates the data file
    /// to the full map size at open, so an existing data file is probed
    /// first through a plain durable-flagged open (which leaves the
    /// file's bytes untouched) — the ephemeral flags are applied only
    /// after the probe verifies the store's kind. A refusal
    /// (`StoreKindMismatch` on a durable store, `AlreadyInitialized` on
    /// a foreign LMDB environment, `FormatMismatch`/`Corruption` on a
    /// stale or forged store) leaves `data.mdb` byte-identical.
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
        // A directory without a data file is fresh: nothing exists that
        // the WRITEMAP ftruncate could damage, so create directly with
        // the ephemeral flags. Anything else is probed WITHOUT the
        // flags first — every refusal must fire before the mutating
        // open. The advisory lock (held above) keeps the probe→reopen
        // window race-free against other bumbledb handles.
        let has_meta = if path.join("data.mdb").try_exists()? {
            Self::probe_ephemeral_kind(path)?
        } else {
            false
        };
        let env = open_env(path, StoreKind::Ephemeral)?;
        if has_meta {
            Self::verify_and_open(env, lock, schema, StoreKind::Ephemeral)
        } else {
            Self::initialize(env, lock, schema, StoreKind::Ephemeral)
        }
    }

    /// The non-mutating kind probe over an EXISTING data file: a plain
    /// durable-flagged open (no `WRITEMAP`, so no ftruncate — the
    /// data file's byte length and contents stay identical; the
    /// byte-identity is pinned by `ephemeral_refusal_on_a_durable_store_
    /// leaves_the_data_file_byte_identical` and its foreign-env twin),
    /// one read transaction, and the version/kind checks in the same
    /// order as [`Environment::verify_and_open`]. Returns `Ok(true)` on
    /// a verified ephemeral store (the caller reopens with the flags
    /// and re-verifies fully, fingerprint included), `Ok(false)` on a
    /// half-created store (empty root, no `_meta` — the crash window
    /// between directory creation and the meta commit), and every
    /// refusal typed:
    ///
    /// - `AlreadyInitialized` — no `_meta` but a non-empty root: a
    ///   foreign LMDB environment (never ftruncate someone else's env);
    /// - `FormatMismatch` — a pre-v5 store (version before kind, as
    ///   everywhere);
    /// - `Corruption(MetaMissing)`/`Corruption(StoreKindInvalid)` — a
    ///   v5 store whose kind marker is absent / undecodable;
    /// - `StoreKindMismatch` — a durable store.
    ///
    /// The probe environment is fully dropped before this returns
    /// (heed closes the LMDB env when the last handle drops), so the
    /// caller's flagged reopen of the same path is legal.
    fn probe_ephemeral_kind(path: &Path) -> Result<bool> {
        let env = open_env(path, StoreKind::Durable)?;
        let rtxn = env.read_txn()?;
        let Some(meta) = env.open_database::<Bytes, Bytes>(&rtxn, Some("_meta"))? else {
            if let Some(root) = env.open_database::<Bytes, Bytes>(&rtxn, None)?
                && !root.is_empty(&rtxn)?
            {
                return Err(Error::AlreadyInitialized);
            }
            return Ok(false);
        };
        let found_version = read_u32(&meta, &rtxn, META_FORMAT_VERSION)?;
        if found_version != FORMAT_VERSION {
            return Err(Error::FormatMismatch {
                found: found_version,
                expected: FORMAT_VERSION,
            });
        }
        let found_kind = read_store_kind(&meta, &rtxn)?;
        if found_kind != StoreKind::Ephemeral {
            return Err(Error::StoreKindMismatch {
                found: found_kind,
                expected: StoreKind::Ephemeral,
            });
        }
        Ok(true)
    }
}
