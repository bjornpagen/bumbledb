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
            Self::BulkLoad { error, .. } => Some(error.as_ref()),
            _ => None,
        }
    }
}
