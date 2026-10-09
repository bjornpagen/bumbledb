//! Schema declaration errors: one variant per illegal shape, so an invalid
//! schema has no checked form. Payloads are ids and owned names.

use std::fmt;

use super::{FieldId, RelationId, SchemaDescriptor, StatementId};

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

    /// The error with relations, fields and rows named as `descriptor`
    /// declares them, and without the statement id (the caller cites the
    /// statement itself). `Display` names everything by id.
    #[must_use]
    pub const fn named<'a>(&'a self, descriptor: &'a SchemaDescriptor) -> Named<'a> {
        Named {
            error: self,
            descriptor,
        }
    }
}

/// A [`SchemaError`] rendered with declared names; see [`SchemaError::named`].
#[derive(Debug, Clone, Copy)]
pub struct Named<'a> {
    error: &'a SchemaError,
    descriptor: &'a SchemaDescriptor,
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
    /// A count of rows bounded by the duration of a target field.
    CapacityDimensionMixing {
        relation: RelationId,
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

/// How a rendered error names relations, fields and rows.
trait Names {
    fn relation(&self, relation: RelationId) -> String;
    fn field(&self, relation: RelationId, field: FieldId) -> String;
    fn row(&self, relation: RelationId, row: RowIndex) -> String;
}

/// By id: `relation 2`, `field 1 of relation 2`, `row 0 of relation 2`.
struct Ids;

impl Names for Ids {
    fn relation(&self, relation: RelationId) -> String {
        format!("relation {}", relation.0)
    }

    fn field(&self, relation: RelationId, field: FieldId) -> String {
        format!("field {} of relation {}", field.0, relation.0)
    }

    fn row(&self, relation: RelationId, row: RowIndex) -> String {
        format!("row {row} of relation {}", relation.0)
    }
}

/// As declared: `` `Task` ``, `` `Task.parent` ``, `` row `Frozen` of `Status` ``.
/// A closed relation's field 0 is its handle `id`. An id the descriptor does
/// not declare falls back to [`Ids`].
struct Declared<'a>(&'a SchemaDescriptor);

impl Names for Declared<'_> {
    fn relation(&self, relation: RelationId) -> String {
        self.0
            .relations
            .get(relation.0 as usize)
            .map_or_else(|| Ids.relation(relation), |r| format!("`{}`", r.name))
    }

    fn field(&self, relation: RelationId, field: FieldId) -> String {
        let Some(declared) = self.0.relations.get(relation.0 as usize) else {
            return Ids.field(relation, field);
        };
        let index = usize::from(field.0);
        let name = match (&declared.extension, index) {
            (Some(_), 0) => Some("id"),
            (Some(_), index) => declared.fields.get(index - 1).map(|f| &*f.name),
            (None, index) => declared.fields.get(index).map(|f| &*f.name),
        };
        name.map_or_else(
            || Ids.field(relation, field),
            |name| format!("`{}.{name}`", declared.name),
        )
    }

    fn row(&self, relation: RelationId, row: RowIndex) -> String {
        self.0
            .relations
            .get(relation.0 as usize)
            .and_then(|r| Some((r, r.extension.as_ref()?.get(row.0)?)))
            .map_or_else(
                || Ids.row(relation, row),
                |(r, declared)| format!("row `{}` of `{}`", declared.handle, r.name),
            )
    }
}

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

fn render_error(f: &mut fmt::Formatter<'_>, error: &SchemaError, names: &dyn Names) -> fmt::Result {
    match error {
        SchemaError::TooManyRelations { count } => {
            write!(f, "{count} relations exceed the 65,536-relation id space")
        }
        SchemaError::DuplicateRelationName { name } => {
            write!(f, "the relation `{name}` is declared twice")
        }
        SchemaError::DuplicateFieldName { relation, name } => write!(
            f,
            "{} declares the field `{name}` twice",
            names.relation(*relation)
        ),
        SchemaError::FixedBytesWidthOutOfRange {
            relation,
            field,
            len,
        } => write!(
            f,
            "{} is bytes<{len}>: a bytes<N> width is 1..=64",
            names.field(*relation, *field)
        ),
        SchemaError::IntervalWidthOutOfRange {
            relation,
            field,
            width,
        } => write!(
            f,
            "{} has interval width {width}: a fixed interval width is at least 1 and below \
             u64::MAX",
            names.field(*relation, *field)
        ),
        SchemaError::RelationTooManyColumns { relation, columns } => write!(
            f,
            "{} spans {columns} columns, over the 65,535-column cap (an interval or uuid \
             field spans two, bytes<N> ⌈N/8⌉)",
            names.relation(*relation)
        ),
        SchemaError::TooManyStatements { count } => {
            write!(f, "{count} statements exceed the 65,536-statement id space")
        }
        SchemaError::EmptyExtension { relation } => write!(
            f,
            "the closed relation {} declares no rows",
            names.relation(*relation)
        ),
        SchemaError::ExtensionTooManyRows { relation, count } => write!(
            f,
            "the closed relation {} declares {count} rows, over the 256-row cap",
            names.relation(*relation)
        ),
        SchemaError::DuplicateExtensionHandle { relation, handle } => write!(
            f,
            "the closed relation {} declares the handle `{handle}` twice",
            names.relation(*relation)
        ),
        SchemaError::ExtensionArityMismatch {
            relation,
            row,
            mismatch,
        } => write!(
            f,
            "{} has {} values for {} columns",
            names.row(*relation, *row),
            mismatch.witnessed,
            mismatch.required
        ),
        SchemaError::ExtensionValueTypeMismatch {
            relation,
            row,
            field,
        } => write!(
            f,
            "{}: the value does not fit the type of {}",
            names.row(*relation, *row),
            names.field(*relation, *field)
        ),
        SchemaError::ExtensionIntervalRay {
            relation,
            row,
            field,
        } => write!(
            f,
            "{}: {} is unbounded; a closed relation's rows are bounded",
            names.row(*relation, *row),
            names.field(*relation, *field)
        ),
        SchemaError::StrOnClosedRelation { relation, field } => write!(
            f,
            "{} is str on a closed relation; its handles are the labels",
            names.field(*relation, *field)
        ),
        SchemaError::Statement { kind, .. } => render_kind(f, kind, names),
    }
}

#[expect(clippy::too_many_lines, reason = "one arm per variant")]
fn render_kind(
    f: &mut fmt::Formatter<'_>,
    kind: &StatementErrorKind,
    names: &dyn Names,
) -> fmt::Result {
    use StatementErrorKind as K;
    match kind {
        K::UnknownRelation { relation } => write!(f, "unknown {}", names.relation(*relation)),
        K::UnknownField { relation, field } => {
            write!(f, "{} has no field {}", names.relation(*relation), field.0)
        }
        K::EmptyProjection { relation } => {
            write!(f, "empty projection on {}", names.relation(*relation))
        }
        K::DuplicateProjectionField { relation, field } => {
            write!(f, "{} is projected twice", names.field(*relation, *field))
        }
        K::DuplicateSelectionField { relation, field } => {
            write!(f, "{} is selected twice", names.field(*relation, *field))
        }
        K::DegenerateSelectionSet {
            relation,
            field,
            len,
        } => write!(
            f,
            "the literal set for {} names {len}: a set selection names at least two \
             literals (one literal is an equality)",
            names.field(*relation, *field)
        ),
        K::DuplicateSelectionLiteral { relation, field } => write!(
            f,
            "the literal set for {} repeats a literal",
            names.field(*relation, *field)
        ),
        K::CapacityInvertedWindow { lo, hi } => write!(
            f,
            "the window `{{{lo}..{hi}}}` is inverted: no measure satisfies hi < lo"
        ),
        K::CapacityIntervalPosition { relation, field } => write!(
            f,
            "the interval {} is a capacity group key; an interval's measure enters \
             through `[Duration(field)]`",
            names.field(*relation, *field)
        ),
        K::CapacityWeightNotU64 { relation, field } => write!(
            f,
            "the weight field {} is not u64; a signed weight would let an insert lower a sum",
            names.field(*relation, *field)
        ),
        K::CapacityWeightNotDuration { relation, field } => write!(
            f,
            "`[Duration(field)]` reads a discrete interval, and {} is not one",
            names.field(*relation, *field)
        ),
        K::CapacityBoundNotU64 { relation, field } => write!(
            f,
            "the bound field {} is not u64; a dependent bound reads a u64 field of the \
             target row",
            names.field(*relation, *field)
        ),
        K::CapacityBoundNotDuration { relation, field } => write!(
            f,
            "`Duration(field)` bounds by a discrete interval, and {} is not one",
            names.field(*relation, *field)
        ),
        K::CapacityDimensionMixing { relation, field } => write!(
            f,
            "a count window bounded by the duration of {} mixes dimensions: weigh the \
             source with `[Duration(field)]`, or bound by a u64 field or literal",
            names.field(*relation, *field)
        ),
        K::FunctionalityMultipleIntervals { relation, field } => write!(
            f,
            "{} is a second interval in the key; a key has at most one",
            names.field(*relation, *field)
        ),
        K::FunctionalityIntervalNotLast { relation, field } => write!(
            f,
            "the interval {} must be the key's last position",
            names.field(*relation, *field)
        ),
        K::DuplicateFunctionality { earlier } => {
            write!(f, "statement {} already keys this field set", earlier.0)
        }
        K::ContainmentArityMismatch { mismatch } => write!(
            f,
            "{} source positions against {} target positions",
            mismatch.witnessed, mismatch.required
        ),
        K::ContainmentTypeMismatch { position } => write!(
            f,
            "the source and target types differ at position {position}"
        ),
        K::SelectedFieldProjected { relation, field } => write!(
            f,
            "{} is both selected and projected",
            names.field(*relation, *field)
        ),
        K::SelectionLiteralTypeMismatch { relation, field } => write!(
            f,
            "a selection literal does not fit the type of {}",
            names.field(*relation, *field)
        ),
        K::NoMatchingTargetKey {
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
        K::NoPointwiseTargetKey {
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
        K::ClosedContainmentInterval { relation } => write!(
            f,
            "an interval position on a containment with the closed relation {}",
            names.relation(*relation)
        ),
        K::ClosedTargetNotHandle {
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
        K::ClosedStatementRefuted { relation, row } => {
            write!(f, "refuted by {}", names.row(*relation, *row))
        }
        K::DuplicateStatement { earlier } => write!(f, "duplicates statement {}", earlier.0),
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::Statement { statement, .. } = self {
            write!(f, "statement {}: ", statement.0)?;
        }
        render_error(f, self, &Ids)
    }
}

impl fmt::Display for StatementErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        render_kind(f, self, &Ids)
    }
}

impl fmt::Display for Named<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        render_error(f, self.error, &Declared(self.descriptor))
    }
}
