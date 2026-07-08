//! `Display` rendering for every error type — formatting runs lazily, only
//! when the host actually prints.

use std::fmt;

use super::{CorruptionError, Error, FactShapeError, FkViolation, SchemaError, ValidationError};

impl fmt::Display for FactShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRelation { relation } => {
                write!(f, "relation {}: not in this schema", relation.0)
            }
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
            Self::ConstraintDuplicateField { relation: r, constraint: c, field: fd } => write!(
                f,
                "relation {}, constraint {}: field {} listed twice",
                r.0, c.0, fd.0
            ),
            Self::DuplicateConstraintFields { relation: r, constraint: c } => write!(
                f,
                "relation {}, constraint {}: another unique constraint covers the same fields",
                r.0, c.0
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
            Self::SelfComparison { index } => {
                write!(f, "comparison {index}: a variable compared with itself")
            }
            Self::ComparisonEnumOrdinalOutOfRange { index, ordinal } => {
                write!(f, "comparison {index}: enum ordinal {ordinal} out of range")
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
                write!(f, "the directory already holds an LMDB environment; open it instead")
            }
            Self::EnvironmentLocked => {
                write!(f, "another live handle holds this environment's lock")
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
            Self::ForeignPreparedQuery => {
                write!(
                    f,
                    "a prepared query executes only against snapshots of the database that prepared it"
                )
            }
            Self::ParamCountMismatch { expected, supplied } => {
                write!(f, "{supplied} parameters supplied, the query takes {expected}")
            }
            Self::ParamTypeMismatch { param, expected } => {
                write!(f, "parameter {}: expected {expected:?}", param.0)
            }
            Self::Overflow { find } => {
                write!(f, "find {find}: aggregate result exceeds its type")
            }
            Self::ResultBytesOverflow => {
                write!(f, "the result buffer's byte heap exceeds u32 offsets (4 GiB)")
            }
            Self::Corruption(err) => write!(f, "corruption: {err}"),
        }
    }
}
