use std::path::Path;

use crate::error::{Error, Result};

/// Takes the exclusive advisory lock enforcing one WRITING handle per
/// path — the lock law is a writer law (ruled 2026-07-23, R17): it
/// belongs to the writing constructors (`Db` handles, durable or
/// ephemeral) and to nothing else; the read-only lane opens `MDB_RDONLY`
/// and takes none (a read-only environment can corrupt nothing, so there
/// is nothing for a lock to protect). A held lock — another process, or
/// another live writing `Environment` on the same path in this process —
/// is `Error::EnvironmentLocked`, converting the silent derived-state
/// corruption of a double-open into a loud open-time failure.
pub(super) fn acquire_lock(path: &Path) -> Result<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.join("bumbledb.lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(Error::EnvironmentLocked),
        Err(std::fs::TryLockError::Error(err)) => Err(Error::Io(err)),
    }
}
