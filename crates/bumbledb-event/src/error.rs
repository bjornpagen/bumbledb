use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// Explicit implementation limits, never an alternative mathematical answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capacity {
    Coordinates,
    Records,
    TableWords,
    OperationSteps,
    MemoEntries,
    SpaceTokens,
    ExplicitTable,
    RegistryEntries,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    Allocation,
    Capacity(Capacity),
    InvalidOrder,
    InvalidCoordinate(u8),
    InvalidTable,
    EmptySpace,
    SpaceMismatch,
    IllegalWorld(u64),
    Poisoned,
    InvalidEncoding,
    UnsupportedVersion(u8),
    UnknownKey,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("event operation cancelled"),
            Self::Allocation => f.write_str("event allocation unavailable"),
            Self::Capacity(kind) => write!(f, "event capacity exceeded: {kind:?}"),
            Self::InvalidOrder => f.write_str("event coordinate order is not a permutation"),
            Self::InvalidCoordinate(v) => write!(f, "invalid event coordinate {v}"),
            Self::InvalidTable => f.write_str("event truth table has an invalid extent or padding"),
            Self::EmptySpace => f.write_str("an event space requires a legal world"),
            Self::SpaceMismatch => f.write_str("event operands require checked space alignment"),
            Self::IllegalWorld(w) => write!(f, "world {w} is outside the event space"),
            Self::Poisoned => f.write_str("event manager was interrupted by a panic"),
            Self::InvalidEncoding => f.write_str("invalid or noncanonical Event encoding"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported Event encoding version {version}")
            }
            Self::UnknownKey => f.write_str("unknown or unowned Event binding key"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::collections::TryReserveError> for Error {
    fn from(_: std::collections::TryReserveError) -> Self {
        Self::Allocation
    }
}

/// Operation control supplied by the embedding engine. `()` never cancels.
pub trait Control {
    /// # Errors
    /// Refuse cancellation or unavailable operation resources explicitly.
    fn checkpoint(&self) -> Result<()>;
}

impl Control for () {
    fn checkpoint(&self) -> Result<()> {
        Ok(())
    }
}

/// Per-owner storage bounds and per-operation workspace bounds.
///
/// Raising these limits changes resource policy, never Event equality. A failed
/// operation may retain canonical intermediates but publishes no Event result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub records: usize,
    pub table_words: usize,
    pub operation_steps: usize,
    pub memo_entries: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            records: 2_000_000,
            table_words: 16_000_000,
            operation_steps: 20_000_000,
            memo_entries: 2_000_000,
        }
    }
}
