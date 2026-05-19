//! Compiled storage schema and public storage descriptors.

use bumbledb_core::schema::{
    CurrentIndexLayout, IndexComponent, IndexKind, RelationDescriptor, SchemaDescriptor, ValueType,
};

use crate::{AccessId, Error, FieldId, RelationId, Result};

/// Compiled storage schema for the LMDB write/read layer.
#[derive(Clone, Debug)]
pub struct StorageSchema {
    pub(crate) descriptor: SchemaDescriptor,
    pub(crate) layouts: Vec<CurrentIndexLayout>,
}

/// Bulk ETL load report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkLoadReport {
    /// Number of logical rows inserted.
    pub rows_inserted: usize,
    /// Storage transaction ID after the bulk load committed.
    pub storage_tx_id: u64,
    /// Number of interned dictionary values after the load committed.
    pub dictionary_entries: usize,
}

/// Durable relation segment metadata visible to query-image builders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentDescriptor {
    /// Relation this segment belongs to.
    pub relation: RelationId,
    /// Monotonic segment ID within the relation.
    pub segment_id: u64,
    /// Inclusive storage transaction ID where this segment becomes visible.
    pub tx_start: u64,
    /// Exclusive storage transaction ID where this segment stops being visible.
    pub tx_end: Option<u64>,
    /// Number of rows represented by this segment.
    pub row_count: usize,
    /// Encoded fixed-width column chunks.
    pub columns: Vec<ColumnSegmentDescriptor>,
    /// Encoded index chunks.
    pub indexes: Vec<IndexSegmentDescriptor>,
}

/// Durable encoded column chunk descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnSegmentDescriptor {
    /// Field represented by this column chunk.
    pub field: FieldId,
    /// Logical value type.
    pub value_type: ValueType,
    /// Fixed encoded width.
    pub width: usize,
    /// LMDB key containing contiguous encoded column bytes.
    pub lmdb_key: Vec<u8>,
    /// Stored byte length.
    pub byte_len: usize,
}

/// Durable encoded index chunk descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexSegmentDescriptor {
    /// Access path represented by this index chunk.
    pub access: AccessId,
    /// Leading fields in index order.
    pub fields: Vec<FieldId>,
    /// Index access kind.
    pub kind: IndexKind,
    /// LMDB key containing encoded index bytes.
    pub lmdb_key: Vec<u8>,
    /// Stored byte length.
    pub byte_len: usize,
    /// Lightweight index statistics summary.
    pub stats: IndexStatsSummary,
}

/// Durable index segment statistics summary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IndexStatsSummary {
    /// Number of encoded entries in the index segment.
    pub row_count: usize,
    /// Number of leading fields represented by this index.
    pub depth: usize,
    /// Stored index chunk bytes.
    pub byte_len: usize,
}

impl StorageSchema {
    /// Builds storage metadata and validates generated index key lengths.
    pub fn new(descriptor: SchemaDescriptor, max_key_size: usize) -> Result<Self> {
        descriptor.validate()?;
        let layouts = descriptor.current_index_layouts(max_key_size)?;
        Ok(Self {
            descriptor,
            layouts,
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
        self.descriptor
            .relations
            .iter()
            .enumerate()
            .find(|(_, relation)| relation.name == name)
            .map(|(id, relation)| (id as u16, relation))
            .ok_or_else(|| Error::unknown_relation(name))
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
        self.layouts
            .iter()
            .find(|layout| layout.relation_name == relation && layout.index_name == index)
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
