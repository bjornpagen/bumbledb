//! Public facade for Bumbledb.
//!
//! This facade exposes the embedded database shell while the lower-level crates
//! carry the experimental schema, storage, and query internals.

#![allow(clippy::result_large_err)]

use std::marker::PhantomData;
use std::path::Path;

pub use bumbledb_core::{encoding, schema};

/// Result type for public Bumbledb operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level public error model.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Storage-layer failure.
    #[error(transparent)]
    Storage(#[from] bumbledb_lmdb::Error),

    /// Schema-layer failure placeholder for later stages.
    #[error("schema error: {0}")]
    Schema(String),

    /// Query-layer failure placeholder for later stages.
    #[error("query error: {0}")]
    Query(String),

    /// Constraint-layer failure placeholder for later stages.
    #[error("constraint error: {0}")]
    Constraint(String),

    /// Internal invariant failure.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Embedded database handle.
pub struct Database {
    inner: bumbledb_lmdb::Environment,
}

impl Database {
    /// Opens or creates a Bumbledb database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            inner: bumbledb_lmdb::Environment::open(path)?,
        })
    }

    /// Returns the storage format version stored in metadata.
    pub fn storage_format_version(&self) -> Result<u32> {
        Ok(self.inner.storage_format_version()?)
    }

    /// Clears stale LMDB reader slots and returns the number cleared.
    pub fn cleanup_stale_readers(&self) -> Result<usize> {
        Ok(self.inner.clear_stale_readers()?)
    }

    /// Runs a closure inside a read snapshot.
    pub fn read<T>(&self, f: impl for<'txn> FnOnce(&ReadTxn<'txn>) -> Result<T>) -> Result<T> {
        self.inner.read(|_| {
            let txn = ReadTxn {
                _private: PhantomData,
            };
            f(&txn)
        })
    }

    /// Runs a closure inside a write transaction.
    pub fn write<T>(
        &self,
        f: impl for<'txn> FnOnce(&mut WriteTxn<'txn>) -> Result<T>,
    ) -> Result<T> {
        self.inner.write(|_| {
            let mut txn = WriteTxn {
                _private: PhantomData,
            };
            f(&mut txn)
        })
    }
}

/// Opaque read transaction token.
pub struct ReadTxn<'txn> {
    _private: PhantomData<&'txn ()>,
}

/// Opaque write transaction token.
pub struct WriteTxn<'txn> {
    _private: PhantomData<&'txn mut ()>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_opens_and_reopens_database() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;

        let db = Database::open(dir.path())?;
        assert_eq!(
            db.storage_format_version()?,
            bumbledb_lmdb::STORAGE_FORMAT_VERSION
        );
        db.read(|_| Ok(()))?;
        db.write(|_| Ok(()))?;
        drop(db);

        let db = Database::open(dir.path())?;
        assert_eq!(
            db.storage_format_version()?,
            bumbledb_lmdb::STORAGE_FORMAT_VERSION
        );
        Ok(())
    }
}
