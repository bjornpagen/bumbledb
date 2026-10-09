//! Schema checking: every judgment over a declaration that needs no
//! storage. A descriptor that checks has a [`Checked`] form, with each
//! statement's target key resolved and every closed-relation statement
//! decided against the closed rows; the engine seals that form.
//!
//! Relations are checked first, in declaration order, then statements in
//! materialized order. The first failure is the error.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::error::{Mismatch, RowIndex, SchemaError, StatementErrorKind, TargetKeyCandidate};
use super::{
    Bound, FieldDescriptor, FieldId, LiteralSet, MAX_EXTENSION_ROWS, MAX_FIXED_BYTES,
    RelationDescriptor, RelationId, Row, SchemaDescriptor, Side, StatementDescriptor, StatementId,
    ValueMismatch, ValueType, Weight, value_matches,
};
use crate::value::Value;

/// The most relations a schema declares: relation ids fit a `u16` word.
pub const MAX_RELATIONS: usize = 1 << 16;

/// The most statements a schema materializes: statement ids are `u16`.
pub const MAX_STATEMENTS: usize = 1 << 16;

/// The most derived columns of one relation.
pub const MAX_COLUMNS: usize = u16::MAX as usize;

/// A checked schema: relations with their sealed fields and closed rows,
/// and statements in materialized order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    pub relations: Box<[CheckedRelation]>,
    pub statements: Box<[CheckedStatement]>,
}

/// A checked relation. A closed relation's fields lead with its handle
/// field `id: u64`, whose value is the row's declaration index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedRelation {
    pub name: Box<str>,
    pub fields: Box<[FieldDescriptor]>,
    pub rows: Option<Box<[CheckedRow]>>,
}

/// One closed-relation row: its handle and one value per sealed field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedRow {
    pub handle: Box<str>,
    pub values: Box<[Value]>,
}

/// One checked statement. Sides are canonical: set selections are sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedStatement {
    /// `R(X) -> R`; `tail` is the interval type of a pointwise key.
    Key {
        relation: RelationId,
        projection: Box<[FieldId]>,
        tail: Option<ValueType>,
    },
    /// `S(Y | φ) <= T(X | ψ)`; `mirror` is the `==` partner.
    Containment {
        source: Side,
        target: Side,
        resolution: Resolution,
        mirror: Option<StatementId>,
    },
    /// `T(X | ψ) <=[w]{lo..hi} S(Y | φ)`.
    Capacity {
        target: Side,
        weight: SealedWeight,
        lo: u64,
        hi: SealedBound,
        source: Side,
        resolution: CapacityResolution,
    },
}

/// How a containment's target is reached. `key_projection` lists the
/// source fields in the key's declared order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Probe the target key with the source's scalar values.
    ScalarProbe {
        key: StatementId,
        key_projection: Box<[FieldId]>,
    },
    /// The source interval must be covered by target intervals of one
    /// pointwise key group.
    IntervalCoverage {
        key: StatementId,
        key_projection: Box<[FieldId]>,
        source_tail: ValueType,
        target_tail: ValueType,
    },
    /// The target is closed: the handles its selection admits.
    Closed { members: MemberSet },
}

/// How a capacity's target groups are reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapacityResolution {
    ScalarProbe {
        key: StatementId,
        key_projection: Box<[FieldId]>,
    },
    Closed {
        members: MemberSet,
    },
}

#[cfg(test)]
mod tests;

/// A set of closed-row indexes (at most [`MAX_EXTENSION_ROWS`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemberSet {
    words: [u64; 4],
}

impl MemberSet {
    #[must_use]
    pub const fn contains(&self, index: u8) -> bool {
        self.words[(index / 64) as usize] & (1 << (index % 64)) != 0
    }

    pub const fn insert(&mut self, index: u8) {
        self.words[(index / 64) as usize] |= 1 << (index % 64);
    }
}

/// A capacity weight with its interval type resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SealedWeight {
    Unit,
    Field(FieldId),
    Duration { field: FieldId, tail: ValueType },
}

impl SealedWeight {
    #[must_use]
    pub const fn to_weight(self) -> Weight {
        match self {
            Self::Unit => Weight::Unit,
            Self::Field(field) => Weight::Field(field),
            Self::Duration { field, .. } => Weight::DurationOf(field),
        }
    }
}

/// A capacity ceiling with its interval type resolved; `*` is `Unbounded`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SealedBound {
    Unbounded,
    Lit(u64),
    TargetField(FieldId),
    Duration { field: FieldId, tail: ValueType },
}

impl SealedBound {
    #[must_use]
    pub const fn to_bound(self) -> Option<Bound> {
        match self {
            Self::Unbounded => None,
            Self::Lit(n) => Some(Bound::Lit(n)),
            Self::TargetField(field) => Some(Bound::TargetField(field)),
            Self::Duration { field, .. } => Some(Bound::TargetDuration(field)),
        }
    }
}

/// Check a declaration.
/// # Errors
/// The first [`SchemaError`] in check order.
pub fn check(descriptor: &SchemaDescriptor) -> Result<Checked, SchemaError> {
    if descriptor.relations.len() > MAX_RELATIONS {
        return Err(SchemaError::TooManyRelations {
            count: descriptor.relations.len(),
        });
    }
    for (index, relation) in descriptor.relations.iter().enumerate() {
        let columns = derived_columns(relation);
        if columns > MAX_COLUMNS {
            return Err(SchemaError::RelationTooManyColumns {
                relation: relation_id(index),
                columns,
            });
        }
    }
    let descriptors = descriptor.materialized_statements();
    if descriptors.len() > MAX_STATEMENTS {
        return Err(SchemaError::TooManyStatements {
            count: descriptors.len(),
        });
    }

    let relations = descriptor
        .relations
        .iter()
        .enumerate()
        .map(|(index, relation)| check_relation(relation_id(index), relation))
        .collect::<Result<Box<[_]>, _>>()?;
    for (index, relation) in relations.iter().enumerate() {
        if relations[..index].iter().any(|r| r.name == relation.name) {
            return Err(SchemaError::DuplicateRelationName {
                name: relation.name.clone(),
            });
        }
    }

    let identities: Vec<StatementIdentity> =
        descriptors.iter().map(StatementIdentity::of).collect();
    let mut statements = Vec::with_capacity(descriptors.len());
    for (index, statement) in descriptors.iter().enumerate() {
        let id = statement_id(index);
        let checked = match statement {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => CheckedStatement::Key {
                relation: *relation,
                projection: projection.clone(),
                tail: check_functionality(id, *relation, projection, &relations, &descriptors)?,
            },
            StatementDescriptor::Containment { source, target } => CheckedStatement::Containment {
                resolution: check_containment(id, source, target, &relations, &descriptors)?,
                source: canonical_side(source),
                target: canonical_side(target),
                mirror: mirror_of(&identities, index),
            },
            StatementDescriptor::Capacity {
                target,
                weight,
                lo,
                hi,
                source,
            } => {
                let capacity = check_capacity(
                    id,
                    target,
                    *weight,
                    *lo,
                    *hi,
                    source,
                    &relations,
                    &descriptors,
                )?;
                CheckedStatement::Capacity {
                    target: canonical_side(target),
                    weight: capacity.weight,
                    lo: *lo,
                    hi: capacity.hi,
                    source: canonical_side(source),
                    resolution: capacity.resolution,
                }
            }
        };
        if let Some(earlier) = identities[..index]
            .iter()
            .position(|identity| *identity == identities[index])
        {
            return Err(StatementErrorKind::DuplicateStatement {
                earlier: statement_id(earlier),
            }
            .at(id));
        }
        statements.push(checked);
    }
    Ok(Checked {
        relations,
        statements: statements.into_boxed_slice(),
    })
}

/// Each containment's `==` partner: the containment with the sides swapped.
#[must_use]
pub fn mirror_links(statements: &[StatementDescriptor]) -> BTreeMap<StatementId, StatementId> {
    let identities: Vec<StatementIdentity> = statements.iter().map(StatementIdentity::of).collect();
    (0..identities.len())
        .filter_map(|index| {
            mirror_of(&identities, index).map(|partner| (statement_id(index), partner))
        })
        .collect()
}

/// The total order of literals: by kind, then by value.
#[must_use]
pub fn literal_order(a: &Value, b: &Value) -> Ordering {
    fn rank(value: &Value) -> u8 {
        match value {
            Value::Bool(_) => 0,
            Value::U64(_) => 1,
            Value::I64(_) => 2,
            Value::String(_) => 3,
            Value::FixedBytes(_) => 4,
            Value::IntervalU64(_) => 5,
            Value::IntervalI64(_) => 6,
            Value::F64(_) => 7,
            Value::Uuid(_) => 8,
            Value::IntervalF64(_) => 9,
        }
    }
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::U64(x), Value::U64(y)) => x.cmp(y),
        (Value::I64(x), Value::I64(y)) => x.cmp(y),
        (Value::F64(x), Value::F64(y)) => x.cmp(y),
        (Value::Uuid(x), Value::Uuid(y)) => x.cmp(y),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::FixedBytes(x), Value::FixedBytes(y)) => x.cmp(y),
        (Value::IntervalU64(x), Value::IntervalU64(y)) => x.bounds().cmp(&y.bounds()),
        (Value::IntervalI64(x), Value::IntervalI64(y)) => x.bounds().cmp(&y.bounds()),
        (Value::IntervalF64(x), Value::IntervalF64(y)) => x.bounds().cmp(&y.bounds()),
        _ => rank(a).cmp(&rank(b)),
    }
}

/// A side with its set selections sorted.
#[must_use]
pub fn canonical_side(side: &Side) -> Side {
    Side {
        relation: side.relation,
        projection: side.projection.clone(),
        selection: side
            .selection
            .iter()
            .map(|(field, literals)| (*field, canonical_literals(literals)))
            .collect(),
    }
}

fn canonical_literals(literals: &LiteralSet) -> LiteralSet {
    match literals {
        LiteralSet::One(_) => literals.clone(),
        LiteralSet::Many(values) => {
            let mut sorted = values.to_vec();
            sorted.sort_by(literal_order);
            LiteralSet::Many(sorted.into_boxed_slice())
        }
    }
}

fn relation_id(index: usize) -> RelationId {
    RelationId(u32::try_from(index).expect("relation count is checked first"))
}

fn statement_id(index: usize) -> StatementId {
    StatementId(u16::try_from(index).expect("statement count is checked first"))
}

fn field_id(index: usize) -> FieldId {
    FieldId(u16::try_from(index).expect("column count is checked first"))
}

/// Derived columns: an interval or uuid field spans two, `bytes<N>` spans
/// `⌈N/8⌉` (at least one), a closed relation's handle one.
fn derived_columns(relation: &RelationDescriptor) -> usize {
    usize::from(relation.extension.is_some())
        + relation
            .fields
            .iter()
            .map(|field| match field.value_type {
                ValueType::Interval { .. } | ValueType::FixedInterval { .. } | ValueType::Uuid => 2,
                ValueType::FixedBytes { len } => usize::from(len).div_ceil(8).max(1),
                _ => 1,
            })
            .sum::<usize>()
}

fn check_relation(
    id: RelationId,
    relation: &RelationDescriptor,
) -> Result<CheckedRelation, SchemaError> {
    let closed = relation.extension.is_some();
    let mut fields = Vec::with_capacity(relation.fields.len() + usize::from(closed));
    if closed {
        fields.push(FieldDescriptor {
            name: "id".into(),
            value_type: ValueType::U64,
        });
    }
    fields.extend(relation.fields.iter().cloned());

    for (index, field) in fields.iter().enumerate() {
        if fields[..index].iter().any(|f| f.name == field.name) {
            return Err(SchemaError::DuplicateFieldName {
                relation: id,
                name: field.name.clone(),
            });
        }
        if let ValueType::FixedBytes { len } = field.value_type
            && (len == 0 || len > MAX_FIXED_BYTES)
        {
            return Err(SchemaError::FixedBytesWidthOutOfRange {
                relation: id,
                field: field_id(index),
                len,
            });
        }
        if let ValueType::FixedInterval { width, .. } = field.value_type
            && (width == 0 || width == u64::MAX)
        {
            return Err(SchemaError::IntervalWidthOutOfRange {
                relation: id,
                field: field_id(index),
                width,
            });
        }
        if closed && field.value_type == ValueType::String {
            return Err(SchemaError::StrOnClosedRelation {
                relation: id,
                field: field_id(index),
            });
        }
    }

    let rows = match &relation.extension {
        None => None,
        Some(rows) => Some(check_extension(id, &fields, rows)?),
    };
    Ok(CheckedRelation {
        name: relation.name.clone(),
        fields: fields.into_boxed_slice(),
        rows,
    })
}

fn check_extension(
    id: RelationId,
    fields: &[FieldDescriptor],
    rows: &[Row],
) -> Result<Box<[CheckedRow]>, SchemaError> {
    if rows.is_empty() {
        return Err(SchemaError::EmptyExtension { relation: id });
    }
    if rows.len() > MAX_EXTENSION_ROWS {
        return Err(SchemaError::ExtensionTooManyRows {
            relation: id,
            count: rows.len(),
        });
    }
    let columns = fields.len() - 1;
    let mut checked = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        if rows[..index].iter().any(|r| r.handle == row.handle) {
            return Err(SchemaError::DuplicateExtensionHandle {
                relation: id,
                handle: row.handle.clone(),
            });
        }
        if row.values.len() != columns {
            return Err(SchemaError::ExtensionArityMismatch {
                relation: id,
                row: RowIndex(index),
                mismatch: Mismatch {
                    witnessed: row.values.len(),
                    required: columns,
                },
            });
        }
        let mut values = Vec::with_capacity(fields.len());
        values.push(Value::U64(
            u64::try_from(index).expect("the row cap fits u64"),
        ));
        for (field, (value, descriptor)) in row.values.iter().zip(&fields[1..]).enumerate() {
            let field = field_id(field + 1);
            value_matches(value, &descriptor.value_type).map_err(|ValueMismatch::Type| {
                SchemaError::ExtensionValueTypeMismatch {
                    relation: id,
                    row: RowIndex(index),
                    field,
                }
            })?;
            let unbounded = match value {
                Value::IntervalU64(interval) => interval.is_ray(),
                Value::IntervalI64(interval) => interval.is_ray(),
                Value::IntervalF64(interval) => !interval.is_bounded(),
                _ => false,
            };
            if unbounded {
                return Err(SchemaError::ExtensionIntervalRay {
                    relation: id,
                    row: RowIndex(index),
                    field,
                });
            }
            values.push(value.clone());
        }
        checked.push(CheckedRow {
            handle: row.handle.clone(),
            values: values.into_boxed_slice(),
        });
    }
    Ok(checked.into_boxed_slice())
}

/// A projection's fields as a sorted, duplicate-free set.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldSet(Box<[FieldId]>);

impl FieldSet {
    fn new(fields: &[FieldId]) -> Result<Self, FieldId> {
        let mut sorted = fields.to_vec();
        sorted.sort_unstable();
        if let Some(duplicate) = sorted
            .windows(2)
            .find_map(|pair| (pair[0] == pair[1]).then_some(pair[0]))
        {
            return Err(duplicate);
        }
        Ok(Self(sorted.into_boxed_slice()))
    }
}

/// A side with selections sorted by field, for statement identity.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedSide {
    relation: RelationId,
    projection: Box<[FieldId]>,
    selection: Box<[(FieldId, LiteralSet)]>,
}

impl NormalizedSide {
    fn new(side: &Side) -> Self {
        let mut selection: Vec<_> = side
            .selection
            .iter()
            .map(|(field, literals)| (*field, canonical_literals(literals)))
            .collect();
        selection.sort_by_key(|(field, _)| *field);
        Self {
            relation: side.relation,
            projection: side.projection.clone(),
            selection: selection.into_boxed_slice(),
        }
    }
}

/// What makes two statements the same statement.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StatementIdentity {
    Functionality {
        relation: RelationId,
        projection: Box<[FieldId]>,
    },
    Containment {
        source: NormalizedSide,
        target: NormalizedSide,
    },
    Capacity {
        target: NormalizedSide,
        weight: Weight,
        lo: u64,
        hi: Option<Bound>,
        source: NormalizedSide,
    },
}

impl StatementIdentity {
    fn of(statement: &StatementDescriptor) -> Self {
        match statement {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => Self::Functionality {
                relation: *relation,
                projection: projection.clone(),
            },
            StatementDescriptor::Containment { source, target } => Self::Containment {
                source: NormalizedSide::new(source),
                target: NormalizedSide::new(target),
            },
            StatementDescriptor::Capacity {
                target,
                weight,
                lo,
                hi,
                source,
            } => Self::Capacity {
                target: NormalizedSide::new(target),
                weight: *weight,
                lo: *lo,
                hi: *hi,
                source: NormalizedSide::new(source),
            },
        }
    }
}

fn mirror_of(identities: &[StatementIdentity], index: usize) -> Option<StatementId> {
    let StatementIdentity::Containment { source, target } = &identities[index] else {
        return None;
    };
    identities
        .iter()
        .enumerate()
        .find(|(other, identity)| {
            *other != index
                && matches!(
                    identity,
                    StatementIdentity::Containment {
                        source: mirror_source,
                        target: mirror_target,
                    } if mirror_source == target && mirror_target == source
                )
        })
        .map(|(other, _)| statement_id(other))
}

/// Interval positions match by element domain; a fixed width does not
/// change coverage.
fn positional_types_match(a: ValueType, b: ValueType) -> bool {
    match (a.interval_element(), b.interval_element()) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

fn interval_positions(fields: &[FieldDescriptor], projection: &[FieldId]) -> Vec<usize> {
    projection
        .iter()
        .enumerate()
        .filter(|(_, field)| fields[usize::from(field.0)].value_type.is_interval())
        .map(|(position, _)| position)
        .collect()
}

fn interval_tail(fields: &[FieldDescriptor], projection: &[FieldId]) -> Option<ValueType> {
    projection.iter().find_map(|field| {
        let ty = fields[usize::from(field.0)].value_type;
        ty.is_interval().then_some(ty)
    })
}

fn known_relation(
    id: StatementId,
    relation: RelationId,
    relations: &[CheckedRelation],
) -> Result<&CheckedRelation, SchemaError> {
    relations
        .get(relation.0 as usize)
        .ok_or(StatementErrorKind::UnknownRelation { relation }.at(id))
}

fn known_field(
    id: StatementId,
    relation: RelationId,
    field: FieldId,
    relations: &[CheckedRelation],
) -> Result<&FieldDescriptor, SchemaError> {
    relations[relation.0 as usize]
        .fields
        .get(usize::from(field.0))
        .ok_or(StatementErrorKind::UnknownField { relation, field }.at(id))
}

fn check_projection(
    id: StatementId,
    relation_id: RelationId,
    projection: &[FieldId],
    relation: &CheckedRelation,
) -> Result<FieldSet, SchemaError> {
    if projection.is_empty() {
        return Err(StatementErrorKind::EmptyProjection {
            relation: relation_id,
        }
        .at(id));
    }
    if let Some(field) = projection
        .iter()
        .find(|field| usize::from(field.0) >= relation.fields.len())
    {
        return Err(StatementErrorKind::UnknownField {
            relation: relation_id,
            field: *field,
        }
        .at(id));
    }
    FieldSet::new(projection).map_err(|field| {
        StatementErrorKind::DuplicateProjectionField {
            relation: relation_id,
            field,
        }
        .at(id)
    })
}

/// Whether a closed row satisfies a side's selection.
fn selected(row: &CheckedRow, selection: &[(FieldId, LiteralSet)]) -> bool {
    selection.iter().all(|(field, literals)| {
        literals
            .literals()
            .contains(&row.values[usize::from(field.0)])
    })
}

/// A discrete interval's duration; `None` for a ray, which closed rows
/// never hold.
fn duration(value: &Value) -> Option<u64> {
    match value {
        Value::IntervalU64(interval) if !interval.is_ray() => {
            Some(interval.end() - interval.start())
        }
        Value::IntervalI64(interval) if !interval.is_ray() => {
            Some(interval.end().abs_diff(interval.start()))
        }
        _ => None,
    }
}

fn overlaps(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::IntervalU64(a), Value::IntervalU64(b)) => {
            a.start() < b.end() && b.start() < a.end()
        }
        (Value::IntervalI64(a), Value::IntervalI64(b)) => {
            a.start() < b.end() && b.start() < a.end()
        }
        (Value::IntervalF64(a), Value::IntervalF64(b)) => {
            a.start() < b.end() && b.start() < a.end()
        }
        _ => unreachable!("one key position holds one interval type"),
    }
}

/// A key: at most one interval field, and it comes last. Closed rows must
/// satisfy it. Returns the interval type of a pointwise key.
fn check_functionality(
    id: StatementId,
    relation_id: RelationId,
    projection: &[FieldId],
    relations: &[CheckedRelation],
    statements: &[StatementDescriptor],
) -> Result<Option<ValueType>, SchemaError> {
    let relation = known_relation(id, relation_id, relations)?;
    let set = check_projection(id, relation_id, projection, relation)?;

    let positions = interval_positions(&relation.fields, projection);
    if positions.len() > 1 {
        return Err(StatementErrorKind::FunctionalityMultipleIntervals {
            relation: relation_id,
            field: projection[positions[1]],
        }
        .at(id));
    }
    let interval = positions.first().copied();
    if let Some(position) = interval
        && position != projection.len() - 1
    {
        return Err(StatementErrorKind::FunctionalityIntervalNotLast {
            relation: relation_id,
            field: projection[position],
        }
        .at(id));
    }
    let tail =
        interval.map(|position| relation.fields[usize::from(projection[position].0)].value_type);

    for (index, earlier) in statements[..usize::from(id.0)].iter().enumerate() {
        if let StatementDescriptor::Functionality {
            relation: r,
            projection: p,
        } = earlier
            && *r == relation_id
            && FieldSet::new(p).is_ok_and(|other| other == set)
        {
            return Err(StatementErrorKind::DuplicateFunctionality {
                earlier: statement_id(index),
            }
            .at(id));
        }
    }

    if let Some(rows) = &relation.rows {
        let scalars = projection.len() - usize::from(interval.is_some());
        for (index, row) in rows.iter().enumerate() {
            for earlier in &rows[..index] {
                let same = projection[..scalars].iter().all(|field| {
                    row.values[usize::from(field.0)] == earlier.values[usize::from(field.0)]
                });
                let collide = same
                    && interval.is_none_or(|position| {
                        let field = usize::from(projection[position].0);
                        overlaps(&row.values[field], &earlier.values[field])
                    });
                if collide {
                    return Err(StatementErrorKind::ClosedStatementRefuted {
                        relation: relation_id,
                        row: RowIndex(index),
                    }
                    .at(id));
                }
            }
        }
    }
    Ok(tail)
}

fn check_side_shape(
    id: StatementId,
    side: &Side,
    relations: &[CheckedRelation],
) -> Result<FieldSet, SchemaError> {
    let relation = known_relation(id, side.relation, relations)?;
    let set = check_projection(id, side.relation, &side.projection, relation)?;
    for (index, (field, literals)) in side.selection.iter().enumerate() {
        if usize::from(field.0) >= relation.fields.len() {
            return Err(StatementErrorKind::UnknownField {
                relation: side.relation,
                field: *field,
            }
            .at(id));
        }
        if side.selection[..index].iter().any(|(f, _)| f == field) {
            return Err(StatementErrorKind::DuplicateSelectionField {
                relation: side.relation,
                field: *field,
            }
            .at(id));
        }
        if let LiteralSet::Many(values) = literals {
            if values.len() < 2 {
                return Err(StatementErrorKind::DegenerateSelectionSet {
                    relation: side.relation,
                    field: *field,
                    len: values.len(),
                }
                .at(id));
            }
            for (at, value) in values.iter().enumerate() {
                if values[..at]
                    .iter()
                    .any(|earlier| literal_order(earlier, value) == Ordering::Equal)
                {
                    return Err(StatementErrorKind::DuplicateSelectionLiteral {
                        relation: side.relation,
                        field: *field,
                    }
                    .at(id));
                }
            }
        }
    }
    Ok(set)
}

fn check_side_selection(
    id: StatementId,
    side: &Side,
    relations: &[CheckedRelation],
) -> Result<(), SchemaError> {
    let relation = &relations[side.relation.0 as usize];
    if let Some((field, _)) = side
        .selection
        .iter()
        .find(|(field, _)| side.projection.contains(field))
    {
        return Err(StatementErrorKind::SelectedFieldProjected {
            relation: side.relation,
            field: *field,
        }
        .at(id));
    }
    for (field, literals) in &side.selection {
        let value_type = &relation.fields[usize::from(field.0)].value_type;
        for literal in literals.literals() {
            value_matches(literal, value_type).map_err(|ValueMismatch::Type| {
                StatementErrorKind::SelectionLiteralTypeMismatch {
                    relation: side.relation,
                    field: *field,
                }
                .at(id)
            })?;
        }
    }
    Ok(())
}

/// The checks a containment and a capacity share. Returns the target
/// projection's field set.
fn check_side_pair(
    id: StatementId,
    source: &Side,
    target: &Side,
    relations: &[CheckedRelation],
) -> Result<FieldSet, SchemaError> {
    check_side_shape(id, source, relations)?;
    let target_set = check_side_shape(id, target, relations)?;
    if source.projection.len() != target.projection.len() {
        return Err(StatementErrorKind::ContainmentArityMismatch {
            mismatch: Mismatch {
                witnessed: source.projection.len(),
                required: target.projection.len(),
            },
        }
        .at(id));
    }
    let source_fields = &relations[source.relation.0 as usize].fields;
    let target_fields = &relations[target.relation.0 as usize].fields;
    if let Some(position) = source
        .projection
        .iter()
        .zip(&target.projection)
        .position(|(s, t)| {
            !positional_types_match(
                source_fields[usize::from(s.0)].value_type,
                target_fields[usize::from(t.0)].value_type,
            )
        })
    {
        return Err(StatementErrorKind::ContainmentTypeMismatch { position }.at(id));
    }
    check_side_selection(id, source, relations)?;
    check_side_selection(id, target, relations)?;
    Ok(target_set)
}

fn check_containment(
    id: StatementId,
    source: &Side,
    target: &Side,
    relations: &[CheckedRelation],
    statements: &[StatementDescriptor],
) -> Result<Resolution, SchemaError> {
    let target_set = check_side_pair(id, source, target, relations)?;
    let source_relation = &relations[source.relation.0 as usize];
    let target_relation = &relations[target.relation.0 as usize];
    let source_closed = source_relation.rows.is_some();
    let target_closed = target_relation.rows.is_some();
    if (source_closed || target_closed)
        && !interval_positions(&target_relation.fields, &target.projection).is_empty()
    {
        return Err(StatementErrorKind::ClosedContainmentInterval {
            relation: if target_closed {
                target.relation
            } else {
                source.relation
            },
        }
        .at(id));
    }

    let resolution = if let Some(rows) = &target_relation.rows {
        Resolution::Closed {
            members: closed_members(id, target, target_relation, rows)?,
        }
    } else {
        resolve_key(
            id,
            source,
            target,
            &target_set,
            relations,
            statements,
            interval_tail(&source_relation.fields, &source.projection),
        )?
    };

    if let (Resolution::Closed { members }, Some(rows)) = (&resolution, &source_relation.rows) {
        let field = usize::from(source.projection[0].0);
        for (index, row) in rows.iter().enumerate() {
            if !selected(row, &source.selection) {
                continue;
            }
            let Value::U64(handle) = row.values[field] else {
                unreachable!("a closed target's id pairs with a u64 source position")
            };
            if !u8::try_from(handle).is_ok_and(|handle| members.contains(handle)) {
                return Err(StatementErrorKind::ClosedStatementRefuted {
                    relation: source.relation,
                    row: RowIndex(index),
                }
                .at(id));
            }
        }
    }
    Ok(resolution)
}

/// A closed target is addressed by its handle `id` alone; its members are
/// the rows its selection admits.
fn closed_members(
    id: StatementId,
    target: &Side,
    relation: &CheckedRelation,
    rows: &[CheckedRow],
) -> Result<MemberSet, SchemaError> {
    if target.projection.len() != 1 || target.projection[0] != FieldId(0) {
        return Err(StatementErrorKind::ClosedTargetNotHandle {
            target: target.relation,
            target_name: relation.name.clone(),
            projection: target.projection.clone(),
            projection_names: field_names(relation, &target.projection),
        }
        .at(id));
    }
    let mut members = MemberSet::default();
    for (index, row) in rows.iter().enumerate() {
        if selected(row, &target.selection) {
            members.insert(u8::try_from(index).expect("the row cap is 256"));
        }
    }
    Ok(members)
}

fn resolve_key(
    id: StatementId,
    source: &Side,
    target: &Side,
    target_set: &FieldSet,
    relations: &[CheckedRelation],
    statements: &[StatementDescriptor],
    source_tail: Option<ValueType>,
) -> Result<Resolution, SchemaError> {
    let target_fields = &relations[target.relation.0 as usize].fields;
    let positions = interval_positions(target_fields, &target.projection);
    if positions.len() > 1 {
        return Err(missing_key(id, target, relations, statements, true));
    }
    let pointwise = !positions.is_empty();
    let Some((key_index, key_projection)) = matching_key(target.relation, target_set, statements)
    else {
        return Err(missing_key(id, target, relations, statements, pointwise));
    };
    let key = statement_id(key_index);
    let key_projection_in_source =
        source_order(&source.projection, &target.projection, key_projection);
    if !pointwise {
        return Ok(Resolution::ScalarProbe {
            key,
            key_projection: key_projection_in_source,
        });
    }
    let Some(target_tail) =
        check_functionality(key, target.relation, key_projection, relations, statements)?
    else {
        unreachable!("a key with an interval projection is pointwise")
    };
    let Some(source_tail) = source_tail else {
        unreachable!("positional types match: an interval target pairs with an interval source")
    };
    Ok(Resolution::IntervalCoverage {
        key,
        key_projection: key_projection_in_source,
        source_tail,
        target_tail,
    })
}

fn matching_key<'a>(
    relation: RelationId,
    want: &FieldSet,
    statements: &'a [StatementDescriptor],
) -> Option<(usize, &'a [FieldId])> {
    statements
        .iter()
        .enumerate()
        .find_map(|(index, statement)| match statement {
            StatementDescriptor::Functionality {
                relation: r,
                projection,
            } if *r == relation && FieldSet::new(projection).is_ok_and(|set| &set == want) => {
                Some((index, projection.as_ref()))
            }
            _ => None,
        })
}

/// The source fields paired with each key field, in key order.
fn source_order(
    source_projection: &[FieldId],
    target_projection: &[FieldId],
    key_projection: &[FieldId],
) -> Box<[FieldId]> {
    key_projection
        .iter()
        .map(|key_field| {
            let position = target_projection
                .iter()
                .position(|field| field == key_field)
                .expect("a set-equal projection holds every key field");
            source_projection[position]
        })
        .collect()
}

fn field_names(relation: &CheckedRelation, projection: &[FieldId]) -> Box<[Box<str>]> {
    projection
        .iter()
        .map(|field| relation.fields[usize::from(field.0)].name.clone())
        .collect()
}

fn missing_key(
    id: StatementId,
    side: &Side,
    relations: &[CheckedRelation],
    statements: &[StatementDescriptor],
    pointwise: bool,
) -> SchemaError {
    let relation = &relations[side.relation.0 as usize];
    let available = statements
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| match statement {
            StatementDescriptor::Functionality {
                relation: r,
                projection,
            } if *r == side.relation => Some(TargetKeyCandidate {
                key: statement_id(index),
                projection: projection.clone(),
                projection_names: field_names(relation, projection),
            }),
            _ => None,
        })
        .collect();
    let target = side.relation;
    let target_name = relation.name.clone();
    let projection = side.projection.clone();
    let projection_names = field_names(relation, &projection);
    if pointwise {
        StatementErrorKind::NoPointwiseTargetKey {
            target,
            target_name,
            projection,
            projection_names,
            available,
        }
        .at(id)
    } else {
        StatementErrorKind::NoMatchingTargetKey {
            target,
            target_name,
            projection,
            projection_names,
            available,
        }
        .at(id)
    }
}

struct CheckedCapacity {
    weight: SealedWeight,
    hi: SealedBound,
    resolution: CapacityResolution,
}

/// A capacity: a literal window that is not inverted, scalar group keys,
/// an unsigned or duration weight, an unsigned or duration bound of the
/// same dimension, and a target key. Closed sources and targets are
/// decided here.
fn check_capacity(
    id: StatementId,
    target: &Side,
    weight: Weight,
    lo: u64,
    hi: Option<Bound>,
    source: &Side,
    relations: &[CheckedRelation],
    statements: &[StatementDescriptor],
) -> Result<CheckedCapacity, SchemaError> {
    if let Some(Bound::Lit(hi)) = hi
        && hi < lo
    {
        return Err(StatementErrorKind::CapacityInvertedWindow { lo, hi }.at(id));
    }
    let target_set = check_side_pair(id, source, target, relations)?;
    let source_relation = &relations[source.relation.0 as usize];
    if let Some(position) = interval_positions(&source_relation.fields, &source.projection).first()
    {
        return Err(StatementErrorKind::CapacityIntervalPosition {
            relation: source.relation,
            field: source.projection[*position],
        }
        .at(id));
    }

    let weight = match weight {
        Weight::Unit => SealedWeight::Unit,
        Weight::Field(field) => {
            if known_field(id, source.relation, field, relations)?.value_type != ValueType::U64 {
                return Err(StatementErrorKind::CapacityWeightNotU64 {
                    relation: source.relation,
                    field,
                }
                .at(id));
            }
            SealedWeight::Field(field)
        }
        Weight::DurationOf(field) => {
            let tail = known_field(id, source.relation, field, relations)?.value_type;
            if !tail.is_discrete_interval() {
                return Err(StatementErrorKind::CapacityWeightNotDuration {
                    relation: source.relation,
                    field,
                }
                .at(id));
            }
            SealedWeight::Duration { field, tail }
        }
    };

    let hi = match hi {
        None => SealedBound::Unbounded,
        Some(Bound::Lit(n)) => SealedBound::Lit(n),
        Some(Bound::TargetField(field)) => {
            if known_field(id, target.relation, field, relations)?.value_type != ValueType::U64 {
                return Err(StatementErrorKind::CapacityBoundNotU64 {
                    relation: target.relation,
                    field,
                }
                .at(id));
            }
            SealedBound::TargetField(field)
        }
        Some(Bound::TargetDuration(field)) => {
            let tail = known_field(id, target.relation, field, relations)?.value_type;
            if !tail.is_discrete_interval() {
                return Err(StatementErrorKind::CapacityBoundNotDuration {
                    relation: target.relation,
                    field,
                }
                .at(id));
            }
            if weight == SealedWeight::Unit {
                return Err(StatementErrorKind::CapacityDimensionMixing { field }.at(id));
            }
            SealedBound::Duration { field, tail }
        }
    };

    let target_relation = &relations[target.relation.0 as usize];
    let resolution = if let Some(rows) = &target_relation.rows {
        CapacityResolution::Closed {
            members: closed_members(id, target, target_relation, rows)?,
        }
    } else {
        let Some((key_index, key_projection)) =
            matching_key(target.relation, &target_set, statements)
        else {
            return Err(missing_key(id, target, relations, statements, false));
        };
        CapacityResolution::ScalarProbe {
            key: statement_id(key_index),
            key_projection: source_order(&source.projection, &target.projection, key_projection),
        }
    };

    if let (Some(parents), Some(children)) = (&target_relation.rows, &source_relation.rows) {
        for (index, parent) in parents.iter().enumerate() {
            if !selected(parent, &target.selection) {
                continue;
            }
            let ceiling = match hi {
                SealedBound::Unbounded => None,
                SealedBound::Lit(n) => Some(n),
                SealedBound::TargetField(field) => match parent.values[usize::from(field.0)] {
                    Value::U64(n) => Some(n),
                    _ => unreachable!("the bound field is u64"),
                },
                SealedBound::Duration { field, .. } => Some(
                    duration(&parent.values[usize::from(field.0)])
                        .expect("closed rows hold bounded intervals"),
                ),
            };
            let measure: u128 = children
                .iter()
                .filter(|child| {
                    selected(child, &source.selection)
                        && source
                            .projection
                            .iter()
                            .zip(&target.projection)
                            .all(|(s, t)| {
                                child.values[usize::from(s.0)] == parent.values[usize::from(t.0)]
                            })
                })
                .map(|child| {
                    u128::from(match weight {
                        SealedWeight::Unit => 1,
                        SealedWeight::Field(field) => match child.values[usize::from(field.0)] {
                            Value::U64(n) => n,
                            _ => unreachable!("the weight field is u64"),
                        },
                        SealedWeight::Duration { field, .. } => {
                            duration(&child.values[usize::from(field.0)])
                                .expect("closed rows hold bounded intervals")
                        }
                    })
                })
                .sum();
            if measure < u128::from(lo) || ceiling.is_some_and(|hi| measure > u128::from(hi)) {
                return Err(StatementErrorKind::ClosedStatementRefuted {
                    relation: target.relation,
                    row: RowIndex(index),
                }
                .at(id));
            }
        }
    }

    Ok(CheckedCapacity {
        weight,
        hi,
        resolution,
    })
}
