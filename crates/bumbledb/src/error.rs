//! The one engine error and the domain verdicts.
//!
//! Everything reachable from user input or disk returns these typed values.
//! Payloads are ids and owned bytes, never formatted strings; `Display`
//! formats only when the host prints. Panics are programmer-invariant
//! violations.

mod convert;
mod display;

pub use crate::ir::validate::error::{
    AggregateRefusal, ComparisonRefusal, FieldRefusal, HeadMismatch, Limit, ParamRefusal,
    RecRefusal, Unordered, ValidationError, VariableRefusal,
};
pub use bumbledb_theory::schema::{
    Mismatch, RowIndex, SchemaError, StatementErrorKind, TargetKeyCandidate,
};

use std::path::PathBuf;

use crate::encoding::InternId;
use crate::ir::ParamId;
use crate::schema::StatementRef;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId, ValueType};

/// Occurrence index of an atom inside the first failing rule (positive
/// atoms first, then negated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtomIndex(pub usize);

/// Find-term / head-position index inside a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindIndex(pub usize);

/// Rule index in the query's rule list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleIndex(pub usize);

macro_rules! display_index {
    ($($ty:ty),* $(,)?) => {
        $(
            impl std::fmt::Display for $ty {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }
        )*
    };
}

display_index!(AtomIndex, FindIndex, RuleIndex);

/// A quantity that crossed its ceiling. Operand order is the type:
/// `observed` is what was measured, `ceiling` is the bound it must
/// not exceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Exceeded<T> {
    pub observed: T,
    pub ceiling: T,
}

/// Impossible stored bytes: a hard error, never a skip or a default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorruptionError {
    InvalidBool(u8),

    /// Order-key bytes representing a noncanonical F64.
    NonCanonicalF64([u8; 8]),

    InvalidInterval([u8; 16]),

    /// A fixed-width interval whose start plus width reaches the ceiling.
    InvalidFixedIntervalStart([u8; 8]),

    /// A query text token the answering image's interner never minted.
    DanglingInternId(InternId),

    WrongFactWidth {
        relation: RelationId,
        row_id: u64,
        mismatch: Mismatch<usize>,
    },

    RowCountMismatch {
        relation: RelationId,
        stored: u64,
    },

    MalformedValue(&'static str),

    NonzeroFixedBytesPad([u8; 8]),

    /// A required `meta` entry is absent or malformed.
    MetaMissing(&'static str),

    /// A physical key has an impossible shape.
    MalformedKey(&'static str),

    /// An index entry names a row that does not exist.
    DanglingIndexEntry,
}

/// A dynamic-surface id that does not resolve: relation, field, fresh
/// field, or key statement. These are not fact shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynIdError {
    UnknownRelation {
        relation: RelationId,
    },

    UnknownField {
        relation: RelationId,
        field: FieldId,
    },

    NotAKeyStatement {
        relation: RelationId,
        statement: StatementId,
    },
}

/// A mis-shaped dynamic fact on the untyped write surface
/// (`insert_dyn`/`delete_dyn`): ETL input is data, so shape
/// problems are typed errors, not panics.
/// Id-resolution failures are [`DynIdError`], nested as [`Self::Id`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactShapeError {
    Id(DynIdError),
    ArityMismatch {
        relation: RelationId,
        mismatch: Mismatch<usize>,
    },
    TypeMismatch {
        relation: RelationId,
        field: FieldId,
    },

    /// A bridge collection's arena offset or payload length exceeds `u32`.
    /// Direct Rust writes have no such arena; operation budgets bound them.
    PayloadBound {
        relation: RelationId,
    },
}

impl From<DynIdError> for FactShapeError {
    fn from(err: DynIdError) -> Self {
        Self::Id(err)
    }
}

/// Which side of a containment statement the commit-time judgment found
/// unsatisfied.
/// `Ord` is citation order: within one statement cited in both
/// directions, source before target ([`Violations`]' sort key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    SourceUnsatisfied,

    TargetRequired,
}

/// Theory rejection inside a successful outer [`Result`]: the candidate
/// formed correctly and either holds or fails the declared theory.
/// Infrastructure failure stays `Err(Error)`; an unadmitted value cannot
/// occupy an admitted slot.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission<T> {
    Accepted(T),
    Rejected(Violations),
}

impl<T> Admission<T> {
    /// # Panics
    #[track_caller]
    pub fn unwrap(self) -> T {
        match self {
            Self::Accepted(value) => value,
            Self::Rejected(violations) => panic!("admission rejected: {violations}"),
        }
    }

    /// # Panics
    #[track_caller]
    pub fn expect(self, msg: &str) -> T {
        match self {
            Self::Accepted(value) => value,
            Self::Rejected(violations) => {
                panic!("{msg}: admission rejected: {violations}")
            }
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Admission<U> {
        match self {
            Self::Accepted(value) => Admission::Accepted(f(value)),
            Self::Rejected(violations) => Admission::Rejected(violations),
        }
    }
}

/// One probe's witnessed judgment: either the obligation holds or it
/// cites exactly one violation. Checkers return this; they never mint
/// an error as a semantic verdict.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Holds,
    Violated(Violation),
}

/// Scalar put-conflict vs pointwise neighbor probe — two conviction
/// shapes, not an optional incumbent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conflict {
    Scalar,

    Pointwise { incumbent: Box<[u8]> },
}

/// [`Violations`]. One body: the typed spine slot, the convicting fact
/// bytes, and a per-law detail. Storage row ids never appear
/// .
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Functionality {
        statement: StatementRef,
        fact: Box<[u8]>,
        conflict: Conflict,
    },

    Containment {
        statement: StatementRef,
        direction: Direction,

        fact: Box<[u8]>,
    },

    /// Capacity sums source weights per target and checks the declared window.
    Capacity {
        statement: StatementRef,

        fact: Box<[u8]>,

        /// width crosses untruncated (ruled 2026-07-24, C3). On
        /// 2026-07-24, C14: the clip serves the verdict, the full sum
        measure: u128,
    },
}

impl Violation {
    pub(crate) fn functionality(
        statement: StatementRef,
        fact: Box<[u8]>,
        conflict: Conflict,
    ) -> Self {
        Self::Functionality {
            statement,
            fact,
            conflict,
        }
    }

    pub(crate) fn containment(
        statement: StatementRef,
        direction: Direction,
        fact: Box<[u8]>,
    ) -> Self {
        Self::Containment {
            statement,
            direction,
            fact,
        }
    }

    pub(crate) fn capacity(statement: StatementRef, fact: Box<[u8]>, measure: u128) -> Self {
        Self::Capacity {
            statement,
            fact,
            measure,
        }
    }

    #[must_use]
    pub const fn statement(&self) -> StatementRef {
        match *self {
            Self::Functionality { statement, .. }
            | Self::Containment { statement, .. }
            | Self::Capacity { statement, .. } => statement,
        }
    }

    #[must_use]
    pub fn statement_id(&self, schema: &crate::schema::Schema) -> StatementId {
        schema.id_of(self.statement())
    }

    #[must_use]
    pub fn fact(&self) -> &[u8] {
        match self {
            Self::Functionality { fact, .. }
            | Self::Containment { fact, .. }
            | Self::Capacity { fact, .. } => fact,
        }
    }

    #[must_use]
    pub fn incumbent(&self) -> Option<&[u8]> {
        match self {
            Self::Functionality {
                conflict: Conflict::Pointwise { incumbent },
                ..
            } => Some(incumbent),
            Self::Functionality {
                conflict: Conflict::Scalar,
                ..
            }
            | Self::Containment { .. }
            | Self::Capacity { .. } => None,
        }
    }
}

/// One cited fact of a violation, decoded to owned field values — the
/// bindings-consumable twin of the violation's canonical fact bytes
/// .
/// Decoding happens AT rejection time, inside the commit boundary,
/// because that is the only time it is possible: an inserted fact's
/// nowhere, so a post-hoc decode helper would misread genuine rejections
/// as corruption. `values` are in sealed field order (a closed
/// relation's synthetic id first), `str` fields resolved to owned
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedFact {
    relation: RelationId,
    values: Box<[bumbledb_theory::Value]>,
}

impl CitedFact {
    pub(crate) fn new(
        relation: RelationId,
        field_count: usize,
        values: Box<[bumbledb_theory::Value]>,
    ) -> Self {
        debug_assert_eq!(
            values.len(),
            field_count,
            "cited-fact values are one per sealed field"
        );
        let _ = field_count;
        Self { relation, values }
    }

    #[must_use]
    pub const fn relation(&self) -> RelationId {
        self.relation
    }

    #[must_use]
    pub fn values(&self) -> &[bumbledb_theory::Value] {
        &self.values
    }
}

/// ```compile_fail
/// let _ = bumbledb::Violations { citations: Box::new([]) };
/// ```
pub(crate) type CitedCitations = Box<[(Violation, Box<[CitedFact]>)]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violations {
    citations: CitedCitations,
    /// One flag per citation: true when offending facts exist beyond the
    /// cited examples (the judge's own per-statement example budget
    /// truncated). Parallel to `citations`; all-false by default so every
    /// pre-existing constructor keeps its meaning.
    truncated: Box<[bool]>,
}

impl Violations {
    #[cfg(test)]
    pub(crate) fn from_pairs(citations: CitedCitations) -> Self {
        debug_assert!(
            !citations.is_empty(),
            "a sealed rejection is nonempty by construction"
        );
        let truncated = vec![false; citations.len()].into_boxed_slice();
        Self {
            citations,
            truncated,
        }
    }

    /// As [`Self::from_pairs`], carrying the judge's per-statement
    /// example-truncation labels (one per citation, same order). The label
    /// says "offending facts exist beyond the cited examples"; the verdict
    /// (the statement set) is never truncated.
    pub(crate) fn from_pairs_with_truncation(
        citations: CitedCitations,
        truncated: Box<[bool]>,
    ) -> Self {
        debug_assert!(
            !citations.is_empty(),
            "a sealed rejection is nonempty by construction"
        );
        debug_assert_eq!(
            citations.len(),
            truncated.len(),
            "one truncation label per citation"
        );
        Self {
            citations,
            truncated,
        }
    }

    /// True when the judge's per-statement example budget dropped offending
    /// facts beyond the examples cited for the violation at `index`. Out of
    /// range reads as false (never a panic on a diagnostic accessor).
    #[must_use]
    pub fn examples_truncated(&self, index: usize) -> bool {
        self.truncated.get(index).copied().unwrap_or(false)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.citations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.citations.is_empty()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Violation> {
        self.citations.get(index).map(|(violation, _)| violation)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[(Violation, Box<[CitedFact]>)] {
        &self.citations
    }

    pub fn iter(&self) -> impl Iterator<Item = &Violation> {
        self.citations.iter().map(|(violation, _)| violation)
    }

    #[must_use]
    pub fn cited_facts(&self, index: usize) -> &[CitedFact] {
        self.citations
            .get(index)
            .map_or(&[], |(_, cited)| cited.as_ref())
    }

    pub fn citations(&self) -> impl Iterator<Item = (&Violation, &[CitedFact])> {
        self.citations
            .iter()
            .map(|(violation, cited)| (violation, cited.as_ref()))
    }
}

fn take_citation((violation, _): (Violation, Box<[CitedFact]>)) -> Violation {
    violation
}

fn citation_ref((violation, _): &(Violation, Box<[CitedFact]>)) -> &Violation {
    violation
}

impl IntoIterator for Violations {
    type Item = Violation;
    type IntoIter = std::iter::Map<
        std::vec::IntoIter<(Violation, Box<[CitedFact]>)>,
        fn((Violation, Box<[CitedFact]>)) -> Violation,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.citations.into_vec().into_iter().map(take_citation)
    }
}

impl<'a> IntoIterator for &'a Violations {
    type Item = &'a Violation;
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (Violation, Box<[CitedFact]>)>,
        fn(&'a (Violation, Box<[CitedFact]>)) -> &'a Violation,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.citations.iter().map(citation_ref)
    }
}

/// Which computation crossed its representation — [`Error::Overflow`]'s
/// payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowKind {
    Aggregate {
        find: FindIndex,
    },

    /// A group has more than `u64::MAX` distinct contributing bindings.
    Cardinality,

    OriginCapacity,
}

/// The in-memory representation whose fixed capacity a query or write reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capacity {
    /// A relation image holds more rows than its 32-bit positions address.
    ResidentRows,
    /// A distinct-row index is full.
    DistinctRows,
    /// A group-by table is full.
    Groups,
    /// A result buffer's byte heap exceeds its 32-bit offsets.
    ResultBytes,
}

/// An OS I/O failure owned by [`Error`]: kind plus raw errno, never a
/// foreign `std::io::Error` whose clone is lossy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoFailure {
    pub kind: std::io::ErrorKind,
    pub raw_os: Option<i32>,
}

impl IoFailure {
    #[must_use]
    pub fn from_io(err: &std::io::Error) -> Self {
        Self {
            kind: err.kind(),
            raw_os: err.raw_os_error(),
        }
    }

    #[must_use]
    pub fn raw_os_error(&self) -> Option<i32> {
        self.raw_os
    }
}

impl From<&std::io::Error> for IoFailure {
    fn from(err: &std::io::Error) -> Self {
        Self::from_io(err)
    }
}

impl From<std::io::Error> for IoFailure {
    fn from(err: std::io::Error) -> Self {
        Self::from_io(&err)
    }
}

/// An LMDB failure owned by [`Error`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LmdbFailure {
    Io(IoFailure),
    Mdb(heed::MdbError),
    Encoding,
    Decoding,
    EnvAlreadyOpened,
}

impl From<heed::Error> for LmdbFailure {
    fn from(err: heed::Error) -> Self {
        match err {
            heed::Error::Io(io) => Self::Io(IoFailure::from_io(&io)),
            heed::Error::Mdb(mdb) => Self::Mdb(mdb),
            heed::Error::Encoding(_) => Self::Encoding,
            heed::Error::Decoding(_) => Self::Decoding,
            heed::Error::EnvAlreadyOpened => Self::EnvAlreadyOpened,
        }
    }
}

/// A persisted counter that would wrap; it is refused, never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counter {
    RowIds,
    Generations,
}

/// A host record key outside the grammar.
pub use crate::storage::store::error::HostKeyFault;

/// The one engine error. Domain rejections are not errors: they are
/// [`crate::WriteOutcome::Rejected`] and [`Admission::Rejected`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The directory holds no bumbledb store of this format.
    NotABumbleDb {
        path: PathBuf,
    },
    /// The store was created with another schema.
    SchemaMismatch,
    /// Another live owner holds the directory lock.
    Locked {
        path: PathBuf,
    },
    /// A create or install destination already exists.
    DestinationExists {
        path: PathBuf,
    },
    /// The database is closing or closed.
    Closed,
    /// A second writer was requested on the thread that holds the writer.
    ReentrantWriter,
    /// Every LMDB reader slot holds a live snapshot.
    ReadersFull,
    /// A write needed more pages than the store's fixed map ceiling.
    Full {
        ceiling: u64,
    },
    Io(IoFailure),
    Lmdb(LmdbFailure),
    Corruption(CorruptionError),
    /// A change set sealed for another schema.
    ForeignSchema,
    /// A witness of another environment.
    ForeignWitness,
    /// A prepared query of another database.
    ForeignPreparedQuery,
    /// Closed relations are ground axioms of the schema, never written.
    ClosedRelationWrite {
        relation: RelationId,
    },
    /// The write closure caught an error after a mutation; nothing commits.
    TransactionPoisoned {
        source: Box<Error>,
    },
    Changes(crate::changes::ChangeError),
    HostKey(HostKeyFault),
    Exhausted(Counter),
    /// A capacity statement asks for the finite measure of a ray.
    CapacityRayMeasure {
        statement: StatementId,
    },
    /// A capacity group's measure exceeds the widened accumulator.
    MeasureOverflow {
        statement: StatementId,
    },
    Schema(SchemaError),
    Compile(crate::schema::CompileError),
    Validation(ValidationError),
    FactShape(FactShapeError),
    ParamCountMismatch {
        mismatch: Mismatch<usize>,
    },
    ParamTypeMismatch {
        param: ParamId,
        expected: ValueType,
    },
    ParamSetExpected {
        param: ParamId,
    },
    ParamScalarExpected {
        param: ParamId,
    },
    ParamElementTypeMismatch {
        param: ParamId,
        element: usize,
        expected: ValueType,
    },
    PointParamAtCeiling {
        param: ParamId,
    },
    Overflow(OverflowKind),
    Scalar {
        find: FindIndex,
        source: crate::ScalarError,
    },
    /// An in-memory representation reached its fixed capacity.
    Capacity(Capacity),
    ResultBytesOverflow,
    /// Cancellation or refused allocation, until it becomes
    /// `Error::Cancelled` / `Error::Allocation`.
    Store(Box<crate::storage::store::StoreError>),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The variant of an [`Error`], without its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    NotABumbleDb,
    SchemaMismatch,
    Locked,
    DestinationExists,
    Closed,
    ReentrantWriter,
    ReadersFull,
    Full,
    Io,
    Lmdb,
    Corruption,
    ForeignSchema,
    ForeignWitness,
    ForeignPreparedQuery,
    ClosedRelationWrite,
    TransactionPoisoned,
    Changes,
    HostKey,
    Exhausted,
    CapacityRayMeasure,
    MeasureOverflow,
    Schema,
    Compile,
    Validation,
    FactShape,
    Param,
    Overflow,
    Scalar,
    Capacity,
    Cancelled,
    Allocation,
}

impl Error {
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::NotABumbleDb { .. } => ErrorKind::NotABumbleDb,
            Self::SchemaMismatch => ErrorKind::SchemaMismatch,
            Self::Locked { .. } => ErrorKind::Locked,
            Self::DestinationExists { .. } => ErrorKind::DestinationExists,
            Self::Closed => ErrorKind::Closed,
            Self::ReentrantWriter => ErrorKind::ReentrantWriter,
            Self::ReadersFull => ErrorKind::ReadersFull,
            Self::Full { .. } => ErrorKind::Full,
            Self::Io(_) => ErrorKind::Io,
            Self::Lmdb(_) => ErrorKind::Lmdb,
            Self::Corruption(_) => ErrorKind::Corruption,
            Self::ForeignSchema => ErrorKind::ForeignSchema,
            Self::ForeignWitness => ErrorKind::ForeignWitness,
            Self::ForeignPreparedQuery => ErrorKind::ForeignPreparedQuery,
            Self::ClosedRelationWrite { .. } => ErrorKind::ClosedRelationWrite,
            Self::TransactionPoisoned { .. } => ErrorKind::TransactionPoisoned,
            Self::Changes(_) => ErrorKind::Changes,
            Self::HostKey(_) => ErrorKind::HostKey,
            Self::Exhausted(_) => ErrorKind::Exhausted,
            Self::CapacityRayMeasure { .. } => ErrorKind::CapacityRayMeasure,
            Self::MeasureOverflow { .. } => ErrorKind::MeasureOverflow,
            Self::Schema(_) => ErrorKind::Schema,
            Self::Compile(_) => ErrorKind::Compile,
            Self::Validation(_) => ErrorKind::Validation,
            Self::FactShape(_) => ErrorKind::FactShape,
            Self::ParamCountMismatch { .. }
            | Self::ParamTypeMismatch { .. }
            | Self::ParamSetExpected { .. }
            | Self::ParamScalarExpected { .. }
            | Self::ParamElementTypeMismatch { .. }
            | Self::PointParamAtCeiling { .. } => ErrorKind::Param,
            Self::Overflow(_) => ErrorKind::Overflow,
            Self::Scalar { .. } => ErrorKind::Scalar,
            Self::Capacity(_) | Self::ResultBytesOverflow => ErrorKind::Capacity,
            Self::Store(error) => match **error {
                crate::storage::store::StoreError::Work(crate::WorkError::Cancelled) => {
                    ErrorKind::Cancelled
                }
                _ => ErrorKind::Allocation,
            },
        }
    }
}
