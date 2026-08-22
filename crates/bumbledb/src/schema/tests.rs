use super::*;

mod member_set;
mod obligations;
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

pub(crate) fn row(handle: &str, values: Vec<Value>) -> Row {
    Row {
        handle: handle.into(),
        values: values.into_boxed_slice(),
    }
}

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

pub(crate) fn side(relation: RelationId, projection: &[FieldId]) -> Side {
    Side {
        relation,
        projection: projection.into(),
        selection: Box::new([]),
    }
}

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

pub(crate) fn fd(relation: RelationId, projection: &[FieldId]) -> StatementDescriptor {
    StatementDescriptor::Functionality {
        relation,
        projection: projection.into(),
    }
}

pub(crate) fn containment(source: Side, target: Side) -> StatementDescriptor {
    StatementDescriptor::Containment { source, target }
}

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

pub(crate) fn capacity(
    source: Side,
    lo: u64,
    hi: Option<u64>,
    target: Side,
) -> StatementDescriptor {
    StatementDescriptor::Capacity {
        target,
        weight: Weight::Unit,
        lo,
        hi: hi.map(Bound::Lit),
        source,
    }
}

pub(crate) fn capacity_weighted(
    target: Side,
    weight: Weight,
    lo: u64,
    hi: Option<Bound>,
    source: Side,
) -> StatementDescriptor {
    StatementDescriptor::Capacity {
        target,
        weight,
        lo,
        hi,
        source,
    }
}

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
