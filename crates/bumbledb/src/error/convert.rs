//! `From` conversions into [`Error`] and the `std::error::Error` impl.

use super::{
    CorruptionError, Counter, DynIdError, Error, FactShapeError, IoFailure, LmdbFailure,
    SchemaError, ValidationError,
};
use crate::canonical::RowError;
use crate::changes::ChangeError;
use crate::storage::store::StoreError;

impl From<heed::Error> for Error {
    fn from(err: heed::Error) -> Self {
        Self::from_store(StoreError::from_heed(err))
    }
}

impl Error {
    pub(crate) fn from_store(error: StoreError) -> Self {
        match error {
            StoreError::Io(io) => Self::Io(io),
            StoreError::Lmdb(lmdb) => Self::Lmdb(lmdb),
            StoreError::UnrecognizedStore { path } => Self::NotABumbleDb { path },
            StoreError::SchemaMismatch => Self::SchemaMismatch,
            StoreError::StoreLocked { path } => Self::Locked { path },
            StoreError::DestinationExists { path } => Self::DestinationExists { path },
            StoreError::Full { ceiling } => Self::Full { ceiling },
            StoreError::ReaderSlotsExhausted => Self::ReadersFull,
            StoreError::Closed => Self::Closed,
            StoreError::ReentrantWriter => Self::ReentrantWriter,
            StoreError::RowIdExhausted => Self::Exhausted(Counter::RowIds),
            StoreError::GenerationExhausted => Self::Exhausted(Counter::Generations),
            StoreError::ForeignSchema => Self::ForeignSchema,
            StoreError::HostKey(fault) => Self::HostKey(fault),
            StoreError::CapacityRayMeasure { statement } => Self::CapacityRayMeasure { statement },
            StoreError::MeasureOverflow { statement } => Self::MeasureOverflow { statement },
            StoreError::Changes(
                ChangeError::Work(work) | ChangeError::Row(RowError::Work(work)),
            ) => Self::Store(Box::new(StoreError::Work(work))),
            StoreError::Changes(
                ChangeError::Allocation | ChangeError::Row(RowError::Allocation),
            ) => Self::Store(Box::new(StoreError::Allocation)),
            StoreError::Changes(changes) => Self::Changes(changes),
            StoreError::Corruption(corruption) => Self::Corruption(corruption),
            StoreError::Compile(compile) => Self::Compile(compile),
            error @ (StoreError::Work(_) | StoreError::Allocation) => Self::Store(Box::new(error)),
        }
    }

    /// The operation stopped because its [`crate::WorkContext`] was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.kind() == super::ErrorKind::Cancelled
    }
}

impl From<StoreError> for Error {
    fn from(error: StoreError) -> Self {
        Self::from_store(error)
    }
}

impl From<ChangeError> for Error {
    fn from(error: ChangeError) -> Self {
        Self::from_store(StoreError::Changes(error))
    }
}

impl From<RowError> for Error {
    fn from(error: RowError) -> Self {
        Self::from_store(StoreError::Changes(ChangeError::Row(error)))
    }
}

impl From<crate::work::WorkError> for Error {
    fn from(error: crate::work::WorkError) -> Self {
        Self::from_store(StoreError::Work(error))
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
            Self::Store(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}
