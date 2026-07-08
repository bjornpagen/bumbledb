//! LMDB environment lifecycle, `_meta` contents, and transaction wrappers
//! (docs/architecture/40-storage.md). Authority: `docs/architecture/40-storage.md`, `60-api.md`.

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

/// Process-unique environment instance ids, minted at `create`/`open`.
/// Starts at 1 — 0 stays "no environment" forever. Per-process uniqueness
/// is exactly sufficient: every piece of derived state keyed by an
/// instance (the view memo, prepared queries) is process-local, and a
/// wiped-and-recreated store necessarily passes through a new
/// [`Environment`].
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// Storage format version, checked before the schema fingerprint on open.
pub const FORMAT_VERSION: u32 = 0;

/// Fixed map size: comfortably above the 1 GB scale axiom, allocated
/// sparsely by the OS. Not configurable — path-only public surface.
const MAP_SIZE: usize = 4 << 30;

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
    /// This environment's process-unique identity (never 0). Prepared
    /// queries record it and refuse to execute against any other
    /// environment's snapshots — the generation clock knows whose clock
    /// it is.
    instance: u64,
    /// The exclusive advisory lock on `<dir>/bumbledb.lock`, held for the
    /// environment's lifetime. Dropping the handle releases it.
    _lock: std::fs::File,
}

impl Environment {
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
    generation: std::cell::OnceCell<u64>,
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

    /// The owning environment's process-unique identity — the value a
    /// prepared query records at prepare and checks at execute.
    pub(crate) fn env_instance(&self) -> u64 {
        self.env.instance
    }

    /// Unwraps the raw transaction for the reader cache (docs/silicon/12):
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
