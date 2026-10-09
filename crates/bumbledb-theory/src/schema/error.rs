//! Schema declaration errors: one variant per illegal shape, so an invalid
//! schema has no checked form. Payloads are ids and owned names.

use std::fmt;

use super::{FieldId, RelationId, StatementId};

/// A witnessed value against the bound it was required to equal: `witnessed`
/// is what was observed, `required` is the bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mismatch<T> {
    pub witnessed: T,
    pub required: T,
}

/// A row's index in a closed relation's extension, in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);

impl fmt::Display for RowIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// One declared key offered as evidence in a target-key rejection.
/// `projection_names` pairs `projection` positionwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetKeyCandidate {
    pub key: StatementId,
    pub projection: Box<[FieldId]>,
    pub projection_names: Box<[Box<str>]>,
}

/// A schema declaration error. Declaration-scoped variants carry no
/// statement id; every statement rejection is [`SchemaError::Statement`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// More relations than the 65,536 a `u16` relation word addresses.
    TooManyRelations {
        count: usize,
    },
    DuplicateRelationName {
        name: Box<str>,
    },
    DuplicateFieldName {
        relation: RelationId,
        name: Box<str>,
    },
    FixedBytesWidthOutOfRange {
        relation: RelationId,
        field: FieldId,
        len: u16,
    },
    IntervalWidthOutOfRange {
        relation: RelationId,
        field: FieldId,
        width: u64,
    },
    RelationTooManyColumns {
        relation: RelationId,
        columns: usize,
    },
    TooManyStatements {
        count: usize,
    },
    EmptyExtension {
        relation: RelationId,
    },
    ExtensionTooManyRows {
        relation: RelationId,
        count: usize,
    },
    DuplicateExtensionHandle {
        relation: RelationId,
        handle: Box<str>,
    },
    ExtensionArityMismatch {
        relation: RelationId,
        row: RowIndex,
        mismatch: Mismatch<usize>,
    },
    ExtensionValueTypeMismatch {
        relation: RelationId,
        row: RowIndex,
        field: FieldId,
    },
    /// A ground axiom with an unbounded interval.
    ExtensionIntervalRay {
        relation: RelationId,
        row: RowIndex,
        field: FieldId,
    },
    /// A closed relation's rows are named by their handles, never by text.
    StrOnClosedRelation {
        relation: RelationId,
        field: FieldId,
    },
    Statement {
        statement: StatementId,
        kind: StatementErrorKind,
    },
}

impl SchemaError {
    /// The statement a statement rejection names.
    #[must_use]
    pub const fn statement(&self) -> Option<StatementId> {
        match self {
            Self::Statement { statement, .. } => Some(*statement),
            _ => None,
        }
    }
}

/// Why one statement is rejected; the statement id lives on
/// [`SchemaError::Statement`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementErrorKind {
    UnknownRelation {
        relation: RelationId,
    },
    UnknownField {
        relation: RelationId,
        field: FieldId,
    },
    EmptyProjection {
        relation: RelationId,
    },
    DuplicateProjectionField {
        relation: RelationId,
        field: FieldId,
    },
    DuplicateSelectionField {
        relation: RelationId,
        field: FieldId,
    },
    /// A set selection names fewer than two literals.
    DegenerateSelectionSet {
        relation: RelationId,
        field: FieldId,
        len: usize,
    },
    DuplicateSelectionLiteral {
        relation: RelationId,
        field: FieldId,
    },
    CapacityInvertedWindow {
        lo: u64,
        hi: u64,
    },
    /// An interval position in a capacity projection.
    CapacityIntervalPosition {
        relation: RelationId,
        field: FieldId,
    },
    /// A `[field]` weight that is not `u64`: a signed weight would let an
    /// insert lower a sum.
    CapacityWeightNotU64 {
        relation: RelationId,
        field: FieldId,
    },
    /// A `[Duration(field)]` weight that is not a discrete interval.
    CapacityWeightNotDuration {
        relation: RelationId,
        field: FieldId,
    },
    /// A dependent bound field that is not `u64`.
    CapacityBoundNotU64 {
        relation: RelationId,
        field: FieldId,
    },
    /// A `Duration(field)` bound that is not a discrete interval.
    CapacityBoundNotDuration {
        relation: RelationId,
        field: FieldId,
    },
    /// A count of rows bounded by a duration.
    CapacityDimensionMixing {
        field: FieldId,
    },
    FunctionalityMultipleIntervals {
        relation: RelationId,
        field: FieldId,
    },
    FunctionalityIntervalNotLast {
        relation: RelationId,
        field: FieldId,
    },
    DuplicateFunctionality {
        earlier: StatementId,
    },
    ContainmentArityMismatch {
        mismatch: Mismatch<usize>,
    },
    ContainmentTypeMismatch {
        position: usize,
    },
    SelectedFieldProjected {
        relation: RelationId,
        field: FieldId,
    },
    SelectionLiteralTypeMismatch {
        relation: RelationId,
        field: FieldId,
    },
    /// No declared key of the target relation has the target projection's
    /// field set.
    NoMatchingTargetKey {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
        available: Box<[TargetKeyCandidate]>,
    },
    /// An interval target projection with no pointwise key of that field set.
    NoPointwiseTargetKey {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
        available: Box<[TargetKeyCandidate]>,
    },
    /// An interval position on a containment with a closed side.
    ClosedContainmentInterval {
        relation: RelationId,
    },
    /// A closed target addressed by anything but its handle `id`.
    ClosedTargetNotHandle {
        target: RelationId,
        target_name: Box<str>,
        projection: Box<[FieldId]>,
        projection_names: Box<[Box<str>]>,
    },
    /// A statement over closed relations that one of their rows refutes.
    ClosedStatementRefuted {
        relation: RelationId,
        row: RowIndex,
    },
    DuplicateStatement {
        earlier: StatementId,
    },
}

impl StatementErrorKind {
    #[must_use]
    pub fn at(self, statement: StatementId) -> SchemaError {
        SchemaError::Statement {
            statement,
            kind: self,
        }
    }
}

impl std::error::Error for SchemaError {}

/// `names` pairs `projection` positionwise.
fn field_set(
    f: &mut fmt::Formatter<'_>,
    projection: &[FieldId],
    names: &[Box<str>],
) -> fmt::Result {
    let mut fields: Vec<_> = projection.iter().copied().zip(names.iter()).collect();
    fields.sort_unstable_by_key(|(field, _)| *field);
    write!(f, "{{")?;
    for (index, (field, name)) in fields.iter().enumerate() {
        if index > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{name} ({})", field.0)?;
    }
    write!(f, "}}")
}

fn target_key_rejection(
    f: &mut fmt::Formatter<'_>,
    target: RelationId,
    target_name: &str,
    projection: &[FieldId],
    projection_names: &[Box<str>],
    available: &[TargetKeyCandidate],
    pointwise: bool,
) -> fmt::Result {
    write!(
        f,
        "target relation {target_name} ({}) projection ",
        target.0
    )?;
    field_set(f, projection, projection_names)?;
    write!(f, " matches no declared key; available keys: ")?;
    if available.is_empty() {
        write!(f, "none")?;
    } else {
        for (index, candidate) in available.iter().enumerate() {
            if index > 0 {
                write!(f, "; ")?;
            }
            write!(f, "statement {} ", candidate.key.0)?;
            field_set(f, &candidate.projection, &candidate.projection_names)?;
        }
    }
    if pointwise {
        write!(
            f,
            "; hint: declare the exact pointwise key `R(prefix…, interval) -> R`"
        )?;
    }
    Ok(())
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyRelations { count } => {
                write!(f, "{count} relations exceed the 65,536-relation id space")
            }
            Self::DuplicateRelationName { name } => write!(f, "duplicate relation name `{name}`"),
            Self::DuplicateFieldName { relation, name } => {
                write!(f, "relation {}: duplicate field name `{name}`", relation.0)
            }
            Self::FixedBytesWidthOutOfRange {
                relation,
                field,
                len,
            } => write!(
                f,
                "relation {}, field {}: bytes<{len}> outside the 1..=64 width range",
                relation.0, field.0
            ),
            Self::IntervalWidthOutOfRange {
                relation,
                field,
                width,
            } => write!(
                f,
                "relation {}, field {}: interval width {width} outside 1..=u64::MAX-1",
                relation.0, field.0
            ),
            Self::RelationTooManyColumns { relation, columns } => write!(
                f,
                "relation {}: {columns} columns exceed the 65,535-column cap \
                 (an interval or uuid field spans two columns, bytes<N> ⌈N/8⌉)",
                relation.0
            ),
            Self::TooManyStatements { count } => {
                write!(f, "{count} statements exceed the 65,536-statement id space")
            }
            Self::EmptyExtension { relation } => {
                write!(f, "relation {}: a closed relation has no rows", relation.0)
            }
            Self::ExtensionTooManyRows { relation, count } => write!(
                f,
                "relation {}: {count} rows exceed the 256-row closed relation cap",
                relation.0
            ),
            Self::DuplicateExtensionHandle { relation, handle } => {
                write!(f, "relation {}: duplicate handle `{handle}`", relation.0)
            }
            Self::ExtensionArityMismatch {
                relation,
                row,
                mismatch,
            } => write!(
                f,
                "relation {}, row {row}: {} values for {} columns",
                relation.0, mismatch.witnessed, mismatch.required
            ),
            Self::ExtensionValueTypeMismatch {
                relation,
                row,
                field,
            } => write!(
                f,
                "relation {}, row {row}: value type mismatch at field {}",
                relation.0, field.0
            ),
            Self::ExtensionIntervalRay {
                relation,
                row,
                field,
            } => write!(
                f,
                "relation {}, row {row}: unbounded interval at field {}; \
                 a closed relation's rows are bounded",
                relation.0, field.0
            ),
            Self::StrOnClosedRelation { relation, field } => write!(
                f,
                "relation {}, field {}: str on a closed relation; the handle is the label",
                relation.0, field.0
            ),
            Self::Statement { statement, kind } => {
                write!(f, "statement {}: {kind}", statement.0)
            }
        }
    }
}

impl fmt::Display for StatementErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRelation { relation } => write!(f, "unknown relation {}", relation.0),
            Self::UnknownField { relation, field } => {
                write!(f, "relation {} has no field {}", relation.0, field.0)
            }
            Self::EmptyProjection { relation } => {
                write!(f, "empty projection on relation {}", relation.0)
            }
            Self::DuplicateProjectionField { relation, field } => write!(
                f,
                "field {} projected twice on relation {}",
                field.0, relation.0
            ),
            Self::DuplicateSelectionField { relation, field } => write!(
                f,
                "field {} selected twice on relation {}",
                field.0, relation.0
            ),
            Self::DegenerateSelectionSet {
                relation,
                field,
                len,
            } => write!(
                f,
                "literal set of {len} on relation {}, field {}: a set selection \
                 names at least two literals (one literal is an equality)",
                relation.0, field.0
            ),
            Self::DuplicateSelectionLiteral { relation, field } => write!(
                f,
                "duplicate literal in the set selection on relation {}, field {}",
                relation.0, field.0
            ),
            Self::CapacityInvertedWindow { lo, hi } => write!(
                f,
                "the window {lo}..{hi} is inverted: no measure satisfies hi < lo"
            ),
            Self::CapacityIntervalPosition { relation, field } => write!(
                f,
                "interval field {} on relation {} in a capacity projection; \
                 an interval's measure enters through `[Duration(field)]`",
                field.0, relation.0
            ),
            Self::CapacityWeightNotU64 { relation, field } => write!(
                f,
                "weight field {} on relation {} is not u64; a signed weight \
                 would let an insert lower a sum",
                field.0, relation.0
            ),
            Self::CapacityWeightNotDuration { relation, field } => write!(
                f,
                "weight field {} on relation {} is not a discrete interval; \
                 `[Duration(field)]` reads an exact integer duration",
                field.0, relation.0
            ),
            Self::CapacityBoundNotU64 { relation, field } => write!(
                f,
                "bound field {} on relation {} is not u64; a dependent bound \
                 reads a u64 field of the target row",
                field.0, relation.0
            ),
            Self::CapacityBoundNotDuration { relation, field } => write!(
                f,
                "bound field {} on relation {} is not a discrete interval; \
                 `Duration(field)` bounds by a target interval's duration",
                field.0, relation.0
            ),
            Self::CapacityDimensionMixing { field } => write!(
                f,
                "a count window bounded by the duration of field {}: weigh the \
                 source with `[Duration(field)]`, or bound by a u64 field or literal",
                field.0
            ),
            Self::FunctionalityMultipleIntervals { relation, field } => write!(
                f,
                "second interval field {} on relation {}: a key has at most one interval",
                field.0, relation.0
            ),
            Self::FunctionalityIntervalNotLast { relation, field } => write!(
                f,
                "interval field {} on relation {} must be the last projection position",
                field.0, relation.0
            ),
            Self::DuplicateFunctionality { earlier } => {
                write!(f, "statement {} already keys this field set", earlier.0)
            }
            Self::ContainmentArityMismatch { mismatch } => write!(
                f,
                "{} source positions against {} target positions",
                mismatch.witnessed, mismatch.required
            ),
            Self::ContainmentTypeMismatch { position } => {
                write!(f, "type mismatch at position {position}")
            }
            Self::SelectedFieldProjected { relation, field } => write!(
                f,
                "field {} on relation {} is both selected and projected",
                field.0, relation.0
            ),
            Self::SelectionLiteralTypeMismatch { relation, field } => write!(
                f,
                "selection literal type mismatch at relation {}, field {}",
                relation.0, field.0
            ),
            Self::NoMatchingTargetKey {
                target,
                target_name,
                projection,
                projection_names,
                available,
            } => target_key_rejection(
                f,
                *target,
                target_name,
                projection,
                projection_names,
                available,
                false,
            ),
            Self::NoPointwiseTargetKey {
                target,
                target_name,
                projection,
                projection_names,
                available,
            } => target_key_rejection(
                f,
                *target,
                target_name,
                projection,
                projection_names,
                available,
                true,
            ),
            Self::ClosedContainmentInterval { relation } => write!(
                f,
                "interval position on a containment with closed relation {}",
                relation.0
            ),
            Self::ClosedTargetNotHandle {
                target,
                target_name,
                projection,
                projection_names,
            } => {
                write!(
                    f,
                    "closed target relation {target_name} ({}) is addressed by its \
                     handle id only; projection ",
                    target.0
                )?;
                field_set(f, projection, projection_names)?;
                write!(
                    f,
                    " must be exactly {{id (0)}} (write the target side as `R(id)`)"
                )
            }
            Self::ClosedStatementRefuted { relation, row } => {
                write!(f, "refuted by row {row} of closed relation {}", relation.0)
            }
            Self::DuplicateStatement { earlier } => {
                write!(f, "duplicates statement {}", earlier.0)
            }
        }
    }
}
