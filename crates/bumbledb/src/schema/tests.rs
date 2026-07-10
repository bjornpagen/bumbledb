//! The schema subsystem's one fixture style (`pub(crate)`: the render and
//! fingerprint test modules import these helpers rather than growing their
//! own).

use super::*;

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

pub(crate) fn enum_type(variants: &[&str]) -> ValueType {
    ValueType::Enum {
        variants: variants.iter().map(|v| Box::from(*v)).collect(),
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
        selection: selection.into_boxed_slice(),
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

/// Holder(id fresh, name string) + Account(id fresh, holder u64, status enum),
/// with the statement `Account(holder) <= Holder(id)`.
fn ledger_slice() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![fresh_field("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    fresh_field("id"),
                    field("holder", ValueType::U64),
                    field("status", enum_type(&["Active", "Closed"])),
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
            name: "R".into(),
            fields,
        }],
        statements: vec![],
    }
}
