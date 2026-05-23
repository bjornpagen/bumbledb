use std::collections::BTreeMap;
use std::time::Instant;

use bumbledb_core::schema::RelationDescriptor;

use crate::query_image::columns::{ColumnImage, encoded_column_builders, finish_column_builders};
use crate::query_image::scope::RelationScope;
use crate::query_image::{
    FieldId, FieldImage, QueryImage, QueryImageScope, RelationAccessComponent, RelationId,
    RelationImage, RelationIndexImage, RelationStats,
};
use crate::storage_schema::FACT_SET_ACCESS_NAME;
use crate::{AccessId, Error, ReadTxn, Result, StorageSchema};

/// Builder for immutable query images.
pub struct QueryImageBuilder<'a, 'env> {
    txn: &'a ReadTxn<'env>,
    schema: &'a StorageSchema,
    scope: QueryImageScope,
}

impl<'a, 'env> QueryImageBuilder<'a, 'env> {
    /// Creates a builder over one read snapshot.
    pub fn new(txn: &'a ReadTxn<'env>, schema: &'a StorageSchema, scope: QueryImageScope) -> Self {
        Self { txn, schema, scope }
    }

    /// Builds the query image.
    pub fn build(self) -> Result<QueryImage> {
        let _span = tracing::debug_span!("bumbledb.query_image.build").entered();
        let start = Instant::now();
        let tx_id = self.txn.last_committed_tx_id()?;
        let mut relations = BTreeMap::new();
        for relation_id in self.scope.relation_ids() {
            let relation = self
                .schema
                .descriptor()
                .relations
                .get(relation_id.0 as usize)
                .ok_or_else(|| Error::internal("query image scope relation out of bounds"))?;
            let relation_scope = self
                .scope
                .relation_scope(relation_id)
                .ok_or_else(|| Error::internal("query image relation scope missing"))?;
            let built = RelationImageBuilder::new(
                self.txn,
                self.schema,
                relation_id,
                relation,
                relation_scope.clone(),
            )
            .build()?;
            relations.insert(relation_id, built.relation);
        }
        Ok(QueryImage::new(
            self.schema,
            tx_id,
            self.scope,
            relations,
            start.elapsed().as_micros(),
        ))
    }
}

struct BuiltRelationImage {
    relation: RelationImage,
}

struct RelationImageBuilder<'a, 'env, 'schema> {
    txn: &'a ReadTxn<'env>,
    schema: &'schema StorageSchema,
    relation_id: RelationId,
    relation: &'schema RelationDescriptor,
    scope: RelationScope,
}

impl<'a, 'env, 'schema> RelationImageBuilder<'a, 'env, 'schema> {
    fn new(
        txn: &'a ReadTxn<'env>,
        schema: &'schema StorageSchema,
        relation_id: RelationId,
        relation: &'schema RelationDescriptor,
        scope: RelationScope,
    ) -> Self {
        Self {
            txn,
            schema,
            relation_id,
            relation,
            scope,
        }
    }

    fn build(self) -> Result<BuiltRelationImage> {
        let _span = tracing::trace_span!(
            "bumbledb.query_image.relation",
            relation = %self.relation.name,
        )
        .entered();
        self.build_from_current_access()
    }

    fn build_from_current_access(self) -> Result<BuiltRelationImage> {
        let fields = self.field_images();
        let mut builders = encoded_column_builders(&fields, 0)?;
        let layout = self
            .schema
            .fact_set_layout(&self.relation.name)
            .ok_or_else(|| Error::unknown_index(&self.relation.name, FACT_SET_ACCESS_NAME))?;
        let component_by_field = layout
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| (component.field_name.as_str(), index))
            .collect::<BTreeMap<_, _>>();

        let fact_set_access = self
            .schema
            .fact_set_index_name(&self.relation.name)
            .ok_or_else(|| Error::unknown_index(&self.relation.name, FACT_SET_ACCESS_NAME))?;
        let scan = self.txn.scan_encoded_access_prefix(
            self.schema,
            &self.relation.name,
            fact_set_access,
            &[],
        )?;
        let mut fact_count = 0usize;
        for item in scan {
            let item = item?;
            fact_count = fact_count
                .checked_add(1)
                .ok_or_else(|| Error::internal("query image fact count overflow"))?;
            for field_image in &fields {
                let field = self
                    .relation
                    .fields
                    .get(field_image.id.0 as usize)
                    .ok_or_else(|| Error::corrupt("query image field id out of bounds"))?;
                let component_index = *component_by_field
                    .get(field.name.as_str())
                    .ok_or_else(|| Error::corrupt("query image missing access component"))?;
                let bytes = item
                    .component(&layout.components, component_index)
                    .ok_or_else(|| Error::corrupt("query image access component missing"))?;
                builders
                    .get_mut(&field_image.id)
                    .ok_or_else(|| Error::corrupt("query image column builder missing"))?
                    .append_bytes(bytes)?;
            }
        }

        if fact_count > u32::MAX as usize {
            return Err(Error::internal(
                "query image fact count exceeds FactId width",
            ));
        }
        for builder in builders.values() {
            if builder.len() != fact_count {
                return Err(Error::corrupt("query image column length mismatch"));
            }
        }
        let columns = finish_column_builders(builders);
        let encoded_column_bytes = columns.values().map(ColumnImage::byte_len).sum();
        let indexes = self
            .schema
            .layouts_for_relation(self.relation_id.0)
            .filter(|layout| {
                self.scope.include_all_indexes
                    || self.scope.indexes.contains(&AccessId(layout.index_id))
            })
            .map(|layout| {
                let mut bytes = Vec::new();
                let scan = self.txn.scan_encoded_access_prefix(
                    self.schema,
                    &self.relation.name,
                    &layout.index_name,
                    &[],
                )?;
                for item in scan {
                    bytes.extend_from_slice(item?.key());
                }
                let prefix_len = 1 + 2 + 2;
                let mut offset = prefix_len;
                let components = layout
                    .components
                    .iter()
                    .map(|component| {
                        let Some(field) = self
                            .relation
                            .fields
                            .iter()
                            .position(|field| field.name == component.field_name)
                            .map(|field| FieldId(field as u16))
                        else {
                            return Ok(None);
                        };
                        let image_component = RelationAccessComponent {
                            field,
                            offset,
                            width: component.encoded_width,
                        };
                        offset += component.encoded_width;
                        Ok(Some(image_component))
                    })
                    .collect::<Result<Option<Vec<_>>>>()?;
                let Some(components) = components else {
                    return Ok(None);
                };
                Ok(Some(RelationIndexImage {
                    access: AccessId(layout.index_id),
                    fields: layout
                        .leading_fields
                        .iter()
                        .filter_map(|field| {
                            self.relation
                                .fields
                                .iter()
                                .position(|relation_field| relation_field.name == *field)
                                .map(|field_id| FieldId(field_id as u16))
                        })
                        .collect(),
                    components,
                    encoded_len: layout.encoded_len,
                    prefix_len,
                    bytes,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let loaded_field_count = fields.len();
        Ok(BuiltRelationImage {
            relation: RelationImage {
                id: self.relation_id,
                name: self.relation.name.clone(),
                fact_count,
                fields,
                columns,
                indexes,
                stats: RelationStats {
                    fact_count,
                    field_count: loaded_field_count,
                    encoded_column_bytes,
                },
            },
        })
    }

    fn field_images(&self) -> Vec<FieldImage> {
        self.relation
            .fields
            .iter()
            .enumerate()
            .filter(|(field_id, _)| {
                self.scope.include_all_columns
                    || self.scope.columns.contains(&FieldId(*field_id as u16))
            })
            .map(|(field_id, field)| FieldImage {
                id: FieldId(field_id as u16),
                name: field.name.clone(),
                value_type: field.value_type.clone(),
                width: field.value_type.encoded_width(),
            })
            .collect()
    }
}
