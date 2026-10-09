//! The one store owner: environment lifecycle, the fixed virtual map, the
//! writer slot, directory ownership and close.
//!
//! Open takes the kernel directory lock first, then checks format and
//! schema before touching anything. The map is one virtual reservation fixed
//! at open: it costs address space, not RAM or disk, because without
//! `WRITEMAP` the file grows only as pages are written.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use heed::types::Bytes;
use heed::{Database, EnvFlags, EnvOpenOptions, RoTxn, RwTxn, WithoutTls};

use super::det_index::DeterminantTable;
use super::fingerprint::Fingerprinter;
use super::format::{
    self, DET_DB, DatabaseId, EnvironmentId, FORMAT, K_DATABASE, K_FORMAT, K_GENERATION,
    K_NEXT_ROW_ID, K_SCHEMA, META_DB, ROWS_DB, StoreIdentity,
};
use super::gate::{GatePass, TransactionGate};
use super::snapshot::OwnedSnapshot;
use crate::error::CorruptionError;
use crate::error::{Error, Result};
use crate::schema::Schema;
use crate::schema::fingerprint::{SchemaFingerprint, fingerprint};
use crate::storage::GenerationId;
use crate::work::WorkContext;

const LOCK_FILE: &str = "bdb.lock";
pub(crate) const DATA_FILE: &str = "data.mdb";
const MAX_READERS: u32 = 1024;
const WAIT_QUANTUM: Duration = Duration::from_millis(1);

/// The default virtual map ceiling: 1 TiB of address space.
pub(crate) const DEFAULT_MAP_CEILING: u64 = 1 << 40;

/// Whether a commit survives a crash or power loss on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Durability {
    /// Every commit is synced to disk.
    #[default]
    Durable,
    /// Commits are not synced (`MDB_NOSYNC`): a crash may lose the latest
    /// commits but never corrupts earlier ones. For a cache rebuilt from an
    /// authoritative log.
    Cache,
}

/// Environment options fixed at open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// The virtual map reservation in bytes: address space, not RAM or disk.
    /// A write that needs more pages fails with [`crate::Error::Full`].
    pub map_ceiling: u64,
    pub durability: Durability,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            map_ceiling: DEFAULT_MAP_CEILING,
            durability: Durability::Durable,
        }
    }
}

pub(crate) struct StoreInner {
    // Field order is drop order: the environment closes before the kernel
    // lock releases.
    pub(crate) env: heed::Env<WithoutTls>,
    pub(crate) meta: Database<Bytes, Bytes>,
    pub(crate) rows: Database<Bytes, Bytes>,
    pub(crate) dets: Database<Bytes, Bytes>,
    pub(crate) gate: TransactionGate,
    writer: WriterSlot,
    ceiling: u64,
    durability: Durability,
    pub(crate) identity: StoreIdentity,
    pub(crate) schema_fp: SchemaFingerprint,
    pub(crate) fingerprinter: Fingerprinter,
    pub(crate) det: DeterminantTable,
    path: PathBuf,
    #[cfg(test)]
    fail_host_after: Mutex<Option<usize>>,
    _lock: DirectoryLock,
}

impl StoreInner {
    /// A write-path LMDB failure: map exhaustion is the typed fixed-ceiling
    /// refusal, everything else keeps its LMDB identity.
    pub(crate) fn txn_error(&self, error: heed::Error) -> Error {
        full_or(error, self.ceiling)
    }

    #[cfg(test)]
    pub(crate) fn fail_host_write(&self, index: usize) -> Result<()> {
        if *self
            .fail_host_after
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            == Some(index)
        {
            return Err(Error::Full {
                ceiling: self.ceiling,
            });
        }
        Ok(())
    }
}

fn full_or(error: heed::Error, ceiling: u64) -> Error {
    if matches!(error, heed::Error::Mdb(heed::MdbError::MapFull)) {
        Error::Full { ceiling }
    } else {
        Error::from(error)
    }
}

/// The one writer: the owning thread's key while held. Waiting polls the
/// caller's cancellation every quantum.
#[derive(Debug, Default)]
struct WriterSlot {
    holder: Mutex<Option<u64>>,
    released: Condvar,
}

/// Releases the writer slot on drop.
pub(crate) struct WriterGuard<'store> {
    slot: &'store WriterSlot,
}

impl Drop for WriterGuard<'_> {
    fn drop(&mut self) {
        *self
            .slot
            .holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        self.slot.released.notify_one();
    }
}

/// The store owner. `Send + Sync`; clones share one environment.
#[derive(Clone)]
pub(crate) struct Store {
    pub(crate) inner: Arc<StoreInner>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("database", &self.inner.identity.database)
            .field("path", &self.inner.path)
            .finish_non_exhaustive()
    }
}

/// Outcome of a bounded close. `Incomplete` keeps the store closing:
/// admission stays refused and the environment closes when the last live
/// snapshot drops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReport {
    Closed,
    Incomplete {
        live_transactions: u64,
        oldest_age: Option<Duration>,
    },
}

/// A write transaction admitted through the gate; the transaction ends
/// before its gate slot releases.
pub(crate) struct GatedRwTxn<'env> {
    pub(crate) txn: RwTxn<'env>,
    ceiling: u64,
    _pass: GatePass,
}

impl GatedRwTxn<'_> {
    pub(crate) fn commit(self) -> Result<()> {
        let ceiling = self.ceiling;
        self.txn.commit().map_err(|error| full_or(error, ceiling))
    }
}

#[expect(
    unsafe_code,
    reason = "heed marks environment opening unsafe: two opens of one path in \
              a process are LMDB UB. The kernel directory lock is taken before \
              every open, so each directory has one live environment."
)]
fn open_env(path: &Path, options: Options) -> Result<heed::Env<WithoutTls>> {
    let mut open = EnvOpenOptions::new().read_txn_without_tls();
    open.map_size(
        usize::try_from(options.map_ceiling).map_err(|_| Error::Full {
            ceiling: options.map_ceiling,
        })?,
    )
    .max_dbs(3)
    .max_readers(MAX_READERS);
    if options.durability == Durability::Cache {
        // SAFETY: NO_SYNC only skips the fsync after commit; LMDB keeps the
        // file consistent (a crash loses the newest commits only).
        unsafe {
            open.flags(EnvFlags::NO_SYNC);
        }
    }
    // SAFETY: one open per directory, enforced by the held kernel lock.
    unsafe { open.open(path) }.map_err(Error::from)
}

/// Owns the directory lock itself, not one handle to it: a subprocess may
/// inherit the file description until exec closes it. Drops after every
/// environment owner.
struct DirectoryLock {
    file: std::fs::File,
}

impl Drop for DirectoryLock {
    fn drop(&mut self) {
        // Drop cannot report an unlock failure; closing the file still
        // releases our description.
        let _ = self.file.unlock();
    }
}

fn acquire_lock(path: &Path) -> Result<DirectoryLock> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.join(LOCK_FILE))?;
    match file.try_lock() {
        Ok(()) => Ok(DirectoryLock { file }),
        Err(std::fs::TryLockError::WouldBlock) => Err(Error::Locked {
            path: path.to_path_buf(),
        }),
        Err(std::fs::TryLockError::Error(err)) => Err(Error::from(err)),
    }
}

fn open_database(
    env: &heed::Env<WithoutTls>,
    rtxn: &RoTxn<'_, WithoutTls>,
    name: &str,
) -> Result<Database<Bytes, Bytes>> {
    env.open_database(rtxn, Some(name))
        .map_err(Error::from)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing(
            "named database",
        )))
}

impl Store {
    /// Create a new store at `path`, which must not exist. The store is
    /// built in a staged sibling and renamed into place.
    pub(crate) fn create(
        path: &Path,
        schema: &Schema,
        database: DatabaseId,
        options: Options,
    ) -> Result<Self> {
        let staging = super::staging::Staging::begin(path)?;
        init_directory(staging.path(), schema, database, options)?;
        staging.publish(schema, options)
    }

    /// Open an existing store; refusal mutates nothing.
    pub(crate) fn open(path: &Path, schema: &Schema, options: Options) -> Result<Self> {
        Self::open_with(path, schema, options, Fingerprinter::Blake3)
    }

    /// Create with every fingerprint forced to `fp`, for collision schedules.
    #[cfg(test)]
    pub(crate) fn create_forced_fingerprint(
        path: &Path,
        schema: &Schema,
        fp: [u8; super::fingerprint::FP_LEN],
    ) -> Result<Self> {
        drop(Self::create(
            path,
            schema,
            DatabaseId::mint(),
            Options::default(),
        )?);
        Self::open_with(
            path,
            schema,
            Options::default(),
            Fingerprinter::Constant(fp),
        )
    }

    fn open_with(
        path: &Path,
        schema: &Schema,
        options: Options,
        fingerprinter: Fingerprinter,
    ) -> Result<Self> {
        let det = DeterminantTable::compile(schema)?;
        let lock = acquire_lock(path)?;
        let env = open_env(path, options)?;
        let schema_fp = fingerprint(schema);
        let rtxn = env.read_txn().map_err(Error::from)?;
        let meta: Database<Bytes, Bytes> = env
            .open_database(&rtxn, Some(META_DB))
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotABumbleDb {
                path: path.to_path_buf(),
            })?;
        let database = format::verify_meta(&meta, &rtxn, path, &schema_fp)?;
        let rows = open_database(&env, &rtxn, ROWS_DB)?;
        let dets = open_database(&env, &rtxn, DET_DB)?;
        rtxn.commit().map_err(Error::from)?;
        Ok(Self {
            inner: Arc::new(StoreInner {
                env,
                meta,
                rows,
                dets,
                gate: TransactionGate::default(),
                writer: WriterSlot::default(),
                ceiling: options.map_ceiling,
                durability: options.durability,
                identity: StoreIdentity {
                    database,
                    environment: EnvironmentId::mint(),
                },
                schema_fp,
                fingerprinter,
                det,
                path: path.to_path_buf(),
                #[cfg(test)]
                fail_host_after: Mutex::new(None),
                _lock: lock,
            }),
        })
    }

    pub(crate) fn identity(&self) -> StoreIdentity {
        self.inner.identity
    }

    pub(crate) fn ceiling(&self) -> u64 {
        self.inner.ceiling
    }

    pub(crate) fn durability(&self) -> Durability {
        self.inner.durability
    }

    /// One coherent owned snapshot.
    pub(crate) fn snapshot(&self, work: &WorkContext) -> Result<OwnedSnapshot> {
        let pass = self.inner.gate.enter(work)?;
        let txn = self
            .inner
            .env
            .clone()
            .static_read_txn()
            .map_err(Error::from)?;
        let generation = read_generation(&self.inner, &txn)?;
        Ok(OwnedSnapshot::capture(
            Arc::clone(&self.inner),
            pass,
            txn,
            generation,
        ))
    }

    /// The single writer. Reentrant acquisition from the owning thread
    /// refuses instead of deadlocking.
    pub(crate) fn writer(&self, work: &WorkContext) -> Result<super::candidate::WriteOwner<'_>> {
        let caller = writer_thread_key();
        let slot = &self.inner.writer;
        let mut holder = slot
            .holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            match *holder {
                None => break,
                Some(owner) if owner == caller => return Err(Error::ReentrantWriter),
                Some(_) => {
                    work.checkpoint()?;
                    holder = slot
                        .released
                        .wait_timeout(holder, WAIT_QUANTUM)
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .0;
                }
            }
        }
        work.checkpoint()?;
        *holder = Some(caller);
        drop(holder);
        Ok(super::candidate::WriteOwner::new(
            self,
            WriterGuard { slot },
            work.clone(),
        ))
    }

    /// Begin one gated write transaction; the caller holds the writer.
    pub(crate) fn gated_write_txn(&self, work: &WorkContext) -> Result<GatedRwTxn<'_>> {
        let pass = self.inner.gate.enter(work)?;
        let txn = self
            .inner
            .env
            .write_txn()
            .map_err(|error| self.inner.txn_error(error))?;
        Ok(GatedRwTxn {
            txn,
            ceiling: self.inner.ceiling,
            _pass: pass,
        })
    }

    /// Length of `data.mdb`: the populated file, not the virtual map.
    pub(crate) fn file_bytes(&self) -> Result<u64> {
        Ok(std::fs::metadata(self.inner.path.join(DATA_FILE))?.len())
    }

    /// Write a compacted copy of the committed state to `dest`, a new file.
    pub(crate) fn write_image(&self, dest: &Path, work: &WorkContext) -> Result<()> {
        let _pass = self.inner.gate.enter(work)?;
        let file = self
            .inner
            .env
            .copy_to_path(dest, heed::CompactionOption::Enabled)
            .map_err(Error::from)?;
        file.sync_all()?;
        Ok(())
    }

    /// Bounded close: stop admitting transactions, wait for live ones until
    /// they drain or the caller cancels, and report. Never invalidates a
    /// live snapshot.
    #[must_use = "an incomplete close reports the live readers to release"]
    pub(crate) fn close(&self, work: &WorkContext) -> CloseReport {
        let (drained, snapshot) = self.inner.gate.begin_close(work);
        if drained {
            CloseReport::Closed
        } else {
            CloseReport::Incomplete {
                live_transactions: snapshot.live,
                oldest_age: snapshot.oldest_age,
            }
        }
    }

    /// The committed generation through a private gated view.
    pub(crate) fn committed_generation(&self, work: &WorkContext) -> Result<GenerationId> {
        let _pass = self.inner.gate.enter(work)?;
        let rtxn = self.inner.env.read_txn().map_err(Error::from)?;
        read_generation(&self.inner, &rtxn)
    }
}

pub(crate) fn read_generation(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
) -> Result<GenerationId> {
    Ok(GenerationId::from_storage(format::read_u64(
        &inner.meta,
        txn,
        K_GENERATION,
        "generation",
    )?))
}

/// Initialize one empty store inside a staging directory; the environment
/// closes before return.
pub(crate) fn init_directory(
    staging: &Path,
    schema: &Schema,
    database: DatabaseId,
    options: Options,
) -> Result<()> {
    let _lock = acquire_lock(staging)?;
    let env = open_env(staging, options)?;
    let mut wtxn = env.write_txn().map_err(Error::from)?;
    let meta: Database<Bytes, Bytes> = env
        .create_database(&mut wtxn, Some(META_DB))
        .map_err(Error::from)?;
    for name in [ROWS_DB, DET_DB] {
        let _: Database<Bytes, Bytes> = env
            .create_database(&mut wtxn, Some(name))
            .map_err(Error::from)?;
    }
    for (key, value) in [
        (K_FORMAT, FORMAT.as_slice()),
        (K_SCHEMA, fingerprint(schema).0.as_slice()),
        (K_DATABASE, database.0.as_slice()),
        (
            K_GENERATION,
            GenerationId::initial()
                .storage_word()
                .to_be_bytes()
                .as_slice(),
        ),
        (K_NEXT_ROW_ID, 1u64.to_be_bytes().as_slice()),
    ] {
        meta.put(&mut wtxn, key, value).map_err(Error::from)?;
    }
    wtxn.commit().map_err(Error::from)?;
    env.force_sync().map_err(Error::from)?;
    Ok(())
}

#[cfg(test)]
impl Store {
    #[expect(
        clippy::used_underscore_binding,
        reason = "the lock is held for its drop; only tests read it"
    )]
    pub(crate) fn duplicate_lock_for_tests(&self) -> std::fs::File {
        self.inner
            ._lock
            .file
            .try_clone()
            .expect("duplicate lock description")
    }

    pub(crate) fn flags_for_tests(&self) -> u32 {
        self.inner.env.get_flags().expect("env flags")
    }

    pub(crate) fn put_meta_for_tests(&self, key: &[u8], value: &[u8]) {
        let mut wtxn = self
            .gated_write_txn(&WorkContext::new())
            .expect("fixture txn");
        self.inner
            .meta
            .put(&mut wtxn.txn, key, value)
            .expect("fixture put");
        wtxn.commit().expect("fixture commit");
    }

    /// Make the Nth host record of a seal fail with `Full`.
    pub(crate) fn fail_host_seal_after(&self, applied_records: Option<usize>) {
        *self
            .inner
            .fail_host_after
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = applied_records;
    }
}

fn writer_thread_key() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    thread_local! {
        static KEY: u64 = NEXT.fetch_add(1, Ordering::Relaxed);
    }
    KEY.with(|key| *key)
}
