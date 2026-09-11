//! The stable local target namespace: kernel exclusion, durable pre-genesis
//! tombstones and no-overwrite final installation.
//!
//! Local final-target publication and local abort share ONE stable
//! target-namespace kernel lock that lives OUTSIDE every staging or
//! materialization directory, so a renamed/replaced directory can never
//! carry the exclusion away. Under that lock, cancelling an unpublished
//! target durably installs a terminal tombstone (the P04 control frame of a
//! `cancelled_before_genesis` authority) BEFORE any genesis exists, and the
//! final genesis installation is no-overwrite and refuses the tombstone —
//! a precomputed rename cannot bypass it. Tombstones and lock files are
//! stable namespace entries, never scratch or cache-eviction targets.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::history::authority::{
    Activation, HeadAuthority, Lifecycle, decode_control, encode_control,
};
use crate::history::{FrameError, IncarnationId};

/// Namespace layout under one caller-supplied stable root:
/// `<hex>.lock` (kernel lock), `<hex>.tombstone` (durable cancellation),
/// `<hex>.activation` (durable one-time activation evidence),
/// `<hex>/db/` (the published materialization), `~stage/` (private staging builds).
pub struct TargetNamespace {
    root: PathBuf,
    hex: String,
}

/// Exclusive ownership of one target namespace, released by the file's
/// destructor (including process death). No expiry, renewal or takeover.
#[derive(Debug)]
pub struct NamespaceLock {
    file: File,
    path: PathBuf,
}

impl Drop for NamespaceLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Why a namespace operation refused.
#[derive(Debug)]
pub enum NamespaceError {
    Io(io::Error),
    /// Another handle owns the namespace right now.
    Busy,
    /// A namespace path is a symlink — hostile layouts refuse.
    Symlink,
    /// The tombstone bytes are malformed or bind a different operation.
    ForeignTombstone,
    /// The activation-evidence bytes are malformed, not an activated
    /// authority, or bind a different activation.
    ForeignActivation,
    /// The target materialization already exists (no-overwrite).
    TargetExists,
    Frame(FrameError),
}

impl From<io::Error> for NamespaceError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FrameError> for NamespaceError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl std::fmt::Display for NamespaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "target namespace: {error}"),
            Self::Busy => write!(f, "target namespace is owned"),
            Self::Symlink => write!(f, "target namespace path is a symlink"),
            Self::ForeignTombstone => write!(f, "tombstone binds a different operation"),
            Self::ForeignActivation => {
                write!(f, "activation evidence binds a different activation")
            }
            Self::TargetExists => write!(f, "target materialization already exists"),
            Self::Frame(error) => write!(f, "tombstone frame: {error:?}"),
        }
    }
}

impl std::error::Error for NamespaceError {}

fn refuse_symlink(path: &Path) -> Result<(), NamespaceError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(NamespaceError::Symlink),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    crate::json::push_hex(&mut out, bytes);
    out
}

static STAGE_SEQ: AtomicU64 = AtomicU64::new(0);

impl TargetNamespace {
    /// Bind the namespace for one planned target incarnation under `root`.
    /// Binding is read-only: status calls never create directories; only
    /// [`Self::lock`] materializes the namespace root.
    /// # Errors
    /// Symlinked namespace paths refuse.
    pub fn new(root: &Path, incarnation: IncarnationId) -> Result<Self, NamespaceError> {
        refuse_symlink(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            hex: hex(incarnation.as_core().as_bytes()),
        })
    }

    #[must_use]
    pub fn target_dir(&self) -> PathBuf {
        self.deployment_dir().join("db")
    }

    /// Stable tenant directory containing the target's materialization. The
    /// local history binding names this directory, never the raw LMDB files.
    #[must_use]
    pub fn deployment_dir(&self) -> PathBuf {
        self.root.join(&self.hex)
    }

    fn lock_path(&self) -> PathBuf {
        self.root.join(format!("{}.lock", self.hex))
    }

    fn tombstone_path(&self) -> PathBuf {
        self.root.join(format!("{}.tombstone", self.hex))
    }

    fn activation_path(&self) -> PathBuf {
        self.root.join(format!("{}.activation", self.hex))
    }

    /// A fresh private staging directory path for one build attempt. Never
    /// adopted by name: only an explicit locked install publishes anything.
    #[must_use]
    pub fn fresh_staging(&self) -> PathBuf {
        let seq = STAGE_SEQ.fetch_add(1, Ordering::Relaxed);
        self.root
            .join("~stage")
            .join(format!("{}.{}.{}", self.hex, std::process::id(), seq))
    }

    /// Acquire the stable namespace lock, without waiting. A paused owner
    /// keeps it; elapsed time proves nothing.
    /// # Errors
    /// `Busy` while another handle owns it; filesystem failures refuse.
    pub fn lock(&self) -> Result<NamespaceLock, NamespaceError> {
        self.lock_file(&self.lock_path())
    }

    /// One live population attempt per target. This lock is retained until
    /// the attempt's admitted work drains and its private store is dropped.
    /// Abort uses the separate namespace lock, so it can fence a live attempt.
    /// # Errors
    /// `Busy` while a prior attempt retains write authority.
    pub fn population_lock(&self) -> Result<NamespaceLock, NamespaceError> {
        self.lock_file(&self.root.join(format!("{}.population-lock", self.hex)))
    }

    fn check_lock(lock: &NamespaceLock, expected: &Path) -> Result<(), NamespaceError> {
        if lock.path != expected {
            return Err(NamespaceError::ForeignTombstone);
        }
        Ok(())
    }

    /// Remove only abandoned attempts for this target after obtaining its
    /// lifetime population lock. No live owner can still reach these stores.
    /// # Errors
    /// Wrong lock ownership, symlinks or filesystem failure.
    pub fn clean_abandoned(&self, attempt: &NamespaceLock) -> Result<(), NamespaceError> {
        Self::check_lock(
            attempt,
            &self.root.join(format!("{}.population-lock", self.hex)),
        )?;
        let root = self.root.join("~stage");
        refuse_symlink(&root)?;
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let prefix = format!("{}.", self.hex);
        for entry in entries {
            let entry = entry?;
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(&prefix))
            {
                refuse_symlink(&entry.path())?;
                fs::remove_dir_all(entry.path())?;
            }
        }
        Ok(())
    }

    fn lock_file(&self, path: &Path) -> Result<NamespaceLock, NamespaceError> {
        fs::create_dir_all(&self.root)?;
        refuse_symlink(path)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err(NamespaceError::Symlink);
        }
        match file.try_lock() {
            Ok(()) => Ok(NamespaceLock {
                file,
                path: path.to_path_buf(),
            }),
            Err(TryLockError::WouldBlock) => Err(NamespaceError::Busy),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }

    /// Whether a published target materialization exists.
    #[must_use]
    pub fn target_exists(&self) -> bool {
        self.target_dir().is_dir()
    }

    /// Read the durable cancellation, if any. The bytes are the P04 control
    /// frame of the cancelled-before-genesis authority; malformed bytes are
    /// corruption evidence, never an absent tombstone.
    /// # Errors
    /// Filesystem failures and malformed frames refuse.
    pub fn read_tombstone(&self, cap: usize) -> Result<Option<HeadAuthority>, NamespaceError> {
        let path = self.tombstone_path();
        refuse_symlink(&path)?;
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.take((cap as u64).saturating_add(1))
            .read_to_end(&mut bytes)?;
        let authority = decode_control(&bytes, cap)?;
        if !matches!(authority.lifecycle, Lifecycle::Deleted { .. }) {
            return Err(NamespaceError::ForeignTombstone);
        }
        Ok(Some(authority))
    }

    /// Durably install the pre-genesis cancellation under the held lock:
    /// create-new, write, fsync file and directory. An existing matching
    /// tombstone is idempotent evidence; a conflicting one refuses. The
    /// tombstone is terminal — nothing ever removes it, and the namespace
    /// is never reused.
    /// # Errors
    /// Conflicting recorded cancellation and filesystem failures.
    pub fn install_tombstone(
        &self,
        lock: &NamespaceLock,
        tombstone: &HeadAuthority,
        cap: usize,
    ) -> Result<(), NamespaceError> {
        Self::check_lock(lock, &self.lock_path())?;
        if let Some(existing) = self.read_tombstone(cap)? {
            if existing == *tombstone {
                return Ok(());
            }
            return Err(NamespaceError::ForeignTombstone);
        }
        let bytes = encode_control(tombstone, cap)?;
        let path = self.tombstone_path();
        publish_record(&self.root, &path, &bytes)
    }

    /// Read the durable activation evidence, if any. The bytes are the P04
    /// control frame captured AT the one-time activation commit — recorded
    /// evidence derived from the target's control (which stays the ONE
    /// authority), readable while a live owner holds the target store open.
    /// Malformed or non-activated bytes are corruption evidence, never an
    /// absent marker.
    /// # Errors
    /// Filesystem failures, malformed frames and non-activated controls.
    pub fn read_activation(&self, cap: usize) -> Result<Option<HeadAuthority>, NamespaceError> {
        let path = self.activation_path();
        refuse_symlink(&path)?;
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.take((cap as u64).saturating_add(1))
            .read_to_end(&mut bytes)?;
        let authority = decode_control(&bytes, cap)?;
        if !matches!(authority.activation, Activation::Activated { .. }) {
            return Err(NamespaceError::ForeignActivation);
        }
        Ok(Some(authority))
    }

    /// Durably record the one-time activation evidence under the held lock:
    /// create-new, write, fsync file and directory. An existing marker whose
    /// identity and activation match is idempotent evidence (an activate
    /// retry heals the crash window between the control commit and this
    /// write); a conflicting one refuses. The marker is never removed.
    /// # Errors
    /// Conflicting recorded activation and filesystem failures.
    pub fn record_activation(
        &self,
        lock: &NamespaceLock,
        activated: &HeadAuthority,
        cap: usize,
    ) -> Result<(), NamespaceError> {
        Self::check_lock(lock, &self.lock_path())?;
        if !matches!(activated.activation, Activation::Activated { .. }) {
            return Err(NamespaceError::ForeignActivation);
        }
        if let Some(existing) = self.read_activation(cap)? {
            if existing.identity == activated.identity
                && existing.activation == activated.activation
            {
                return Ok(());
            }
            return Err(NamespaceError::ForeignActivation);
        }
        let bytes = encode_control(activated, cap)?;
        let path = self.activation_path();
        publish_record(&self.root, &path, &bytes)
    }

    /// No-overwrite final installation of a completely built staging
    /// directory, under the held lock: refuse the tombstone, refuse an
    /// existing target, rename, fsync the namespace directory. Durability
    /// completes before the lock is released by the caller.
    /// # Errors
    /// A recorded cancellation, an existing target and filesystem failures.
    pub fn install_target(
        &self,
        lock: &NamespaceLock,
        staged: &Path,
        cap: usize,
    ) -> Result<(), NamespaceError> {
        Self::check_lock(lock, &self.lock_path())?;
        if self.read_tombstone(cap)?.is_some() {
            return Err(NamespaceError::ForeignTombstone);
        }
        let target = self.target_dir();
        let deployment = self.deployment_dir();
        refuse_symlink(&deployment)?;
        refuse_symlink(&target)?;
        if target.exists() {
            return Err(NamespaceError::TargetExists);
        }
        fs::create_dir_all(&deployment)?;
        fs::rename(staged, &target)?;
        File::open(&deployment)?.sync_all()?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
}

// Write and sync private bytes before atomically linking the final name.
// Process death can leave an unused private file, never a torn authoritative
// record; create-new linking also prevents overwriting another operation.
fn publish_record(root: &Path, path: &Path, bytes: &[u8]) -> Result<(), NamespaceError> {
    struct Remove(PathBuf);
    impl Drop for Remove {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let seq = STAGE_SEQ.fetch_add(1, Ordering::Relaxed);
    let temporary = root.join(format!(".transition-record-{}-{seq}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let _remove = Remove(temporary.clone());
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::hard_link(&temporary, path)?;
    File::open(root)?.sync_all()?;
    Ok(())
}
