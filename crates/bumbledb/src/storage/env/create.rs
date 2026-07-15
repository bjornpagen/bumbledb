use std::path::Path;
use std::sync::atomic::Ordering;

use heed::WithoutTls;
use heed::types::Bytes;

use crate::error::{Error, Result};
use crate::schema::Schema;
use crate::schema::fingerprint::fingerprint;

use super::acquire_lock::acquire_lock;
use super::open_env::open_env;
use super::{
    Environment, FORMAT_VERSION, META_DICT_NEXT_ID, META_FINGERPRINT, META_FORMAT_VERSION,
    META_STORE_KIND, META_TX_ID, NEXT_INSTANCE, StoreKind,
};

impl Environment {
    /// Initializes a fresh DURABLE environment: creates the three
    /// databases and writes format version, store kind, schema
    /// fingerprint, and storage tx id 0.
    ///
    /// # Errors
    ///
    /// `Io` on directory creation, `EnvironmentLocked` if another handle
    /// holds the environment, `AlreadyInitialized` on a directory that
    /// already holds any LMDB environment (bumbledb's or anyone else's),
    /// `Lmdb` on any LMDB failure.
    pub fn create(path: &Path, schema: &Schema) -> Result<Self> {
        std::fs::create_dir_all(path)?;
        let lock = acquire_lock(path)?;
        let env = open_env(path, StoreKind::Durable)?;
        Self::initialize(env, lock, schema, StoreKind::Durable)
    }

    /// The shared initialization body ([`Environment::create`] durable,
    /// [`Environment::ephemeral`]'s fresh-directory arm): refuses an
    /// already-initialized directory, creates the three databases, and
    /// writes the `_meta` block — the store kind included, so the store
    /// carries its durability claim on disk from birth.
    pub(super) fn initialize(
        env: heed::Env<WithoutTls>,
        lock: std::fs::File,
        schema: &Schema,
        kind: StoreKind,
    ) -> Result<Self> {
        let mut wtxn = env.write_txn()?;
        // Refuse to re-initialize: rewriting `_meta` over live `_data`
        // would reset the tx id and the dictionary counter — silent
        // corruption from one wrong call. Production create never
        // destroys data any more than production open does.
        if env
            .open_database::<Bytes, Bytes>(&wtxn, Some("_meta"))?
            .is_some()
        {
            return Err(Error::AlreadyInitialized);
        }
        // No `_meta`, but a non-empty unnamed root means the directory
        // holds someone else's LMDB environment (named databases live as
        // root entries, so this covers foreign named DBs too) — refuse
        // rather than move in. A half-created bumbledb store (crash
        // between directory creation and the meta commit) has an empty
        // root and still proceeds.
        if let Some(root) = env.open_database::<Bytes, Bytes>(&wtxn, None)?
            && !root.is_empty(&wtxn)?
        {
            return Err(Error::AlreadyInitialized);
        }
        let meta = env.create_database(&mut wtxn, Some("_meta"))?;
        let data = env.create_database(&mut wtxn, Some("_data"))?;
        let dict = env.create_database(&mut wtxn, Some("_dict"))?;
        meta.put(
            &mut wtxn,
            META_FORMAT_VERSION,
            FORMAT_VERSION.to_le_bytes().as_slice(),
        )?;
        meta.put(&mut wtxn, META_STORE_KIND, [kind.meta_byte()].as_slice())?;
        meta.put(
            &mut wtxn,
            META_FINGERPRINT,
            fingerprint(schema).0.as_slice(),
        )?;
        meta.put(&mut wtxn, META_TX_ID, 0u64.to_le_bytes().as_slice())?;
        meta.put(&mut wtxn, META_DICT_NEXT_ID, 0u64.to_le_bytes().as_slice())?;
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
