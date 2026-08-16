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

fn clone_io(err: &std::io::Error) -> std::io::Error {
    match err.raw_os_error() {
        Some(code) => std::io::Error::from_raw_os_error(code),
        None => std::io::Error::from(err.kind()),
    }
}

fn clone_heed(err: &heed::Error) -> heed::Error {
    match err {
        heed::Error::Io(io) => heed::Error::Io(clone_io(io)),
        heed::Error::Mdb(mdb) => heed::Error::Mdb(*mdb),
        heed::Error::EnvAlreadyOpened => heed::Error::EnvAlreadyOpened,
        heed::Error::Encoding(_) | heed::Error::Decoding(_) => {
            heed::Error::Io(std::io::Error::from(std::io::ErrorKind::Other))
        }
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Self::FormatMismatch { found, expected } => Self::FormatMismatch {
                found: *found,
                expected: *expected,
            },
            Self::SchemaMismatch { found, expected } => Self::SchemaMismatch {
                found: *found,
                expected: *expected,
            },
            Self::AlreadyInitialized => Self::AlreadyInitialized,
            Self::NotInitialized => Self::NotInitialized,
            Self::EnvironmentLocked => Self::EnvironmentLocked,
            Self::StoreKindMismatch { found, expected } => Self::StoreKindMismatch {
                found: *found,
                expected: *expected,
            },
            Self::DescriptorMissing => Self::DescriptorMissing,
            Self::Io(err) => Self::Io(clone_io(err)),
            Self::Lmdb(err) => Self::Lmdb(clone_heed(err)),
            Self::ReadersFull { max_readers } => Self::ReadersFull {
                max_readers: *max_readers,
            },
            Self::Schema(err) => Self::Schema(err.clone()),
            Self::Validation(err) => Self::Validation(*err),
            Self::FactShape(err) => Self::FactShape(err.clone()),
            Self::CommitRejected { violations } => Self::CommitRejected {
                violations: violations.clone(),
            },
            Self::FreshExhausted { relation, field } => Self::FreshExhausted {
                relation: *relation,
                field: *field,
            },
            Self::ClosedRelationWrite { relation } => Self::ClosedRelationWrite {
                relation: *relation,
            },
            Self::GenerationMoved { witnessed, current } => Self::GenerationMoved {
                witnessed: *witnessed,
                current: *current,
            },
            Self::CommitSync { retries, error } => Self::CommitSync {
                retries: *retries,
                error: clone_io(error),
            },
            Self::TransactionPoisoned { source } => Self::TransactionPoisoned {
                source: source.clone(),
            },
            Self::ForeignPreparedQuery => Self::ForeignPreparedQuery,
            Self::ForeignSnapshot => Self::ForeignSnapshot,
            Self::ParamCountMismatch { expected, supplied } => Self::ParamCountMismatch {
                expected: *expected,
                supplied: *supplied,
            },
            Self::ParamTypeMismatch { param, expected } => Self::ParamTypeMismatch {
                param: *param,
                expected: expected.clone(),
            },
            Self::ParamSetExpected { param } => Self::ParamSetExpected { param: *param },
            Self::ParamScalarExpected { param } => Self::ParamScalarExpected { param: *param },
            Self::ParamElementTypeMismatch {
                param,
                element,
                expected,
            } => Self::ParamElementTypeMismatch {
                param: *param,
                element: *element,
                expected: expected.clone(),
            },
            Self::PointParamAtCeiling { param } => Self::PointParamAtCeiling { param: *param },
            Self::MeasureOfRay { start, end } => Self::MeasureOfRay {
                start: *start,
                end: *end,
            },
            Self::CapacityRayMeasure { statement, fact } => Self::CapacityRayMeasure {
                statement: *statement,
                fact: fact.clone(),
            },
            Self::DerivedBudgetExceeded { rounds, tuples } => Self::DerivedBudgetExceeded {
                rounds: *rounds,
                tuples: *tuples,
            },
            Self::Overflow(kind) => Self::Overflow(*kind),
            Self::ResultBytesOverflow => Self::ResultBytesOverflow,
            Self::Corruption(err) => Self::Corruption(*err),
        }
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
            Self::TransactionPoisoned { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}
