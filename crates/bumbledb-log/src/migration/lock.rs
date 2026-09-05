//! The stable local target namespace: kernel exclusion, durable pre-genesis
//! tombstones and no-overwrite final installation (chapter 22).
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
/// `<hex>/` (the published target), `~stage/` (private staging builds).
pub struct TargetNamespace {
    root: PathBuf,
    hex: String,
}

/// Exclusive ownership of one target namespace, released by the file's
/// destructor (including process death). No expiry, renewal or takeover.
#[derive(Debug)]
pub struct NamespaceLock {
    file: File,
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
    super::json::push_hex(&mut out, bytes);
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
        fs::create_dir_all(&self.root)?;
        let path = self.lock_path();
        refuse_symlink(&path)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)?;
        if !file.metadata()?.is_file() {
            return Err(NamespaceError::Symlink);
        }
        match file.try_lock() {
            Ok(()) => Ok(NamespaceLock { file }),
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
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
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
        _lock: &NamespaceLock,
        tombstone: &HeadAuthority,
        cap: usize,
    ) -> Result<(), NamespaceError> {
        if let Some(existing) = self.read_tombstone(cap)? {
            if existing == *tombstone {
                return Ok(());
            }
            return Err(NamespaceError::ForeignTombstone);
        }
        let bytes = encode_control(tombstone, cap)?;
        let path = self.tombstone_path();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(error.into());
        }
        drop(file);
        File::open(&self.root)?.sync_all()?;
        Ok(())
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
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
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
        _lock: &NamespaceLock,
        activated: &HeadAuthority,
        cap: usize,
    ) -> Result<(), NamespaceError> {
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
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(error.into());
        }
        drop(file);
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }

    /// No-overwrite final installation of a completely built staging
    /// directory, under the held lock: refuse the tombstone, refuse an
    /// existing target, rename, fsync the namespace directory. Durability
    /// completes before the lock is released by the caller.
    /// # Errors
    /// A recorded cancellation, an existing target and filesystem failures.
    pub fn install_target(
        &self,
        _lock: &NamespaceLock,
        staged: &Path,
        cap: usize,
    ) -> Result<(), NamespaceError> {
        if self.read_tombstone(cap)?.is_some() {
            return Err(NamespaceError::ForeignTombstone);
        }
        let target = self.target_dir();
        refuse_symlink(&target)?;
        if target.exists() {
            return Err(NamespaceError::TargetExists);
        }
        fs::rename(staged, &target)?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
}
