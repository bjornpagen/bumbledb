//! Kernel-held local ownership. A paused process retains its lock; only
//! closing the owning file (including process death) releases it. Lock
//! files are stable namespace entries, never scratch to unlink or replace.
//! This does not provide remote fencing or make the legacy filesystem
//! object's separate body/generation writes crash-atomic.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::{LEASE_NAMESPACE, StoreKey, TEMP_NAMESPACE};

/// A bound on waiting for an emulator mutation, NOT the owner's lifetime.
/// Directory opens are one-shot and never wait or steal ownership.
const MUTATION_WAIT: Duration = Duration::from_secs(5);
const LOCK_RETRY: Duration = Duration::from_millis(5);
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Exclusive local mutation ownership, released by the file's destructor.
/// No clone, caller-controlled unlock, renewal, expiry, or historical token exists.
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

/// Owns a directory namespace even while its materialization is absent,
/// renamed, or removed. Declare it after native resources so it drops last.
#[derive(Debug)]
pub struct DirectoryLock {
    directory: PathBuf,
    _lock: HeldLock,
}

impl DirectoryLock {
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }
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
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "lock is not a file",
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
/// the directory: `parent/~lease/name/owner.lock`, not inside it. Existing
/// filesystem case/normalization aliases therefore name the same lock too.
/// The parent is a trusted local namespace; replacing it concurrently as
/// the OS user is outside this advisory-lock contract.
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
    let file = open_lock(&namespace.join(name), "owner.lock")?;
    Ok(DirectoryLock {
        directory,
        _lock: try_hold(file)?,
    })
}

/// Hold exclusion through the entire filesystem-emulator mutation. An
/// exhausted wait returns `WouldBlock`; it never authorizes a takeover.
/// No routine constructor/cleanup removes this lock or its ancestors.
///
/// # Errors
/// Invalid keys, lock contention and filesystem failures are explicit.
pub fn acquire_mutation(root: &Path, key: &StoreKey) -> io::Result<HeldLock> {
    let namespace = root.join(LEASE_NAMESPACE);
    refuse_symlink(&namespace)?;
    let file = open_lock(&namespace.join(key.as_str()), "mutation.lock")?;
    let start = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(HeldLock { file }),
            Err(TryLockError::Error(err)) => return Err(err),
            Err(TryLockError::WouldBlock) => {
                let Some(remaining) = MUTATION_WAIT.checked_sub(start.elapsed()) else {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "local mutation wait exhausted",
                    ));
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
