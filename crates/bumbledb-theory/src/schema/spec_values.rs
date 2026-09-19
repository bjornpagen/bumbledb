//! The schema grammar is shared by owned input and admitted values. Mapping
//! literal payloads cannot invent a descriptor, field, projection or statement.
use super::spec::{
    ClosedSpecData as ClosedSpec, LiteralSetSpecData as LiteralSetSpec,
    LiteralSpecData as LiteralSpec, RelationSpecData as RelationSpec, RowSpecData as RowSpec,
    SchemaSpecData as SchemaSpec, SideSpecData as SideSpec, StatementSpecData as StatementSpec,
};

impl<V> SchemaSpec<V> {
    /// Transform every literal payload in declaration order, stopping on the
    /// first failure. Names, handles, field types and law structure are retained.
    /// # Errors
    /// The literal interpreter's error, unchanged.
    pub fn try_map_values<U, E>(
        self,
        mut map: impl FnMut(V) -> Result<U, E>,
    ) -> Result<SchemaSpec<U>, E> {
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
                        StatementSpec::Fd {
                            relation,
                            projection,
                        } => StatementSpec::Fd {
                            relation,
                            projection,
                        },
                        StatementSpec::Containment {
                            source,
                            target,
                            bidirectional,
                        } => StatementSpec::Containment {
                            source: side(source, &mut map)?,
                            target: side(target, &mut map)?,
                            bidirectional,
                        },
                        StatementSpec::Capacity {
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

fn literal<V, U, E>(
    literal: LiteralSpec<V>,
    map: &mut impl FnMut(V) -> Result<U, E>,
) -> Result<LiteralSpec<U>, E> {
    Ok(match literal {
        LiteralSpec::Value(value) => LiteralSpec::Value(map(value)?),
        LiteralSpec::Handle(handle) => LiteralSpec::Handle(handle),
    })
}
fn side<V, U, E>(
    side: SideSpec<V>,
    map: &mut impl FnMut(V) -> Result<U, E>,
) -> Result<SideSpec<U>, E> {
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
                        LiteralSetSpec::One(value) => LiteralSetSpec::One(literal(value, map)?),
                        LiteralSetSpec::Many(values) => LiteralSetSpec::Many(
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
