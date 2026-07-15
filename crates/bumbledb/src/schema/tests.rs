//! The schema subsystem's one fixture style (`pub(crate)`: the render and
//! fingerprint test modules import these helpers rather than growing their
//! own).

use super::*;

mod member_set;
mod reject;
mod valid;

pub(crate) fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

pub(crate) fn fresh_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    }
}

/// One ground axiom: `handle(values...)`.
pub(crate) fn row(handle: &str, values: Vec<Value>) -> Row {
    Row {
        handle: handle.into(),
        values: values.into_boxed_slice(),
    }
}

/// A closed relation: declared intrinsic columns plus its extension.
pub(crate) fn closed(
    name: &str,
    fields: Vec<FieldDescriptor>,
    rows: Vec<Row>,
) -> RelationDescriptor {
    RelationDescriptor {
        name: name.into(),
        fields,
        extension: Some(rows.into_boxed_slice()),
    }
}

/// An unselected side: `R(X)`.
pub(crate) fn side(relation: RelationId, projection: &[FieldId]) -> Side {
    Side {
        relation,
        projection: projection.into(),
        selection: Box::new([]),
    }
}

/// A selected side: `R(X | σ)`.
pub(crate) fn side_where(
    relation: RelationId,
    projection: &[FieldId],
    selection: Vec<(FieldId, Value)>,
) -> Side {
    Side {
        relation,
        projection: projection.into(),
        selection: selection
            .into_iter()
            .map(|(field, literal)| (field, LiteralSet::One(literal)))
            .collect(),
    }
}

/// `R(X) -> R`.
pub(crate) fn fd(relation: RelationId, projection: &[FieldId]) -> StatementDescriptor {
    StatementDescriptor::Functionality {
        relation,
        projection: projection.into(),
    }
}

/// `source <= target`.
pub(crate) fn containment(source: Side, target: Side) -> StatementDescriptor {
    StatementDescriptor::Containment { source, target }
}

/// A side whose σ carries whole literal sets — the disjunctive form.
pub(crate) fn side_where_sets(
    relation: RelationId,
    projection: &[FieldId],
    selection: Vec<(FieldId, LiteralSet)>,
) -> Side {
    Side {
        relation,
        projection: projection.into(),
        selection: selection.into_boxed_slice(),
    }
}

/// `source in lo..hi per target`.
pub(crate) fn cardinality(
    source: Side,
    lo: u64,
    hi: Option<u64>,
    target: Side,
) -> StatementDescriptor {
    StatementDescriptor::Cardinality {
        source,
        lo,
        hi,
        target,
    }
}

/// `order relation(position) per relation(grouping) [by ranking]`.
pub(crate) fn order_mark(
    relation: RelationId,
    position: FieldId,
    grouping: &[FieldId],
    ranking: Option<RankChain>,
) -> StatementDescriptor {
    StatementDescriptor::Order {
        relation,
        position,
        grouping: grouping.into(),
        ranking,
    }
}

/// Holder(id fresh, name string) + Account(id fresh, holder u64, status u64),
/// with the statement `Account(holder) <= Holder(id)`.
fn ledger_slice() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![fresh_field("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![
                    fresh_field("id"),
                    field("holder", ValueType::U64),
                    field("status", ValueType::U64),
                ],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: side(RelationId(1), &[FieldId(1)]),
            target: side(RelationId(0), &[FieldId(0)]),
        }],
    }
}

fn one_relation(fields: Vec<FieldDescriptor>) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields,
        }],
        statements: vec![],
    }
}
