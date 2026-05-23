use std::collections::{BTreeMap, BTreeSet};

use crate::query_image::{FieldId, RelationId};
use crate::{AccessId, StorageSchema};

/// Cache key for an immutable query image.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QueryImageKey {
    /// Schema fingerprint for the image.
    pub schema: bumbledb_core::schema::SchemaFingerprint,
    /// Last committed storage transaction ID visible to the image.
    pub tx_id: u64,
    /// Relation/index/column scope loaded into this image.
    pub scope: QueryImageScopeKey,
}

/// Stable cache key for a query-image scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryImageScopeKey(pub [u8; 32]);

/// Explicit relation scope for a query image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryImageScope {
    relations: BTreeMap<RelationId, RelationScope>,
}

/// Explicit field/index scope for one relation image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RelationScope {
    pub(super) columns: BTreeSet<FieldId>,
    pub(super) indexes: BTreeSet<AccessId>,
    pub(super) include_all_columns: bool,
    pub(super) include_all_indexes: bool,
}

impl PartialOrd for QueryImageKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueryImageKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.schema.0, self.tx_id, self.scope).cmp(&(other.schema.0, other.tx_id, other.scope))
    }
}

impl QueryImageScope {
    /// Full-schema image scope.
    #[cfg(test)]
    pub fn full(schema: &StorageSchema) -> Self {
        let relations = schema
            .descriptor()
            .relations
            .iter()
            .enumerate()
            .map(|(id, relation)| {
                let relation_id = RelationId(id as u16);
                let indexes = schema
                    .layouts_for_relation(relation_id.0)
                    .map(|layout| AccessId(layout.index_id))
                    .collect();
                (
                    relation_id,
                    RelationScope {
                        columns: (0..relation.fields.len())
                            .map(|field| FieldId(field as u16))
                            .collect(),
                        indexes,
                        include_all_columns: true,
                        include_all_indexes: true,
                    },
                )
            })
            .collect();
        Self { relations }
    }

    /// Scope containing all fields and indexes for selected relations.
    #[cfg(test)]
    pub fn relations_all(
        schema: &StorageSchema,
        relation_ids: impl IntoIterator<Item = RelationId>,
    ) -> Self {
        let mut relations = BTreeMap::new();
        for relation_id in relation_ids {
            let Some(relation) = schema.descriptor().relations.get(relation_id.0 as usize) else {
                continue;
            };
            let indexes = schema
                .layouts_for_relation(relation_id.0)
                .map(|layout| AccessId(layout.index_id))
                .collect();
            relations.insert(
                relation_id,
                RelationScope {
                    columns: (0..relation.fields.len())
                        .map(|field| FieldId(field as u16))
                        .collect(),
                    indexes,
                    include_all_columns: true,
                    include_all_indexes: true,
                },
            );
        }
        Self { relations }
    }

    pub(crate) fn relations_scoped(
        schema: &StorageSchema,
        scopes: BTreeMap<RelationId, (BTreeSet<FieldId>, BTreeSet<AccessId>)>,
    ) -> Self {
        let mut relations = BTreeMap::new();
        for (relation_id, (columns, indexes)) in scopes {
            let Some(relation) = schema.descriptor().relations.get(relation_id.0 as usize) else {
                continue;
            };
            let columns = columns
                .into_iter()
                .filter(|field| (field.0 as usize) < relation.fields.len())
                .collect();
            let valid_indexes = schema
                .layouts_for_relation(relation_id.0)
                .map(|layout| AccessId(layout.index_id))
                .collect::<BTreeSet<_>>();
            let indexes = indexes
                .into_iter()
                .filter(|index| valid_indexes.contains(index))
                .collect();
            relations.insert(
                relation_id,
                RelationScope {
                    columns,
                    indexes,
                    include_all_columns: false,
                    include_all_indexes: false,
                },
            );
        }
        Self { relations }
    }

    /// Returns a stable structural cache key for this scope.
    pub fn key(&self) -> QueryImageScopeKey {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"bumbledb.query_image_scope.v1");
        for (relation_id, scope) in &self.relations {
            hasher.update(&relation_id.0.to_be_bytes());
            hasher.update(&[u8::from(scope.include_all_columns)]);
            hasher.update(&[u8::from(scope.include_all_indexes)]);
            hasher.update(&(scope.columns.len() as u64).to_be_bytes());
            for field in &scope.columns {
                hasher.update(&field.0.to_be_bytes());
            }
            hasher.update(&(scope.indexes.len() as u64).to_be_bytes());
            for index in &scope.indexes {
                hasher.update(&index.0.to_be_bytes());
            }
        }
        QueryImageScopeKey(*hasher.finalize().as_bytes())
    }

    pub(super) fn relation_scope(&self, relation: RelationId) -> Option<&RelationScope> {
        self.relations.get(&relation)
    }

    pub(super) fn relation_ids(&self) -> impl Iterator<Item = RelationId> + '_ {
        self.relations.keys().copied()
    }
}
