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

/// Account(id serial, holder u64 -> Holder.id, status enum) + Holder(id serial, name string).
fn ledger_slice() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![serial_field("id"), field("name", ValueType::String)],
                constraints: vec![],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    serial_field("id"),
                    field("holder", ValueType::U64),
                    field("status", enum_type(&["Active", "Closed"])),
                ],
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "account_holder".into(),
                    fields: Box::new([FieldId(1)]),
                    target_relation: RelationId(0),
                    // Holder's auto-unique on its serial `id` field.
                    target_constraint: ConstraintId(0),
                }],
            },
        ],
    }
}

fn one_relation(
    fields: Vec<FieldDescriptor>,
    constraints: Vec<ConstraintDescriptor>,
) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "R".into(),
            fields,
            constraints,
        }],
    }
}
