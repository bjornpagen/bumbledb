//! The workspace error taxonomy (PRD 04; skeleton per `docs/architecture/60-api.md`).
//!
//! Everything reachable from user input or disk returns these typed errors;
//! panics are reserved for programmer-invariant violations. Payloads carry
//! ids, not formatted strings — `Display` (PRD 28) formats lazily.

use crate::ir::{ParamId, VarId};
use crate::schema::fingerprint::SchemaFingerprint;
use crate::schema::{ConstraintId, FieldId, RelationId};

/// Corruption detected while decoding stored bytes — a hard error, never a
/// skip, never a default (`docs/architecture/40-storage.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorruptionError {
    /// A Bool byte other than `0x00`/`0x01` — there is no distinct "true".
    InvalidBool(u8),
    /// An Enum ordinal at or beyond the declared variant count.
    EnumOrdinalOutOfRange { ordinal: u8, variant_count: u16 },
    /// The `_meta` database or one of its required keys is absent or
    /// malformed: the environment is not a usable bumbledb database.
    MetaMissing,
    /// An intern id with no reverse dictionary entry — a fact referencing it
    /// is corrupt.
    DanglingInternId(u64),
    /// A row id obtained from `M`/`U` has no `F` entry in the same snapshot.
    MissingFact { relation: RelationId, row_id: u64 },
    /// A stored fact's length differs from the schema's fact width.
    WrongFactWidth {
        relation: RelationId,
        row_id: u64,
        expected: usize,
        actual: usize,
    },
    /// The `F` scan yielded a different number of rows than the stored `S`
    /// row count — the derived counters have desynced from the facts.
    RowCountMismatch { relation: RelationId, stored: u64 },
}

/// A schema declaration error (PRD 02's validation boundary). Every illegal
/// schema shape has a distinct variant; an invalid schema is
/// unconstructible, not flagged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    DuplicateRelationName {
        name: Box<str>,
    },
    DuplicateFieldName {
        relation: RelationId,
        name: Box<str>,
    },
    DuplicateConstraintName {
        relation: RelationId,
        name: Box<str>,
    },
    EnumWithoutVariants {
        relation: RelationId,
        field: FieldId,
    },
    EnumTooManyVariants {
        relation: RelationId,
        field: FieldId,
        count: usize,
    },
    DuplicateEnumVariant {
        relation: RelationId,
        field: FieldId,
        variant: Box<str>,
    },
    SerialOnNonU64 {
        relation: RelationId,
        field: FieldId,
    },
    UnknownConstraintField {
        relation: RelationId,
        constraint: ConstraintId,
        field: FieldId,
    },
    UniqueWithoutFields {
        relation: RelationId,
        constraint: ConstraintId,
    },
    UniqueDuplicateField {
        relation: RelationId,
        constraint: ConstraintId,
        field: FieldId,
    },
    /// The constraint's guard key would exceed LMDB's key ceiling
    /// (`storage::keys::MAX_GUARD_WIDTH`) once embedded in a Restrict key.
    GuardKeyTooWide {
        relation: RelationId,
        constraint: ConstraintId,
        width: usize,
    },
    UnknownFkTargetRelation {
        relation: RelationId,
        constraint: ConstraintId,
        target: RelationId,
    },
    UnknownFkTargetConstraint {
        relation: RelationId,
        constraint: ConstraintId,
        target: ConstraintId,
    },
    FkTargetNotUnique {
        relation: RelationId,
        constraint: ConstraintId,
        target: ConstraintId,
    },
    FkArityMismatch {
        relation: RelationId,
        constraint: ConstraintId,
    },
    FkFieldTypeMismatch {
        relation: RelationId,
        constraint: ConstraintId,
        position: usize,
    },
}

/// How a foreign-key constraint failed at commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FkViolation {
    /// An inserted fact references a target unique key that does not exist
    /// in the final state (forward check).
    MissingTarget { fact_bytes: Box<[u8]> },
    /// A deleted unique key still has a live referrer in the final state
    /// (Restrict check); `relation`/`constraint` on the error name the
    /// *target* side, this names the referencing fact.
    RemainingReference {
        source_relation: RelationId,
        source_row: u64,
    },
}

/// A query validation error (the IR boundary, PRD 14): one variant per
/// roster item in `docs/architecture/20-query-ir.md`, returned at prepare
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    UnknownRelation {
        atom: usize,
        relation: RelationId,
    },
    UnknownField {
        atom: usize,
        field: FieldId,
    },
    DuplicateFieldBinding {
        atom: usize,
        field: FieldId,
    },
    VariableTypeConflict {
        var: VarId,
    },
    LiteralTypeMismatch {
        atom: usize,
        field: FieldId,
    },
    EnumOrdinalOutOfRange {
        atom: usize,
        field: FieldId,
        ordinal: u8,
    },
    ParamUnanchored {
        param: ParamId,
    },
    ParamTypeConflict {
        param: ParamId,
    },
    /// Type rules violated: cross-type comparison, or an order operator on
    /// a non-integer type.
    IllegalComparison {
        index: usize,
    },
    /// Neither side is a variable — write the query you mean.
    ConstantComparison {
        index: usize,
    },
    /// Datalog safety: a find (or aggregate-input) variable bound by no atom.
    UnboundFindVariable {
        var: VarId,
    },
    ComparisonOnlyVariable {
        var: VarId,
    },
    EmptyFinds,
    DuplicateFindTerm {
        index: usize,
    },
    NoAtoms,
    /// Sum/Min/Max over a non-integer variable.
    AggregateInputType {
        find: usize,
    },
    /// Count is nullary; it carries no variable.
    CountWithVariable {
        find: usize,
    },
    /// Sum/Min/Max require a variable.
    AggregateWithoutVariable {
        find: usize,
    },
    AggregateOverGroupKey {
        find: usize,
    },
    /// Planner cap: the exhaustive left-deep DP accepts at most
    /// `plan::planner::MAX_OCCURRENCES` atom occurrences.
    TooManyAtoms {
        count: usize,
    },
    /// Planner cap: at most 128 distinct variables (dense bitset width).
    TooManyVariables {
        count: usize,
    },
}

/// The one workspace error type, categorized per `docs/architecture/60-api.md`.
///
/// The Validation (IR boundary, PRD 14) and Write (PRDs 07-08) categories
/// gain their variants in the PRDs that raise them.
#[derive(Debug)]
pub enum Error {
    // --- Open errors ---
    /// Storage format version mismatch — checked before the fingerprint.
    FormatMismatch {
        found: u32,
        expected: u32,
    },
    /// Schema fingerprint mismatch: the compiled schema is not the stored one.
    SchemaMismatch {
        found: SchemaFingerprint,
        expected: SchemaFingerprint,
    },
    Io(std::io::Error),
    Lmdb(heed::Error),

    // --- Declaration / validation errors ---
    Schema(SchemaError),
    Validation(ValidationError),

    // --- Write errors ---
    /// A foreign-key invariant would be violated by the committed state:
    /// the whole transaction aborts.
    ForeignKeyViolation {
        relation: RelationId,
        constraint: ConstraintId,
        violation: FkViolation,
    },
    /// Two live facts claimed one unique key: the commit-time invariant is
    /// violated and the whole transaction aborts.
    UniqueViolation {
        relation: RelationId,
        constraint: ConstraintId,
        fact_bytes: Box<[u8]>,
    },
    /// A serial sequence reached `u64::MAX`; the generator can issue no
    /// further values for this field.
    SerialExhausted {
        relation: RelationId,
        field: FieldId,
    },

    // --- Runtime errors ---
    /// An aggregate's final value exceeds its result type (the once-at-
    /// finalization range check; deterministic under any fold order).
    Overflow {
        find: usize,
    },
    /// Hard corruption error, never a skip.
    Corruption(CorruptionError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<heed::Error> for Error {
    fn from(err: heed::Error) -> Self {
        Self::Lmdb(err)
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

impl From<CorruptionError> for Error {
    fn from(err: CorruptionError) -> Self {
        Self::Corruption(err)
    }
}
