use std::path::Path;

use heed::types::Bytes;

use crate::error::Result;
use crate::schema::Schema;

use super::acquire_lock::acquire_lock;
use super::open_env::open_env;
use super::{Environment, StoreKind};

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
    /// # Errors
    ///
    /// `Io` on directory creation, `EnvironmentLocked` if another handle
    /// holds the environment, `AlreadyInitialized` on a directory
    /// holding a foreign LMDB environment, `FormatMismatch`/
    /// `StoreKindMismatch`/`SchemaMismatch` on an existing store that
    /// fails verification, `Lmdb` otherwise.
    pub fn ephemeral(path: &Path, schema: &Schema) -> Result<Self> {
        std::fs::create_dir_all(path)?;
        let lock = acquire_lock(path)?;
        let env = open_env(path, StoreKind::Ephemeral)?;
        // Probe for an existing bumbledb store. The probe transaction is
        // dropped (aborted), so its dbi registration rolls back and the
        // arm below re-opens handles under its own transaction.
        let has_meta = {
            let probe = env.write_txn()?;
            env.open_database::<Bytes, Bytes>(&probe, Some("_meta"))?
                .is_some()
        };
        if has_meta {
            Self::verify_and_open(env, lock, schema, StoreKind::Ephemeral)
        } else {
            Self::initialize(env, lock, schema, StoreKind::Ephemeral)
        }
    }
}
