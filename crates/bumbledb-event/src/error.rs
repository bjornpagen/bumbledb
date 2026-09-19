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
    ProgramNodes,
    ProgramSteps,
    FixedPointIterations,
    PartitionCells,
    Diagnostics,
    DescriptorBytes,
    DescriptorItems,
    ArithmeticBits,
    ArithmeticSteps,
    LawCells,
    FunctionCells,
    FunctionSteps,
    FunctionMemo,
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
    MapArity,
    MapOutsideSupport,
    IncompleteImage,
    RoleMismatch,
    EnvironmentMismatch,
    NoFaces,
    PartialRelation,
    NonFunctionalRelation,
    ProgramMismatch,
    NonMonotoneProgram,
    FixedPointInvariant,
    PartitionOverlap,
    PartitionGap,
    PartitionArity,
    PartitionIndex,
    UnsafePolicy,
    IncompletePolicy,
    InvalidRational,
    DivisionByZero,
    NegativeMass,
    LawOverlap,
    LawNotNormalized,
    MissingLaw,
    FunctionOverlap,
    FunctionInvariant,
    KernelNotNormalized,
    DescriptorClaimMismatch,
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
            Self::MapArity => f.write_str("event map needs one readout per target coordinate"),
            Self::MapOutsideSupport => {
                f.write_str("event map sends a legal source world outside target support")
            }
            Self::IncompleteImage => {
                f.write_str("event map does not reach every legal target world")
            }
            Self::RoleMismatch => f.write_str("event membership depends on an undeclared face"),
            Self::EnvironmentMismatch => {
                f.write_str("event relation maps do not preserve one shared environment")
            }
            Self::NoFaces => f.write_str("a face product needs at least one endpoint"),
            Self::PartialRelation => {
                f.write_str("a function graph needs an output for every input")
            }
            Self::NonFunctionalRelation => {
                f.write_str("relation has multiple outputs for one input")
            }
            Self::ProgramMismatch => f.write_str("expression belongs to a different Event program"),
            Self::NonMonotoneProgram => f.write_str("Event program has unproved positive variance"),
            Self::FixedPointInvariant => {
                f.write_str("Event fixed-point execution violated its finite order contract")
            }
            Self::PartitionOverlap => f.write_str("distinct Event partition cells overlap"),
            Self::PartitionGap => f.write_str("Event partition does not cover its required parent"),
            Self::PartitionArity => f.write_str("Event partition needs one value per cell"),
            Self::PartitionIndex => f.write_str("Event partition index is outside the roster"),
            Self::UnsafePolicy => f.write_str("strategy policy includes an unpermitted action"),
            Self::IncompletePolicy => f.write_str("strategy policy omits a required state"),
            Self::InvalidRational => f.write_str("invalid exact rational"),
            Self::DivisionByZero => f.write_str("exact arithmetic division by zero"),
            Self::NegativeMass => f.write_str("a probability law cannot have negative mass"),
            Self::LawOverlap => f.write_str("probability density pieces overlap"),
            Self::LawNotNormalized => f.write_str("the joint probability law does not sum to one"),
            Self::FunctionOverlap => f.write_str("finite function pieces overlap"),
            Self::FunctionInvariant => {
                f.write_str("finite function contraction violated its coordinate invariant")
            }
            Self::KernelNotNormalized => {
                f.write_str("conditional law does not sum to one on every parent fibre")
            }
            Self::DescriptorClaimMismatch => {
                f.write_str("source descriptor claim disagrees with its recomputed result")
            }
            Self::MissingLaw => f.write_str("the Event space has no designated probability law"),
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
