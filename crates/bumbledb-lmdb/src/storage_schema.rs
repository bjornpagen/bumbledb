//! Compiled storage schema and public storage descriptors.

use std::collections::BTreeMap;

use bumbledb_core::schema::{
    CurrentIndexLayout, IndexComponent, IndexKind, RelationDescriptor, SchemaDescriptor,
};

use crate::{AccessId, Error, RelationId, Result};

/// Compiled storage schema for the LMDB write/read layer.
#[derive(Clone, Debug)]
pub struct StorageSchema {
    pub(crate) descriptor: SchemaDescriptor,
    pub(crate) layouts: Vec<CurrentIndexLayout>,
    relation_by_name: BTreeMap<String, RelationId>,
    layout_by_relation_name: BTreeMap<(String, String), AccessId>,
}

pub(crate) const TUPLE_SET_ACCESS_NAME: &str = "tuple_set";

/// Bulk ETL load report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkLoadReport {
    /// Number of rows newly inserted; exact duplicates are not counted.
    pub rows_inserted: usize,
    /// Storage transaction ID after the bulk load committed.
    pub storage_tx_id: u64,
    /// Number of interned dictionary values after the load committed.
    pub dictionary_entries: usize,
}

impl StorageSchema {
    /// Builds storage metadata and validates generated index key lengths.
    pub fn new(descriptor: SchemaDescriptor, max_key_size: usize) -> Result<Self> {
        descriptor.validate()?;
        let layouts = descriptor.current_index_layouts(max_key_size)?;
        let relation_by_name = descriptor
            .relations
            .iter()
            .enumerate()
            .map(|(id, relation)| (relation.name.clone(), RelationId(id as u16)))
            .collect();
        let layout_by_relation_name = layouts
            .iter()
            .map(|layout| {
                (
                    (layout.relation_name.clone(), layout.index_name.clone()),
                    AccessId(layout.index_id),
                )
            })
            .collect();
        Ok(Self {
            descriptor,
            layouts,
            relation_by_name,
            layout_by_relation_name,
        })
    }

    /// Returns the underlying schema descriptor.
    pub fn descriptor(&self) -> &SchemaDescriptor {
        &self.descriptor
    }

    /// Returns generated current index layouts.
    pub fn layouts(&self) -> &[CurrentIndexLayout] {
        &self.layouts
    }

    /// Returns planner-facing access paths for a relation.
    pub fn access_paths(&self, relation_name: &str) -> Result<Vec<AccessPathDescriptor>> {
        let (relation_id, _) = self.relation(relation_name)?;
        Ok(self
            .layouts_for_relation(relation_id)
            .map(AccessPathDescriptor::from_layout)
            .collect())
    }

    pub(crate) fn relation(&self, name: &str) -> Result<(u16, &RelationDescriptor)> {
        let relation_id = self
            .relation_by_name
            .get(name)
            .ok_or_else(|| Error::unknown_relation(name))?;
        let relation = self
            .descriptor
            .relations
            .get(relation_id.0 as usize)
            .ok_or_else(|| Error::unknown_relation(name))?;
        Ok((relation_id.0, relation))
    }

    pub(crate) fn layouts_for_relation(
        &self,
        relation_id: u16,
    ) -> impl Iterator<Item = &CurrentIndexLayout> {
        self.layouts
            .iter()
            .filter(move |layout| layout.relation_id == relation_id)
    }

    pub(crate) fn layout(&self, relation: &str, index: &str) -> Option<&CurrentIndexLayout> {
        self.layout_by_relation_name
            .get(&(relation.to_owned(), index.to_owned()))
            .and_then(|access| {
                self.layouts
                    .iter()
                    .find(|layout| layout.relation_name == relation && layout.index_id == access.0)
            })
    }

    pub(crate) fn tuple_set_layout(&self, relation: &str) -> Option<&CurrentIndexLayout> {
        self.layout(relation, TUPLE_SET_ACCESS_NAME)
    }

    pub(crate) fn tuple_set_index_name(&self, relation: &str) -> Option<&str> {
        self.tuple_set_layout(relation)
            .map(|layout| layout.index_name.as_str())
    }
}

/// Planner-facing access path descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPathDescriptor {
    /// Relation name.
    pub relation_name: String,
    /// Index name.
    pub index_name: String,
    /// Index kind.
    pub kind: IndexKind,
    /// Leading fields usable as an index prefix.
    pub leading_fields: Vec<String>,
    /// Full encoded components in index-key order.
    pub components: Vec<IndexComponent>,
}

impl AccessPathDescriptor {
    fn from_layout(layout: &CurrentIndexLayout) -> Self {
        Self {
            relation_name: layout.relation_name.clone(),
            index_name: layout.index_name.clone(),
            kind: layout.kind,
            leading_fields: layout.leading_fields.clone(),
            components: layout.components.clone(),
        }
    }
}
