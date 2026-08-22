//! `From` conversions into [`Error`] and the `std::error::Error` impl.

use super::{
    CorruptionError, DynIdError, Error, FactShapeError, IoFailure, LmdbFailure, SchemaError,
    ValidationError,
};

impl From<heed::Error> for Error {
    fn from(err: heed::Error) -> Self {
        match err {
            // and the remedy is releasing snapshots, not diagnosing LMDB.
            heed::Error::Mdb(heed::MdbError::ReadersFull) => Self::ReadersFull {
                max_readers: crate::storage::env::MAX_READERS,
            },
            other => Self::Lmdb(LmdbFailure::from(other)),
        }
    }
}

impl Error {
    /// macOS: the data-page `pwrite`s, `fcntl(F_FULLFSYNC)`, the

    pub(crate) fn from_commit(err: heed::Error) -> Self {
        match err {
            heed::Error::Io(error) => Self::CommitSync {
                retries: 0,
                error: IoFailure::from_io(&error),
            },
            other => other.into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(IoFailure::from_io(&err))
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

impl From<DynIdError> for Error {
    fn from(err: DynIdError) -> Self {
        Self::FactShape(err.into())
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

impl std::error::Error for IoFailure {}

impl std::error::Error for LmdbFailure {}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.descriptor().source
    }
}
