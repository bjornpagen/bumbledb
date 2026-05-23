fn query_image_scope_for_query(schema: &StorageSchema, query: &NormalizedQuery) -> QueryImageScope {
    QueryImageScope::relations_all(schema, query.atoms.iter().map(|atom| atom.relation))
}
