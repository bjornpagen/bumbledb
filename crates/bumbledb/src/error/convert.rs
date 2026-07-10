//! `From` conversions into [`Error`] and the `std::error::Error` impl.

use super::{CorruptionError, Error, FactShapeError, SchemaError, ValidationError};

impl From<heed::Error> for Error {
    fn from(err: heed::Error) -> Self {
        match err {
            // `MDB_READERS_FULL` gets a named error carrying the fixed
            // reader-table size: the failure is "one snapshot too many",
            // and the remedy is releasing snapshots, not diagnosing LMDB.
            heed::Error::Mdb(heed::MdbError::ReadersFull) => Self::ReadersFull {
                max_readers: crate::storage::env::MAX_READERS,
            },
            other => Self::Lmdb(other),
        }
    }
}

impl Error {
    /// Parses `mdb_txn_commit`'s failure once at the durability boundary
    /// (the trust-boundary rule, applied to the OS): a raw OS errno out
    /// of the commit — heed's `Io`, minted from `MDB_*`-range-external
    /// return codes — comes from the commit's write/sync syscalls (on
    /// macOS: the data-page `pwrite`s, `fcntl(F_FULLFSYNC)`, the
    /// `O_DSYNC` meta write; `mdb.c` surfaces the errno raw with no
    /// fallback sync), so it becomes the typed [`Error::CommitSync`]
    /// naming phase and syscall class. Every other failure keeps its
    /// established mapping.
    pub(crate) fn from_commit(err: heed::Error) -> Self {
        match err {
            heed::Error::Io(error) => Self::CommitSync { retries: 0, error },
            other => other.into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<SchemaError> for Error {
    fn from(err: SchemaError) -> Self {
        Self::Schema(err)
    }
}

impl From<ValidationError> for Error {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
    }
}

impl From<FactShapeError> for Error {
    fn from(err: FactShapeError) -> Self {
        Self::FactShape(err)
    }
}

impl From<CorruptionError> for Error {
    fn from(err: CorruptionError) -> Self {
        Self::Corruption(err)
    }
}

impl std::error::Error for Error {
    /// Chains only where the payload *is* an underlying error; the
    /// structured variants carry data payloads deliberately invisible
    /// to chain-walking (the decision is documented on [`Error`]).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Lmdb(err) => Some(err),
            Self::CommitSync { error, .. } => Some(error),
            Self::BulkLoad { error, .. } => Some(error.as_ref()),
            _ => None,
        }
    }
}
