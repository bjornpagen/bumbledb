use std::path::Path;
use std::sync::atomic::Ordering;

use heed::WithoutTls;

use crate::error::{Error, Result};
use crate::schema::Schema;
use crate::schema::fingerprint::{canonical_descriptor, fingerprint_of_descriptor};

use super::acquire_lock::acquire_lock;
use super::open_env::{OpenLane, open_env};
use super::read_meta::{MetaBlock, classify_meta_block};
use super::{
    Environment, FORMAT_VERSION, META_DICT_NEXT_ID, META_FINGERPRINT, META_FORMAT_VERSION,
    META_SCHEMA_DESCRIPTOR, META_STORE_KIND, META_TX_ID, NEXT_INSTANCE, StoreKind,
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
        let env = open_env(path, OpenLane::Write(StoreKind::Durable))?;
        let created = Self::initialize(env, lock, schema, StoreKind::Durable)?;
        // The birth's dirent chain (finding 022): LMDB fsyncs data.mdb's
        // CONTENTS at the initialize commit but never opens a directory,
        // so the dirents for data.mdb and the store directory itself are
        // the one create-time gap a power loss can exploit — every
        // commit would report fsynced success into a file whose name
        // never became durable. Closed with the same chain sync
        // `Db::compact` ships (one mechanism, `super::sync_dirent_chain`).
        super::sync_dirent_chain(path)?;
        crate::obs::event(
            crate::obs::names::CREATE_DURABLE,
            crate::obs::Category::Storage,
            2,
            0,
        );
        Ok(created)
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
        // One meta-block classification, shared with `verify_and_open`
        // and the ephemeral probe (`read_meta::classify_meta_block`):
        // an initialized store refuses — rewriting `_meta` over live
        // `_data` would reset the tx id and the dictionary counter,
        // silent corruption from one wrong call — a foreign LMDB
        // environment refuses inside the classifier, and a half-created
        // bumbledb store (crash between directory creation and the meta
        // commit; empty root, no `_meta`) proceeds: creation heals it.
        match classify_meta_block(&env, &wtxn)? {
            MetaBlock::Present(_) => return Err(Error::AlreadyInitialized),
            MetaBlock::HalfCreated => {}
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
        // The fingerprint and the descriptor are one value twice: the
        // canonical bytes are hashed for the fingerprint and persisted
        // whole beside it, so the store is self-describing from birth
        // (readers: `Environment::exhume`, `Db::verify_store` —
        // `docs/architecture/50-storage.md` § the `_meta` block).
        let descriptor = canonical_descriptor(schema);
        meta.put(
            &mut wtxn,
            META_FINGERPRINT,
            fingerprint_of_descriptor(&descriptor).0.as_slice(),
        )?;
        meta.put(&mut wtxn, META_SCHEMA_DESCRIPTOR, descriptor.as_slice())?;
        meta.put(&mut wtxn, META_TX_ID, 0u64.to_le_bytes().as_slice())?;
        meta.put(&mut wtxn, META_DICT_NEXT_ID, 0u64.to_le_bytes().as_slice())?;
        wtxn.commit()?;
        Ok(Self {
            env,
            meta,
            data,
            dict,
            instance: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
            _lock: Some(lock),
            dirty_marker: None,
        })
    }
}
