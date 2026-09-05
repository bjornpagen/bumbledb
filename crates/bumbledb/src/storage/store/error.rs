//! Typed successor-store failures. Distinct physical conditions stay
//! distinct: map exhaustion, growth refusal, blocked resize, reader-slot
//! exhaustion, disk failure and corruption are different diagnostics, never
//! one boolean. This enum is store-local; the P00 error hub re-exports it
//! (see the P02 hub patch request) rather than flattening it into `Error`.

use std::path::PathBuf;
use std::time::Duration;

use crate::error::{IoFailure, LmdbFailure};
use crate::work::WorkError;

pub type StoreResult<T> = std::result::Result<T, StoreError>;

/// Corruption observed inside an otherwise recognized successor store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreCorruption {
    /// A required `_core_meta` entry is absent or the wrong width.
    MetaMissing(&'static str),
    /// A physical key in a store namespace has an impossible shape.
    MalformedKey(&'static str),
    /// A membership/determinant entry references a row that does not exist.
    DanglingIndexEntry,
}

/// A host-record grammar refusal, structured so the integration facade can
/// surface the exact [`super::host::HostSealError`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyFault {
    TooLong { actual: usize },
    NotStrictlyOrdered,
    LengthOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Filesystem failure outside LMDB.
    Io(IoFailure),
    /// LMDB failure that is not one of the named conditions below.
    Lmdb(LmdbFailure),
    /// The directory holds no successor-family store. Refused before any
    /// cleanup, write, or adoption; old-format files are never "repaired".
    UnrecognizedStore { path: PathBuf },
    /// Recognized family, incompatible layout counter.
    LayoutMismatch { found: u32, expected: u32 },
    /// The store's schema fingerprint disagrees with the caller's schema.
    SchemaMismatch,
    /// Another live owner holds the directory's kernel lock.
    StoreLocked { path: PathBuf },
    /// `create` refused because the destination already exists.
    DestinationExists { path: PathBuf },
    /// Staging rename reached the destination, but a later sync/open step
    /// failed. The destination path may exist; callers retain cleanup
    /// ownership of the exact staging identity.
    InstallSettlementFailed {
        path: PathBuf,
        detail: Box<StoreError>,
    },
    /// Live read transactions blocked exclusive map access within the
    /// caller's budget. The caller can release snapshots and retry.
    ResizeBlockedByReaders {
        live_transactions: u64,
        oldest_age: Option<Duration>,
    },
    /// The map was full and no further growth is possible (address space,
    /// configured ceiling, or the platform refused the larger mapping).
    MapGrowthExhausted {
        map_bytes: u64,
        requested_bytes: u64,
        detail: Option<LmdbFailure>,
    },
    /// The map filled up during a transaction and the caller asked for no
    /// automatic growth, or growth succeeded but the same delta still did
    /// not fit after the bounded number of growth attempts.
    MapFull { map_bytes: u64 },
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
    /// The opaque host-record grammar was violated (bounded key width,
    /// strictly increasing unique keys, representable lengths).
    HostKey(HostKeyFault),
    /// The judge refused to complete (undefined ray duration, measure
    /// overflow): an explicit resource/semantic refusal, never a fabricated
    /// domain rejection.
    JudgeRefused {
        statement: bumbledb_theory::schema::StatementId,
        detail: &'static str,
    },
    /// Work budget, deadline or cancellation stopped the operation.
    Work(WorkError),
    /// A fallible in-memory allocation was refused by the host.
    Allocation,
    /// Malformed input change/row bytes (from the canonical boundary).
    Changes(crate::changes::ChangeError),
    /// The recognized store contains impossible bytes.
    Corruption(StoreCorruption),
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

    /// True exactly for the LMDB map-full condition; the candidate path
    /// converts this into abort → grow → reapply, never a partial commit.
    pub(crate) fn is_map_full(error: &heed::Error) -> bool {
        matches!(error, heed::Error::Mdb(heed::MdbError::MapFull))
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
            Self::Io(io) => write!(f, "store I/O failure: {:?}", io.kind),
            Self::Lmdb(err) => write!(f, "store LMDB failure: {err:?}"),
            Self::UnrecognizedStore { path } => {
                write!(f, "no successor-family store at {}", path.display())
            }
            Self::LayoutMismatch { found, expected } => {
                write!(f, "store layout {found}, this build reads {expected}")
            }
            Self::SchemaMismatch => f.write_str("store schema fingerprint mismatch"),
            Self::StoreLocked { path } => {
                write!(f, "store directory {} is owned elsewhere", path.display())
            }
            Self::DestinationExists { path } => {
                write!(f, "create destination {} already exists", path.display())
            }
            Self::InstallSettlementFailed { path, detail } => {
                write!(
                    f,
                    "install settlement failed at {} after publish: {detail}",
                    path.display()
                )
            }
            Self::ResizeBlockedByReaders {
                live_transactions,
                oldest_age,
            } => write!(
                f,
                "resize blocked by {live_transactions} live transaction(s), oldest {oldest_age:?}"
            ),
            Self::MapGrowthExhausted {
                map_bytes,
                requested_bytes,
                ..
            } => write!(
                f,
                "map growth exhausted at {map_bytes} bytes ({requested_bytes} requested)"
            ),
            Self::MapFull { map_bytes } => write!(f, "map full at {map_bytes} bytes"),
            Self::ReaderSlotsExhausted => f.write_str("LMDB reader slots exhausted"),
            Self::Closed => f.write_str("store is closing or closed"),
            Self::ReentrantWriter => f.write_str("writer session is already owned by this thread"),
            Self::RowIdExhausted => f.write_str("local physical row identifiers exhausted"),
            Self::GenerationExhausted => f.write_str("store generation exhausted"),
            Self::ForeignSchema => f.write_str("change set sealed for a different schema"),
            Self::HostKey(fault) => match fault {
                HostKeyFault::TooLong { actual } => write!(
                    f,
                    "host key has {actual} bytes; limit is {}",
                    super::keys::HOST_KEY_MAX
                ),
                HostKeyFault::NotStrictlyOrdered => {
                    f.write_str("host keys must be strictly ordered and unique")
                }
                HostKeyFault::LengthOverflow => f.write_str("host record byte length overflow"),
            },
            Self::JudgeRefused { statement, detail } => {
                write!(f, "judgment refused at statement {}: {detail}", statement.0)
            }
            Self::Work(err) => err.fmt(f),
            Self::Allocation => f.write_str("in-memory allocation refused"),
            Self::Changes(err) => err.fmt(f),
            Self::Corruption(what) => write!(f, "store corruption: {what:?}"),
            Self::Compile(err) => write!(f, "store compile: {err}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Work(err) => Some(err),
            Self::Changes(err) => Some(err),
            Self::Compile(err) => Some(err),
            Self::InstallSettlementFailed { detail, .. } => Some(detail),
            _ => None,
        }
    }
}
