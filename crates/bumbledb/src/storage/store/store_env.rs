//! The one store owner: environment lifecycle, the fixed virtual map, the
//! close gate, directory ownership and close.
//!
//! Create publishes through a staged sibling directory (lock, write meta,
//! commit, fsync files and dirent chain, rename, fsync the parent), so a
//! crash leaves no destination or a complete one. Open takes the kernel lock
//! first, then verifies family, layout and schema before touching anything.
//! The map is one virtual reservation fixed at open: it costs address space,
//! not RAM or disk, because without `WRITEMAP` the file grows only as pages
//! are written.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, TryLockError};
use std::time::Duration;

use heed::types::Bytes;
use heed::{Database, EnvOpenOptions, RoTxn, RwTxn, WithoutTls};

use super::candidate::{CandidateState, Judgment};
use super::copy::FreshDestination;
use super::det_index::DeterminantTable;
use super::error::{StoreError, StoreResult};
use super::fingerprint::Fingerprinter;
use super::format::{
    self, CoreStoreId, DATA_DB, EnvironmentId, FAMILY, K_FAMILY, K_GENERATION, K_LAYOUT,
    K_NEXT_ROW_ID, K_SCHEMA, K_STORE_ID, LAYOUT, META_DB, StoreIdentity,
};
use super::gate::{GatePass, TransactionGate};
use super::judge_bridge::SchemaJudge;
use super::snapshot::OwnedSnapshot;
use crate::schema::Schema;
use crate::schema::fingerprint::{SchemaFingerprint, fingerprint};
use crate::storage::GenerationId;
use crate::work::WorkContext;

const LOCK_FILE: &str = "bumbledb.lock";
const MAX_READERS: u32 = 1024;

/// The default virtual map ceiling: 1 TiB of address space.
pub const DEFAULT_MAP_CEILING: u64 = 1 << 40;

pub(crate) struct StoreInner {
    // Field order is drop order: transactions are gone (gate drained or the
    // holders own env clones), the environment closes before the kernel
    // lock releases, and the lock releases last.
    pub(crate) env: heed::Env<WithoutTls>,
    pub(crate) meta: Database<Bytes, Bytes>,
    pub(crate) data: super::keys::DataTree,
    pub(crate) keys: super::keys::KeyLayout,
    pub(crate) gate: TransactionGate,
    writer: Mutex<()>,
    writer_thread: AtomicU64,
    ceiling: u64,
    pub(crate) identity: StoreIdentity,
    pub(crate) schema_fp: SchemaFingerprint,
    pub(crate) fingerprinter: Fingerprinter,
    /// Compiled schema-derived determinant projections: index maintenance
    /// and probe-side projection share this one table (see
    /// [`super::det_index`]).
    pub(crate) det: DeterminantTable,
    path: PathBuf,
    #[cfg(test)]
    pub(crate) fail_host_after: Mutex<Option<usize>>,
    /// Kernel-held directory ownership; held through native close, released
    /// last by field order.
    _lock: DirectoryLock,
}

/// The store owner. `Send + Sync`; clones share one environment and its
/// close gate and admitted readers.
pub struct Store {
    pub(crate) inner: Arc<StoreInner>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("store", &self.inner.identity.store)
            .field("environment", &self.inner.identity.environment)
            .field("path", &self.inner.path)
            .finish()
    }
}

/// Outcome of a bounded close attempt. `Incomplete` retains Closing state:
/// admission stays refused, resources release when the live snapshots drop,
/// and the caller may join with another `close` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReport {
    Closed,
    Incomplete {
        live_transactions: u64,
        oldest_age: Option<Duration>,
    },
}

/// A write transaction admitted through the gate. Declared txn-before-pass:
/// the transaction aborts/commits before its gate slot releases.
pub(crate) struct GatedRwTxn<'env> {
    pub(crate) txn: RwTxn<'env>,
    ceiling: u64,
    _pass: GatePass,
}

impl GatedRwTxn<'_> {
    pub(crate) fn commit(self) -> StoreResult<()> {
        let ceiling = self.ceiling;
        self.txn.commit().map_err(|error| {
            if matches!(error, heed::Error::Mdb(heed::MdbError::MapFull)) {
                StoreError::Full { ceiling }
            } else {
                StoreError::from_heed(error)
            }
        })
    }
}

impl StoreInner {
    /// A write-path LMDB failure: map exhaustion is the typed fixed-ceiling
    /// refusal, everything else keeps its LMDB identity.
    pub(crate) fn txn_error(&self, error: heed::Error) -> StoreError {
        if matches!(error, heed::Error::Mdb(heed::MdbError::MapFull)) {
            StoreError::Full {
                ceiling: self.ceiling,
            }
        } else {
            StoreError::from_heed(error)
        }
    }
}

#[expect(
    unsafe_code,
    reason = "heed marks environment opening unsafe: double-opening one path \
              in a process is LMDB UB. The kernel directory lock is acquired \
              before every open, so each directory has one live environment."
)]
fn open_env(path: &Path, ceiling: u64) -> StoreResult<heed::Env<WithoutTls>> {
    let mut options = EnvOpenOptions::new().read_txn_without_tls();
    options
        .map_size(usize::try_from(ceiling).map_err(|_| StoreError::Full { ceiling })?)
        .max_dbs(2)
        .max_readers(MAX_READERS);
    // SAFETY: single open per directory, enforced by the held kernel lock;
    // no env flags are set — durable defaults only.
    unsafe { options.open(path) }.map_err(StoreError::from_heed)
}

/// Owns the lock, not merely one reference to its file description. A
/// concurrent subprocess can inherit that description until exec closes
/// it; closing only our File would leave the old flock held in that window.
/// This guard must drop after every native environment/transaction owner.
struct DirectoryLock {
    file: std::fs::File,
}

impl Drop for DirectoryLock {
    fn drop(&mut self) {
        // Drop cannot propagate an I/O refusal or panic during unwinding.
        // On unlock failure, File still closes normally: inherited handles
        // may prolong exclusion, but we never reopen before native close.
        let _ = self.file.unlock();
    }
}

fn acquire_lock(path: &Path) -> StoreResult<DirectoryLock> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.join(LOCK_FILE))?;
    match file.try_lock() {
        Ok(()) => Ok(DirectoryLock { file }),
        Err(std::fs::TryLockError::WouldBlock) => Err(StoreError::StoreLocked {
            path: path.to_path_buf(),
        }),
        Err(std::fs::TryLockError::Error(err)) => Err(StoreError::from(err)),
    }
}

pub(crate) fn sync_dirent_chain(dir: &Path) -> std::io::Result<()> {
    let parent = match dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    for d in [dir, parent] {
        std::fs::File::open(d)?.sync_all()?;
    }
    Ok(())
}

impl Store {
    /// Create a new store. The destination must not exist; a crash before
    /// the final rename leaves only a staging sibling, never a half store.
    /// Returns the minted [`FreshDestination`] capability for snapshot adoption.
    /// # Errors
    /// `DestinationExists`, lock/I/O/LMDB failures.
    pub fn create(
        path: &Path,
        schema: &Schema,
        ceiling: u64,
    ) -> StoreResult<(Self, FreshDestination)> {
        if path.exists() {
            return Err(StoreError::DestinationExists {
                path: path.to_path_buf(),
            });
        }
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }
        let staging = staging_path(path)?;
        let created: StoreResult<()> = (|| {
            init_staging_directory(&staging, path, schema, ceiling)?;
            for entry in std::fs::read_dir(&staging)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    std::fs::File::open(entry.path())?.sync_all()?;
                }
            }
            sync_dirent_chain(&staging)?;
            std::fs::rename(&staging, path)?;
            sync_dirent_chain(path)?;
            Ok(())
        })();
        if let Err(error) = created {
            let _ = std::fs::remove_dir_all(&staging);
            if path.exists() {
                return Err(StoreError::InstallSettlementFailed {
                    path: path.to_path_buf(),
                    detail: Box::new(error),
                });
            }
            return Err(error);
        }
        Self::open(path, schema, ceiling).map(|store| (store, FreshDestination::mint()))
    }

    /// Populate a new store in a private staging directory, then publish it
    /// atomically to `dest`. Prefer [`super::staging`] for the
    /// full unready → admitted → installed ownership path.
    /// # Errors
    /// `DestinationExists`, population failure, lock/I/O/LMDB failures.
    pub fn install_populated(
        dest: &Path,
        schema: &Schema,
        ceiling: u64,
        work: &WorkContext,
        populate: impl FnOnce(&super::staging::StageWriter<'_>, &WorkContext) -> StoreResult<()>,
    ) -> StoreResult<Self> {
        super::staging::install_populated(dest, schema, ceiling, work, populate)
    }

    /// Open an existing store. Acquires the kernel lock first; verifies the
    /// family, layout and schema against one read view **before** any
    /// cleanup, adoption, or write. A refused open mutates nothing.
    /// # Errors
    /// `StoreLocked`, `UnrecognizedStore`, `LayoutMismatch`,
    /// `SchemaMismatch`, `Compile`, corruption, lock/I/O/LMDB failures.
    pub fn open(path: &Path, schema: &Schema, ceiling: u64) -> StoreResult<Self> {
        Self::open_with(path, schema, ceiling, Fingerprinter::Blake3)
    }

    /// HASH-02 probe constructor: open with a caller-supplied
    /// fingerprint function. Bench/test builds only — the production
    /// constructors above cannot reach a non-BLAKE3 fingerprinter, so the
    /// default hash role is never weakened by this seam.
    /// # Errors
    /// As [`Store::open`].
    #[cfg(any(test, feature = "testing"))]
    pub fn open_with_fingerprinter(
        path: &Path,
        schema: &Schema,
        ceiling: u64,
        fingerprinter: Fingerprinter,
    ) -> StoreResult<Self> {
        Self::open_with(path, schema, ceiling, fingerprinter)
    }

    /// HASH-02 probe constructor: create with the production protocol,
    /// then select the forced-collision bucket function before publication so a bench
    /// probe can drive insert/contains/delete/judgment/export through real
    /// collision buckets. Bench/test builds only; a store written this way
    /// is refused by production reopen (its membership keys do not match).
    /// # Errors
    /// As [`Store::create`].
    /// # Panics
    /// If the production constructor no longer returns an exclusively owned store.
    #[cfg(any(test, feature = "testing"))]
    pub fn create_forced_fingerprint(
        path: &Path,
        schema: &Schema,
        ceiling: u64,
        fp: [u8; super::fingerprint::FP_LEN],
    ) -> StoreResult<Self> {
        // The production constructor returns an empty store with one owner.
        // Select the test fingerprinter before publishing it; no row/index
        // was written, so there is no reason to close and reopen the store.
        let (mut store, _) = Self::create(path, schema, ceiling)?;
        Arc::get_mut(&mut store.inner)
            .expect("freshly created store has one owner")
            .fingerprinter = Fingerprinter::Constant(fp);
        Ok(store)
    }

    fn open_with(
        path: &Path,
        schema: &Schema,
        ceiling: u64,
        fingerprinter: Fingerprinter,
    ) -> StoreResult<Self> {
        let det = DeterminantTable::compile(schema)?;
        let keys = super::keys::KeyLayout::for_schema(schema)?;
        let lock = acquire_lock(path)?;
        let env = open_env(path, ceiling)?;
        let schema_fp = fingerprint(schema);
        let (meta, data, store_id) = {
            let rtxn = env.read_txn().map_err(StoreError::from_heed)?;
            let meta: Option<Database<Bytes, Bytes>> = env
                .open_database(&rtxn, Some(META_DB))
                .map_err(StoreError::from_heed)?;
            let Some(meta) = meta else {
                return Err(StoreError::UnrecognizedStore {
                    path: path.to_path_buf(),
                });
            };
            let store_id = format::verify_meta(&meta, &rtxn, path, &schema_fp)?;
            let data = env
                .database_options()
                .types::<Bytes, Bytes>()
                .key_comparator::<super::keys::PhysicalComparator>()
                .name(DATA_DB)
                .open(&rtxn)
                .map_err(StoreError::from_heed)?;
            let Some(data) = data else {
                return Err(StoreError::Corruption(
                    super::error::StoreCorruption::MetaMissing("data database"),
                ));
            };
            // heed: commit the read transaction used for database opening.
            rtxn.commit().map_err(StoreError::from_heed)?;
            (meta, super::keys::DataTree(data), store_id)
        };
        Ok(Self {
            inner: Arc::new(StoreInner {
                env,
                meta,
                data,
                keys,
                gate: TransactionGate::default(),
                writer: Mutex::new(()),
                writer_thread: AtomicU64::new(0),
                ceiling,
                identity: StoreIdentity {
                    store: store_id,
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

    #[must_use]
    pub fn identity(&self) -> StoreIdentity {
        self.inner.identity
    }

    #[must_use]
    pub fn store_id(&self) -> CoreStoreId {
        self.inner.identity.store
    }

    #[must_use]
    pub fn environment_id(&self) -> EnvironmentId {
        self.inner.identity.environment
    }

    #[must_use]
    pub fn schema_fingerprint(&self) -> SchemaFingerprint {
        self.inner.schema_fp
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// One coherent owned snapshot: rows, generation, and host attachment
    /// all derive from the single read transaction opened here.
    /// # Errors
    /// Refuses a closing store, exhausted reader slots, or stopped work.
    pub fn snapshot(&self, work: &WorkContext) -> StoreResult<OwnedSnapshot> {
        let pass = self.inner.gate.enter(work)?;
        let txn = self
            .inner
            .env
            .clone()
            .static_read_txn()
            .map_err(StoreError::from_heed)?;
        let generation = read_generation(&self.inner, &txn)?;
        Ok(OwnedSnapshot::capture(
            Arc::clone(&self.inner),
            pass,
            txn,
            generation,
        ))
    }

    /// The single writer capability. One per store; reentrant acquisition
    /// from the owning thread refuses instead of deadlocking.
    /// # Errors
    /// Refuses a closing store, reentrancy, or stopped work while waiting.
    pub fn writer(&self, work: &WorkContext) -> StoreResult<super::candidate::WriteOwner<'_>> {
        let caller = writer_thread_key();
        if self.inner.writer_thread.load(Ordering::Acquire) == caller {
            return Err(StoreError::ReentrantWriter);
        }
        let guard = loop {
            work.checkpoint()?;
            match self.inner.writer.try_lock() {
                Ok(guard) => break guard,
                Err(TryLockError::Poisoned(poisoned)) => break poisoned.into_inner(),
                Err(TryLockError::WouldBlock) => {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        };
        self.inner.writer_thread.store(caller, Ordering::Release);
        Ok(super::candidate::WriteOwner::new(self, guard, work.clone()))
    }

    pub(crate) fn release_writer_thread(&self) {
        self.inner.writer_thread.store(0, Ordering::Release);
    }

    /// Begin one gated write transaction (writer mutex already held by the
    /// calling owner).
    pub(crate) fn gated_write_txn(&self, work: &WorkContext) -> StoreResult<GatedRwTxn<'_>> {
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

    /// The map ceiling fixed at open.
    #[must_use]
    pub fn ceiling(&self) -> u64 {
        self.inner.ceiling
    }

    /// Length of `data.mdb`: the populated file, not the virtual map.
    /// # Errors
    /// I/O failure reading the file metadata.
    pub fn file_bytes(&self) -> StoreResult<u64> {
        Ok(std::fs::metadata(self.inner.path.join("data.mdb"))?.len())
    }

    /// Bounded close: stop admitting transactions, drain within the work
    /// budget, and report. `Incomplete` keeps Closing state — admission
    /// stays refused, retained snapshots keep their own environment clones
    /// alive, and the directory lock releases only when the last of them
    /// drops (lock release is last by field order). Close never invalidates
    /// a live snapshot.
    /// # Errors
    /// None: the report states reality; repeated close joins the drain.
    #[must_use = "an incomplete close reports the live readers to release"]
    pub fn close(&self, work: &WorkContext) -> CloseReport {
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

    /// Read the committed generation through a private gated view.
    /// # Errors
    /// Storage failure or a closing store.
    pub fn committed_generation(&self, work: &WorkContext) -> StoreResult<GenerationId> {
        let _pass = self.inner.gate.enter(work)?;
        let rtxn = self.inner.env.read_txn().map_err(StoreError::from_heed)?;
        read_generation(&self.inner, &rtxn)
    }

    /// Complete production judgment over the committed populated state.
    /// Used by unready admission; does not mint a lawful parent.
    pub(crate) fn judge_populated(
        &self,
        schema: &Schema,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Box<[crate::schema::judge::JudgedViolation]>>> {
        let owner = self.writer(work)?;
        let txn = self.gated_write_txn(work)?;
        let state = CandidateState::of_committed(self, &txn);
        let judged = SchemaJudge::new(schema).judge_complete(&state, work);
        drop(txn);
        drop(owner);
        judged
    }
}

pub(crate) fn read_generation(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
) -> StoreResult<GenerationId> {
    Ok(GenerationId::from_storage(format::read_u64(
        &inner.meta,
        txn,
        K_GENERATION,
        "generation",
    )?))
}

/// Initialize one empty successor store inside an already-created staging
/// directory. The kernel lock is held until the environment closes.
pub(crate) fn init_staging_directory(
    staging: &Path,
    dest: &Path,
    schema: &Schema,
    ceiling: u64,
) -> StoreResult<()> {
    let _lock = acquire_lock(staging)?;
    let store_id = CoreStoreId::mint(dest);
    let schema_fp = fingerprint(schema);
    let env = open_env(staging, ceiling)?;
    let mut wtxn = env.write_txn().map_err(StoreError::from_heed)?;
    let meta: Database<Bytes, Bytes> = env
        .create_database(&mut wtxn, Some(META_DB))
        .map_err(StoreError::from_heed)?;
    let _data = env
        .database_options()
        .types::<Bytes, Bytes>()
        .key_comparator::<super::keys::PhysicalComparator>()
        .name(DATA_DB)
        .create(&mut wtxn)
        .map_err(StoreError::from_heed)?;
    let put = |wtxn: &mut RwTxn<'_>, key: &[u8], value: &[u8]| {
        meta.put(wtxn, key, value).map_err(StoreError::from_heed)
    };
    put(&mut wtxn, K_FAMILY, FAMILY)?;
    put(&mut wtxn, K_LAYOUT, &LAYOUT.to_be_bytes())?;
    put(&mut wtxn, K_STORE_ID, &store_id.0)?;
    put(&mut wtxn, K_SCHEMA, &schema_fp.0)?;
    put(
        &mut wtxn,
        K_GENERATION,
        &GenerationId::initial().storage_word().to_be_bytes(),
    )?;
    put(&mut wtxn, K_NEXT_ROW_ID, &1u64.to_be_bytes())?;
    wtxn.commit().map_err(StoreError::from_heed)?;
    Ok(())
}

/// Publication evidence for one install attempt. Rename success — not
/// `dest.exists()` — distinguishes this attempt's install from a preexisting
/// destination.
#[derive(Debug)]
pub(crate) enum PublishOutcome {
    Installed(Store),
    PublishedUnsettled { dest: PathBuf, detail: StoreError },
    DestinationOccupied { path: PathBuf },
    NotPublished(StoreError),
}

/// Fsync, rename staging into `dest`, fsync the chain, and reopen.
pub(crate) fn publish_staging(
    staging: &Path,
    dest: &Path,
    schema: &Schema,
    ceiling: u64,
    work: &WorkContext,
) -> PublishOutcome {
    if dest.exists() {
        return PublishOutcome::DestinationOccupied {
            path: dest.to_path_buf(),
        };
    }
    if let Err(detail) = work.checkpoint() {
        return PublishOutcome::NotPublished(StoreError::Work(detail));
    }
    if let Err(detail) = (|| -> StoreResult<()> {
        for entry in std::fs::read_dir(staging)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                std::fs::File::open(entry.path())?.sync_all()?;
            }
        }
        sync_dirent_chain(staging)?;
        Ok(())
    })() {
        return PublishOutcome::NotPublished(detail);
    }
    match std::fs::rename(staging, dest) {
        Ok(()) => {}
        Err(_error) if dest.exists() => {
            return PublishOutcome::DestinationOccupied {
                path: dest.to_path_buf(),
            };
        }
        Err(error) => return PublishOutcome::NotPublished(StoreError::from(error)),
    }
    if let Err(detail) = sync_dirent_chain(dest) {
        return PublishOutcome::PublishedUnsettled {
            dest: dest.to_path_buf(),
            detail: StoreError::from(detail),
        };
    }
    match Store::open(dest, schema, ceiling) {
        Ok(store) => PublishOutcome::Installed(store),
        Err(detail) => PublishOutcome::PublishedUnsettled {
            dest: dest.to_path_buf(),
            detail,
        },
    }
}

pub(crate) fn staging_path(dest: &Path) -> StoreResult<PathBuf> {
    static NONCE: AtomicU64 = AtomicU64::new(1);
    for _ in 0..16 {
        let nonce = u64::from(std::process::id())
            ^ NONCE
                .fetch_add(1, Ordering::Relaxed)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let name = dest.file_name().unwrap_or(dest.as_os_str());
        let staging =
            dest.with_file_name(format!("{}.staging.{nonce:016x}", name.to_string_lossy()));
        match std::fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
            Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
                return Err(StoreError::from(error));
            }
            Err(_) => {}
        }
    }
    Err(StoreError::from(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "exhausted staging nonces",
    )))
}

/// Test-only fixture surgery and inspection. Never reachable from a
/// production build; mutating meta through these hooks intentionally makes
/// the fixture refuse or misbehave in the exact way under test.
#[cfg(test)]
impl Store {
    #[expect(
        clippy::used_underscore_binding,
        reason = "the regression retains a duplicate of the exact directory-lock description"
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

    pub(crate) fn force_layout_for_tests(&self, layout: u32) {
        let mut wtxn = self
            .gated_write_txn(&super::tests::work())
            .expect("layout fixture txn");
        self.inner
            .meta
            .put(&mut wtxn.txn, K_LAYOUT, &layout.to_be_bytes())
            .expect("layout fixture put");
        wtxn.commit().expect("layout fixture commit");
    }

    pub(crate) fn corrupt_family_for_tests(&self) {
        let mut wtxn = self
            .gated_write_txn(&super::tests::work())
            .expect("family fixture txn");
        self.inner
            .meta
            .put(&mut wtxn.txn, K_FAMILY, b"WRONGFAM")
            .expect("family fixture put");
        wtxn.commit().expect("family fixture commit");
    }

    pub(crate) fn force_next_row_id_for_tests(&self, next: u64) {
        let mut wtxn = self
            .gated_write_txn(&super::tests::work())
            .expect("row id fixture txn");
        self.inner
            .meta
            .put(&mut wtxn.txn, K_NEXT_ROW_ID, &next.to_be_bytes())
            .expect("row id fixture put");
        wtxn.commit().expect("row id fixture commit");
    }

    /// Make the Nth applied host record fail with a map-full error during
    /// seal, proving a failed seal drops the whole private transaction.
    pub(crate) fn fail_host_seal_after(&self, applied_records: Option<usize>) {
        *self
            .inner
            .fail_host_after
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = applied_records;
    }
}

fn writer_thread_key() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    thread_local! {
        static KEY: u64 = NEXT.fetch_add(1, Ordering::Relaxed);
    }
    KEY.with(|key| *key)
}
