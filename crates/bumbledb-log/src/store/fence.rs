//! Kernel-held local exclusion. A paused process retains its lock; only
//! closing the owning file (including process death) releases it. Lock files
//! are stable namespace entries, never scratch to unlink or replace. No
//! wall-clock TTL, token predecessor chain, renewal or check-then-rename
//! proof exists; time does not mint a competing owner.
//!
//! Two distinct scopes share this one kernel mechanism (C07):
//!
//! - **Directory ownership** ([`acquire_directory`]): one owning process per
//!   local materialization, acquired before reading recovery scratch or
//!   deleting anything, held through native close, released last.
//! - **Object mutation** ([`acquire_mutation`]): the whole critical section
//!   of one filesystem-store conditional operation. It exists only inside the
//!   filesystem adapter; it is not a tenant authority and does not fence S3.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use bumbledb::work::{ByteKind, WorkContext, WorkError};

use super::{LEASE_NAMESPACE, TEMP_NAMESPACE, key_ok};

/// A bound on waiting for a filesystem-store mutation lock, NOT an owner's
/// lifetime. Directory opens are one-shot and never wait or steal ownership.
const MUTATION_WAIT: Duration = Duration::from_secs(5);
const LOCK_RETRY: Duration = Duration::from_millis(5);
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Preserve cooperative refusal separately from filesystem failure.
#[derive(Debug)]
pub enum WorkIoError {
    Io(io::Error),
    Work(WorkError),
}

impl From<io::Error> for WorkIoError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<WorkError> for WorkIoError {
    fn from(value: WorkError) -> Self {
        Self::Work(value)
    }
}

impl WorkIoError {
    pub(crate) fn into_io(self) -> io::Error {
        match self {
            Self::Io(error) => error,
            Self::Work(error) => io::Error::other(error),
        }
    }
}

fn checkpoint(work: Option<&WorkContext>) -> Result<(), WorkIoError> {
    if let Some(work) = work {
        work.checkpoint()?;
    }
    Ok(())
}

/// Exclusive local ownership, released by the file's destructor. No clone,
/// caller-controlled unlock, renewal, expiry, or historical token exists.
#[derive(Debug)]
pub struct HeldLock {
    file: File,
}

impl Drop for HeldLock {
    fn drop(&mut self) {
        // A concurrent std::process::Command may briefly inherit this fd
        // between fork and exec despite CLOEXEC. Unlock on owner teardown,
        // rather than relying on the child to close the last descriptor.
        // The field still closes normally even if unlock reports an error.
        let _ = self.file.unlock();
    }
}

/// Persistent lock-file identity. Successor acquisition must reuse this
/// inode; recovery never unlinks or replaces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockIdentity {
    pub dev: u64,
    pub ino: u64,
}

/// Owns a directory namespace even while its materialization is absent,
/// renamed, or removed. Declare it after native resources so it drops last.
#[derive(Debug)]
pub struct DirectoryLock {
    directory: PathBuf,
    lock_path: PathBuf,
    _lock: HeldLock,
}

impl DirectoryLock {
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The persistent lock-file path. Never unlinked as stale recovery.
    #[must_use]
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    /// Device/inode of the persistent lock file. Process death releases the
    /// kernel lock; the inode remains.
    ///
    /// # Errors
    /// Filesystem metadata failure.
    #[cfg(unix)]
    pub fn lock_inode(&self) -> io::Result<LockIdentity> {
        lock_identity(&self.lock_path)
    }
}

/// Selected repository-lock owner (C8): kernel-held directory exclusion
/// over a persistent lock-file inode. Never unlink/replace as stale
/// recovery; process death releases the OS lock. Drain is Drop.
pub type RepositoryLock = DirectoryLock;

/// Acquire the existing kernel directory exclusion (C8). Same-process
/// duplicate generation must refuse. L11/L14 expose this to Effect.
pub fn acquire_repository_lock(directory: &Path) -> io::Result<RepositoryLock> {
    acquire_directory(directory)
}

fn refuse_symlink(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "local ownership path is a symlink",
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(unix)]
pub(crate) fn lock_identity(path: &Path) -> io::Result<LockIdentity> {
    use std::os::unix::fs::MetadataExt;
    let meta = fs::metadata(path)?;
    Ok(LockIdentity {
        dev: meta.dev(),
        ino: meta.ino(),
    })
}

fn open_lock(parent: &Path, name: &str) -> io::Result<File> {
    refuse_symlink(parent)?;
    fs::create_dir_all(parent)?;
    let path = parent.join(name);
    refuse_symlink(&path)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)?;
    // A poisoned/garbage lock body is irrelevant: the kernel lock is the
    // authority, never parsed content. Only the file's SHAPE is checked.
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "lock is not a regular file",
        ));
    }
    Ok(file)
}

fn try_hold(file: File) -> io::Result<HeldLock> {
    match file.try_lock() {
        Ok(()) => Ok(HeldLock { file }),
        Err(TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "local directory or mutation is already owned",
        )),
        Err(TryLockError::Error(err)) => Err(err),
    }
}

/// Acquire before reading recovery scratch, creating the materialization,
/// opening LMDB, or deleting anything in it. The stable lock lives beside
/// the directory: `parent/~lease/name/owner.lock`, not inside it, so
/// renaming/replacing the materialization cannot mint a second lock inode
/// for the same authority. Existing filesystem case/normalization aliases
/// name the same lock too. The parent is a trusted local namespace;
/// replacing it concurrently as the OS user is outside this advisory-lock
/// contract.
///
/// # Errors
/// Returns `WouldBlock` immediately while another handle owns the path.
/// Symlinked materialization/lock paths and filesystem failures refuse.
pub fn acquire_directory(directory: &Path) -> io::Result<DirectoryLock> {
    let absolute = std::path::absolute(directory)?;
    let name = absolute
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "directory must have a name"))?;
    if name == LEASE_NAMESPACE || name == TEMP_NAMESPACE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "directory names a reserved ownership namespace",
        ));
    }
    let parent = absolute.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "directory must have a parent")
    })?;
    fs::create_dir_all(parent)?;
    let parent = fs::canonicalize(parent)?;
    let directory = parent.join(name);
    refuse_symlink(&directory)?;
    let namespace = parent.join(LEASE_NAMESPACE);
    refuse_symlink(&namespace)?;
    let lock_path = namespace.join(name).join("owner.lock");
    let file = open_lock(lock_path.parent().expect("owner.lock parent"), "owner.lock")?;
    Ok(DirectoryLock {
        directory,
        lock_path,
        _lock: try_hold(file)?,
    })
}

/// Hold exclusion through one entire filesystem-store conditional mutation.
/// An exhausted wait returns `WouldBlock`; it never authorizes a takeover.
/// No routine constructor/cleanup removes this lock or its ancestors.
///
/// # Errors
/// Invalid keys, lock contention and filesystem failures are explicit.
pub fn acquire_mutation(root: &Path, key: &str) -> io::Result<HeldLock> {
    acquire_mutation_checked(root, key, None).map_err(WorkIoError::into_io)
}

/// The same lock and wait ceiling, with an operation's earlier stop.
///
/// # Errors
/// Refuses filesystem errors, exhausted wait, cancellation or deadline.
pub fn acquire_mutation_with(
    root: &Path,
    key: &str,
    work: &WorkContext,
) -> Result<HeldLock, WorkIoError> {
    acquire_mutation_checked(root, key, Some(work))
}

pub(crate) fn acquire_mutation_checked(
    root: &Path,
    key: &str,
    work: Option<&WorkContext>,
) -> Result<HeldLock, WorkIoError> {
    checkpoint(work)?;
    if !key_ok(key) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid store key").into());
    }
    let _paths = work
        .map(|work| {
            let root_len = root.as_os_str().len() as u64;
            let key_len = key.len() as u64;
            let bytes = root_len
                .checked_mul(3)
                .and_then(|bytes| {
                    key_len
                        .checked_mul(2)
                        .and_then(|key| bytes.checked_add(key))
                })
                .and_then(|bytes| bytes.checked_add(96))
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "lock path size overflow")
                })?;
            work.reserve(ByteKind::Working, bytes)
                .map_err(WorkIoError::from)
        })
        .transpose()?;
    let namespace = root.join(LEASE_NAMESPACE);
    refuse_symlink(&namespace)?;
    let file = open_lock(&namespace.join(key), "mutation.lock")?;
    let start = Instant::now();
    loop {
        checkpoint(work)?;
        match file.try_lock() {
            Ok(()) => return Ok(HeldLock { file }),
            Err(TryLockError::Error(err)) => return Err(err.into()),
            Err(TryLockError::WouldBlock) => {
                let Some(remaining) = MUTATION_WAIT.checked_sub(start.elapsed()) else {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "local mutation wait exhausted",
                    )
                    .into());
                };
                std::thread::sleep(remaining.min(LOCK_RETRY));
            }
        }
    }
}

/// Exclusive temp under `{root}/~tmp`, fsynced. Its owner removes it;
/// elapsed time cannot prove that another thread/process abandoned it.
///
/// # Errors
pub fn synced_temp(root: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let dir = root.join(TEMP_NAMESPACE);
    fs::create_dir_all(&dir)?;
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let temp = dir.join(format!("{}.{}", std::process::id(), seq));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    if let Err(err) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(temp)
}

/// Fsync `path`'s parent directory.
///
/// # Errors
pub fn sync_parent(path: &Path) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "object path has no parent"))?;
    File::open(dir)?.sync_all()
}

/// Fsync every ancestor of `path` up to and including `root`.
///
/// # Errors
pub fn sync_ancestors(path: &Path, root: &Path) -> io::Result<()> {
    let mut current = path.parent();
    while let Some(dir) = current {
        File::open(dir)?.sync_all()?;
        if dir == root {
            break;
        }
        current = dir.parent();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let path = std::env::temp_dir().join(format!(
            "bdb-log-fence-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("root");
        path
    }

    #[test]
    fn same_process_duplicate_generation_refuses_without_unlinking() {
        let root = scratch("same-proc");
        let tenant = root.join("tenant");
        let first = acquire_repository_lock(&tenant).expect("first owner");
        #[cfg(unix)]
        let inode = first.lock_inode().expect("inode");
        let lock_path = first.lock_path().to_path_buf();
        assert!(lock_path.exists(), "lock inode is persistent");
        let second = acquire_repository_lock(&tenant);
        assert_eq!(
            second.expect_err("same-process contention").kind(),
            io::ErrorKind::WouldBlock
        );
        assert!(lock_path.exists(), "refusal does not delete the inode");
        drop(first);
        let successor = acquire_repository_lock(&tenant).expect("released");
        assert_eq!(successor.lock_path(), lock_path.as_path());
        #[cfg(unix)]
        assert_eq!(
            successor.lock_inode().expect("same inode"),
            inode,
            "successor reuses the persistent inode"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn garbage_lock_body_is_inert_and_never_parsed() {
        let root = scratch("garbage");
        let tenant = root.join("tenant");
        fs::create_dir_all(root.join("~lease/tenant")).expect("lease");
        fs::write(root.join("~lease/tenant/owner.lock"), b"pid=999 stale").expect("poison");
        let lock = acquire_directory(&tenant).expect("kernel lock, not body");
        assert!(lock.lock_path().exists());
        drop(lock);
        assert!(
            root.join("~lease/tenant/owner.lock").exists(),
            "release does not unlink the inode"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
