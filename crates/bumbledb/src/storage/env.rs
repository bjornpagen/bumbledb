//! LMDB environment lifecycle, `_meta` contents, and transaction wrappers
#[cfg(test)]
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn, RwTxn, WithoutTls};

mod acquire_lock;
mod create;
mod debug;
mod escaped_fresh;
pub(crate) use escaped_fresh::FreshMarks;
pub(crate) mod host;
mod maintenance;
mod open;
mod open_env;
mod publish;
mod read_meta;
mod readtxn;
mod txn;
mod writetxn;

pub(crate) use publish::PublishCatalog;
pub(crate) use read_meta::MetaKey;
#[cfg(test)]
use read_meta::{StoreMeta, parse_meta};

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(crate) struct CatalogIdentity(Arc<CatalogIdentityCell>);

struct CatalogIdentityCell {
    _private: (),
}

impl CatalogIdentity {
    pub(crate) fn mint() -> Self {
        Self(Arc::new(CatalogIdentityCell { _private: () }))
    }

    pub(crate) fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Version 1: statement-keyed `U` and statement-scoped `R` layouts
/// . Version 2: the
/// str-only untagged dictionary (`bytes<N>` inline in facts, never
/// interned) — version 1 stores carry tagged dictionary entries that
/// would decode wrong, so they refuse to open (the two-oracle run
/// caught a v1 store silently mis-decoding; a format change without a
/// version bump is that bug's whole class). Version 3: the
/// dependency-vocabulary extension — the canonical schema encoding
/// Storage format version, checked before the schema fingerprint on open.
pub const FORMAT_VERSION: u32 = 8;

/// The persisted storage transaction id: the generation a snapshot
/// witnessed and a state-changing commit advances. This is not the
/// process-local reader-cache sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct GenerationId(u64);

impl GenerationId {
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_storage(word: u64) -> Self {
        Self(word)
    }

    pub(crate) const fn storage_word(self) -> u64 {
        self.0
    }

    pub(crate) const fn initial() -> Self {
        Self(0)
    }

    /// The next persisted generation after a state-changing commit.
    pub(crate) const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl std::fmt::Display for GenerationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Dropping WRITEMAP removed the last full-map ftruncate, so the raise costs
/// nothing: capacity refusal reverts to the filesystem's own lazy behavior. The
/// retracted comment also claimed EVERY open ftruncates the map (the
/// container-filesystem `ENOSPC` warning) — that was true only of WRITEMAP
/// opens; verdict read off mdb.c and pinned in-tree by the refusal tests' `< 1
/// GiB` fixture bounds.
const MAP_SIZE: usize = 32 << 30;

pub(crate) const MAX_READERS: u32 = 1024;

#[cfg(test)]
const META_FORMAT_VERSION: &[u8] = MetaKey::FORMAT_VERSION.key;
#[cfg(test)]
const META_FINGERPRINT: &[u8] = MetaKey::FINGERPRINT.key;
const META_TX_ID: &[u8] = MetaKey::GENERATION.key;
const META_DICT_NEXT_ID: &[u8] = MetaKey::DICT_NEXT.key;

/// The LMDB substrate: environment plus the three named databases.
/// Durability is LMDB defaults — fsync per commit;
/// public constructors. A bench-only NOSYNC open lane exists behind
/// a crate-hidden entry; it is not a store kind.
pub struct Environment {
    env: heed::Env<WithoutTls>,
    meta: Database<Bytes, Bytes>,
    data: Database<Bytes, Bytes>,
    dict: Database<Bytes, Bytes>,

    identity: CatalogIdentity,
    /// Advisory lock — held by ownership until drop.
    #[allow(dead_code)]
    lock: std::fs::File,

    /// (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
    escaped_fresh: Mutex<escaped_fresh::FreshMarks>,

    pending_fresh_flush: Mutex<escaped_fresh::FlushState>,

    #[cfg(test)]
    fail_fresh_flush: AtomicU32,
    #[cfg(test)]
    fail_host_after: Mutex<Option<usize>>,
}

pub(crate) fn refuse_existing_destination(path: &std::path::Path) -> crate::error::Result<()> {
    if path.exists() {
        Err(crate::error::Error::DestinationExists {
            path: path.to_path_buf(),
        })
    } else {
        Ok(())
    }
}

pub(crate) fn sync_dirent_chain(dir: &std::path::Path) -> std::io::Result<()> {
    for d in [dir, parent_dir(dir)] {
        std::fs::File::open(d)?.sync_all()?;
    }
    Ok(())
}

fn parent_dir(dir: &std::path::Path) -> &std::path::Path {
    match dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => std::path::Path::new("."),
    }
}

/// Test-only `_meta` fixture surgery: version-mismatch fixtures built
/// by mutating a real store's meta block.
#[cfg(test)]
impl Environment {
    pub(crate) fn force_format_version_for_tests(&self, version: u32) -> crate::error::Result<()> {
        let mut wtxn = self.env.write_txn()?;
        self.meta.put(
            &mut wtxn,
            META_FORMAT_VERSION,
            version.to_le_bytes().as_slice(),
        )?;
        wtxn.commit()?;
        Ok(())
    }
}

impl Environment {
    /// LMDB pieces; the escaped-fresh maps start empty (a reopen has
    pub(super) fn assemble(
        env: heed::Env<WithoutTls>,
        meta: Database<Bytes, Bytes>,
        data: Database<Bytes, Bytes>,
        dict: Database<Bytes, Bytes>,
        lock: std::fs::File,
    ) -> Self {
        Self {
            env,
            meta,
            data,
            dict,
            identity: CatalogIdentity::mint(),
            lock,
            escaped_fresh: Mutex::new(escaped_fresh::FreshMarks::default()),
            pending_fresh_flush: Mutex::new(escaped_fresh::FlushState::default()),
            #[cfg(test)]
            fail_fresh_flush: AtomicU32::new(0),
            #[cfg(test)]
            fail_host_after: Mutex::new(None),
        }
    }

    pub(crate) fn identity(&self) -> &CatalogIdentity {
        &self.identity
    }

    pub(crate) fn dict(&self) -> Database<Bytes, Bytes> {
        self.dict
    }

    pub(crate) fn data(&self) -> Database<Bytes, Bytes> {
        self.data
    }

    #[cfg(test)]
    pub(crate) fn read_store_meta(&self) -> crate::error::Result<StoreMeta> {
        let rtxn = self.read_txn()?;
        parse_meta(&self.meta, rtxn.raw())
    }
}

/// A read snapshot over the environment.
pub struct ReadTxn<'env> {
    env: &'env Environment,
    txn: RoTxn<'static, WithoutTls>,

    generation: std::cell::OnceCell<GenerationId>,
}

impl ReadTxn<'_> {
    pub(crate) fn raw(&self) -> &RoTxn<'_, AnyTls> {
        &self.txn
    }

    pub(crate) fn env(&self) -> &Environment {
        self.env
    }

    pub(crate) fn identity(&self) -> &CatalogIdentity {
        self.env.identity()
    }

    pub(crate) fn into_raw_txn(self) -> RoTxn<'static, WithoutTls> {
        self.txn
    }
}

/// The write transaction over the environment.
pub struct WriteTxn<'env> {
    env: &'env Environment,
    txn: RwTxn<'env>,
}

impl<'env> WriteTxn<'env> {
    /// The underlying heed transaction (reader: `storage::dict` — LMDB
    pub(crate) fn raw(&self) -> &RoTxn<'_, AnyTls> {
        &self.txn
    }

    pub(crate) fn raw_mut(&mut self) -> &mut RwTxn<'env> {
        &mut self.txn
    }

    pub(crate) fn env(&self) -> &Environment {
        self.env
    }
}
