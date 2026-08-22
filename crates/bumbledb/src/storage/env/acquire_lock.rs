use std::path::Path;

use crate::error::{Error, Result};

/// Takes the exclusive advisory lock enforcing one WRITING handle per
/// path. A held lock — another process, or another live writing
/// `Environment` on the same path in this process — is
/// `Error::EnvironmentLocked`.
pub(super) fn acquire_lock(path: &Path) -> Result<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.join("bumbledb.lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(Error::EnvironmentLocked),
        Err(std::fs::TryLockError::Error(err)) => Err(Error::from(err)),
    }
}
