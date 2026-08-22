//! LMDB environment lifecycle, `_meta` contents, and transaction wrappers
//! (docs/architecture/50-storage.md). Authority: `docs/architecture/50-storage.md`, `70-api.md`.

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

/// Process-local owner token. Identity is the allocation address, not a
/// counter: not persisted, not derived from payload, unique per
/// [`Environment::assemble`], shared by every read of that open
/// environment. Prepared queries and conditional-write witnesses hold
/// a clone; `same` is [`Arc::ptr_eq`].
#[derive(Clone)]
pub(crate) struct CatalogIdentity(Arc<CatalogIdentityCell>);

struct CatalogIdentityCell {
    /// Unique heap object; the address is the identity.
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

/// Storage format version, checked before the schema fingerprint on open.
/// Version 1: statement-keyed `U` and statement-scoped `R` layouts
/// (`docs/architecture/50-storage.md` § Key layout). Version 2: the
/// str-only untagged dictionary (`bytes<N>` inline in facts, never
/// interned) — version 1 stores carry tagged dictionary entries that
/// would decode wrong, so they refuse to open (the two-oracle run
/// caught a v1 store silently mis-decoding; a format change without a
/// version bump is that bug's whole class). Version 3: the
/// dependency-vocabulary extension — the canonical schema encoding
/// changed (literal-set selections, the count-window and
/// order-mark statement forms), so every stored fingerprint of a v2
/// store is computed under a retired encoding (every encoding change
/// bumps — `docs/architecture/50-storage.md` § open-time checks).
/// Version 4: the order purge — the statement spine sum shrank (the
/// order-mark form and its `R`-edge namespace left the vocabulary), so
/// the canonical schema encoding changed again; nothing deployed
/// carries an order statement, and a v3 store's fingerprint is computed
/// under a retired encoding. Version 5: a store-kind marker lived in
/// `_meta` (retired pre-publish: kind is not data). Version 6: the one id allocator
/// (ruled 2026-07-23, R16) — on a fresh-keyed relation the first fresh
/// field's value IS the `F` row id, that auto-key's `U` tree is gone,
/// and the `S` row-id high-water exists only where no fresh field
/// does, so a v5 store's `F` row ids, auto-key `U` entries, and `S`
/// counters all decode wrong under the merged mint. Version 7 is the
/// capacity cutover (ruled 2026-07-24): the canonical schema encoding
/// moved (the weight descriptor, dependent bounds, the re-minted
/// statement-form tag) and the `R` namespace gained the weighted
/// value slot, so every v6 fingerprint and every weighted-statement
/// `R` entry decodes wrong — one bump covers both. Version 8 is
/// admission provenance: every ordinary writable handle began from a
/// complete empty admission, a raw copy of an admitted instance, a
/// compact of an admitted format-8 store, or an incremental commit on
/// an admitted format-8 base. Open is version, then fingerprint.
/// Pre-publish the `_meta` roster was revised in place to four keys
/// (format, fingerprint, generation, dict-next). Every earlier
/// version — format 7 included — refuses on every open surface. No
/// format-7 decoder and no migration path exist — ETL is the story.
pub const FORMAT_VERSION: u32 = 8;

/// The persisted storage transaction id: the generation a snapshot
/// witnessed and a state-changing commit advances. This is not the
/// process-local reader-cache sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct GenerationId(u64);

impl GenerationId {
    /// The numeric id, for diagnostics and external observability.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Decodes the persisted `_meta` word at the storage boundary.
    pub(crate) const fn from_storage(word: u64) -> Self {
        Self(word)
    }

    /// Encodes the id back to the persisted `_meta` word.
    pub(crate) const fn storage_word(self) -> u64 {
        self.0
    }

    /// The generation of a newly created store.
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

/// Fixed map size: comfortably above the 1 GB scale axiom. Not
/// configurable — path-only public surface. The map is an
/// address-space reservation, never an allocation: no open path
/// truncates or preallocates `data.mdb` to the map (LMDB's full-map
/// ftruncate lives only under `MDB_WRITEMAP` — `mdb_env_map`, mdb.c —
/// and no constructor carries that flag), so a store's data file holds
/// exactly the pages ever committed, on every filesystem.
///
/// RETRACTION (cleanup-0.5.0 ruling 1): 4 GiB → 32 GiB. The 4 GiB
/// ceiling was priced when a retired constructor materialized its FULL
/// map at every open (WRITEMAP's ftruncate on non-sparse filesystems;
/// explicit block preallocation on sparse ones). Dropping WRITEMAP
/// removed the last full-map ftruncate, so the raise costs nothing:
/// capacity refusal reverts to the filesystem's own lazy behavior.
/// The retracted comment also claimed EVERY open ftruncates the map
/// (the container-filesystem `ENOSPC` warning) — that was true only
/// of WRITEMAP opens; verdict read off mdb.c and pinned in-tree by
/// the refusal tests' `< 1 GiB` fixture bounds.
///
/// The consequence still worth naming — **the hard capacity
/// ceiling**: resize is deliberately gone (the PRD 22 dead end:
/// `mdb_env_set_mapsize` racing readers — see [`super::commit::write`]'s
/// gravestone), so a store that fills the map has hit the wall: the
/// commit surfaces [`crate::error::Error::Lmdb`] wrapping LMDB's
/// `MDB_MAP_FULL` (`heed::MdbError::MapFull`), nothing persists, and
/// the remedy is a new store, never a knob.
const MAP_SIZE: usize = 32 << 30;

/// Fixed reader-table size: comfortably above any plausible snapshot
/// concurrency — inter-query parallelism is the design's scaling axis
/// (`00-product.md`), and `MDB_NOTLS` binds slots to open *transaction
/// objects* (the parked reader included), so LMDB's default 126 would cap
/// concurrent snapshots, not threads. Measured cost of the raise: 64
/// bytes of lock file per slot (one cache line) — 8,192 bytes at the
/// default, 65,664 at 1024, a 56 KiB delta. Not configurable — a
/// decision, not a knob. The slot past the table is the typed
/// [`crate::error::Error::ReadersFull`], never a raw LMDB passthrough.
pub(crate) const MAX_READERS: u32 = 1024;

/// `_meta` key aliases — [`MetaKey`] owns the table.
#[cfg(test)]
const META_FORMAT_VERSION: &[u8] = MetaKey::FORMAT_VERSION.key;
#[cfg(test)]
const META_FINGERPRINT: &[u8] = MetaKey::FINGERPRINT.key;
const META_TX_ID: &[u8] = MetaKey::GENERATION.key;
const META_DICT_NEXT_ID: &[u8] = MetaKey::DICT_NEXT.key;

/// The LMDB substrate: environment plus the three named databases.
///
/// Durability is LMDB defaults — fsync per commit;
/// `NOSYNC`/`WRITEMAP`/`MAPASYNC` are not expressible through the
/// public constructors. A bench-only NOSYNC open lane exists behind
/// a crate-hidden entry; it is not a store kind.
pub struct Environment {
    env: heed::Env<WithoutTls>,
    meta: Database<Bytes, Bytes>,
    data: Database<Bytes, Bytes>,
    dict: Database<Bytes, Bytes>,
    /// This environment's process-distinct identity. Prepared queries
    /// record it and refuse to execute against any other environment's
    /// snapshots — the generation clock knows whose clock it is.
    identity: CatalogIdentity,
    /// Advisory lock — held by ownership until drop.
    #[allow(dead_code)]
    lock: std::fs::File,
    /// Process-lifetime escaped `Q` high-water: once `reserve` has handed
    /// an id to the host, this floor never retreats in this process —
    /// even when the counters-only disk flush fails
    /// (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`).
    escaped_fresh: Mutex<escaped_fresh::FreshMarks>,
    /// Dirty `Q` marks whose durable write has not yet succeeded. Retried
    /// at the next write begin; a still-failing retry poisons `reserve` until
    /// the burn is durable. Clean vs parked is the enum, not map emptiness.
    pending_fresh_flush: Mutex<escaped_fresh::FlushState>,
    /// Test-only: remaining injected failures of the escaped-id flush.
    #[cfg(test)]
    fail_fresh_flush: AtomicU32,
}

/// A fresh-store constructor refuses any existing destination, including
/// an empty directory — an existing path is a previous claim on the name.
pub(crate) fn refuse_existing_destination(path: &std::path::Path) -> crate::error::Result<()> {
    if path.exists() {
        Err(crate::error::Error::DestinationExists {
            path: path.to_path_buf(),
        })
    } else {
        Ok(())
    }
}

/// Fsyncs `dir`'s dirent chain: the directory itself, then its parent —
/// what a power loss must survive for entries inside `dir` to still
/// exist. LMDB fsyncs file CONTENTS per commit and never opens a
/// directory (no directory fsync anywhere in mdb.c), so this is the
/// one mechanism behind its two callers: `Db::compact`'s copy and
/// `Environment::create`'s birth (finding 022). Directories above the
/// immediate parent are the caller's own story.
pub(crate) fn sync_dirent_chain(dir: &std::path::Path) -> std::io::Result<()> {
    for d in [dir, parent_dir(dir)] {
        std::fs::File::open(d)?.sync_all()?;
    }
    Ok(())
}

/// `dir`'s parent, or `.` when the path has none — where the chain ends.
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
    /// Overwrites the stored format version — the version-mismatch
    /// fixture.
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
    /// The one construction site for the handle: open/create fill the
    /// LMDB pieces; the escaped-fresh maps start empty (a reopen has
    /// no in-process high-water — disk `Q` is the floor).
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
        }
    }

    /// This environment's process-distinct identity (readers: prepared
    /// queries via [`ReadTxn::identity`]; `Db::write_from`'s
    /// witness check, which compares a snapshot's identity against the
    /// database being written).
    pub(crate) fn identity(&self) -> &CatalogIdentity {
        &self.identity
    }

    /// The `_dict` database handle (reader: `storage::dict`).
    pub(crate) fn dict(&self) -> Database<Bytes, Bytes> {
        self.dict
    }

    /// The `_data` database handle (readers: `storage::delta` probes,
    /// `storage::commit`).
    pub(crate) fn data(&self) -> Database<Bytes, Bytes> {
        self.data
    }

    /// Parses the live `_meta` block.
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
    /// Snapshot-constant by definition (the tx id is read *inside* this
    /// snapshot), so one `_meta` get serves every `generation()` caller —
    /// the cache asks once per occurrence per execution otherwise.
    generation: std::cell::OnceCell<GenerationId>,
}

impl ReadTxn<'_> {
    /// The underlying heed transaction (reader: `storage::dict` lookups).
    pub(crate) fn raw(&self) -> &RoTxn<'_, AnyTls> {
        &self.txn
    }

    /// The owning environment (reader: `storage::dict`).
    pub(crate) fn env(&self) -> &Environment {
        self.env
    }

    /// The owning environment's process-distinct identity — the value a
    /// prepared query records at prepare and checks at execute.
    pub(crate) fn identity(&self) -> &CatalogIdentity {
        self.env.identity()
    }

    /// Unwraps the raw transaction for the reader cache:
    /// the snapshot stays open, parked for the next same-generation read.
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
    /// write transactions read their own writes).
    pub(crate) fn raw(&self) -> &RoTxn<'_, AnyTls> {
        &self.txn
    }

    /// The underlying heed transaction, mutably (reader: `storage::dict`).
    pub(crate) fn raw_mut(&mut self) -> &mut RwTxn<'env> {
        &mut self.txn
    }

    /// The owning environment (reader: `storage::dict`).
    pub(crate) fn env(&self) -> &Environment {
        self.env
    }
}
