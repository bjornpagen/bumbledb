//! Store failures. Each becomes the [`crate::Error`] variant of the same
//! name; only cancellation and refused allocation stay boxed.

use std::path::PathBuf;

use crate::error::{CorruptionError, IoFailure, LmdbFailure};
use crate::work::WorkError;

pub type StoreResult<T> = std::result::Result<T, StoreError>;

/// A host key outside the grammar: longer than the key limit, or not
/// strictly after the previous key of the same seal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyFault {
    TooLong { actual: usize },
    NotStrictlyOrdered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Filesystem failure outside LMDB.
    Io(IoFailure),
    /// LMDB failure that is not one of the named conditions below.
    Lmdb(LmdbFailure),
    /// The directory holds no store of this format; refused before any write.
    UnrecognizedStore {
        path: PathBuf,
    },
    /// The store's schema fingerprint disagrees with the caller's schema.
    SchemaMismatch,
    /// Another live owner holds the directory's kernel lock.
    StoreLocked {
        path: PathBuf,
    },
    /// `create` refused because the destination already exists.
    DestinationExists {
        path: PathBuf,
    },
    /// A write needed more pages than the fixed virtual map ceiling holds.
    Full {
        ceiling: u64,
    },
    /// LMDB's reader table is full; a distinct condition from map exhaustion.
    ReaderSlotsExhausted,
    /// The store is closing or closed; no new transaction is admitted.
    Closed,
    /// A second writer was requested from the thread that already owns the
    /// writer session; the store has exactly one writer at a time.
    ReentrantWriter,
    /// Local physical row identifiers are exhausted (u64 wrap refused).
    RowIdExhausted,
    /// The durable generation counter would wrap (refused, never reused).
    GenerationExhausted,
    /// The submitted `ChangeSet` belongs to a different schema.
    ForeignSchema,
    HostKey(HostKeyFault),
    /// A capacity weight or bound asks for the finite measure of a ray.
    CapacityRayMeasure {
        statement: bumbledb_theory::schema::StatementId,
    },
    /// A capacity group's measure overflows the widened accumulator.
    MeasureOverflow {
        statement: bumbledb_theory::schema::StatementId,
    },
    /// Cancellation or unavailable allocation capacity stopped the operation.
    Work(WorkError),
    /// A fallible in-memory allocation was refused by the host.
    Allocation,
    /// Malformed input change/row bytes (from the canonical boundary).
    Changes(crate::changes::ChangeError),
    /// The recognized store contains impossible bytes.
    Corruption(CorruptionError),
    /// Schema compilation failed (interned projection ids exhausted).
    /// Distinct from corruption: the on-disk store is intact.
    Compile(crate::schema::CompileError),
}

impl StoreError {
    pub(crate) fn from_heed(error: heed::Error) -> Self {
        match error {
            heed::Error::Mdb(heed::MdbError::ReadersFull) => Self::ReaderSlotsExhausted,
            heed::Error::Io(io) => Self::Io(IoFailure::from_io(&io)),
            other => Self::Lmdb(LmdbFailure::from(other)),
        }
    }
}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(IoFailure::from_io(&error))
    }
}

impl From<WorkError> for StoreError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl From<crate::changes::ChangeError> for StoreError {
    fn from(error: crate::changes::ChangeError) -> Self {
        Self::Changes(error)
    }
}

impl From<crate::canonical::RowError> for StoreError {
    fn from(error: crate::canonical::RowError) -> Self {
        Self::Changes(crate::changes::ChangeError::Row(error))
    }
}

impl From<crate::schema::CompileError> for StoreError {
    fn from(error: crate::schema::CompileError) -> Self {
        Self::Compile(error)
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Work(err) => err.fmt(f),
            Self::Allocation => f.write_str("in-memory allocation refused"),
            other => crate::Error::from_store(other.clone()).fmt(f),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Work(err) => Some(err),
            Self::Changes(err) => Some(err),
            Self::Compile(err) => Some(err),
            _ => None,
        }
    }
}
