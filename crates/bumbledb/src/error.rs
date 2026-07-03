//! The workspace error taxonomy (PRD 04; categories per
//! `docs/architecture/60-api.md`).
//!
//! Everything reachable from user input or disk returns these typed errors;
//! panics are reserved for programmer-invariant violations. Payloads carry
//! ids and owned fact bytes, never formatted strings — no `format!` runs on
//! a hot path; `Display` formats lazily when the host actually prints.

use std::fmt;

use crate::ir::{ParamId, VarId};
use crate::schema::fingerprint::SchemaFingerprint;
use crate::schema::{ConstraintId, FieldId, RelationId, ValueType};

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
    /// A live `M` entry's `F` row or `U` guard was absent at delete time —
    /// the write-side M/F disagreement (the read side raises
    /// [`CorruptionError::MissingFact`]).
    MembershipDesync { relation: RelationId, row_id: u64 },
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
    /// A stored value (a counter, row id, or dictionary id) failed to
    /// decode; the static string names which kind — a diagnosis, not a
    /// formatted payload.
    MalformedValue(&'static str),
    /// A stored string's bytes are not UTF-8 — distinct from a dangling id
    /// (the reverse entry exists; its content is mojibake).
    NonUtf8Intern(u64),
    /// The reverse dictionary entry's tag byte disagrees with the field
    /// type that referenced it (a String field carrying a Bytes id, or
    /// vice versa).
    InternTagMismatch(u64),
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
    /// *target* side, this carries the referencing fact itself (storage row
    /// ids never surface — `docs/architecture/10-data-model.md`).
    RemainingReference {
        source_relation: RelationId,
        fact_bytes: Box<[u8]>,
    },
}

/// A mis-shaped dynamic fact on the untyped write surface
/// (`insert_dyn`/`delete_dyn`/`bulk_load`): ETL input is data, so shape
/// problems are typed errors, not panics (`docs/architecture/60-api.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactShapeError {
    ArityMismatch {
        relation: RelationId,
        expected: usize,
        supplied: usize,
    },
    TypeMismatch {
        relation: RelationId,
        field: FieldId,
    },
    EnumOrdinalOutOfRange {
        relation: RelationId,
        field: FieldId,
        ordinal: u8,
    },
    /// `Value::String` bytes are not UTF-8.
    InvalidUtf8 {
        relation: RelationId,
        field: FieldId,
    },
}

impl fmt::Display for FactShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArityMismatch {
                relation,
                expected,
                supplied,
            } => write!(
                f,
                "relation {}: {supplied} values for {expected} fields",
                relation.0
            ),
            Self::TypeMismatch { relation, field } => {
                write!(
                    f,
                    "relation {}, field {}: wrong value kind",
                    relation.0, field.0
                )
            }
            Self::EnumOrdinalOutOfRange {
                relation,
                field,
                ordinal,
            } => write!(
                f,
                "relation {}, field {}: enum ordinal {ordinal} out of range",
                relation.0, field.0
            ),
            Self::InvalidUtf8 { relation, field } => write!(
                f,
                "relation {}, field {}: string bytes are not UTF-8",
                relation.0, field.0
            ),
        }
    }
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
    /// Param ids must be dense (0..n): a gap would be a positional slot
    /// whose supplied value is never type-checked.
    ParamIdGap {
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
    /// `create` refused a directory that already holds a bumbledb
    /// environment — re-initializing `_meta` over live data would be
    /// silent corruption; open it instead.
    AlreadyInitialized,
    Io(std::io::Error),
    Lmdb(heed::Error),

    // --- Declaration / validation errors ---
    Schema(SchemaError),
    Validation(ValidationError),
    /// A mis-shaped dynamic fact on the ETL surface (data, not code).
    FactShape(FactShapeError),

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
    /// Bind-time: the supplied parameter count does not match the query's.
    ParamCountMismatch {
        expected: usize,
        supplied: usize,
    },
    /// Bind-time: a supplied parameter's structural type does not match
    /// the anchor-inferred one.
    ParamTypeMismatch {
        param: ParamId,
        expected: ValueType,
    },
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

impl fmt::Display for CorruptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBool(byte) => write!(f, "invalid Bool byte {byte:#04x}"),
            Self::EnumOrdinalOutOfRange {
                ordinal,
                variant_count,
            } => write!(f, "enum ordinal {ordinal} beyond {variant_count} variants"),
            Self::MetaMissing => write!(f, "the _meta database is absent or malformed"),
            Self::DanglingInternId(id) => write!(f, "intern id {id} has no dictionary entry"),
            Self::MissingFact { relation, row_id } => {
                write!(f, "relation {}: row {row_id} has no fact", relation.0)
            }
            Self::MembershipDesync { relation, row_id } => write!(
                f,
                "relation {}: membership entry for row {row_id} desynced from its F/U entries",
                relation.0
            ),
            Self::WrongFactWidth {
                relation,
                row_id,
                expected,
                actual,
            } => write!(
                f,
                "relation {}: row {row_id} is {actual} bytes, schema says {expected}",
                relation.0
            ),
            Self::RowCountMismatch { relation, stored } => write!(
                f,
                "relation {}: stored row count {stored} desynced from the facts",
                relation.0
            ),
            Self::MalformedValue(kind) => write!(f, "malformed stored value: {kind}"),
            Self::NonUtf8Intern(id) => write!(f, "intern id {id}: stored bytes are not UTF-8"),
            Self::InternTagMismatch(id) => {
                write!(
                    f,
                    "intern id {id}: reverse-entry tag disagrees with the field type"
                )
            }
        }
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Short bindings: r = relation, c = constraint, fd = field.
        match self {
            Self::DuplicateRelationName { name } => write!(f, "duplicate relation name `{name}`"),
            Self::DuplicateFieldName { relation: r, name } => {
                write!(f, "relation {}: duplicate field name `{name}`", r.0)
            }
            Self::DuplicateConstraintName { relation: r, name } => {
                write!(f, "relation {}: duplicate constraint name `{name}`", r.0)
            }
            Self::EnumWithoutVariants { relation: r, field: fd } => {
                write!(f, "relation {}, field {}: enum with no variants", r.0, fd.0)
            }
            Self::EnumTooManyVariants { relation: r, field: fd, count } => write!(
                f,
                "relation {}, field {}: {count} enum variants exceed the u8 ordinal",
                r.0, fd.0
            ),
            Self::DuplicateEnumVariant { relation: r, field: fd, variant } => write!(
                f,
                "relation {}, field {}: duplicate enum variant `{variant}`",
                r.0, fd.0
            ),
            Self::SerialOnNonU64 { relation: r, field: fd } => {
                write!(f, "relation {}, field {}: serial requires u64", r.0, fd.0)
            }
            Self::UnknownConstraintField { relation: r, constraint: c, field: fd } => write!(
                f,
                "relation {}, constraint {}: unknown field {}",
                r.0, c.0, fd.0
            ),
            Self::UniqueWithoutFields { relation: r, constraint: c } => write!(
                f,
                "relation {}, constraint {}: unique over no fields",
                r.0, c.0
            ),
            Self::UniqueDuplicateField { relation: r, constraint: c, field: fd } => write!(
                f,
                "relation {}, constraint {}: field {} listed twice",
                r.0, c.0, fd.0
            ),
            Self::GuardKeyTooWide { relation: r, constraint: c, width } => write!(
                f,
                "relation {}, constraint {}: {width}-byte guard key exceeds the LMDB ceiling",
                r.0, c.0
            ),
            Self::UnknownFkTargetRelation { relation: r, constraint: c, target } => write!(
                f,
                "relation {}, constraint {}: unknown fk target relation {}",
                r.0, c.0, target.0
            ),
            Self::UnknownFkTargetConstraint { relation: r, constraint: c, target } => write!(
                f,
                "relation {}, constraint {}: unknown fk target constraint {}",
                r.0, c.0, target.0
            ),
            Self::FkTargetNotUnique { relation: r, constraint: c, target } => write!(
                f,
                "relation {}, constraint {}: fk target constraint {} is not unique",
                r.0, c.0, target.0
            ),
            Self::FkArityMismatch { relation: r, constraint: c } => write!(
                f,
                "relation {}, constraint {}: fk arity differs from its target",
                r.0, c.0
            ),
            Self::FkFieldTypeMismatch { relation: r, constraint: c, position } => write!(
                f,
                "relation {}, constraint {}: fk field type differs from its target at position {position}",
                r.0, c.0
            ),
        }
    }
}

impl fmt::Display for FkViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTarget { fact_bytes } => write!(
                f,
                "an inserted fact ({} bytes) references a missing target key",
                fact_bytes.len()
            ),
            Self::RemainingReference {
                source_relation,
                fact_bytes,
            } => write!(
                f,
                "a deleted key is still referenced by a relation-{} fact ({} bytes)",
                source_relation.0,
                fact_bytes.len()
            ),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRelation { atom, relation } => {
                write!(f, "atom {atom}: unknown relation {}", relation.0)
            }
            Self::UnknownField { atom, field } => {
                write!(f, "atom {atom}: unknown field {}", field.0)
            }
            Self::DuplicateFieldBinding { atom, field } => {
                write!(f, "atom {atom}: field {} bound twice", field.0)
            }
            Self::VariableTypeConflict { var } => {
                write!(f, "variable {} bound at conflicting types", var.0)
            }
            Self::LiteralTypeMismatch { atom, field } => {
                write!(f, "atom {atom}: literal type mismatch at field {}", field.0)
            }
            Self::EnumOrdinalOutOfRange {
                atom,
                field,
                ordinal,
            } => write!(
                f,
                "atom {atom}: enum ordinal {ordinal} out of range at field {}",
                field.0
            ),
            Self::ParamIdGap { param } => {
                write!(f, "parameter ids are not dense: {} is unused", param.0)
            }
            Self::ParamTypeConflict { param } => {
                write!(f, "parameter {} anchored at conflicting types", param.0)
            }
            Self::IllegalComparison { index } => {
                write!(f, "comparison {index}: type rules violated")
            }
            Self::ConstantComparison { index } => {
                write!(f, "comparison {index}: neither side is a variable")
            }
            Self::UnboundFindVariable { var } => {
                write!(f, "find variable {} bound by no atom", var.0)
            }
            Self::ComparisonOnlyVariable { var } => {
                write!(f, "variable {} appears only in comparisons", var.0)
            }
            Self::EmptyFinds => write!(f, "the find list is empty"),
            Self::DuplicateFindTerm { index } => write!(f, "find term {index} is a duplicate"),
            Self::NoAtoms => write!(f, "the query has no atoms"),
            Self::AggregateInputType { find } => {
                write!(f, "find {find}: aggregate over a non-integer variable")
            }
            Self::CountWithVariable { find } => {
                write!(f, "find {find}: Count is nullary")
            }
            Self::AggregateWithoutVariable { find } => {
                write!(f, "find {find}: this aggregate requires a variable")
            }
            Self::AggregateOverGroupKey { find } => {
                write!(f, "find {find}: aggregate over a group-key variable")
            }
            Self::TooManyAtoms { count } => {
                write!(f, "{count} atom occurrences exceed the planner cap")
            }
            Self::TooManyVariables { count } => {
                write!(f, "{count} distinct variables exceed the 128-bit bitset")
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FormatMismatch { found, expected } => {
                write!(f, "storage format version {found}, this build expects {expected}")
            }
            Self::SchemaMismatch { .. } => {
                write!(f, "the compiled schema's fingerprint is not the stored one")
            }
            Self::AlreadyInitialized => {
                write!(f, "the directory already holds a bumbledb environment; open it instead")
            }
            Self::Io(err) => write!(f, "io: {err}"),
            Self::Lmdb(err) => write!(f, "lmdb: {err}"),
            Self::Schema(err) => write!(f, "schema declaration: {err}"),
            Self::Validation(err) => write!(f, "query validation: {err}"),
            Self::FactShape(err) => write!(f, "dynamic fact: {err}"),
            Self::ForeignKeyViolation {
                relation,
                constraint,
                violation,
            } => write!(
                f,
                "foreign key violation (relation {}, constraint {}): {violation}",
                relation.0, constraint.0
            ),
            Self::UniqueViolation {
                relation,
                constraint,
                fact_bytes,
            } => write!(
                f,
                "unique violation (relation {}, constraint {}): a live fact ({} bytes) already claims the key",
                relation.0,
                constraint.0,
                fact_bytes.len()
            ),
            Self::SerialExhausted { relation, field } => write!(
                f,
                "serial sequence exhausted (relation {}, field {})",
                relation.0, field.0
            ),
            Self::ParamCountMismatch { expected, supplied } => {
                write!(f, "{supplied} parameters supplied, the query takes {expected}")
            }
            Self::ParamTypeMismatch { param, expected } => {
                write!(f, "parameter {}: expected {expected:?}", param.0)
            }
            Self::Overflow { find } => {
                write!(f, "find {find}: aggregate result exceeds its type")
            }
            Self::Corruption(err) => write!(f, "corruption: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Lmdb(err) => Some(err),
            _ => None,
        }
    }
}
