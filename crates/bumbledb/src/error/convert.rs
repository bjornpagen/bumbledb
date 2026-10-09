//! `From` conversions into [`Error`] and the `std::error::Error` impl.

use super::{
    CorruptionError, DynIdError, Error, FactShapeError, IoFailure, LmdbFailure, SchemaError,
    ValidationError,
};
use crate::canonical::RowError;
use crate::changes::ChangeError;
use crate::work::WorkError;

impl From<heed::Error> for Error {
    fn from(error: heed::Error) -> Self {
        match error {
            heed::Error::Mdb(heed::MdbError::ReadersFull) => Self::ReadersFull,
            heed::Error::Io(io) => Self::Io(IoFailure::from_io(&io)),
            other => Self::Lmdb(LmdbFailure::from(other)),
        }
    }
}

impl Error {
    /// The operation stopped because its [`crate::WorkContext`] was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }
}

impl From<WorkError> for Error {
    fn from(error: WorkError) -> Self {
        match error {
            WorkError::Cancelled => Self::Cancelled,
            WorkError::Allocation => Self::Allocation,
        }
    }
}

/// Cancellation and refused allocation inside a change or row error are the
/// cancellation and allocation errors.
impl From<ChangeError> for Error {
    fn from(error: ChangeError) -> Self {
        match error {
            ChangeError::Work(work) | ChangeError::Row(RowError::Work(work)) => work.into(),
            ChangeError::Allocation | ChangeError::Row(RowError::Allocation) => Self::Allocation,
            other => Self::Changes(other),
        }
    }
}

impl From<RowError> for Error {
    fn from(error: RowError) -> Self {
        ChangeError::Row(error).into()
    }
}

impl From<crate::schema::CompileError> for Error {
    fn from(error: crate::schema::CompileError) -> Self {
        Self::Compile(error)
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
        match self {
            Self::Io(err) => Some(err),
            Self::Lmdb(err) => Some(err),
            Self::TransactionPoisoned { source } => Some(source.as_ref()),
            Self::Changes(err) => Some(err),
            Self::Compile(err) => Some(err),
            Self::Scalar { source, .. } => Some(source),
            _ => None,
        }
    }
}
