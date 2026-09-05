//! `From` conversions into [`Error`] and the `std::error::Error` impl.
use super::{
    CorruptionError, DynIdError, Error, FactShapeError, IoFailure, LmdbFailure, SchemaError,
    ValidationError,
};

impl From<heed::Error> for Error {
    fn from(err: heed::Error) -> Self {
        match err {
            // and the remedy is releasing snapshots, not diagnosing LMDB.
            heed::Error::Mdb(heed::MdbError::ReadersFull) => {
                Self::from_store(crate::storage::store::StoreError::ReaderSlotsExhausted)
            }
            other => Self::Lmdb(LmdbFailure::from(other)),
        }
    }
}

impl Error {
    /// Box a successor-store condition into the shared error surface.
    pub(crate) fn from_store(error: crate::storage::store::StoreError) -> Self {
        Self::Store(Box::new(error))
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

/// Infallible operand sources (heap image rows) satisfy the generic
/// `Error: From<O::Error>` bounds; no value ever exists to convert.
impl From<std::convert::Infallible> for Error {
    fn from(infallible: std::convert::Infallible) -> Self {
        match infallible {}
    }
}

impl std::error::Error for IoFailure {}

impl std::error::Error for LmdbFailure {}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.descriptor().source
    }
}
