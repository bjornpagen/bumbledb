use super::*;

pub(super) fn query_image_scope_for_query(
    schema: &StorageSchema,
    query: &NormalizedQuery,
) -> QueryImageScope {
    let mut scopes =
        BTreeMap::<crate::RelationId, (BTreeSet<FieldId>, BTreeSet<crate::AccessId>)>::new();

    for atom in &query.atoms {
        let entry = scopes.entry(atom.relation).or_default();
        let required_fields = atom
            .fields
            .iter()
            .filter(|field| !matches!(field.term, NormTerm::Wildcard))
            .map(|field| field.field)
            .collect::<BTreeSet<_>>();
        entry.0.extend(required_fields.iter().copied());

        let relation = schema.descriptor().relations.get(atom.relation.0 as usize);
        let Ok(paths) = schema.access_paths(&atom.relation_name) else {
            continue;
        };
        for path in paths {
            let Some(layout) = schema.layout(&atom.relation_name, &path.index_name) else {
                continue;
            };
            let path_fields = relation
                .map(|relation| {
                    path.components
                        .iter()
                        .filter_map(|component| {
                            relation
                                .fields
                                .iter()
                                .position(|field| field.name == component.field_name)
                                .map(|field| FieldId(field as u16))
                        })
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            let include_access = path.kind == IndexKind::FactSet
                || (!required_fields.is_empty() && required_fields.is_subset(&path_fields));
            if include_access {
                entry.1.insert(crate::AccessId(layout.index_id));
            }
        }
    }

    QueryImageScope::relations_scoped(schema, scopes)
}
