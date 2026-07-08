use super::*;

mod reject;
mod valid;

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn serial_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Serial,
    }
}

fn enum_type(variants: &[&str]) -> ValueType {
    ValueType::Enum {
        variants: variants.iter().map(|v| Box::from(*v)).collect(),
    }
}

/// An unselected side: `R(X)`.
fn side(relation: RelationId, projection: &[FieldId]) -> Side {
    Side {
        relation,
        projection: projection.into(),
        selection: Box::new([]),
    }
}

/// Holder(id serial, name string) + Account(id serial, holder u64, status enum),
/// with the statement `Account(holder) <= Holder(id)`.
fn ledger_slice() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![serial_field("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    serial_field("id"),
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
