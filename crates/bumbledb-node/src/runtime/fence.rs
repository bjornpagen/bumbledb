//! Kernel-held directory ownership. Only closing the owning file (including
//! process death) releases it; the lock file is a stable namespace entry,
//! never unlinked, and its body is never read.
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

/// The reserved sibling namespace holding every directory's lock file.
const LEASE_NAMESPACE: &str = "~lease";

/// Exclusive ownership of one directory path, released when dropped.
#[derive(Debug)]
pub struct DirectoryLock {
    directory: PathBuf,
    file: File,
}

impl DirectoryLock {
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

impl Drop for DirectoryLock {
    fn drop(&mut self) {
        // A concurrently spawned child can inherit the descriptor between
        // fork and exec, so release explicitly instead of relying on close.
        let _ = self.file.unlock();
    }
}

fn refuse_symlink(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ownership path is a symlink",
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Acquire `directory` without waiting. The lock lives beside it at
/// `parent/~lease/name/owner.lock`, so replacing the directory cannot mint a
/// second lock for the same path.
///
/// # Errors
/// `WouldBlock` while another handle owns the path; `InvalidInput` for a
/// symlinked or reserved path; filesystem failures as they occur.
pub fn acquire_directory(directory: &Path) -> io::Result<DirectoryLock> {
    let absolute = std::path::absolute(directory)?;
    let name = absolute
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "directory must have a name"))?;
    if name == LEASE_NAMESPACE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "directory names the reserved ownership namespace",
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
    let holder = namespace.join(name);
    refuse_symlink(&holder)?;
    fs::create_dir_all(&holder)?;
    let path = holder.join("owner.lock");
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
            "lock is not a regular file",
        ));
    }
    match file.try_lock() {
        Ok(()) => Ok(DirectoryLock { directory, file }),
        Err(TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "directory is already owned",
        )),
        Err(TryLockError::Error(error)) => Err(error),
    }
}
