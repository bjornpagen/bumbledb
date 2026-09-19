//! The schema grammar is shared by owned input and admitted values. Mapping
//! literal payloads cannot invent a descriptor, field, projection or statement.
use super::spec::{
    ClosedSpec, LiteralSetSpec, LiteralSetSpecData as LiteralSetInput, LiteralSpec,
    LiteralSpecData as LiteralInput, RelationSpec, RowSpec, SchemaSpec,
    SchemaSpecData as SchemaInput, SideSpec, SideSpecData as SideInput, StatementSpec,
    StatementSpecData as StatementInput,
};
use crate::Value;

impl<V> SchemaInput<V> {
    /// Interpret literal payloads into the concrete authoring grammar in declaration
    /// order, stopping on the first failure. Names, handles, field types and law
    /// structure are retained.
    /// # Errors
    /// The literal interpreter's error, unchanged.
    pub fn try_into_spec<E>(
        self,
        mut map: impl FnMut(V) -> Result<Value, E>,
    ) -> Result<SchemaSpec, E> {
        Ok(SchemaSpec {
            relations: self
                .relations
                .into_iter()
                .map(|relation| {
                    Ok(RelationSpec {
                        name: relation.name,
                        fields: relation.fields,
                        closed: relation
                            .closed
                            .map(|closed| {
                                Ok(ClosedSpec {
                                    newtype: closed.newtype,
                                    rows: closed
                                        .rows
                                        .into_iter()
                                        .map(|row| {
                                            Ok(RowSpec {
                                                handle: row.handle,
                                                values: row
                                                    .values
                                                    .into_iter()
                                                    .map(|value| literal(value, &mut map))
                                                    .collect::<Result<_, E>>()?,
                                            })
                                        })
                                        .collect::<Result<_, E>>()?,
                                })
                            })
                            .transpose()?,
                    })
                })
                .collect::<Result<_, E>>()?,
            statements: self
                .statements
                .into_iter()
                .map(|statement| {
                    Ok(match statement {
                        StatementInput::Fd {
                            relation,
                            projection,
                        } => StatementSpec::Fd {
                            relation,
                            projection,
                        },
                        StatementInput::Containment {
                            source,
                            target,
                            bidirectional,
                        } => StatementSpec::Containment {
                            source: side(source, &mut map)?,
                            target: side(target, &mut map)?,
                            bidirectional,
                        },
                        StatementInput::Capacity {
                            target,
                            weight,
                            window,
                            source,
                        } => StatementSpec::Capacity {
                            target: side(target, &mut map)?,
                            weight,
                            window,
                            source: side(source, &mut map)?,
                        },
                    })
                })
                .collect::<Result<_, E>>()?,
        })
    }
}

fn literal<V, E>(
    literal: LiteralInput<V>,
    map: &mut impl FnMut(V) -> Result<Value, E>,
) -> Result<LiteralSpec, E> {
    Ok(match literal {
        LiteralInput::Value(value) => LiteralSpec::Value(map(value)?),
        LiteralInput::Handle(handle) => LiteralSpec::Handle(handle),
    })
}
fn side<V, E>(
    side: SideInput<V>,
    map: &mut impl FnMut(V) -> Result<Value, E>,
) -> Result<SideSpec, E> {
    Ok(SideSpec {
        relation: side.relation,
        projection: side.projection,
        selection: side
            .selection
            .into_iter()
            .map(|(field, set)| {
                Ok((
                    field,
                    match set {
                        LiteralSetInput::One(value) => LiteralSetSpec::One(literal(value, map)?),
                        LiteralSetInput::Many(values) => LiteralSetSpec::Many(
                            values
                                .into_iter()
                                .map(|value| literal(value, map))
                                .collect::<Result<_, E>>()?,
                        ),
                    },
                ))
            })
            .collect::<Result<_, E>>()?,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn admitted_empty_schemas_and_literal_free_statements_need_no_type_annotation() {
        use super::super::spec::{SchemaSpec, SideSpec, StatementSpec};
        let schema = SchemaSpec {
            relations: vec![],
            statements: vec![],
        };
        let statement = StatementSpec::Fd {
            relation: "R".into(),
            projection: vec!["id".into()].into(),
        };
        let side = SideSpec {
            relation: "R".into(),
            projection: vec!["id".into()].into(),
            selection: vec![],
        };
        assert!(schema.relations.is_empty());
        assert!(matches!(statement, StatementSpec::Fd { .. }));
        assert!(side.selection.is_empty());
    }
}
