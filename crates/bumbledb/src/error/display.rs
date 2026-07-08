//! `Display` rendering for every error type — formatting runs lazily, only
//! when the host actually prints.

use std::fmt;

use super::{CorruptionError, Direction, Error, FactShapeError, SchemaError, ValidationError};

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
            Self::InvalidInterval(bytes) => {
                write!(f, "interval bytes {bytes:02x?}: start >= end")
            }
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
        // Short bindings: r = relation, fd = field.
        match self {
            Self::DuplicateRelationName { name } => write!(f, "duplicate relation name `{name}`"),
            Self::DuplicateFieldName { relation: r, name } => {
                write!(f, "relation {}: duplicate field name `{name}`", r.0)
            }
            Self::EnumWithoutVariants {
                relation: r,
                field: fd,
            } => {
                write!(f, "relation {}, field {}: enum with no variants", r.0, fd.0)
            }
            Self::EnumTooManyVariants {
                relation: r,
                field: fd,
                count,
            } => write!(
                f,
                "relation {}, field {}: {count} enum variants exceed the u8 ordinal",
                r.0, fd.0
            ),
            Self::DuplicateEnumVariant {
                relation: r,
                field: fd,
                variant,
            } => write!(
                f,
                "relation {}, field {}: duplicate enum variant `{variant}`",
                r.0, fd.0
            ),
            Self::SerialOnNonU64 {
                relation: r,
                field: fd,
            } => {
                write!(f, "relation {}, field {}: serial requires u64", r.0, fd.0)
            }
            Self::StatementUnknownRelation {
                statement: s,
                relation: r,
            } => write!(f, "statement {}: unknown relation {}", s.0, r.0),
            Self::StatementUnknownField {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: relation {} has no field {}",
                s.0, r.0, fd.0
            ),
            Self::EmptyProjection {
                statement: s,
                relation: r,
            } => write!(f, "statement {}: empty projection on relation {}", s.0, r.0),
            Self::DuplicateProjectionField {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: field {} projected twice on relation {}",
                s.0, fd.0, r.0
            ),
            Self::DuplicateSelectionField {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: field {} selected twice on relation {}",
                s.0, fd.0, r.0
            ),
            Self::FunctionalityMultipleIntervals {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: second interval field {} on relation {} — the ordered guard answers one dimension",
                s.0, fd.0, r.0
            ),
            Self::FunctionalityIntervalNotLast {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: interval field {} on relation {} must be the final projection position",
                s.0, fd.0, r.0
            ),
            Self::DuplicateFunctionality {
                statement: s,
                earlier,
            } => write!(
                f,
                "statement {}: statement {} already keys this field set",
                s.0, earlier.0
            ),
            Self::GuardKeyTooWide { statement: s, width } => write!(
                f,
                "statement {}: {width}-byte guard key exceeds the key-size ceiling",
                s.0
            ),
            Self::ContainmentArityMismatch {
                statement: s,
                source,
                target,
            } => write!(
                f,
                "statement {}: {source} source positions against {target} target positions",
                s.0
            ),
            Self::ContainmentTypeMismatch {
                statement: s,
                position,
            } => write!(
                f,
                "statement {}: structural type mismatch at position {position}",
                s.0
            ),
            Self::SelectedFieldProjected {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: field {} on relation {} is both selected and projected",
                s.0, fd.0, r.0
            ),
            Self::SelectionLiteralTypeMismatch {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: selection literal type mismatch at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::SelectionEnumOrdinalOutOfRange {
                statement: s,
                relation: r,
                field: fd,
                ordinal,
            } => write!(
                f,
                "statement {}: enum ordinal {ordinal} out of range at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::SelectionLiteralNotUtf8 {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: string literal is not UTF-8 at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::SelectionIntervalEmpty {
                statement: s,
                relation: r,
                field: fd,
            } => write!(
                f,
                "statement {}: interval literal start >= end at relation {}, field {}",
                s.0, r.0, fd.0
            ),
            Self::NoMatchingTargetKey {
                statement: s,
                relation: r,
            } => write!(
                f,
                "statement {}: target projection matches no key of relation {}",
                s.0, r.0
            ),
            Self::NoPointwiseTargetKey {
                statement: s,
                relation: r,
            } => write!(
                f,
                "statement {}: no pointwise key of relation {} carries the interval position",
                s.0, r.0
            ),
            Self::DuplicateStatement {
                statement: s,
                earlier,
            } => write!(
                f,
                "statement {}: duplicates statement {} — write it once",
                s.0, earlier.0
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
                write!(
                    f,
                    "storage format version {found}, this build expects {expected}"
                )
            }
            Self::SchemaMismatch { .. } => {
                write!(f, "the compiled schema's fingerprint is not the stored one")
            }
            Self::AlreadyInitialized => {
                write!(
                    f,
                    "the directory already holds an LMDB environment; open it instead"
                )
            }
            Self::EnvironmentLocked => {
                write!(f, "another live handle holds this environment's lock")
            }
            Self::Io(err) => write!(f, "io: {err}"),
            Self::Lmdb(err) => write!(f, "lmdb: {err}"),
            Self::Schema(err) => write!(f, "schema declaration: {err}"),
            Self::Validation(err) => write!(f, "query validation: {err}"),
            Self::FactShape(err) => write!(f, "dynamic fact: {err}"),
            Self::FunctionalityViolation { statement, .. } => write!(
                f,
                "statement {}: functionality violated — two live facts claim one key",
                statement.0
            ),
            Self::ContainmentViolation {
                statement, side, ..
            } => match side {
                Direction::SourceUnsatisfied => write!(
                    f,
                    "statement {}: containment violated — an inserted source fact has no target",
                    statement.0
                ),
                Direction::TargetRequired => write!(
                    f,
                    "statement {}: containment violated — a deleted target key is still required",
                    statement.0
                ),
            },
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
                write!(
                    f,
                    "{supplied} parameters supplied, the query takes {expected}"
                )
            }
            Self::ParamTypeMismatch { param, expected } => {
                write!(f, "parameter {}: expected {expected:?}", param.0)
            }
            Self::Overflow { find } => {
                write!(f, "find {find}: aggregate result exceeds its type")
            }
            Self::ResultBytesOverflow => {
                write!(
                    f,
                    "the result buffer's byte heap exceeds u32 offsets (4 GiB)"
                )
            }
            Self::Corruption(err) => write!(f, "corruption: {err}"),
        }
    }
}
