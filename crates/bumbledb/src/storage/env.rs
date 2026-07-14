//! LMDB environment lifecycle, `_meta` contents, and transaction wrappers
//! (docs/architecture/50-storage.md). Authority: `docs/architecture/50-storage.md`, `70-api.md`.

use std::sync::atomic::AtomicU64;

use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn, RwTxn, WithoutTls};

mod acquire_lock;
mod create;
mod debug;
mod maintenance;
mod open;
mod open_env;
mod read_meta;
mod readtxn;
mod txn;
mod writetxn;

#[cfg(test)]
mod tests;

/// Process-distinct environment instance ids, minted at `create`/`open`.
/// Starts at 1 — 0 stays "no environment" forever. Per-process distinctness
/// is exactly sufficient: every piece of derived state keyed by an
/// instance (the view memo, prepared queries) is process-local, and a
/// wiped-and-recreated store necessarily passes through a new
/// [`Environment`].
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// Storage format version, checked before the schema fingerprint on open.
/// Version 1: statement-keyed `U` and statement-scoped `R` layouts
/// (`docs/architecture/50-storage.md` § Key layout). Version 2: the
/// str-only untagged dictionary (`bytes<N>` inline in facts, never
/// interned) — version 1 stores carry tagged dictionary entries that
/// would decode wrong, so they refuse to open (the two-oracle run
/// caught a v1 store silently mis-decoding; a format change without a
/// version bump is that bug's whole class). No other version opens and
/// no migration path exists — ETL is the story.
pub const FORMAT_VERSION: u32 = 2;

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

/// Fixed map size: comfortably above the 1 GB scale axiom, allocated
/// sparsely by the OS. Not configurable — path-only public surface.
const MAP_SIZE: usize = 4 << 30;

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

/// `_meta` keys, single-byte.
const META_FORMAT_VERSION: &[u8] = &[0];
const META_FINGERPRINT: &[u8] = &[1];
const META_TX_ID: &[u8] = &[2];
const META_DICT_NEXT_ID: &[u8] = &[3];

/// The LMDB substrate: environment plus the three named databases.
///
/// Durability is LMDB defaults — fsync per commit; `NOSYNC`/`WRITEMAP`/
/// `MAPASYNC` are not expressible through this type.
pub struct Environment {
    env: heed::Env<WithoutTls>,
    meta: Database<Bytes, Bytes>,
    data: Database<Bytes, Bytes>,
    dict: Database<Bytes, Bytes>,
    /// This environment's process-distinct identity (never 0). Prepared
    /// queries record it and refuse to execute against any other
    /// environment's snapshots — the generation clock knows whose clock
    /// it is.
    instance: u64,
    /// The exclusive advisory lock on `<dir>/bumbledb.lock`, held for the
    /// environment's lifetime. Dropping the handle releases it.
    _lock: std::fs::File,
}

impl Environment {
    /// This environment's process-distinct identity (readers: prepared
    /// queries via [`ReadTxn::env_instance`]; `Db::write_from`'s
    /// witness check, which compares a snapshot's identity against the
    /// database being written).
    pub(crate) fn instance(&self) -> u64 {
        self.instance
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
    pub(crate) fn env_instance(&self) -> u64 {
        self.env.instance
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
